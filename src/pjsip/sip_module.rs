use std::{
    ffi::{CStr, CString},
    sync::Weak,
};

use pjproject_sys as pj;

use crate::{Error, PjStatus};

use super::PjSipEndpoint;

pub struct PjSipModule {
    pjsip_module: PjSipModuleRef,
    sip_endpt: Weak<PjSipEndpoint>,
}

unsafe impl Send for PjSipModule {}
unsafe impl Sync for PjSipModule {}

impl PjSipModule {
    pub fn new<S: AsRef<CStr>>(name: S) -> Result<Self, Error> {
        let mut module = Box::new(unsafe { std::mem::zeroed::<pj::pjsip_module>() });
        module.name = unsafe { pj::pj_str(name.as_ref().to_owned().into_raw()) };
        module.id = -1;

        Ok(Self {
            pjsip_module: Box::into_raw(module).into(),
            sip_endpt: Weak::new(),
        })
    }

    pub fn id(&self) -> i32 {
        self.pjsip_module.id()
    }

    pub fn as_ptr(&self) -> *const pj::pjsip_module {
        self.pjsip_module.as_ptr()
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pjsip_module {
        self.pjsip_module.as_ptr() as *mut _
    }

    pub fn registered(&mut self, sip_endpt: Weak<PjSipEndpoint>) {
        self.sip_endpt = sip_endpt;
    }
}

impl Drop for PjSipModule {
    fn drop(&mut self) {
        let module = unsafe { Box::from_raw(self.as_mut_ptr()) };

        if let Some(sip_endpoint) = self.sip_endpt.upgrade() {
            let status = unsafe {
                pj::pjsip_endpt_unregister_module((*sip_endpoint).as_mut_ptr(), self.as_mut_ptr())
            };
            if let Err(err) = PjStatus::result_for_status(status) {
                tracing::error!("Failed to unregister module from sip_endpoint: {err}");
            }
        }

        let name = unsafe { CString::from_raw(module.name.ptr) };
        drop(name);
        drop(module);
    }
}

pub struct PjSipModuleRef {
    pjsip_module: *const pj::pjsip_module,
}

impl PjSipModuleRef {
    pub fn as_ptr(&self) -> *const pj::pjsip_module {
        self.pjsip_module
    }

    pub fn id(&self) -> i32 {
        unsafe { (*self.pjsip_module).id }
    }
}

impl From<*const pj::pjsip_module> for PjSipModuleRef {
    fn from(value: *const pj::pjsip_module) -> Self {
        Self {
            pjsip_module: value,
        }
    }
}

impl From<*mut pj::pjsip_module> for PjSipModuleRef {
    fn from(value: *mut pj::pjsip_module) -> Self {
        Self {
            pjsip_module: value,
        }
    }
}

pub struct PjSipModuleRefMut {
    pjsip_module: *mut pj::pjsip_module,
}

impl PjSipModuleRefMut {
    pub fn id(&self) -> i32 {
        unsafe { (*self.pjsip_module).id }
    }
}

impl From<*mut pj::pjsip_module> for PjSipModuleRefMut {
    fn from(value: *mut pj::pjsip_module) -> Self {
        Self {
            pjsip_module: value,
        }
    }
}
