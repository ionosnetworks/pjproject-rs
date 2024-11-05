use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    sync::{atomic::AtomicBool, Arc, Weak},
    time::Duration,
};

use itertools::Itertools;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use pjproject_rs as pj;

use crate::{Error, PJSIP};

/* Codec constants */
static INIT_PJLIB: Lazy<Result<(), Error>> = Lazy::new(init_pjlib);
static AUDIO_CODECS: Lazy<[Codec; 5]> = Lazy::new(|| {
    [
        Codec::new(0, c"PCMU", 8000, 1, 64000, 20, c"G.711 ULaw"),
        Codec::new(3, c"GSM", 8000, 1, 13200, 20, c"GSM"),
        Codec::new(4, c"G723", 8000, 1, 6400, 30, c"G.723.1"),
        Codec::new(8, c"PCMA", 8000, 1, 64000, 20, c"G.711 ALaw"),
        Codec::new(18, c"G729", 8000, 1, 8000, 20, c"G.729"),
    ]
});

type CallMap = HashMap<String, Option<Arc<Mutex<Call>>>>;

#[allow(dead_code)]
pub struct PjSip {
    pub pool: pj::PjPool,
    pub sip_endpt: Arc<pj::PjSipEndpoint>,
    pub sip_endpt_thread_running: Arc<AtomicBool>,
    pub udp_transport_hostport: pj::PjSipHostPort,
    pub sip_module: pj::PjSipModule,
    pub media_endpt: Arc<pj::PjMediaEndpt>,
    pub event_mgr: Arc<pj::PjMediaEventMgr>,
    pub calls: Arc<Mutex<CallMap>>,
}

pub fn get_sip() -> Result<&'static PjSip, Error> {
    PJSIP
        .get()
        .ok_or(Error::Validation("PjSip is not set".into()))
}

fn init_pjlib() -> Result<(), Error> {
    pj::pj_init().and_then(|_| pj::pjlib_util_init())?;

    Ok(())
}

