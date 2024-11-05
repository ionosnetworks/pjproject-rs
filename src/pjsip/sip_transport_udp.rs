use pjproject_sys as pj;

use crate::{Error, PjSipHostPortRef, PjSockaddrInRef, PjStatus};

use super::PjSipEndpoint;

pub struct PjSipTransportUdp {
    pjsip_transport: *const pj::pjsip_transport,
}

impl PjSipTransportUdp {
    pub fn new_from_endpoint(
        endpoint: &PjSipEndpoint,
        local: &PjSockaddrInRef,
        a_name: Option<&PjSipHostPortRef>,
        async_cnt: u32,
    ) -> Result<Self, Error> {
        let a_name = match a_name {
            Some(a_name) => a_name.as_ptr(),
            None => std::ptr::null_mut(),
        };

        let mut pjsip_transport = std::ptr::null_mut();
        let status = unsafe {
            pj::pjsip_udp_transport_start(
                endpoint.as_mut_ptr(),
                local.as_ptr(),
                a_name,
                async_cnt,
                &mut pjsip_transport,
            )
        };

        PjStatus::result_for_status(status).map(|_| PjSipTransportUdp { pjsip_transport })
    }

    pub fn as_ref(&self) -> &pj::pjsip_transport {
        unsafe { &*self.pjsip_transport }
    }

    pub fn local_name(&self) -> PjSipHostPortRef {
        (&self.as_ref().local_name).into()
    }
}
