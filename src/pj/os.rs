use pjproject_sys as pj;

use crate::{Error, PjStatus};

#[derive(Copy, Clone)]
pub struct PjTimestamp(pub(crate) pj::pj_timestamp);

impl PjTimestamp {
    pub fn new() -> Result<Self, Error> {
        let (ts, status) = unsafe {
            let mut ts = std::mem::zeroed();
            let status = pj::pj_get_timestamp(&mut ts);

            (ts, status)
        };

        PjStatus::result_for_status(status).map(|_| Self(ts))
    }

    pub fn freq() -> Result<Self, Error> {
        let (ts, status) = unsafe {
            let mut ts = std::mem::zeroed();
            let status = pj::pj_get_timestamp_freq(&mut ts);

            (ts, status)
        };

        PjStatus::result_for_status(status).map(|_| Self(ts))
    }

    pub fn ts(&self) -> u64 {
        unsafe { self.0.u64_ }
    }

    pub fn set_ts(&mut self, ts: u64) {
        self.0.u64_ = ts;
    }
}
