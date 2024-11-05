use std::ffi::CStr;

use pjproject_sys as pj;

use crate::{Error, PjPoolRef, PjSipUserAgentRef, PjStatus};

pub struct PjSipDialog {
    dialog: *mut pj::pjsip_dialog,
}

impl PjSipDialog {
    pub fn new<S: AsRef<CStr>, T: AsRef<CStr>, U: AsRef<CStr>, V: AsRef<CStr>>(
        ua: PjSipUserAgentRef,
        local_uri: S,
        local_contact: T,
        remote_uri: U,
        target: V,
    ) -> Result<Self, Error> {
        let mut dialog = std::ptr::null_mut();
        let status = unsafe {
            pj::pjsip_dlg_create_uac(
                ua.as_ptr() as *mut _,
                &pj::pj_str(local_uri.as_ref().as_ptr() as *mut i8),
                &pj::pj_str(local_contact.as_ref().as_ptr() as *mut i8),
                &pj::pj_str(remote_uri.as_ref().as_ptr() as *mut i8),
                &pj::pj_str(target.as_ref().as_ptr() as *mut i8),
                &mut dialog,
            )
        };

        PjStatus::result_for_status(status).map(|_| Self { dialog })
    }

    pub fn as_mut_ptr(&mut self) -> *mut pj::pjsip_dialog {
        self.dialog
    }

    /** Forcefully terminate dialog. Dialog may have already been destroyed
     * and this will return an error if so. Should be ok to ignore the error */
    pub fn terminate(self) -> Result<(), Error> {
        let status = unsafe { pj::pjsip_dlg_terminate(self.dialog) };

        PjStatus::result_for_status(status)
    }

    pub fn pool(&self) -> PjPoolRef {
        PjPoolRef::from((unsafe { *self.dialog }).pool)
    }
}
