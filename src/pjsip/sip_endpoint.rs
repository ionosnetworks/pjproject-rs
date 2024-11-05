use std::{ffi::CStr, sync::Arc};

use pjproject_sys as pj;

use crate::{
    Error, PjCachingPool, PjIoqueue, PjSipInvCallback, PjSipModule, PjSipTransportUdp,
    PjSockaddrInRef, PjStatus, PjTimeVal,
};

use super::PjSipHostPortRef;

pub const DEFAULT_SIP_PORT: u16 = 5060;

pub struct PjSipEndpoint {
    pjsip_endpoint: *mut pj::pjsip_endpoint,
    #[allow(dead_code)]
    caching_pool: PjCachingPool,
}

unsafe impl Send for PjSipEndpoint {}
unsafe impl Sync for PjSipEndpoint {}

impl PjSipEndpoint {
    pub fn new<S: AsRef<CStr>>(mut caching_pool: PjCachingPool, name: S) -> Result<Self, Error> {
        let mut pjsip_endpoint = std::ptr::null_mut();
        let status = unsafe {
            pj::pjsip_endpt_create(
                caching_pool.factory_mut().as_mut(),
                name.as_ref().as_ptr() as *const i8,
                &mut pjsip_endpoint,
            )
        };

        PjStatus::result_for_status(status).map(|_| Self {
            pjsip_endpoint,
            caching_pool,
        })
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pjsip_endpoint {
        self.pjsip_endpoint
    }

    pub fn udp_transport_start(
        &self,
        local: &PjSockaddrInRef,
        a_name: Option<&PjSipHostPortRef>,
        async_cnt: u32,
    ) -> Result<PjSipTransportUdp, Error> {
        PjSipTransportUdp::new_from_endpoint(self, local, a_name, async_cnt)
    }

    pub fn init_tsx_layer_module(&self) -> Result<(), Error> {
        let status = unsafe { pj::pjsip_tsx_layer_init_module(self.as_mut_ptr()) };

        PjStatus::result_for_status(status)
    }

    pub fn init_ua_module(&self) -> Result<(), Error> {
        let status = unsafe { pj::pjsip_ua_init_module(self.as_mut_ptr(), std::ptr::null()) };

        PjStatus::result_for_status(status)
    }

    pub fn init_100rel_module(&self) -> Result<(), Error> {
        let status = unsafe { pj::pjsip_100rel_init_module(self.as_mut_ptr()) };

        PjStatus::result_for_status(status)
    }

    pub fn init_inv_usage<T>(&self, inv_cb: &PjSipInvCallback<T>) -> Result<(), Error> {
        let status = unsafe { pj::pjsip_inv_usage_init(self.as_mut_ptr(), inv_cb.as_ptr()) };

        PjStatus::result_for_status(status)
    }

    pub fn register_module(endpt: Arc<Self>, module: &mut PjSipModule) -> Result<(), Error> {
        let status =
            unsafe { pj::pjsip_endpt_register_module((*endpt).as_mut_ptr(), module.as_mut_ptr()) };

        PjStatus::result_for_status(status).map(|_| {
            module.registered(Arc::downgrade(&endpt));
        })
    }

    pub fn unregister_module(&self, module: &mut PjSipModule) -> Result<(), Error> {
        let status =
            unsafe { pj::pjsip_endpt_unregister_module(self.as_mut_ptr(), module.as_mut_ptr()) };

        PjStatus::result_for_status(status)
    }

    pub fn get_ioqueue<'a>(&'a self) -> PjIoqueue<'a> {
        let ioqueue = unsafe { pj::pjsip_endpt_get_ioqueue(self.as_mut_ptr()) };

        PjIoqueue::from(ioqueue)
    }

    pub fn handle_events(&self, timeout: &PjTimeVal) -> Result<(), Error> {
        let status = unsafe { pj::pjsip_endpt_handle_events(self.as_mut_ptr(), &timeout.0) };

        PjStatus::result_for_status(status)
    }
}

impl Drop for PjSipEndpoint {
    fn drop(&mut self) {
        unsafe {
            pj::pjsip_endpt_destroy(self.as_mut_ptr());
        };
    }
}
