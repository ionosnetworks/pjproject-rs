use std::marker::{self, PhantomData};

use pjproject_sys as pj;

pub struct PjIoqueue<'a> {
    ioqueue: *mut pj::pj_ioqueue_t,
    marker: marker::PhantomData<&'a ()>,
}

impl<'a> PjIoqueue<'a> {
    pub fn as_ptr(&self) -> *const pj::pj_ioqueue_t {
        self.ioqueue
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pj_ioqueue_t {
        self.ioqueue
    }
}

impl<'a> From<*mut pj::pj_ioqueue_t> for PjIoqueue<'a> {
    fn from(value: *mut pj::pj_ioqueue_t) -> Self {
        Self {
            ioqueue: value,
            marker: PhantomData,
        }
    }
}
