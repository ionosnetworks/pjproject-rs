use std::ffi::{CStr, CString};

use pjproject_sys as pj;

use crate::{Error, PjSipInvSession, PjStatus};

pub struct PjSipTxData {
    pjsip_tx_data: *mut pj::pjsip_tx_data,
}

impl PjSipTxData {
    pub fn inv_invite<T>(inv_sess: &PjSipInvSession<T>) -> Result<PjSipTxData, Error> {
        let mut pjsip_tx_data = std::ptr::null_mut();
        let status = unsafe { pj::pjsip_inv_invite(inv_sess.as_mut_ptr(), &mut pjsip_tx_data) };

        if pjsip_tx_data.is_null() {
            return Err(Error::Validation("Got null creating PjSipTxData".into()));
        }

        PjStatus::result_for_status(status).map(|_| Self { pjsip_tx_data })
    }

    pub fn inv_end_session<T, S: AsRef<CStr>>(
        inv_sess: &PjSipInvSession<T>,
        status_text: Option<S>,
    ) -> Result<PjSipTxData, Error> {
        let mut pjsip_tx_data = std::ptr::null_mut();
        let st_text = status_text
            .as_ref()
            .map(|s| s.as_ref().to_owned().into_raw())
            .unwrap_or(std::ptr::null_mut());

        let status = unsafe {
            pj::pjsip_inv_end_session(
                inv_sess.as_mut_ptr(),
                603,
                &pj::pj_str(st_text),
                &mut pjsip_tx_data,
            )
        };

        if pjsip_tx_data.is_null() {
            return Err(Error::Validation("Got null creating PjSipTxData".into()));
        }

        unsafe {
            if status_text.is_some() {
                let _ = CString::from_raw(st_text);
            }
        }

        PjStatus::result_for_status(status).map(|_| Self { pjsip_tx_data })
    }

    pub fn as_ptr(&self) -> *const pj::pjsip_tx_data {
        self.pjsip_tx_data
    }

    pub fn as_mut_ptr(&mut self) -> *mut pj::pjsip_tx_data {
        self.pjsip_tx_data
    }
}
