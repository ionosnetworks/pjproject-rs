use std::{
    ffi::{CStr, CString},
    fmt::{Debug, Display},
    marker::PhantomData,
    os::raw::c_void,
    sync::Arc,
};

use itertools::Itertools;
use pjproject_sys as pj;

use crate::{
    Error, PjMediaEndpt, PjMediaSdpSession, PjMediaSdpSessionRef, PjMediaStreamInfo, PjSipDialog,
    PjSipEvent, PjSipTxData, PjStatus,
};

// @Todo(skf): Create PjSipInvSessionRef for the inv_callbacks and others so
// that we don't drop module data if from_ptr is used
#[derive(Clone)]
pub struct PjSipInvSession<T> {
    pjsip_inv_session: *mut pj::pjsip_inv_session,
    /// T: module data type
    phantom: PhantomData<T>,
}

unsafe impl<T> Send for PjSipInvSession<T> {}
unsafe impl<T> Sync for PjSipInvSession<T> {}

impl<T> PjSipInvSession<T> {
    /// NB: After this function is called, dialog may be destroyed.
    pub fn create_uac(
        dialog: &mut PjSipDialog,
        local_sdp: &PjMediaSdpSession,
        options: u32,
    ) -> Result<Self, Error> {
        let mut inv_sess = unsafe { std::mem::zeroed() };
        let status = unsafe {
            pj::pjsip_inv_create_uac(
                dialog.as_mut_ptr(),
                local_sdp.as_ptr(),
                options,
                &mut inv_sess,
            )
        };

        PjStatus::result_for_status(status).map(|_| Self {
            pjsip_inv_session: inv_sess,
            phantom: PhantomData,
        })
    }

    pub fn as_ptr(&self) -> *const pj::pjsip_inv_session {
        self.pjsip_inv_session
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pjsip_inv_session {
        self.pjsip_inv_session
    }

    pub fn as_ref(&self) -> &pj::pjsip_inv_session {
        unsafe { &*self.as_ptr() }
    }

    pub fn obj_name(&self) -> CString {
        unsafe {
            CString::from_vec_unchecked(
                self.as_ref()
                    .obj_name
                    .iter()
                    .filter_map(|i| if *i == 0 { None } else { Some(*i as u8) })
                    .collect_vec(),
            )
        }
    }

    pub fn insert_mod_data(&mut self, idx: usize, mod_data: T) {
        unsafe {
            let mod_data = Arc::new(mod_data);
            let idx = idx % (*self.pjsip_inv_session).mod_data.len();
            if !(*self.pjsip_inv_session).mod_data[idx].is_null() {
                let old_mod_data = Arc::from_raw((*self.pjsip_inv_session).mod_data[idx]);
                drop(old_mod_data);
            }

            (*self.pjsip_inv_session).mod_data[idx] = Arc::into_raw(mod_data) as *mut c_void;
        }
    }

    pub fn get_mod_data(&self, idx: usize) -> Option<Arc<T>> {
        unsafe {
            if idx >= (*self.pjsip_inv_session).mod_data.len() {
                return None;
            }

            let data = (*self.pjsip_inv_session).mod_data[idx] as *mut T;
            if data.is_null() {
                return None;
            }
            let data = Arc::from_raw(data);
            let ret = data.clone();
            std::mem::forget(data);
            Some(ret)
        }
    }

    pub fn create_invite_req(&mut self) -> Result<PjSipTxData, Error> {
        PjSipTxData::inv_invite(&self)
    }

    pub fn send_msg(&mut self, tx_data: &mut PjSipTxData) -> Result<(), Error> {
        let status =
            unsafe { pj::pjsip_inv_send_msg(self.pjsip_inv_session, tx_data.as_mut_ptr()) };

        PjStatus::result_for_status(status)
    }

    pub fn end_session(&mut self) -> Result<(), Error> {
        let mut tx_data = PjSipTxData::inv_end_session(&self, None::<&CStr>)?;

        self.send_msg(&mut tx_data)?;

        Ok(())
    }

    pub fn get_state(&self) -> PjSipInvState {
        unsafe { ((*self.pjsip_inv_session).state as u8).into() }
    }

    pub fn get_active_local_neg_sdp(&self) -> Result<PjMediaSdpSessionRef, Error> {
        let mut sdp = unsafe { std::mem::zeroed() };
        let status = unsafe {
            pj::pjmedia_sdp_neg_get_active_local((*self.pjsip_inv_session).neg, &mut sdp)
        };

        PjStatus::result_for_status(status).map(|_| PjMediaSdpSessionRef::from(sdp))
    }

    pub fn get_active_remote_neg_sdp(&self) -> Result<PjMediaSdpSessionRef, Error> {
        let mut sdp = unsafe { std::mem::zeroed() };
        let status = unsafe {
            pj::pjmedia_sdp_neg_get_active_remote((*self.pjsip_inv_session).neg, &mut sdp)
        };

        PjStatus::result_for_status(status).map(|_| PjMediaSdpSessionRef::from(sdp))
    }

    pub fn stream_info_from_sdp(
        &self,
        media_endpt: &PjMediaEndpt,
        local_sdp: &PjMediaSdpSessionRef,
        remote_sdp: &PjMediaSdpSessionRef,
        stream_idx: u32,
    ) -> Result<PjMediaStreamInfo, Error> {
        PjMediaStreamInfo::from_sdp(&self, media_endpt, local_sdp, remote_sdp, stream_idx)
    }
}

impl<T> Drop for PjSipInvSession<T> {
    fn drop(&mut self) {
        println!("dropping PjSipInvSession");
        unsafe {
            for ptr in (*self.pjsip_inv_session).mod_data.iter_mut() {
                if !ptr.is_null() {
                    let mod_data = Arc::from_raw(*ptr as *mut T);
                    drop(mod_data);
                }
                *ptr = std::ptr::null_mut();
            }
        }
        println!("dropped PjSipInvSession");
    }
}

impl<T> From<*mut pj::pjsip_inv_session> for PjSipInvSession<T> {
    fn from(value: *mut pj::pjsip_inv_session) -> Self {
        Self {
            pjsip_inv_session: value,
            phantom: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum PjSipInvState {
    Null,
    Calling,
    Incoming,
    Early,
    Connecting,
    Confirmed,
    Disconnected,
    Unknown,
}

impl From<u8> for PjSipInvState {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Null,
            1 => Self::Calling,
            2 => Self::Incoming,
            3 => Self::Early,
            4 => Self::Connecting,
            5 => Self::Confirmed,
            6 => Self::Disconnected,
            _ => Self::Unknown,
        }
    }
}

impl Display for PjSipInvState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PjSipInvState::Null => "Null",
                PjSipInvState::Calling => "Calling",
                PjSipInvState::Incoming => "Incoming",
                PjSipInvState::Early => "Early",
                PjSipInvState::Connecting => "Connecting",
                PjSipInvState::Confirmed => "Confirmed",
                PjSipInvState::Disconnected => "Disconnected",
                PjSipInvState::Unknown => "Unknown",
            }
        )
    }
}

