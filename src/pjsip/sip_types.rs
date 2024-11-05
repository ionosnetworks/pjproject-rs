use std::{
    borrow::Borrow,
    ffi::{CStr, CString},
    fmt::{Debug, Display},
    ops::Deref,
};

use pjproject_sys as pj;

pub struct PjSipHostPort {
    pjsip_host_port: PjSipHostPortRef,
}

unsafe impl Send for PjSipHostPort {}
unsafe impl Sync for PjSipHostPort {}

impl PjSipHostPort {
    pub fn new<S: AsRef<CStr>>(host: S, port: u16) -> Self {
        let host = host.as_ref().to_owned();
        let mut pjsip_host_port = Box::new(unsafe { std::mem::zeroed::<pj::pjsip_host_port>() });
        pjsip_host_port.host = unsafe { pj::pj_str(host.into_raw()) };
        pjsip_host_port.port = port as _;

        Self {
            pjsip_host_port: PjSipHostPortRef::from(Box::into_raw(pjsip_host_port)),
        }
    }

    pub fn host(&self) -> &CStr {
        self.pjsip_host_port.host()
    }

    pub fn port(&self) -> u16 {
        self.pjsip_host_port.port()
    }

    pub fn as_mut_ptr(&mut self) -> *mut pj::pjsip_host_port {
        self.as_ptr() as *mut _
    }
}

impl Drop for PjSipHostPort {
    fn drop(&mut self) {
        unsafe {
            let host_port = Box::from_raw(self.as_mut_ptr());
            let host = CString::from_raw(host_port.host.ptr);
            drop(host);
            drop(host_port)
        }
    }
}

impl Display for PjSipHostPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}",
            self.pjsip_host_port.host().to_string_lossy(),
            self.pjsip_host_port.port(),
        )
    }
}

impl Debug for PjSipHostPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Deref for PjSipHostPort {
    type Target = PjSipHostPortRef;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl AsRef<PjSipHostPortRef> for PjSipHostPort {
    #[inline]
    fn as_ref(&self) -> &PjSipHostPortRef {
        &self.pjsip_host_port
    }
}

impl AsMut<PjSipHostPortRef> for PjSipHostPort {
    #[inline]
    fn as_mut(&mut self) -> &mut PjSipHostPortRef {
        &mut self.pjsip_host_port
    }
}

impl Borrow<PjSipHostPortRef> for PjSipHostPort {
    #[inline]
    fn borrow(&self) -> &PjSipHostPortRef {
        &self.pjsip_host_port
    }
}

pub struct PjSipHostPortRef {
    pjsip_host_port: *const pj::pjsip_host_port,
}

impl PjSipHostPortRef {
    pub fn as_ptr(&self) -> *const pj::pjsip_host_port {
        self.pjsip_host_port
    }

    pub fn as_ref(&self) -> &pj::pjsip_host_port {
        unsafe { &*self.pjsip_host_port }
    }

    pub fn host(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.as_ref().host.ptr) }
    }

    pub fn port(&self) -> u16 {
        self.as_ref().port as _
    }
}

impl From<&pj::pjsip_host_port> for PjSipHostPortRef {
    fn from(value: &pj::pjsip_host_port) -> Self {
        Self {
            pjsip_host_port: value as *const _,
        }
    }
}

impl From<*const pj::pjsip_host_port> for PjSipHostPortRef {
    fn from(value: *const pj::pjsip_host_port) -> Self {
        Self {
            pjsip_host_port: value,
        }
    }
}

impl From<*mut pj::pjsip_host_port> for PjSipHostPortRef {
    fn from(value: *mut pj::pjsip_host_port) -> Self {
        Self {
            pjsip_host_port: value,
        }
    }
}

impl Display for PjSipHostPortRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}",
            self.host().to_string_lossy(),
            self.as_ref().port
        )
    }
}

impl Debug for PjSipHostPortRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl ToOwned for PjSipHostPortRef {
    type Owned = PjSipHostPort;

    fn to_owned(&self) -> Self::Owned {
        let host = self.host().to_owned();
        let port = self.port();

        Self::Owned::new(host, port)
    }
}
