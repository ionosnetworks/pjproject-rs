use pjproject_sys as pj;

use crate::{Error, PjCachingPool, PjIoqueue, PjStatus};

pub struct PjMediaEndpt {
    endpt: PjMediaEndptRef,
    #[allow(dead_code)]
    caching_pool: PjCachingPool,
}

unsafe impl Send for PjMediaEndpt {}
unsafe impl Sync for PjMediaEndpt {}

impl PjMediaEndpt {
    pub fn new<'a>(
        mut caching_pool: PjCachingPool,
        ioqueue: Option<PjIoqueue<'a>>,
        worker_cnt: u32,
    ) -> Result<Self, Error> {
        let mut endpt = unsafe { std::mem::zeroed() };
        let status = unsafe {
            pj::pjmedia_endpt_create2(
                caching_pool.factory_mut().as_mut(),
                ioqueue
                    .map(|ioq| ioq.as_mut_ptr())
                    .unwrap_or(std::ptr::null_mut()),
                worker_cnt,
                &mut endpt,
            )
        };

        PjStatus::result_for_status(status).map(|_| Self {
            endpt: PjMediaEndptRef::from(endpt),
            caching_pool,
        })
    }

    pub fn init_g711_codec(&mut self) -> Result<(), Error> {
        let status = unsafe { pj::pjmedia_codec_g711_init(self.as_mut_ptr()) };

        PjStatus::result_for_status(status)
    }

    pub fn as_ptr(&self) -> *const pj::pjmedia_endpt {
        self.endpt.as_ptr()
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pjmedia_endpt {
        self.as_ptr() as *mut _
    }
}

impl Drop for PjMediaEndpt {
    fn drop(&mut self) {
        unsafe {
            pj::pjmedia_endpt_destroy2(self.as_mut_ptr());
        };
    }
}

impl AsRef<PjMediaEndptRef> for PjMediaEndpt {
    #[inline]
    fn as_ref(&self) -> &PjMediaEndptRef {
        &self.endpt
    }
}

impl AsMut<PjMediaEndptRef> for PjMediaEndpt {
    #[inline]
    fn as_mut(&mut self) -> &mut PjMediaEndptRef {
        &mut self.endpt
    }
}

#[derive(Clone)]
pub struct PjMediaEndptRef {
    endpt: *const pj::pjmedia_endpt,
}

impl PjMediaEndptRef {
    pub fn as_ptr(&self) -> *const pj::pjmedia_endpt {
        self.endpt
    }
}

impl From<&pj::pjmedia_endpt> for PjMediaEndptRef {
    fn from(value: &pj::pjmedia_endpt) -> Self {
        Self {
            endpt: value as *const _,
        }
    }
}

impl From<*const pj::pjmedia_endpt> for PjMediaEndptRef {
    fn from(value: *const pj::pjmedia_endpt) -> Self {
        Self { endpt: value }
    }
}

impl From<*mut pj::pjmedia_endpt> for PjMediaEndptRef {
    fn from(value: *mut pj::pjmedia_endpt) -> Self {
        Self { endpt: value }
    }
}
