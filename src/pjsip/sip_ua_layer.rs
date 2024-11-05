use pjproject_sys as pj;

use crate::Error;

use super::PjSipModuleRef;

pub type PjSipUserAgentRef = PjSipModuleRef;

impl PjSipUserAgentRef {
    pub fn pjsip_ua_instance() -> Result<Self, Error> {
        let ua = unsafe { pj::pjsip_ua_instance() };
        if ua.is_null() {
            return Err(Error::Validation("ua_instance returned null".into()));
        }
        Ok(PjSipModuleRef::from(ua as *const _))
    }
}
