#![allow(non_camel_case_types)]
use std::{
    ffi::{CStr, CString},
    marker::PhantomData,
};

use pjproject_sys as pj;

use crate::{Error, PjStatus};

pub static PJ_AF_UNSPEC: &pj::pj_uint16_t = unsafe { &pj::PJ_AF_UNSPEC };
pub static PJ_AF_UNIX: &pj::pj_uint16_t = unsafe { &pj::PJ_AF_UNIX };
pub static PJ_AF_INET: &pj::pj_uint16_t = unsafe { &pj::PJ_AF_INET };
pub static PJ_AF_INET6: &pj::pj_uint16_t = unsafe { &pj::PJ_AF_INET6 };
pub static PJ_AF_PACKET: &pj::pj_uint16_t = unsafe { &pj::PJ_AF_PACKET };
pub static PJ_AF_IRDA: &pj::pj_uint16_t = unsafe { &pj::PJ_AF_IRDA };

#[derive(Copy, Clone)]
pub enum AF {
    PJ_AF_UNSPEC,
    PJ_AF_UNIX,
    PJ_AF_INET,
    PJ_AF_INET6,
    PJ_AF_PACKET,
    PJ_AF_IRDA,
}

impl AF {
    pub fn as_u16(&self) -> u16 {
        match self {
            AF::PJ_AF_UNSPEC => *PJ_AF_UNSPEC,
            AF::PJ_AF_UNIX => *PJ_AF_UNIX,
            AF::PJ_AF_INET => *PJ_AF_INET,
            AF::PJ_AF_INET6 => *PJ_AF_INET6,
            AF::PJ_AF_PACKET => *PJ_AF_PACKET,
            AF::PJ_AF_IRDA => *PJ_AF_IRDA,
        }
    }
}

pub struct PjSockaddr {
    sockaddr: PjSockaddrRef<'static>,
}

impl PjSockaddr {
    pub fn as_ptr(&self) -> *const pj::pj_sockaddr {
        self.sockaddr.as_ptr()
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pj_sockaddr {
        self.as_ptr() as *mut _
    }

    pub fn with_port(&mut self, port: u16) -> &mut Self {
        unsafe { pj::pj_sockaddr_set_port(self.as_mut_ptr(), port) };
        self
    }
}

impl<'a> AsRef<PjSockaddrRef<'a>> for PjSockaddr {
    #[inline]
    fn as_ref(&self) -> &PjSockaddrRef<'a> {
        &self.sockaddr
    }
}

pub struct PjSockaddrRef<'a> {
    sockaddr: *const pj::pj_sockaddr,
    phantom: PhantomData<&'a pj::pj_sockaddr>,
}

impl<'a> PjSockaddrRef<'a> {
    pub fn as_ptr(&self) -> *const pj::pj_sockaddr {
        self.sockaddr
    }
}

impl<'a> From<&'a pj::pj_sockaddr> for PjSockaddrRef<'a> {
    fn from(value: &pj::pj_sockaddr) -> Self {
        Self {
            sockaddr: value as *const _,
            phantom: PhantomData,
        }
    }
}

impl<'a> From<&'a mut pj::pj_sockaddr> for PjSockaddrRef<'a> {
    fn from(value: &mut pj::pj_sockaddr) -> Self {
        Self {
            sockaddr: value as *const _,
            phantom: PhantomData,
        }
    }
}

impl<'a> From<*const pj::pj_sockaddr> for PjSockaddrRef<'a> {
    fn from(value: *const pj::pj_sockaddr) -> Self {
        Self {
            sockaddr: value,
            phantom: PhantomData,
        }
    }
}

impl<'a> From<*mut pj::pj_sockaddr> for PjSockaddrRef<'a> {
    fn from(value: *mut pj::pj_sockaddr) -> Self {
        Self {
            sockaddr: value,
            phantom: PhantomData,
        }
    }
}

/// IPv4
pub struct PjSockaddrIn {
    sockaddr_in: PjSockaddrInRef<'static>,
}

impl PjSockaddrIn {
    pub fn new<S: AsRef<CStr>>(sin_addr: Option<S>, sin_port: u16) -> Result<Self, Error> {
        let mut sockaddr_in = Box::new(unsafe { std::mem::zeroed::<pj::pj_sockaddr_in>() });
        let addr = sin_addr.as_ref().map(|a| a.as_ref().to_owned());
        let addr = addr.map(|a| unsafe { pj::pj_str(a.into_raw()) });
        let status = unsafe {
            pj::pj_sockaddr_in_init(
                sockaddr_in.as_mut(),
                match &addr {
                    Some(a) => a,
                    None => std::ptr::null_mut(),
                },
                sin_port,
            )
        };

        unsafe {
            let addr = addr.map(|s| CString::from_raw(s.ptr));
            drop(addr);
        }

        PjStatus::result_for_status(status).map(|_| Self {
            sockaddr_in: PjSockaddrInRef::from(Box::into_raw(sockaddr_in)),
        })
    }

