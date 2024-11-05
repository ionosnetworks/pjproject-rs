use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
};

use parking_lot::{Mutex, MutexGuard};
use pjproject_sys as pj;

use crate::{Error, PjPool, PjStatus};

use super::PjMediaPort;

const DEFAULT_PTIME: u32 = 20;
const DEFAULT_BITS_PER_SAMPLE: u32 = 16;
const DEFAULT_CLOCK_RATE: u32 = 8000;
const DEFAULT_CHANNEL_COUNT: u32 = 1;
const DEFAULT_SAMPLES_PER_FRAME: u32 = 80;
const DEFAULT_BUF_SIZE_MULTIPLIER: usize = 3;

#[derive(Clone)]
pub struct WavStreamer {
    port: PjMediaPort,
    port_data: Arc<Mutex<Inner>>,
    initialized: Arc<AtomicBool>,
    pool: Arc<Mutex<PjPool>>,
}

unsafe impl Send for WavStreamer {}
unsafe impl Sync for WavStreamer {}

struct Inner {
    ptime: u32,
    buf: VecDeque<u8>,
    buf_size: usize,
    fmt_tag: u32,
    bytes_per_sample: u32,
    timescale: u32,
    buf_size_multiplier: usize,
}

impl WavStreamer {
    pub fn new(
        ptime: Option<u32>,
        buf_size: Option<usize>,
        timescale: u32,
        buf_size_multiplier: Option<usize>,
    ) -> Result<Self, Error> {
        let mut pool = PjPool::default_with_name(c"WavStreamer");

        let name = c"wav_streamer";
        let port = unsafe {
            pj::pj_pool_calloc(
                pool.as_mut_ptr(),
                1,
                std::mem::size_of::<pj::pjmedia_port>(),
            ) as *mut pj::pjmedia_port
        };

        let status = unsafe {
            pj::pjmedia_port_info_init(
                &mut (*port).info,
                &pj::pj_str(name.as_ptr() as *mut _),
                crate::fourcc_to_int(crate::PJMEDIA_SIG_PORT_WAV_PLAYER),
                DEFAULT_CLOCK_RATE,
                DEFAULT_CHANNEL_COUNT,
                DEFAULT_BITS_PER_SAMPLE,
                DEFAULT_SAMPLES_PER_FRAME,
            )
        };
        let _ = PjStatus::result_for_status(status as _)?;

        let ptime = ptime.unwrap_or(DEFAULT_PTIME);
        let buf_size = buf_size.unwrap_or(0);
        let buf_size_multiplier = buf_size_multiplier.unwrap_or(DEFAULT_BUF_SIZE_MULTIPLIER);

        let buf = VecDeque::with_capacity(buf_size);
        let port_data = Arc::new(Mutex::new(Inner {
            ptime,
            buf,
            buf_size,
            fmt_tag: 0,
            bytes_per_sample: 0,
            timescale,
            buf_size_multiplier,
        }));

        unsafe {
            (*port).get_frame = Some(Self::get_frame);
            (*port).port_data.pdata = Weak::into_raw(Arc::downgrade(&port_data)) as *mut _;
        }

        Ok(Self {
            port: PjMediaPort::from(port),
            port_data,
            initialized: Arc::new(AtomicBool::new(false)),
            pool: Arc::new(Mutex::new(pool)),
        })
    }

    pub fn clock_rate(&self) -> u32 {
        unsafe { self.port.as_ref().info.fmt.det.aud.clock_rate }
    }

    pub fn channel_count(&self) -> u32 {
        unsafe { self.port.as_ref().info.fmt.det.aud.channel_count }
    }

    pub fn bits_per_sample(&self) -> u32 {
        unsafe { self.port.as_ref().info.fmt.det.aud.bits_per_sample }
    }

    pub fn frame_time_usec(&self) -> u32 {
        unsafe { self.port.as_ref().info.fmt.det.aud.frame_time_usec }
    }

    pub fn initialized(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }

    pub fn get_port(&self) -> PjMediaPort {
        self.port.clone()
    }

    pub fn clear_buffer(&mut self) {
        self.port_data.lock().buf.clear();
    }

    pub fn add_stereo_port(
        &mut self,
        channel_cnt: u32,
        options: u32,
    ) -> Result<PjMediaPort, Error> {
        let port =
            PjMediaPort::stereo(&mut self.pool.lock(), &mut self.port, channel_cnt, options)?;
        self.port = port;

        Ok(self.port.clone())
    }

    pub fn add_resample_port(
        &mut self,
        clock_rate: u32,
        options: u32,
    ) -> Result<PjMediaPort, Error> {
        let port =
            PjMediaPort::resample(&mut self.pool.lock(), &mut self.port, clock_rate, options)?;
        self.port = port;

        Ok(self.port.clone())
    }

