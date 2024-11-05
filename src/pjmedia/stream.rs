use pjproject_sys as pj;

use crate::{
    Error, PjMediaEndpt, PjMediaSdpSessionRef, PjPool, PjSipInvSession, PjSockaddrRef, PjStatus,
};

use super::{PjMediaCodecRef, PjMediaDir, PjMediaPort, PjMediaTransport};

pub struct PjMediaStream {
    stream: *mut pj::pjmedia_stream,
    #[allow(dead_code)]
    pool: PjPool,
}

unsafe impl Send for PjMediaStream {}
unsafe impl Sync for PjMediaStream {}

impl PjMediaStream {
    pub fn new<T>(
        media_endpt: &PjMediaEndpt,
        stream_info: &PjMediaStreamInfo,
        transport: &mut PjMediaTransport<T>,
    ) -> Result<Self, Error> {
        let mut pool = PjPool::default_with_name(c"stream");
        let mut stream = unsafe { std::mem::zeroed() };
        let status = unsafe {
            pj::pjmedia_stream_create(
                media_endpt.as_mut_ptr(),
                pool.as_mut_ptr(),
                &stream_info.stream_info,
                transport.as_mut_ptr(),
                std::ptr::null_mut(),
                &mut stream,
            )
        };

        PjStatus::result_for_status(status).map(|_| Self { stream, pool })
    }

    pub fn as_ptr(&self) -> *const pj::pjmedia_stream {
        self.stream
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pjmedia_stream {
        self.stream
    }

    pub fn get_port(&self) -> Result<PjMediaPort, Error> {
        PjMediaPort::from_stream(self)
    }

    pub fn start(&mut self) -> Result<(), Error> {
        let status = unsafe { pj::pjmedia_stream_start(self.as_mut_ptr()) };

        PjStatus::result_for_status(status)
    }

    pub fn pause(&mut self, dir: PjMediaDir) -> Result<(), Error> {
        let status = unsafe { pj::pjmedia_stream_pause(self.as_mut_ptr(), dir.into()) };

        PjStatus::result_for_status(status)
    }
}

impl Drop for PjMediaStream {
    fn drop(&mut self) {
        let status = unsafe { pj::pjmedia_stream_destroy(self.as_mut_ptr()) };
        if let Err(err) = PjStatus::result_for_status(status) {
            tracing::error!("Failed to destroy stream: {err}");
        }
    }
}

pub struct PjMediaStreamInfo {
    pub stream_info: pj::pjmedia_stream_info,
}

unsafe impl Send for PjMediaStreamInfo {}
unsafe impl Sync for PjMediaStreamInfo {}

impl PjMediaStreamInfo {
    pub fn from_sdp<T>(
        inv_sess: &PjSipInvSession<T>,
        media_endpt: &PjMediaEndpt,
        local_sdp: &PjMediaSdpSessionRef,
        remote_sdp: &PjMediaSdpSessionRef,
        stream_idx: u32,
    ) -> Result<Self, Error> {
        let mut si = unsafe { std::mem::zeroed() };
        let status = unsafe {
            pj::pjmedia_stream_info_from_sdp(
                &mut si,
                inv_sess.as_ref().pool,
                media_endpt.as_mut_ptr(),
                local_sdp.as_ptr(),
                remote_sdp.as_ptr(),
                stream_idx,
            )
        };

        PjStatus::result_for_status(status).map(|_| Self { stream_info: si })
    }

    pub fn fmt<'a>(&'a self) -> PjMediaCodecRef<'a> {
        PjMediaCodecRef::from(&self.stream_info.fmt)
    }

    pub fn tx_pt(&self) -> u32 {
        self.stream_info.tx_pt
    }

    pub fn rem_addr<'a>(&'a self) -> PjSockaddrRef<'a> {
        PjSockaddrRef::from(&self.stream_info.rem_addr)
    }

    pub fn rem_rtcp<'a>(&'a self) -> PjSockaddrRef<'a> {
        PjSockaddrRef::from(&self.stream_info.rem_rtcp)
    }

    pub fn vad_enabled(&self) -> bool {
        unsafe { (*self.stream_info.param).setting.vad() != 0 }
    }

    pub fn disable_vad(&mut self) {
        unsafe {
            (*self.stream_info.param).setting.set_vad(0);
        }
    }

    pub fn enable_vad(&mut self) {
        unsafe {
            (*self.stream_info.param).setting.set_vad(1);
        }
    }
}
