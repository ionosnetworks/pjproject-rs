use pjproject_sys as pj;

use crate::{Error, PjPool, PjStatus};

pub struct PjMediaEventMgr {
    event_mgr: *mut pj::pjmedia_event_mgr,
    #[allow(dead_code)]
    pool: PjPool,
}

unsafe impl Send for PjMediaEventMgr {}
unsafe impl Sync for PjMediaEventMgr {}

impl PjMediaEventMgr {
    pub fn new(options: u32) -> Result<Self, Error> {
        let mut pool = PjPool::default_with_name(c"event-mgr");
        let mut event_mgr = unsafe { std::mem::zeroed() };
        let status =
            unsafe { pj::pjmedia_event_mgr_create(pool.as_mut_ptr(), options, &mut event_mgr) };

        PjStatus::result_for_status(status).map(|_| Self { event_mgr, pool })
    }

    pub fn as_ptr(&self) -> *mut pj::pjmedia_event_mgr {
        self.event_mgr
    }
}

impl Drop for PjMediaEventMgr {
    fn drop(&mut self) {
        unsafe {
            pj::pjmedia_event_mgr_destroy(self.as_ptr());
        }
    }
}