    pub fn initialize(&mut self, data: &[u8]) -> Result<(), Error> {
        let header_size = std::mem::size_of::<pj::pjmedia_wave_hdr>();
        if data.len() < header_size {
            return Err(Error::Validation("Invalid wav header".into()));
        }

        let mut port_data = self.port_data.lock();
        if !self.initialized.load(Ordering::SeqCst) {
            let wave_hdr = unsafe {
                let mut wave_hdr = std::mem::zeroed::<pj::pjmedia_wave_hdr>();
                std::ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    (&mut wave_hdr) as *const _ as *mut _,
                    std::mem::size_of_val(&wave_hdr),
                );

                /* Normalize WAVE header fields values from little-endian to host
                 * byte order.
                 */
                pj::pjmedia_wave_hdr_file_to_host(&mut wave_hdr);

                /* Validate WAVE data. */
                if wave_hdr.riff_hdr.riff != crate::fourcc_to_int(crate::PJMEDIA_RIFF_TAG)
                    || wave_hdr.riff_hdr.wave != crate::fourcc_to_int(crate::PJMEDIA_WAVE_TAG)
                    || wave_hdr.fmt_hdr.fmt != crate::fourcc_to_int(crate::PJMEDIA_FMT_TAG)
                    || wave_hdr.data_hdr.data != crate::fourcc_to_int(crate::PJMEDIA_DATA_TAG)
                {
                    return Err(Error::PjError(PjStatus::new(
                        pj::PJMEDIA_ENOTVALIDWAVE as _,
                    )));
                }

                /* Validate format and its attributes (i.e: bits per sample, block align) */
                let status: i32 = match wave_hdr.fmt_hdr.fmt_tag as u32 {
                    pj::pjmedia_wave_fmt_tag_PJMEDIA_WAVE_FMT_TAG_PCM => {
                        if wave_hdr.fmt_hdr.bits_per_sample != 16
                            || wave_hdr.fmt_hdr.block_align != 2 * wave_hdr.fmt_hdr.nchan
                        {
                            pj::PJMEDIA_EWAVEUNSUPP as _
                        } else {
                            0
                        }
                    }
                    pj::pjmedia_wave_fmt_tag_PJMEDIA_WAVE_FMT_TAG_ALAW
                    | pj::pjmedia_wave_fmt_tag_PJMEDIA_WAVE_FMT_TAG_ULAW => {
                        if wave_hdr.fmt_hdr.bits_per_sample != 8
                            || wave_hdr.fmt_hdr.block_align != wave_hdr.fmt_hdr.nchan
                        {
                            pj::PJMEDIA_ENOTVALIDWAVE as _
                        } else {
                            0
                        }
                    }
                    _ => pj::PJMEDIA_EWAVEUNSUPP as _,
                };
                let _ = PjStatus::result_for_status(status as _)?;

                wave_hdr
            };

            port_data.fmt_tag = wave_hdr.fmt_hdr.fmt_tag as _;
            port_data.bytes_per_sample = wave_hdr.fmt_hdr.bits_per_sample as u32 / 8;

            /* Update port info. */
            let name = self.port.as_ref().info.name;
            let samples_per_frame =
                port_data.ptime * wave_hdr.fmt_hdr.sample_rate * wave_hdr.fmt_hdr.nchan as u32
                    / 1000;

            let status = unsafe {
                pj::pjmedia_port_info_init(
                    &mut self.port.as_mut().info,
                    &name,
                    crate::fourcc_to_int(crate::PJMEDIA_SIG_PORT_WAV_PLAYER),
                    wave_hdr.fmt_hdr.sample_rate,
                    wave_hdr.fmt_hdr.nchan as _,
                    DEFAULT_BITS_PER_SAMPLE,
                    samples_per_frame,
                )
            };
            let _ = PjStatus::result_for_status(status as _)?;

            let bytes_per_timescale =
                (wave_hdr.fmt_hdr.bytes_per_sec / 1000 * port_data.timescale) as usize;
            if port_data.buf_size == 0 {
                port_data.buf_size = bytes_per_timescale * port_data.buf_size_multiplier;
                port_data.buf = VecDeque::with_capacity(port_data.buf_size);
            } else {
                if bytes_per_timescale > port_data.buf_size {
                    return Err(Error::Validation(format!(
                        "buf_size({}) is too small for data recevied every timescale of {}ms",
                        port_data.buf_size, port_data.timescale
                    )));
                }
            }

            let data_off = std::mem::size_of_val(&wave_hdr);
            let size_to_add = std::cmp::min(port_data.buf_size, data.len());
            port_data.buf.extend(&data[data_off..size_to_add]);

            self.initialized.store(true, Ordering::SeqCst);
        }