impl Debug for PjSipInvState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}[{}]", *self as u8)
    }
}

pub struct PjSipInvCallback<T> {
    pjsip_inv_callback: pj::pjsip_inv_callback,
    pub on_state_changed: Option<fn(PjSipInvSession<T>, PjSipEvent)>,
    pub on_media_update: Option<fn(PjSipInvSession<T>, PjStatus)>,
}

impl<T> Default for PjSipInvCallback<T> {
    fn default() -> Self {
        Self {
            pjsip_inv_callback: pj::pjsip_inv_callback {
                on_state_changed: None,
                on_new_session: None,
                on_tsx_state_changed: None,
                on_rx_offer: None,
                on_rx_offer2: None,
                on_rx_reinvite: None,
                on_create_offer: None,
                on_media_update: None,
                on_send_ack: None,
                on_redirected: None,
            },
            on_state_changed: None,
            on_media_update: None,
        }
    }
}

impl<T> PjSipInvCallback<T> {
    pub fn as_ptr(&self) -> *const pj::pjsip_inv_callback {
        &self.pjsip_inv_callback
    }

    pub fn with_on_state_changed<F>(&mut self, cb: F) -> &mut Self
    where
        F: Fn(&mut PjSipInvSession<T>, &mut PjSipEvent),
    {
        self.pjsip_inv_callback.on_state_changed = Some(Self::wrap_inv_evt(cb));

        self
    }

    pub fn with_on_new_session<F>(&mut self, cb: F) -> &mut Self
    where
        F: Fn(&mut PjSipInvSession<T>, &mut PjSipEvent),
    {
        self.pjsip_inv_callback.on_new_session = Some(Self::wrap_inv_evt(cb));

        self
    }

    pub fn with_on_media_update<F>(&mut self, cb: F) -> &mut Self
    where
        F: Fn(&PjSipInvSession<T>, PjStatus),
    {
        self.pjsip_inv_callback.on_media_update = Some(Self::wrap_inv_status(cb));

        self
    }

    fn wrap_inv_evt<F: Fn(&mut PjSipInvSession<T>, &mut PjSipEvent)>(
        _: F,
    ) -> unsafe extern "C" fn(inv: *mut pj::pjsip_inv_session, evt: *mut pj::pjsip_event) {
        assert!(std::mem::size_of::<F>() == 0);

        unsafe extern "C" fn wrapped<T, F: Fn(&mut PjSipInvSession<T>, &mut PjSipEvent)>(
            inv_ptr: *mut pj::pjsip_inv_session,
            evt_ptr: *mut pj::pjsip_event,
        ) {
            let mut inv = PjSipInvSession::from(inv_ptr);
            let mut evt = PjSipEvent::from(evt_ptr);
            std::mem::transmute::<_, &F>(&())(&mut inv, &mut evt);
            // @Todo(skf): with Re versions we wouldn't mem::forget
            std::mem::forget(inv);
            std::mem::forget(evt);
        }

        wrapped::<T, F>
    }

    fn wrap_inv_status<F: Fn(&PjSipInvSession<T>, PjStatus)>(
        _: F,
    ) -> unsafe extern "C" fn(inv: *mut pj::pjsip_inv_session, status: pj::pj_status_t) {
        assert!(std::mem::size_of::<F>() == 0);

        unsafe extern "C" fn wrapped<T, F: Fn(&PjSipInvSession<T>, PjStatus)>(
            inv_ptr: *mut pj::pjsip_inv_session,
            status: pj::pj_status_t,
        ) {
            let mut inv = PjSipInvSession::from(inv_ptr);
            let status = PjStatus::new(status);
            std::mem::transmute::<_, &F>(&())(&mut inv, status);
            std::mem::forget(inv);
        }

        wrapped::<T, F>
    }
}
