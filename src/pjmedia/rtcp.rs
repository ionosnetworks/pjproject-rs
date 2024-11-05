use std::ffi::{CStr, CString};

use bytes::BytesMut;
use pjproject_sys as pj;

use crate::Error;

pub struct PjMediaRtcpSession {
    rtcp_session: pj::pjmedia_rtcp_session,
}

unsafe impl Send for PjMediaRtcpSession {}
unsafe impl Sync for PjMediaRtcpSession {}

impl PjMediaRtcpSession {
    pub fn new<S: AsRef<CStr>>(
        name: S,
        clock_rate: u32,
        samples_per_frame: u32,
        ssrc: u32,
    ) -> Self {
        let name = name.as_ref().to_owned().into_raw();
        let mut rtcp_session = unsafe { std::mem::zeroed() };
        unsafe {
            pj::pjmedia_rtcp_init(&mut rtcp_session, name, clock_rate, samples_per_frame, ssrc);
        };

        Self { rtcp_session }
    }

    pub fn rtcp_tx_rtp(&mut self, ptsize: usize) {
        unsafe { pj::pjmedia_rtcp_tx_rtp(&mut self.rtcp_session, ptsize as _) };
    }

    pub fn build_rtcp(&mut self) -> Result<RtcpPacket, Error> {
        let mut rtcp_pkt = unsafe { std::mem::zeroed() };
        let mut rtcp_len = 0;

        /* Build RTCP packet */
        unsafe {
            pj::pjmedia_rtcp_build_rtcp(&mut self.rtcp_session, &mut rtcp_pkt, &mut rtcp_len)
        };

        if rtcp_len <= 0 {
            return Err(Error::Validation(format!(
                "Invalid rtcp packet: {rtcp_len}"
            )));
        }

        let mut b = BytesMut::with_capacity(rtcp_len as _);
        b.extend_from_slice(unsafe {
            std::slice::from_raw_parts(rtcp_pkt as *const u8, rtcp_len as _)
        });

        Ok(RtcpPacket(b))
    }
}

impl Drop for PjMediaRtcpSession {
    fn drop(&mut self) {
        unsafe {
            let name = CString::from_raw(self.rtcp_session.name);
            drop(name);
        }
    }
}

pub struct RtcpPacket(pub BytesMut);