        Ok(())
    }

    pub fn add_data(&self, data: &[u8]) {
        let mut port_data = self.port_data.lock();

        let free_spc = port_data.buf_size - port_data.buf.len();
        if data.len() > free_spc {
            let size_to_remove = std::cmp::min(port_data.buf_size, data.len()) - free_spc;
            port_data.buf.drain(0..size_to_remove);
        }

        let size_to_add = std::cmp::min(port_data.buf_size, data.len());
        port_data.buf.extend(&data[0..size_to_add]);
        MutexGuard::unlock_fair(port_data);
    }

    unsafe extern "C" fn get_frame(
        port: *mut pj::pjmedia_port,
        frame: *mut pj::pjmedia_frame,
    ) -> i32 {
        let strmr = Weak::from_raw((*port).port_data.pdata as *const Mutex<Inner>);
        match strmr.upgrade() {
            Some(strmr) => {
                let (data, fmt_tag) = {
                    let mut strmr = strmr.lock();
                    let size_to_read = std::cmp::min((*frame).size, strmr.buf.len());
                    let data = strmr.buf.drain(0..size_to_read).collect::<Vec<_>>();
                    let fmt_tag = strmr.fmt_tag;

                    if size_to_read < (*frame).size {
                        MutexGuard::unlock_fair(strmr);
                    }

                    (data, fmt_tag)
                };
                (*frame).type_ = pj::pjmedia_frame_type_PJMEDIA_FRAME_TYPE_AUDIO;
                (*frame).timestamp.u64_ = 0;
                std::ptr::copy_nonoverlapping(data.as_ptr(), (*frame).buf as *mut _, data.len());
                if data.len() < (*frame).size {
                    std::ptr::write_bytes(
                        (*frame).buf.offset(data.len() as _) as *mut u8,
                        0,
                        (*frame).size - data.len(),
                    );
                }

                if fmt_tag == pj::pjmedia_wave_fmt_tag_PJMEDIA_WAVE_FMT_TAG_ULAW
                    || fmt_tag == pj::pjmedia_wave_fmt_tag_PJMEDIA_WAVE_FMT_TAG_ALAW
                {
                    let dst = ((*frame).buf as *mut u16).offset(((*frame).size - 1) as _);
                    let src = ((*frame).buf as *mut u8).offset(((*frame).size - 1) as _);

                    if fmt_tag == pj::pjmedia_wave_fmt_tag_PJMEDIA_WAVE_FMT_TAG_ULAW {
                        for i in 0..(*frame).size as isize {
                            let off = *src.offset(-1 * i) as usize;
                            *dst.offset(-1 * i) = pj::pjmedia_ulaw2linear_tab[off] as _;
                        }
                    } else {
                        for i in 0..(*frame).size as isize {
                            let off = *src.offset(-1 * i) as usize;
                            *dst.offset(-1 * i) = pj::pjmedia_alaw2linear_tab[off] as _;
                        }
                    }
                }
            }
            None => {
                std::mem::forget(strmr);
                (*frame).type_ = pj::pjmedia_frame_type_PJMEDIA_FRAME_TYPE_NONE;
                (*frame).size = 0;
                return crate::PJ_EEOF;
            }
        }

        std::mem::forget(strmr);
        pj::pj_constants__PJ_SUCCESS as _
    }

    pub fn builder() -> WavStreamerBuilder {
        WavStreamerBuilder::default()
    }
}

#[derive(Default)]
pub struct WavStreamerBuilder {
    ptime: Option<u32>,
    buf_size: Option<usize>,
    timescale: u32,
    buf_size_multiplier: Option<usize>,
}

impl WavStreamerBuilder {
    pub fn ptime(&mut self, ptime: u32) -> &mut Self {
        self.ptime.replace(ptime);
        self
    }

    pub fn buf_size(&mut self, buf_size: usize) -> &mut Self {
        self.buf_size.replace(buf_size);
        self
    }

    pub fn timescale(&mut self, timescale: u32) -> &mut Self {
        self.timescale = timescale;
        self
    }

    pub fn buf_size_multiplier(&mut self, buf_size_multiplier: usize) -> &mut Self {
        self.buf_size_multiplier.replace(buf_size_multiplier);
        self
    }

    pub fn build(&mut self) -> Result<WavStreamer, Error> {
        WavStreamer::new(
            self.ptime,
            self.buf_size,
            self.timescale,
            self.buf_size_multiplier,
        )
    }
}