    pub fn as_ptr(&self) -> *const pj::pj_sockaddr_in {
        self.sockaddr_in.as_ptr()
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pj_sockaddr_in {
        self.as_ptr() as *mut _
    }

    pub fn as_mut(&mut self) -> &mut pj::pj_sockaddr_in {
        unsafe { &mut *self.as_mut_ptr() }
    }

    pub fn with_family(&mut self, family: AF) -> &mut Self {
        self.as_mut().sin_family = family.as_u16() as _;
        self
    }

    pub fn with_addr_str<S: AsRef<CStr>>(&mut self, addr: S) -> &mut Self {
        unsafe {
            pj::pj_sockaddr_in_set_str_addr(
                self.as_mut_ptr(),
                &pj::pj_str(addr.as_ref().as_ptr() as _),
            )
        };
        self
    }

    pub fn with_addr(&mut self, addr: u32) -> &mut Self {
        unsafe { pj::pj_sockaddr_in_set_addr(self.as_mut_ptr(), addr) };
        self
    }

    pub fn with_port(&mut self, port: u16) -> &mut Self {
        unsafe { pj::pj_sockaddr_in_set_port(self.as_mut_ptr(), port) };
        self
    }
}

impl Default for PjSockaddrIn {
    fn default() -> Self {
        Self {
            sockaddr_in: PjSockaddrInRef::from(Box::into_raw(Box::new(unsafe {
                std::mem::zeroed::<pj::pj_sockaddr_in>()
            }))),
        }
    }
}

impl Drop for PjSockaddrIn {
    fn drop(&mut self) {
        unsafe {
            let sockaddr_in = Box::from_raw(self.as_mut_ptr());
            drop(sockaddr_in);
        }
    }
}

impl<'a> AsRef<PjSockaddrInRef<'a>> for PjSockaddrIn {
    #[inline]
    fn as_ref(&self) -> &PjSockaddrInRef<'a> {
        &self.sockaddr_in
    }
}

impl<'a> std::borrow::Borrow<PjSockaddrInRef<'a>> for PjSockaddrIn {
    fn borrow(&self) -> &PjSockaddrInRef<'a> {
        self.as_ref()
    }
}

pub struct PjSockaddrInRef<'a> {
    sockaddr_in: *const pj::pj_sockaddr_in,
    phantom: PhantomData<&'a pj::pj_sockaddr_in>,
}

impl<'a> PjSockaddrInRef<'a> {
    pub fn as_ptr(&self) -> *const pj::pj_sockaddr_in {
        self.sockaddr_in
    }

    pub fn get_port(&self) -> u16 {
        unsafe { pj::pj_sockaddr_get_port(self.as_ptr() as *const _) }
    }
}

impl<'a> From<&'a pj::pj_sockaddr_in> for PjSockaddrInRef<'a> {
    fn from(value: &pj::pj_sockaddr_in) -> Self {
        Self {
            sockaddr_in: value as *const _,
            phantom: PhantomData,
        }
    }
}

impl<'a> From<*const pj::pj_sockaddr_in> for PjSockaddrInRef<'a> {
    fn from(value: *const pj::pj_sockaddr_in) -> Self {
        Self {
            sockaddr_in: value,
            phantom: PhantomData,
        }
    }
}

impl<'a> From<*mut pj::pj_sockaddr_in> for PjSockaddrInRef<'a> {
    fn from(value: *mut pj::pj_sockaddr_in) -> Self {
        Self {
            sockaddr_in: value,
            phantom: PhantomData,
        }
    }
}

impl<'a> From<PjSockaddrRef<'a>> for PjSockaddrInRef<'a> {
    fn from(value: PjSockaddrRef<'a>) -> Self {
        Self {
            sockaddr_in: value.as_ptr() as *const _,
            phantom: PhantomData,
        }
    }
}

impl<'a> ToOwned for PjSockaddrInRef<'a> {
    type Owned = PjSockaddrIn;

    fn to_owned(&self) -> Self::Owned {
        let dst = unsafe {
            let mut dst = Box::new(std::mem::zeroed::<pj::pj_sockaddr_in>());
            let addr_len = std::mem::size_of::<pj::pj_sockaddr_in>();
            std::ptr::copy_nonoverlapping(self.as_ptr(), dst.as_mut(), addr_len);

            dst
        };

        Self::Owned {
            sockaddr_in: PjSockaddrInRef::from(Box::into_raw(dst)),
        }
    }
}