pub fn init_sip<S: AsRef<CStr>>(
    local_addr: Option<S>,
    local_port: Option<u16>,
) -> Result<PjSip, Error> {
    let local_addr = local_addr.as_ref().map(|a| a.as_ref());
    let local_port = local_port.unwrap_or(5060);

    if let Err(err) = INIT_PJLIB.as_ref() {
        return Err(Error::Validation(format!(
            "pjlib was not initialized properly: {err}"
        )));
    }

    /* Must create a pool factory before we can allocate any memory. */
    let pool = pj::PjPool::default_with_name(c"nvr-ai");

    let hostname = pj::pj_gethostname();
    let sip_endpt = pj::PjSipEndpoint::new(pj::PjCachingPool::default(), hostname)?;

    /* Add UDP transport. */
    let udp_transport_hostport = {
        let addr = match &local_addr {
            Some(host) => pj::PjSockaddrIn::new(Some(&host), local_port)
                .map_err(|err| Error::Validation(format!("Failed to create sockaddr_in: {err}")))?,
            None => {
                let mut addr = pj::PjSockaddrIn::default();
                addr.with_family(pj::AF::PJ_AF_INET)
                    .with_port(local_port)
                    .with_addr(0);

                addr
            }
        };

        let a_name = local_addr
            .as_ref()
            .map(|u| pj::PjSipHostPort::new(u, local_port));

        let a_name = a_name.as_ref().map(|a| a.as_ref());
        let tp = sip_endpt.udp_transport_start(addr.as_ref(), a_name, 1)?;

        let tp_local_name = tp.local_name();
        tracing::info!("SIP UDP listening on {tp_local_name}");

        tp_local_name.to_owned()
    };

    /*
     * Init transaction layer.
     * This will create/initialize transaction hash tables etc.
     */
    sip_endpt.init_tsx_layer_module()?;

    /*  Initialize UA layer. */
    sip_endpt.init_ua_module()?;

    /* Initialize 100rel support */
    sip_endpt.init_100rel_module()?;

    /*  Init invite session module. */
    {
        /* Init the callback for INVITE session: */
        let mut inv_cb = pj::PjSipInvCallback::default();
        inv_cb.with_on_state_changed(call_on_state_changed);
        inv_cb.with_on_media_update(call_on_media_update);

        /* Initialize invite session module:  */
        sip_endpt.init_inv_usage(&inv_cb)?;
    }

    let sip_endpt = Arc::new(sip_endpt);

    /* Register our module to receive incoming requests. */
    let mut sip_module = pj::PjSipModule::new(c"mod-siprtpapp")
        .map_err(|err| Error::Validation(format!("Failed to create pjsip module: {err}")))?;
    pj::PjSipEndpoint::register_module(sip_endpt.clone(), &mut sip_module)?;

    /*
     * Init media stack.
     */
    let mut media_endpt = pj::PjMediaEndpt::new(pj::PjCachingPool::default(), None, 1)?;
    media_endpt.init_g711_codec()?;

    let event_mgr = Arc::new(pj::PjMediaEventMgr::new(0)?);

    let sip_endpt2 = sip_endpt.clone();
    std::thread::spawn(move || {
        let timeout = pj::PjTimeVal::new(0, 10);
        loop {
            let _ = sip_endpt2.handle_events(&timeout);
        }
    });

    Ok(PjSip {
        pool,
        sip_endpt,
        sip_endpt_thread_running: Arc::new(AtomicBool::new(true)),
        udp_transport_hostport,
        sip_module,
        media_endpt: Arc::new(media_endpt),
        event_mgr,
        calls: Arc::new(Mutex::new(HashMap::default())),
    })
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Codec {
    pt: u32, // payload type: eg PCMU, G711 ...
    name: CString,
    clock_rate: u32,
    channel: u32,
    bit_rate: u32,
    ptime: u32, // audio frame time
    description: CString,
}

impl Codec {
    pub fn new<S: AsRef<CStr>, T: AsRef<CStr>>(
        pt: u32,
        name: S,
        clock_rate: u32,
        channel: u32,
        bit_rate: u32,
        ptime: u32,
        description: T,
    ) -> Self {
        Self {
            pt,
            name: name.as_ref().into(),
            clock_rate,
            channel,
            ptime,
            bit_rate,
            description: description.as_ref().into(),
        }
    }
}

/* A bidirectional media stream created when the call is active. */
pub struct MediaStream {
    pub transport: pj::PjMediaTransport<Option<()>>,
    pub tx_quit: flume::Sender<()>,
    pub rx_quit: flume::Receiver<()>,
    pub thread: Option<std::thread::JoinHandle<()>>,
}

impl MediaStream {
    pub fn new(
        transport: pj::PjMediaTransport<Option<()>>,
        tx_quit: flume::Sender<()>,
        rx_quit: flume::Receiver<()>,
    ) -> Self {
        Self {
            transport,
            tx_quit,
            rx_quit,
            thread: None,
        }
    }

    pub fn destroy(&mut self) -> Result<(), Error> {
        if let Some(thread) = self.thread.take() {
            let _ = self.tx_quit.send(());
            if let Err(_) = thread.join() {
                return Err(Error::Validation(
                    "Failed to join media stream thread".into(),
                ));
            }
        }
        self.transport.detach(None)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum Msg {
    InvState(pj::PjSipInvState),
    Media(Result<(), Error>),
}

type ModDataT = Weak<Mutex<Call>>;
pub struct Call {
    pub id: String,
    pub inv: Option<Arc<Mutex<pj::PjSipInvSession<ModDataT>>>>,
    pub media: Arc<Mutex<MediaStream>>,
    pub wav_strmr: Option<pj::WavStreamer>,
    pub tx: flume::Sender<Msg>,
    pub rx: flume::Receiver<Msg>,
    pub timescale: u32,
}

impl Call {
    pub fn make_call<S: AsRef<str>, T: AsRef<CStr>>(
        call_id: S,
        timescale: u32,
        dst_uri: T,
    ) -> Result<Arc<Mutex<Call>>, Error> {
        let sip = get_sip()?;
        let call_id = call_id.as_ref().to_string();
        let (tx_quit, rx_quit) = flume::bounded(1);
        static RTP_START_PORT: u16 = 4000;

        /* RTP port counter */
        let rtp_port = RTP_START_PORT & 0xFFFE;
        let mut k = 0;
        let mut err_status;
        let call_media = loop {
            tracing::info!(
                "creating media transport on host: {}, port: {}",
                sip.udp_transport_hostport.host().to_string_lossy(),
                rtp_port + k
            );
            match pj::PjMediaTransport::builder()
                .name(c"siprtp")
                .addr(sip.udp_transport_hostport.host())
                .port(rtp_port + k)
                .options(0)
                .build(&sip.media_endpt)
            {
                Ok(transport) => {
                    break MediaStream::new(transport, tx_quit.clone(), rx_quit);
                }
                Err(err) => err_status = Some(err),
            }

            if k >= 100 {
                if let Some(err) = err_status {
                    return Err(Error::PjError(err));
                }
            }

            k += 1;
        };

        let ua = pj::PjSipUserAgentRef::pjsip_ua_instance()?;
        let local_uri = CString::new(format!("sip:{}", sip.udp_transport_hostport)).unwrap();
        let mut dialog = pj::PjSipDialog::new(ua, &local_uri, &local_uri, dst_uri.as_ref(), dst_uri.as_ref())?;

        let sdp = create_sdp(&call_media.transport, sip.udp_transport_hostport.host())?;

        /* Create the INVITE session. */
        let mut inv_sess = pj::PjSipInvSession::create_uac(&mut dialog, &sdp, 0)?;

        /* Attach call data to invite session */
        let mod_id = sip.sip_module.id();
        if mod_id.is_negative() {
            return Err(Error::Validation(format!(
                "Did not register sip module: {mod_id}"
            )));
        }

        let (tx, rx) = flume::unbounded();
        let call = Call {
            id: call_id.clone(),
            inv: None,
            media: Arc::new(Mutex::new(call_media)),
            tx,
            rx: rx.clone(),
            wav_strmr: None,
            timescale,
        };
        let call = Arc::new(Mutex::new(call));
        inv_sess.insert_mod_data(mod_id as usize, Arc::downgrade(&call));
        let inv_sess = Arc::new(Mutex::new(inv_sess));
        {
            let mut call = call.lock();
            call.inv.replace(inv_sess.clone());
        }
        {
            let mut inv_sess = inv_sess.lock();
            /* Create initial INVITE request.
             * This INVITE request will contain a perfectly good request and
             * an SDP body as well.
             */
            let mut tdata = inv_sess.create_invite_req()?;

            /* Send initial INVITE request.
             * From now on, the invite session's state will be reported to us
             * via the invite session callbacks.
             */
            inv_sess.send_msg(&mut tdata)?;
        }

        loop {
            match rx.recv_timeout(Duration::from_secs(30)) {
                Ok(msg) => match msg {
                    Msg::InvState(state) => {
                        tracing::info!(call_id = call_id, "Call state: [{state}]");
                        match state {
                            pjproject_rs::PjSipInvState::Disconnected => {
                                return Err(Error::Validation("Call disconnected".into()))
                            }
                            pjproject_rs::PjSipInvState::Unknown => {
                                return Err(Error::Validation("Call got as unknown state".into()))
                            }
                            _ => (),
                        }
                    }
                    Msg::Media(res) => {
                        let _ = res?;
                        break;
                    }
                },
                Err(_) => return Err(Error::Validation("Calling timeout".into())),
            }
        }

        Ok(call)
    }

    pub fn hang_up(&mut self) -> Result<(), Error> {
        if let Some(inv) = self.inv.clone() {
            inv.lock().end_session()?;
        }

        Ok(())
    }
}

pub fn create_sdp<T, S: AsRef<CStr>>(
    transport: &pj::PjMediaTransport<T>,
    local_addr: S,
) -> Result<pj::PjMediaSdpSession, Error> {
    /* Get transport info */
    let tpinfo = transport.info()?;

    let sdp_conn = pj::PjMediaSdpConn::new(c"IN", c"IP4", local_addr.as_ref(), 0, 0);
    /* Add format and rtpmap for each codec. */
    let mut attrs = AUDIO_CODECS
        .iter()
        .map(|codec| {
            pj::PjMediaSdpRtpMap::new(
                &CString::new(codec.pt.to_string()).unwrap(),
                &codec.name,
                codec.clock_rate,
                if codec.channel == 1 {
                    None::<CString>
                } else {
                    Some(CString::new(codec.channel.to_string()).unwrap())
                },
            )
            .to_attr()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut desc_fmt = AUDIO_CODECS
        .iter()
        .map(|codec| CString::new(codec.pt.to_string()).unwrap())
        .collect_vec();

    /* Add sendrecv attribute. */
    let sendrecv_attr = pj::PjMediaSdpAttr::new(c"sendrecv", c"");

    /*
     * Add support telephony event
     */
    desc_fmt.push(c"121".to_owned());

    /* Add rtpmap. */
    let rtpmap2 = pj::PjMediaSdpAttr::new(c"rtpmap", c"121 telephone-event/8000");

    /* Add fmtp */
    let fmtp = pj::PjMediaSdpAttr::new(c"fmtp", c"121 0-15");

    attrs.extend([sendrecv_attr, rtpmap2, fmtp]);
    let sdp_media = pj::PjMediaSdpMedia::new(
        pj::PjMediaSdpMediaType::Audio,
        c"RTP/AVP",
        tpinfo.rtp_ipv4_port(),
        1,
        &desc_fmt,
        attrs,
    );

    let sdp = pj::PjMediaSdpSession::builder()
        .origin_user(c"pjsip-siprtp")
        .sdp_conn(sdp_conn)
        .sdp_media(vec![sdp_media])
        .build()?;

    Ok(sdp)
}

fn get_call(inv: &pj::PjSipInvSession<ModDataT>) -> Result<Arc<Mutex<Call>>, Error> {
    let sip = get_sip()?;

    let module_id = sip.sip_module.id();
    if module_id.is_negative() {
        return Err(Error::Validation(
            "Sip module has a negative id, may not have been registered".into(),
        ));
    }

    let call = match inv.get_mod_data(module_id as usize) {
        Some(c) => match Weak::upgrade(&*c) {
            Some(c) => c,
            None => {
                return Err(Error::Validation(
                    "Call data not found, may have been dropped".into(),
                ))
            }
        },
        None => {
            return Err(Error::Validation(
                "Failed to retrieve call data from invite session, may not have been inserted"
                    .into(),
            ));
        }
    };

    Ok(call)
}

fn call_on_state_changed(inv: &mut pj::PjSipInvSession<ModDataT>, _evt: &mut pj::PjSipEvent) {
    let inv_name = inv.obj_name().to_string_lossy().into_owned();

    let call = match get_call(inv) {
        Ok(c) => c,
        Err(err) => {
            tracing::error!(
                inv_name = inv_name,
                "Failed to get call data from invite session: {err}"
            );
            return;
        }
    };
    let (call_id, call_tx) = {
        let call = call.lock();
        (call.id.clone(), call.tx.clone())
    };

    if let Err(_err) = call_tx.send(Msg::InvState(inv.get_state())) {
        tracing::error!(
            inv_name = inv_name,
            call_id = call_id,
            "Failed to send call state change call recevier"
        );
        return;
    }
}

fn call_on_media_update(inv: &pj::PjSipInvSession<ModDataT>, status: pj::PjStatus) {
    let inv_name = inv.obj_name().to_string_lossy().into_owned();
    let call = match get_call(inv) {
        Ok(c) => c,
        Err(err) => {
            tracing::error!(
                inv_name = inv_name,
                "Failed to get call data from invite session: {err}"
            );
            return;
        }
    };

    let (call_id, call_tx) = {
        let call = call.lock();
        (call.id.clone(), call.tx.clone())
    };

    let call_media = call.lock().media.clone();
    /* If this is a mid-call media update, then destroy existing media */
    {
        let mut media = call_media.lock();
        if media.thread.is_some() {
            if let Err(err) = media.destroy() {
                tracing::error!("error destroying media thread: {err}");
                return;
            }
        }
    }

    if status.is_err() {
        if let Err(_err) = call_tx.send(Msg::Media(Err(pj::Error::PjError(status).into()))) {
            tracing::error!(
                inv_name = inv_name,
                call_id = call_id,
                "Failed to send call media error status: {status}",
            );
        }
        return;
    }

    let setup = || {
        let sip = get_sip()?;
        let mut media_endpt = sip.media_endpt.clone();
        /* Capture stream definition from the SDP */
        let local_sdp = inv.get_active_local_neg_sdp()?;
        let remote_sdp = inv.get_active_remote_neg_sdp()?;

        let local_sdp_media = local_sdp.media();
        if local_sdp_media.is_empty() || local_sdp_media[0].desc_port() == 0 {
            return Err(Error::Validation("Audio media inactive".into()));
        }

        let stream_idx = 0;
        let mut si =
            inv.stream_info_from_sdp(&mut media_endpt, &local_sdp, &remote_sdp, stream_idx)?;
        // pjmedia_silence_det_apply in silencedet.c is causing arithmetic errors leading to crashes
        // so disable it
        si.disable_vad();

        let stream = {
            let mut media = call_media.lock();
            /* Attach media to transport */
            media.transport.attach(
                None,
                &pj::SockaddrTRef::IPv4(si.rem_addr().into()),
                Some(&pj::SockaddrTRef::IPv4(si.rem_rtcp().into())),
                None,
                None,
            )?;

            /* Start media transport */
            media.transport.start()?;

            let stream = pj::PjMediaStream::new(&sip.media_endpt, &si, &mut media.transport)?;

            stream
        };

        let wav_strmr = {
            let mut call = call.lock();
            match &call.wav_strmr {
                Some(w) => w.clone(),
                None => {
                    let wav_strmr = pj::WavStreamer::builder()
                        .timescale(call.timescale)
                        .buf_size_multiplier(5)
                        .build()
                        .unwrap();
                    call.wav_strmr.replace(wav_strmr.clone());

                    wav_strmr
                }
            }
        };

        let call_media2 = call_media.clone();
        let wav_strmr2 = wav_strmr.clone();
        let handle = std::thread::spawn(move || media_thread(call_media2, stream, wav_strmr2));

        call_media.lock().thread.replace(handle);

        Ok::<_, Error>(())
    };

    if let Err(_err) = call_tx.send(Msg::Media(setup())) {
        tracing::error!(
            inv_name = inv_name,
            call_id = call_id,
            "Failed to send call media result msg",
        );
    }
}

fn media_thread(
    media: Arc<Mutex<MediaStream>>,
    stream: pj::PjMediaStream,
    wav_strmr: pj::WavStreamer,
) {
    let quit = media.lock().rx_quit.clone();

    while !wav_strmr.initialized() {
        if let Ok(_) = quit.try_recv() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    let mut stream = stream;
    let mut wav_strmr = wav_strmr;
    let mut attach_stream_to_wav = || {
        let mut stream_port = stream.get_port()?;

        let stream_clock_rate = stream_port.clock_rate();
        let stream_channel_cnt = stream_port.channel_count();
        let stream_frame_time_usec = stream_port.frame_time_usec();
        println!("stream_clock_rate: {stream_clock_rate}, stream_channel_cnt: {stream_channel_cnt}, stream_frame_time_usec: {stream_frame_time_usec}");

        let wav_channel_cnt = wav_strmr.channel_count();
        let wav_clock_rate = wav_strmr.clock_rate();
        let wav_frame_time_usec = wav_strmr.frame_time_usec();
        println!("wav_clock_rate: {wav_clock_rate}, wav_channel_cnt: {wav_channel_cnt}, wav_frame_time_usec: {wav_frame_time_usec}");

        if stream_channel_cnt != wav_channel_cnt {
            wav_strmr.add_stereo_port(stream_channel_cnt, 0)?;
        }

        if stream_clock_rate != wav_clock_rate {
            wav_strmr.add_resample_port(stream_clock_rate, 0)?;
        }

        let wav_clock_rate = wav_strmr.clock_rate();
        let wav_channel_cnt = wav_strmr.channel_count();
        let wav_frame_time_usec = wav_strmr.frame_time_usec();
        println!("wav_clock_rate: {wav_clock_rate}, wav_channel_cnt: {wav_channel_cnt}, wav_frame_time_usec: {wav_frame_time_usec}");

        let mut master_port =
            pj::PjMediaMasterPort::new(&mut stream_port, &mut wav_strmr.get_port(), 0)?;

        master_port.start()?;
        stream.start()?;

        Ok::<_, Error>(master_port)
    };

    let mut master_port = match attach_stream_to_wav() {
        Ok(m) => m,
        Err(err) => {
            tracing::error!("Failed to connect call stream to wav_strmr: {err}");
            return;
        }
    };

    let _ = quit.recv();

    if let Err(err) = master_port.stop() {
        tracing::error!("Failed to pjmedia_master_port_stop: {err}");
        return;
    }
}
