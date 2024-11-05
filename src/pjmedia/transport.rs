use std::{
    ffi::{c_void, CStr, CString},
    marker::PhantomData,
    // sync::Arc,
};

use pjproject_sys as pj;

use crate::{Error, PjMediaEndpt, PjStatus, RtcpPacket, RtpPacket, SockaddrTRef, AF};

pub struct PjMediaTransport<U> {
    transport: *mut pj::pjmedia_transport,
    phantom: PhantomData<U>,
}

unsafe impl<U> Send for PjMediaTransport<U> {}
unsafe impl<U> Sync for PjMediaTransport<U> {}

impl<U> PjMediaTransport<U> {
    pub fn new<S: AsRef<CStr>, T: AsRef<CStr>>(
        endpt: &PjMediaEndpt,
        name: S,
        af: Option<AF>,
        addr: Option<T>,
        port: u16,
        options: u32,
    ) -> Result<Self, Error> {
        let mut transport = std::ptr::null_mut();
        let name = name.as_ref().as_ptr() as *const i8;
        let addr = addr.as_ref().map(|a| a.as_ref());

        let status = unsafe {
            if let Some(addr) = addr {
                let addr = pj::pj_str(addr.as_ptr() as *mut i8);
                if let Some(af) = af {
                    pj::pjmedia_transport_udp_create3(
                        endpt.as_mut_ptr(),
                        af.as_u16() as _,
                        name,
                        &addr,
                        port as _,
                        options,
                        &mut transport,
                    )
                } else {
                    pj::pjmedia_transport_udp_create2(
                        endpt.as_mut_ptr(),
                        name,
                        &addr,
                        port as _,
                        options,
                        &mut transport,
                    )
                }
            } else {
                pj::pjmedia_transport_udp_create(
                    endpt.as_mut_ptr(),
                    name,
                    port as _,
                    options,
                    &mut transport,
                )
            }
        };

        PjStatus::result_for_status(status).map(|_| Self {
            transport,
            phantom: PhantomData,
        })
    }

    pub fn as_ptr(&self) -> *const pj::pjmedia_transport {
        self.transport
    }

    pub fn as_mut_ptr(&mut self) -> *mut pj::pjmedia_transport {
        self.transport
    }

    pub fn as_ref(&self) -> &pj::pjmedia_transport {
        unsafe { &*self.as_ptr() }
    }

    pub fn builder() -> PjMediaTransportBuilder<U> {
        PjMediaTransportBuilder::default()
    }

    pub fn info(&self) -> Result<PjMediaTransportInfo, Error> {
        PjMediaTransportInfo::from_transport(self)
    }

    pub fn attach(
        &self,
        user_data: Option<U>,
        rem_addr: &SockaddrTRef,
        rem_rtcp: Option<&SockaddrTRef>,
        rtp_cb: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, i64)>,
        rtcp_cb: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, i64)>,
    ) -> Result<(), Error> {
        let user_data = match user_data {
            Some(u) => Box::into_raw(Box::new(u)) as *mut c_void,
            None => std::ptr::null_mut(),
        };
        let (rem_addr_ptr, addr_len) = match &rem_addr {
            SockaddrTRef::IPv4(addr) => (
                addr.as_ptr() as *const c_void,
                std::mem::size_of::<pj::pj_sockaddr_in>(),
            ),
            SockaddrTRef::IPv6(addr) => (
                addr.as_ptr() as *const c_void,
                std::mem::size_of::<pj::pj_sockaddr_in6>(),
            ),
        };
        let rem_rtcp_ptr = rem_rtcp.as_ref().map(|a| match a {
            SockaddrTRef::IPv4(addr) => addr.as_ptr() as *const c_void,
            SockaddrTRef::IPv6(addr) => addr.as_ptr() as *const c_void,
        });
        let status = unsafe {
            let op = (*self.transport).op;
            if op.is_null() {
                return Err(Error::Validation("transport has no op".into()));
            }
            if let Some(attach2) = (*op).attach2.as_ref() {
                let mut param = std::mem::zeroed::<pj::pjmedia_transport_attach_param>();
                param.user_data = user_data;
                pj::pj_sockaddr_cp((&mut param.rem_addr) as *const _ as *mut _, rem_addr_ptr);
                match rem_rtcp {
                    Some(a) => {
                        let rem_rtcp = match a {
                            SockaddrTRef::IPv4(a) => a.as_ptr() as *const c_void,
                            SockaddrTRef::IPv6(a) => a.as_ptr() as *const c_void,
                        };
                        pj::pj_sockaddr_cp((&mut param.rem_rtcp) as *const _ as *mut _, rem_rtcp);
                    }
                    None => {
                        match &rem_addr {
                            SockaddrTRef::IPv4(a) => {
                                let mut rem_rtcp = a.to_owned();
                                rem_rtcp.with_port(rem_addr.get_port() + 1);
                                pj::pj_sockaddr_cp(
                                    (&mut param.rem_rtcp) as *const _ as *mut _,
                                    rem_rtcp.as_ptr() as *const _,
                                );
                            }
                            SockaddrTRef::IPv6(a) => {
                                let mut rem_rtcp = a.to_owned();
                                rem_rtcp.with_port(rem_addr.get_port() + 1);
                                pj::pj_sockaddr_cp(
                                    (&mut param.rem_rtcp) as *const _ as *mut _,
                                    rem_rtcp.as_ptr() as *const _,
                                );
                            }
                        };
                    }
                }
                param.addr_len = addr_len as _;
                param.rtp_cb = rtp_cb;
                param.rtcp_cb = rtcp_cb;
                attach2(self.transport, &mut param)
            } else if let Some(attach) = (*op).attach.as_ref() {
                attach(
                    self.transport,
                    user_data,
                    rem_addr_ptr,
                    rem_rtcp_ptr.unwrap_or(std::ptr::null()),
                    addr_len as _,
                    rtp_cb,
                    rtcp_cb,
                )
            } else {
                return Err(Error::Validation("transport op has no attach".into()));
            }
        };

        PjStatus::result_for_status(status)
    }

    pub fn detach(&self, user_data: Option<U>) -> Result<(), Error> {
        unsafe {
            if !(*self.transport).op.is_null() {
                if let Some(detach) = (*(*self.transport).op).detach {
                    let user_data = match user_data {
                        Some(user_data) => Box::into_raw(Box::new(user_data)) as *mut _,
                        None => std::ptr::null_mut(),
                    };
                    detach(self.transport, user_data);
                    if !user_data.is_null() {
                        let _ = Box::from_raw(user_data);
                    }
                } else {
                    return Err(Error::Validation("transport doesnt have detach".into()));
                }
            } else {
                return Err(Error::Validation("transport op is null".into()));
            }
        };

        Ok(())
    }

    pub fn start(&mut self) -> Result<(), Error> {
        let status = unsafe {
            if !(*self.transport).op.is_null() {
                if let Some(start) = (*(*self.transport).op).media_start {
                    start(
                        self.transport,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        0,
                    )
                } else {
                    return Err(Error::Validation("transport doesn't have start".into()));
                }
            } else {
                return Err(Error::Validation("transport op is null".into()));
            }
        };

        PjStatus::result_for_status(status)
    }

    pub fn send_rtp(&mut self, packet: &RtpPacket) -> Result<(), Error> {
        let status = unsafe {
            if !(*self.transport).op.is_null() {
                if let Some(send_rtp) = (*(*self.transport).op).send_rtp {
                    send_rtp(
                        self.transport,
                        packet.0.as_ptr() as *const _,
                        packet.0.len(),
                    )
                } else {
                    return Err(Error::Validation("transport doesn't have send_rtp".into()));
                }
            } else {
                return Err(Error::Validation("transport op is null".into()));
            }
        };

        PjStatus::result_for_status(status)
    }

    pub fn send_rtcp(&mut self, packet: &RtcpPacket) -> Result<(), Error> {
        let status = unsafe {
            if !(*self.transport).op.is_null() {
                if let Some(send_rtcp) = (*(*self.transport).op).send_rtcp {
                    send_rtcp(
                        self.transport,
                        packet.0.as_ptr() as *const _,
                        packet.0.len(),
                    )
                } else {
                    return Err(Error::Validation("transport doesn't have send_rtcp".into()));
                }
            } else {
                return Err(Error::Validation("transport op is null".into()));
            }
        };

        PjStatus::result_for_status(status)
    }
}

