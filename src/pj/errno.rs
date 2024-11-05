use pjproject_sys as pj;

pub const PJ_ERRNO_START: i32 = pj::PJ_ERRNO_START as i32;
pub const PJ_ERRNO_START_STATUS: i32 = pj::PJ_ERRNO_START_STATUS as i32;

pub const PJ_EINVAL: i32 = PJ_ERRNO_START_STATUS as i32 + 4;
pub const PJ_ENOTSUP: i32 = PJ_ERRNO_START_STATUS as i32 + 12;
pub const PJ_EEOF: i32 = PJ_ERRNO_START_STATUS as i32 + 16;