/// IPv6
pub struct PjSockaddrIn6 {
    sockaddr_in6: PjSockaddrIn6Ref<'static>,
}

impl PjSockaddrIn6 {
    pub fn as_ptr(&self) -> *const pj::pj_sockaddr_in6 {
        self.sockaddr_in6.as_ptr()
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pj_sockaddr_in6 {
        self.as_ptr() as *mut _
    }

    pub fn with_port(&mut self, port: u16) -> &mut Self {
        unsafe { pj::pj_sockaddr_set_port(self.as_mut_ptr() as *mut pj::pj_sockaddr, port) };
        self
    }
}

impl Default for PjSockaddrIn6 {
    fn default() -> Self {
        Self {
            sockaddr_in6: PjSockaddrIn6Ref::from(Box::into_raw(Box::new(unsafe {
                std::mem::zeroed::<pj::pj_sockaddr_in6>()
            }))),
        }
    }
}

impl Drop for PjSockaddrIn6 {
    fn drop(&mut self) {
        unsafe {
            let sockaddr_in6 = Box::from_raw(self.as_mut_ptr());
            drop(sockaddr_in6);
        }
    }
}

impl<'a> AsRef<PjSockaddrIn6Ref<'a>> for PjSockaddrIn6 {
    #[inline]
    fn as_ref(&self) -> &PjSockaddrIn6Ref<'a> {
        &self.sockaddr_in6
    }
}

impl<'a> std::borrow::Borrow<PjSockaddrIn6Ref<'a>> for PjSockaddrIn6 {
    fn borrow(&self) -> &PjSockaddrIn6Ref<'a> {
        self.as_ref()
    }
}

pub struct PjSockaddrIn6Ref<'a> {
    sockaddr_in6: *const pj::pj_sockaddr_in6,
    phantom: PhantomData<&'a pj::pj_sockaddr_in6>,
}

impl<'a> PjSockaddrIn6Ref<'a> {
    pub fn as_ptr(&self) -> *const pj::pj_sockaddr_in6 {
        self.sockaddr_in6
    }

    pub fn get_port(&self) -> u16 {
        unsafe { pj::pj_sockaddr_get_port(self.as_ptr() as *const _) }
    }
}

impl<'a> From<&'a pj::pj_sockaddr_in6> for PjSockaddrIn6Ref<'a> {
    fn from(value: &pj::pj_sockaddr_in6) -> Self {
        Self {
            sockaddr_in6: value as *const _,
            phantom: PhantomData,
        }
    }
}

impl<'a> From<*const pj::pj_sockaddr_in6> for PjSockaddrIn6Ref<'a> {
    fn from(value: *const pj::pj_sockaddr_in6) -> Self {
        Self {
            sockaddr_in6: value,
            phantom: PhantomData,
        }
    }
}

impl<'a> From<*mut pj::pj_sockaddr_in6> for PjSockaddrIn6Ref<'a> {
    fn from(value: *mut pj::pj_sockaddr_in6) -> Self {
        Self {
            sockaddr_in6: value,
            phantom: PhantomData,
        }
    }
}

impl<'a> ToOwned for PjSockaddrIn6Ref<'a> {
    type Owned = PjSockaddrIn6;

    fn to_owned(&self) -> Self::Owned {
        let dst = unsafe {
            let mut dst = Box::new(std::mem::zeroed::<pj::pj_sockaddr_in6>());
            let addr_len = std::mem::size_of::<pj::pj_sockaddr_in6>();
            std::ptr::copy_nonoverlapping(self.as_ptr(), dst.as_mut(), addr_len);

            dst
        };

        Self::Owned {
            sockaddr_in6: PjSockaddrIn6Ref::from(Box::into_raw(dst)),
        }
    }
}

pub enum SockaddrT {
    IPv4(PjSockaddrIn),
    IPv6(PjSockaddrIn6),
}

pub enum SockaddrTRef<'a> {
    IPv4(PjSockaddrInRef<'a>),
    IPv6(PjSockaddrIn6Ref<'a>),
}

impl<'a> SockaddrTRef<'a> {
    pub fn get_port(&self) -> u16 {
        match self {
            SockaddrTRef::IPv4(a) => a.get_port(),
            SockaddrTRef::IPv6(a) => a.get_port(),
        }
    }
}

pub fn pj_gethostname() -> CString {
    unsafe {
        let hostname = pj::pj_gethostname();
        CStr::from_ptr((*hostname).ptr).to_owned()
    }
}