impl<U> Drop for PjMediaTransport<U> {
    fn drop(&mut self) {
        unsafe {
            if let Some(f) = (*(*self.transport).op).destroy {
                f(self.transport);
            } else {
            }
        };
    }
}

pub struct PjMediaTransportBuilder<U> {
    name: Option<CString>,
    addr_family: Option<AF>,
    addr: Option<CString>,
    port: u16,
    options: u32,
    phantom: PhantomData<U>,
}

impl<U> PjMediaTransportBuilder<U> {
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn name<S: AsRef<CStr>>(mut self, name: S) -> Self {
        self.name.replace(name.as_ref().to_owned());
        self
    }

    pub fn addr_family(mut self, af: AF) -> Self {
        self.addr_family.replace(af);
        self
    }

    pub fn addr<A: AsRef<CStr>>(mut self, addr: A) -> Self {
        self.addr.replace(addr.as_ref().to_owned());
        self
    }

    pub fn options(mut self, options: u32) -> Self {
        self.options = options;
        self
    }

    pub fn build(self, endpt: &PjMediaEndpt) -> Result<PjMediaTransport<U>, Error> {
        PjMediaTransport::new(
            endpt,
            self.name.unwrap_or(c"".to_owned()),
            self.addr_family,
            self.addr,
            self.port,
            self.options,
        )
    }
}

impl<U> Default for PjMediaTransportBuilder<U> {
    fn default() -> Self {
        Self {
            name: Default::default(),
            addr_family: Default::default(),
            addr: Default::default(),
            port: Default::default(),
            options: Default::default(),
            phantom: PhantomData,
        }
    }
}

pub struct PjMediaTransportInfo {
    info: pj::pjmedia_transport_info,
}

impl PjMediaTransportInfo {
    pub fn from_transport<T>(transport: &PjMediaTransport<T>) -> Result<Self, Error> {
        let mut info = unsafe { std::mem::zeroed::<pj::pjmedia_transport_info>() };
        info.sock_info.rtp_sock = pj::PJ_INVALID_SOCKET as _;
        info.sock_info.rtcp_sock = pj::PJ_INVALID_SOCKET as _;

        let status = unsafe {
            if transport.as_ref().op.is_null() || (*transport.as_ref().op).get_info.is_none() {
                crate::PJ_ENOTSUP
            } else {
                ((*transport.as_ref().op).get_info).unwrap()(
                    transport.as_ptr() as *mut _,
                    &mut info,
                )
            }
        };

        PjStatus::result_for_status(status).map(|_| Self { info })
    }

    pub fn rtp_ipv4_port(&self) -> u16 {
        unsafe { pj::pj_ntohs(self.info.sock_info.rtp_addr_name.ipv4.sin_port) }
    }
}
