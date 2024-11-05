use std::fmt;

use pjproject_sys as pj;

#[derive(Clone, Copy)]
pub struct PjStatus(pub i32);

impl fmt::Debug for PjStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{self}"))
    }
}

impl PjStatus {
    pub fn new(status: i32) -> Self {
        Self(status)
    }

    pub fn result_for_status(status: i32) -> Result<(), crate::Error> {
        if status == pj::pj_constants__PJ_SUCCESS as _ {
            return Ok(());
        }

        Err(crate::Error::PjError(Self::new(status)))
    }

    pub fn is_success(&self) -> bool {
        self.0 == pj::pj_constants__PJ_SUCCESS as i32
    }

    pub fn is_err(&self) -> bool {
        !self.is_success()
    }

    pub fn as_result(&self) -> Result<(), crate::Error> {
        if self.is_success() {
            Ok(())
        } else {
            Err(crate::Error::PjError(self.clone()))
        }
    }
}

impl fmt::Display for PjStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut msg = Vec::with_capacity(pj::PJ_ERR_MSG_SIZE as _);
        unsafe {
            let pj_str = pj::pj_strerror(
                self.0,
                msg.as_mut_ptr() as *mut i8,
                pj::PJ_ERR_MSG_SIZE as _,
            );
            msg.set_len(pj_str.slen as _);
        }
        let msg = String::from_utf8_lossy(&msg);
        write!(f, "msg: {msg} [status={}]", self.0)
    }
}
