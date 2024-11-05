use pjproject_sys as pj;

#[allow(dead_code)]
pub struct PjSipEvent {
    pjsip_event: *mut pj::pjsip_event,
}

impl From<*mut pj::pjsip_event> for PjSipEvent {
    fn from(value: *mut pj::pjsip_event) -> Self {
        Self { pjsip_event: value }
    }
}
