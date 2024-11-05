use pjproject_sys as pj;

use crate::{Error, PjPool, PjStatus};

use super::PjMediaPort;

pub struct PjMediaMasterPort {
    port: *mut pj::pjmedia_master_port,
    #[allow(dead_code)]
    pool: PjPool,
}

unsafe impl Send for PjMediaMasterPort {}
unsafe impl Sync for PjMediaMasterPort {}

impl PjMediaMasterPort {
    pub fn new(
        src_port: &mut PjMediaPort,
        dst_port: &mut PjMediaPort,
        options: u32,
    ) -> Result<Self, Error> {
        let mut pool = PjPool::default_with_name(c"master-port");
        let mut port = unsafe { std::mem::zeroed() };
        let status = unsafe {
            pj::pjmedia_master_port_create(
                pool.as_mut_ptr(),
                src_port.as_mut_ptr(),
                dst_port.as_mut_ptr(),
                options,
                &mut port,
            )
        };

        PjStatus::result_for_status(status).map(|_| Self { port, pool })
    }

    pub fn as_ptr(&self) -> *const pj::pjmedia_master_port {
        self.port
    }

    pub fn as_mut_ptr(&mut self) -> *mut pj::pjmedia_master_port {
        self.port
    }

    pub fn start(&mut self) -> Result<(), Error> {
        let status = unsafe { pj::pjmedia_master_port_start(self.as_mut_ptr()) };

        PjStatus::result_for_status(status)
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        let status = unsafe { pj::pjmedia_master_port_stop(self.as_mut_ptr()) };

        PjStatus::result_for_status(status)
    }
}

impl Drop for PjMediaMasterPort {
    fn drop(&mut self) {
        let status = unsafe { pj::pjmedia_master_port_destroy(self.as_mut_ptr(), 0) };
        if let Err(err) = PjStatus::result_for_status(status) {
            tracing::error!("Failed to destroy master port: {err}");
        }
    }
}
