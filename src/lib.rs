use pjproject_sys as pj_sys;

pub mod error;
pub mod pj;
pub mod pjmedia;
pub mod pjsip;
pub mod pjsip_ua;
pub mod status;

pub use pj::*;
pub use pjmedia::*;
pub use pjsip::*;
pub use pjsip_ua::*;
pub use status::*;

pub use error::*;
pub use pjproject_sys;

pub fn pj_init() -> Result<(), Error> {
    let status = unsafe { pj_sys::pj_init() };

    PjStatus::result_for_status(status)
}

pub fn pj_shutdown() {
    unsafe { pj_sys::pj_shutdown() };
}

pub fn pjlib_util_init() -> Result<(), Error> {
    let status = unsafe { pj_sys::pjlib_util_init() };

    PjStatus::result_for_status(status)
}
