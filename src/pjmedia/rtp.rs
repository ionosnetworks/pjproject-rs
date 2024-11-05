use bytes::BytesMut;
use pjproject_sys as pj;

use crate::{Error, PjStatus};

pub struct PjMediaRtpSession {
    rtp_session: pj::pjmedia_rtp_session,
}

impl PjMediaRtpSession {
    pub fn new(default_pt: i32, sender_ssrc: u32) -> Result<Self, Error> {
        let mut rtp_session = unsafe { std::mem::zeroed() };
        let status =
            unsafe { pj::pjmedia_rtp_session_init(&mut rtp_session, default_pt, sender_ssrc) };

        PjStatus::result_for_status(status).map(|_| Self { rtp_session })
    }
}

pub struct RtpPacket(pub BytesMut);

impl RtpPacket {
    pub fn new(
        sess: &mut PjMediaRtpSession,
        pt: u32,
        m: u32,
        ts_len: u32,
        payload: &[u8],
    ) -> Result<Self, Error> {
        let mut p_hdr = unsafe { std::mem::zeroed() };
        let mut hdrlen = 0;

        /* Format RTP header */
        let status = unsafe {
            pj::pjmedia_rtp_encode_rtp(
                &mut sess.rtp_session,
                pt as _,
                m as _,
                payload.len() as _, // bytes_per_frame: Payload length in bytes.
                ts_len as _,        // samples_per_frame: Timestamp length.
                &mut p_hdr,
                &mut hdrlen,
            )
        };

        if hdrlen <= 0 {
            return Err(Error::Validation(format!("invalid rtp hdrlen: {hdrlen}")));
        }

        let hdrlen = hdrlen as usize;

        let hdr = p_hdr as *const u8;
        let mut packet = BytesMut::zeroed(hdrlen + payload.len());

        /* Copy RTP header to packet */
        unsafe { std::ptr::copy_nonoverlapping(hdr, packet.as_mut_ptr(), hdrlen) };

        /* Copy payload to packet */
        packet[hdrlen..].copy_from_slice(payload);

        PjStatus::result_for_status(status).map(|_| Self(packet))
    }
}
