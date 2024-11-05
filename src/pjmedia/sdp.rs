use std::{
    ffi::{CStr, CString},
    fmt::Display,
};

use pjproject_sys as pj;

use crate::{Error, PjStatus, PjTimeVal};

pub struct PjMediaSdpSession {
    sdp_session: PjMediaSdpSessionRef,
}

impl PjMediaSdpSession {
    pub fn new<S: AsRef<CStr>, T: AsRef<CStr>, U: AsRef<CStr>, V: AsRef<CStr>, W: AsRef<CStr>>(
        origin_user: S,
        name: Option<W>,
        version: Option<u64>,
        net_type: Option<T>,
        addr_type: Option<U>,
        addr: Option<V>,
        sdp_conn: Option<PjMediaSdpConn>,
        sdp_media: Option<Vec<PjMediaSdpMedia>>,
    ) -> Result<Self, Error> {
        let name = name
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or(c"pjsip")
            .to_owned();
        let origin_user = origin_user.as_ref().to_owned();
        let version = version.unwrap_or(PjTimeVal::timeofday()?.sec() as u64 + 2_208_988_800u64);
        let net_type = net_type
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or(c"IN")
            .to_owned();
        let addr_type = addr_type
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or(c"IP4")
            .to_owned();
        let addr = addr
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or(unsafe {
                let hostname = pj::pj_gethostname();
                CStr::from_ptr((*hostname).ptr)
            })
            .to_owned();

        /* Create and initialize basic SDP session */
        let sdp = Box::new(unsafe { std::mem::zeroed::<pj::pjmedia_sdp_session>() });
        let sdp = Box::into_raw(sdp);
        unsafe {
            (*sdp).name = pj::pj_str(name.into_raw());
            (*sdp).origin.user = pj::pj_str(origin_user.into_raw());
            (*sdp).origin.version = version;
            (*sdp).origin.id = version;
            (*sdp).origin.net_type = pj::pj_str(net_type.into_raw());
            (*sdp).origin.addr_type = pj::pj_str(addr_type.into_raw());
            (*sdp).origin.addr = pj::pj_str(addr.into_raw());
        }

        /* Since we only support one media stream at present, put the
         * SDP connection line in the session level.
         */
        if let Some(sdp_conn) = sdp_conn {
            unsafe {
                (*sdp).conn = sdp_conn.into_raw();
            }
        }

        /* Create media stream 0: */
        if let Some(sdp_media) = sdp_media {
            unsafe {
                let media_count = std::cmp::min((*sdp).media.len(), sdp_media.len());
                (*sdp).media_count = media_count as _;
                for (i, media) in sdp_media.into_iter().take(media_count).enumerate() {
                    (*sdp).media[i] = media.into_raw();
                }
            }
        }

        Ok(Self {
            sdp_session: PjMediaSdpSessionRef::from(sdp),
        })
    }

    pub fn as_mut_ptr(&mut self) -> *mut pj::pjmedia_sdp_session {
        self.as_ptr() as *mut _
    }

    pub fn builder() -> PjMediaSdpSessionBuilder {
        PjMediaSdpSessionBuilder::default()
    }
}

impl Drop for PjMediaSdpSession {
    fn drop(&mut self) {
        unsafe {
            let sdp_session = Box::from_raw(self.as_mut_ptr());
            let name = CString::from_raw(sdp_session.name.ptr);
            let origin_user = CString::from_raw(sdp_session.origin.user.ptr);
            let net_type = CString::from_raw(sdp_session.origin.net_type.ptr);
            let addr_type = CString::from_raw(sdp_session.origin.addr_type.ptr);
            let addr = CString::from_raw(sdp_session.origin.addr.ptr);

            if !sdp_session.conn.is_null() {
                let sdp_conn = PjMediaSdpConn::from_raw(sdp_session.conn);
                drop(sdp_conn);
            }
            for m in sdp_session.media.iter().take(sdp_session.media_count as _) {
                let media = PjMediaSdpMedia::from_raw(*m);
                drop(media);
            }
            drop(sdp_session);
            drop(name);
            drop(origin_user);
            drop(net_type);
            drop(addr_type);
            drop(addr);
        }
    }
}

impl AsRef<PjMediaSdpSessionRef> for PjMediaSdpSession {
    #[inline]
    fn as_ref(&self) -> &PjMediaSdpSessionRef {
        &self.sdp_session
    }
}

impl AsMut<PjMediaSdpSessionRef> for PjMediaSdpSession {
    #[inline]
    fn as_mut(&mut self) -> &mut PjMediaSdpSessionRef {
        &mut self.sdp_session
    }
}

impl std::ops::Deref for PjMediaSdpSession {
    type Target = PjMediaSdpSessionRef;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl std::borrow::Borrow<PjMediaSdpSessionRef> for PjMediaSdpSession {
    #[inline]
    fn borrow(&self) -> &PjMediaSdpSessionRef {
        &self.sdp_session
    }
}

pub struct PjMediaSdpSessionRef {
    sdp_session: *const pj::pjmedia_sdp_session,
}

impl PjMediaSdpSessionRef {
    pub fn as_ptr(&self) -> *const pj::pjmedia_sdp_session {
        self.sdp_session
    }

    pub fn as_ref(&self) -> &pj::pjmedia_sdp_session {
        unsafe { &*self.as_ptr() }
    }

    pub fn media(&self) -> Vec<PjMediaSdpMediaRef> {
        self.as_ref()
            .media
            .iter()
            .take(self.as_ref().media_count as _)
            .map(|m| PjMediaSdpMediaRef::from(*m))
            .collect()
    }
}

impl From<&pj::pjmedia_sdp_session> for PjMediaSdpSessionRef {
    fn from(value: &pj::pjmedia_sdp_session) -> Self {
        Self {
            sdp_session: value as *const _,
        }
    }
}

impl From<*const pj::pjmedia_sdp_session> for PjMediaSdpSessionRef {
    fn from(value: *const pj::pjmedia_sdp_session) -> Self {
        Self { sdp_session: value }
    }
}

impl From<*mut pj::pjmedia_sdp_session> for PjMediaSdpSessionRef {
    fn from(value: *mut pj::pjmedia_sdp_session) -> Self {
        Self { sdp_session: value }
    }
}

#[derive(Default)]
pub struct PjMediaSdpSessionBuilder {
    origin_user: Option<CString>,
    name: Option<CString>,
    version: Option<u64>,
    net_type: Option<CString>,
    addr_type: Option<CString>,
    addr: Option<CString>,
    sdp_conn: Option<PjMediaSdpConn>,
    sdp_media: Option<Vec<PjMediaSdpMedia>>,
}

impl PjMediaSdpSessionBuilder {
    pub fn origin_user<S: AsRef<CStr>>(&mut self, origin_user: S) -> &mut Self {
        self.origin_user.replace(origin_user.as_ref().to_owned());
        self
    }

    pub fn name<S: AsRef<CStr>>(&mut self, name: S) -> &mut Self {
        self.name.replace(name.as_ref().to_owned());
        self
    }

    pub fn version(&mut self, version: u64) -> &mut Self {
        self.version.replace(version);
        self
    }

    pub fn net_type<S: AsRef<CStr>>(&mut self, net_type: S) -> &mut Self {
        self.net_type.replace(net_type.as_ref().to_owned());
        self
    }

    pub fn addr_type<S: AsRef<CStr>>(&mut self, addr_type: S) -> &mut Self {
        self.addr_type.replace(addr_type.as_ref().to_owned());
        self
    }

    pub fn addr<S: AsRef<CStr>>(&mut self, addr: S) -> &mut Self {
        self.addr.replace(addr.as_ref().to_owned());
        self
    }

    pub fn sdp_conn(&mut self, sdp_conn: PjMediaSdpConn) -> &mut Self {
        self.sdp_conn.replace(sdp_conn);
        self
    }

    pub fn sdp_media(&mut self, sdp_media: Vec<PjMediaSdpMedia>) -> &mut Self {
        self.sdp_media.replace(sdp_media);
        self
    }

    pub fn build(&mut self) -> Result<PjMediaSdpSession, Error> {
        PjMediaSdpSession::new(
            self.origin_user
                .as_ref()
                .map(|u| u.as_c_str())
                .unwrap_or(c"pjsip"),
            self.name.as_ref().map(|u| u.as_c_str()),
            self.version,
            self.net_type.as_ref().map(|u| u.as_c_str()),
            self.addr_type.as_ref().map(|u| u.as_c_str()),
            self.addr.as_ref().map(|u| u.as_c_str()),
            self.sdp_conn.take(),
            self.sdp_media.take(),
        )
    }
}

pub struct PjMediaSdpConn {
    sdp_conn: PjMediaSdpConnRef,
    drop: bool,
}

impl PjMediaSdpConn {
    pub fn new<T: AsRef<CStr>, U: AsRef<CStr>, V: AsRef<CStr>>(
        net_type: T,
        addr_type: U,
        addr: V,
        multicast_ttl: u8,
        mulitcast_num_addrs: u8,
    ) -> Self {
        let net_type = net_type.as_ref().to_owned();
        let addr_type = addr_type.as_ref().to_owned();
        let addr = addr.as_ref().to_owned();
        let sdp_conn = Box::new(unsafe {
            pj::pjmedia_sdp_conn {
                net_type: pj::pj_str(net_type.into_raw()),
                addr_type: pj::pj_str(addr_type.into_raw()),
                addr: pj::pj_str(addr.into_raw()),
                ttl: multicast_ttl,
                no_addr: mulitcast_num_addrs,
            }
        });

        Self {
            sdp_conn: PjMediaSdpConnRef::from(Box::into_raw(sdp_conn)),
            drop: true,
        }
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pjmedia_sdp_conn {
        self.as_ptr() as *mut _
    }

    pub fn into_raw(self) -> *mut pj::pjmedia_sdp_conn {
        let mut s = self;
        s.drop = false;
        s.as_mut_ptr()
    }

    pub unsafe fn from_raw(s: *mut pj::pjmedia_sdp_conn) -> Self {
        Self {
            sdp_conn: PjMediaSdpConnRef::from(s),
            drop: true,
        }
    }
}

impl Drop for PjMediaSdpConn {
    fn drop(&mut self) {
        if self.drop {
            unsafe {
                let sdp_conn = Box::from_raw(self.as_mut_ptr());
                let net_type = CString::from_raw(sdp_conn.net_type.ptr);
                let addr_type = CString::from_raw(sdp_conn.addr_type.ptr);
                let addr = CString::from_raw(sdp_conn.addr.ptr);
                drop(net_type);
                drop(addr_type);
                drop(addr);
                drop(sdp_conn);
            }
        }
    }
}

impl AsRef<PjMediaSdpConnRef> for PjMediaSdpConn {
    #[inline]
    fn as_ref(&self) -> &PjMediaSdpConnRef {
        &self.sdp_conn
    }
}

impl AsMut<PjMediaSdpConnRef> for PjMediaSdpConn {
    #[inline]
    fn as_mut(&mut self) -> &mut PjMediaSdpConnRef {
        &mut self.sdp_conn
    }
}

impl std::ops::Deref for PjMediaSdpConn {
    type Target = PjMediaSdpConnRef;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

pub struct PjMediaSdpConnRef {
    sdp_conn: *const pj::pjmedia_sdp_conn,
}

impl PjMediaSdpConnRef {
    pub fn as_ptr(&self) -> *const pj::pjmedia_sdp_conn {
        self.sdp_conn
    }
}

impl From<&pj::pjmedia_sdp_conn> for PjMediaSdpConnRef {
    fn from(value: &pj::pjmedia_sdp_conn) -> Self {
        Self {
            sdp_conn: value as *const _,
        }
    }
}

impl From<*const pj::pjmedia_sdp_conn> for PjMediaSdpConnRef {
    fn from(value: *const pj::pjmedia_sdp_conn) -> Self {
        Self { sdp_conn: value }
    }
}

impl From<*mut pj::pjmedia_sdp_conn> for PjMediaSdpConnRef {
    fn from(value: *mut pj::pjmedia_sdp_conn) -> Self {
        Self { sdp_conn: value }
    }
}

pub enum PjMediaSdpMediaType {
    Audio,
    Video,
    UnSupported,
}

impl From<&CStr> for PjMediaSdpMediaType {
    fn from(value: &CStr) -> Self {
        match value.as_ref() {
            v if v == c"audio" => Self::Audio,
            v if v == c"video" => Self::Video,
            _ => Self::UnSupported,
        }
    }
}

impl AsRef<CStr> for PjMediaSdpMediaType {
    fn as_ref(&self) -> &CStr {
        match self {
            PjMediaSdpMediaType::Audio => c"audio",
            PjMediaSdpMediaType::Video => c"video",
            PjMediaSdpMediaType::UnSupported => c"",
        }
    }
}

pub struct PjMediaSdpMedia {
    sdp_media: PjMediaSdpMediaRef,
    drop: bool,
}

impl PjMediaSdpMedia {
    pub fn new<S: AsRef<CStr>, T: AsRef<CStr>>(
        desc_media: PjMediaSdpMediaType,
        desc_transport: T,
        port: u16,
        port_count: u32,
        fmt: &[S],
        attr: Vec<PjMediaSdpAttr>,
    ) -> Self {
        let mut sdp_media = Box::new(unsafe { std::mem::zeroed::<pj::pjmedia_sdp_media>() });
        let desc_media = desc_media.as_ref().to_owned();
        let desc_transport = desc_transport.as_ref().as_ref().to_owned();

        unsafe {
            sdp_media.desc.media = pj::pj_str(desc_media.into_raw());
            sdp_media.desc.transport = pj::pj_str(desc_transport.into_raw());
        }
        sdp_media.desc.port = port;
        sdp_media.desc.port_count = port_count;

        let fmt_count = std::cmp::min(fmt.len(), sdp_media.desc.fmt.len());
        sdp_media.desc.fmt_count = fmt_count as _;
        let desc_fmts = (0..fmt_count).map(|i| fmt[i].as_ref()).collect::<Vec<_>>();
        for (i, fmt) in desc_fmts.into_iter().enumerate() {
            (*sdp_media).desc.fmt[i] = unsafe { pj::pj_str(fmt.as_ref().to_owned().into_raw()) };
        }

        let attr_count = std::cmp::min(attr.len(), sdp_media.attr.len());
        sdp_media.attr_count = attr_count as _;
        for (i, attr) in attr.into_iter().enumerate() {
            sdp_media.attr[i] = attr.into_raw();
        }

        let sdp_media = PjMediaSdpMediaRef::from(Box::into_raw(sdp_media));

        Self {
            sdp_media,
            drop: true,
        }
    }

    pub fn as_ptr(&self) -> *const pj::pjmedia_sdp_media {
        self.sdp_media.as_ptr()
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pjmedia_sdp_media {
        self.as_ptr() as *mut _
    }

    pub fn into_raw(self) -> *mut pj::pjmedia_sdp_media {
        let mut s = self;
        s.drop = false;
        let ptr = s.as_mut_ptr();
        ptr
    }

    pub unsafe fn from_raw(s: *mut pj::pjmedia_sdp_media) -> Self {
        Self {
            sdp_media: PjMediaSdpMediaRef::from(s),
            drop: true,
        }
    }
}

impl Drop for PjMediaSdpMedia {
    fn drop(&mut self) {
        if self.drop {
            unsafe {
                let mut sdp_media = Box::from_raw(self.as_mut_ptr());
                let desc_media = CString::from_raw(sdp_media.desc.media.ptr);
                let desc_transport = CString::from_raw(sdp_media.desc.transport.ptr);
                for fmt in sdp_media
                    .desc
                    .fmt
                    .iter_mut()
                    .take(sdp_media.desc.fmt_count as _)
                {
                    let desc_fmt = CString::from_raw(fmt.ptr);
                    drop(desc_fmt);
                }
                for attr in sdp_media.attr.iter_mut().take(sdp_media.attr_count as _) {
                    let attr = PjMediaSdpAttr::from_raw(*attr);
                    drop(attr);
                }
                drop(desc_media);
                drop(desc_transport);
                drop(sdp_media);
            }
        }
    }
}

impl AsRef<PjMediaSdpMediaRef> for PjMediaSdpMedia {
    #[inline]
    fn as_ref(&self) -> &PjMediaSdpMediaRef {
        &self.sdp_media
    }
}

impl AsMut<PjMediaSdpMediaRef> for PjMediaSdpMedia {
    #[inline]
    fn as_mut(&mut self) -> &mut PjMediaSdpMediaRef {
        &mut self.sdp_media
    }
}

pub struct PjMediaSdpMediaRef {
    sdp_media: *const pj::pjmedia_sdp_media,
}

impl PjMediaSdpMediaRef {
    pub fn as_ptr(&self) -> *const pj::pjmedia_sdp_media {
        self.sdp_media
    }

    pub fn as_ref(&self) -> &pj::pjmedia_sdp_media {
        unsafe { &*self.as_ptr() }
    }

    pub fn desc_port(&self) -> u16 {
        self.as_ref().desc.port
    }
}

impl From<&pj::pjmedia_sdp_media> for PjMediaSdpMediaRef {
    fn from(value: &pj::pjmedia_sdp_media) -> Self {
        Self {
            sdp_media: value as *const _,
        }
    }
}

impl From<*const pj::pjmedia_sdp_media> for PjMediaSdpMediaRef {
    fn from(value: *const pj::pjmedia_sdp_media) -> Self {
        Self { sdp_media: value }
    }
}

impl From<*mut pj::pjmedia_sdp_media> for PjMediaSdpMediaRef {
    fn from(value: *mut pj::pjmedia_sdp_media) -> Self {
        Self { sdp_media: value }
    }
}

pub struct PjMediaSdpRtpMap {
    rtpmap: PjMediaSdpRtpMapRef,
}

impl PjMediaSdpRtpMap {
    pub fn new<S: AsRef<CStr>, T: AsRef<CStr>, U: AsRef<CStr>>(
        pt: S,
        enc_name: T,
        clock_rate: u32,
        param: Option<U>,
    ) -> Self {
        let pt = pt.as_ref().to_owned();
        let enc_name = enc_name.as_ref().to_owned();
        let param = param.as_ref().map(|p| p.as_ref().to_owned());

        let mut rtpmap = Box::new(unsafe { std::mem::zeroed::<pj::pjmedia_sdp_rtpmap>() });
        unsafe {
            rtpmap.pt = pj::pj_str(pt.into_raw());
            rtpmap.enc_name = pj::pj_str(enc_name.into_raw());
            rtpmap.clock_rate = clock_rate;
            if let Some(p) = param {
                rtpmap.param = pj::pj_str(p.into_raw());
            }
        }

        Self {
            rtpmap: PjMediaSdpRtpMapRef::from(Box::into_raw(rtpmap)),
        }
    }

    pub fn as_ptr(&self) -> *const pj::pjmedia_sdp_rtpmap {
        self.rtpmap.as_ptr()
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pjmedia_sdp_rtpmap {
        self.as_ptr() as *mut _
    }

    pub fn to_attr(&self) -> Result<PjMediaSdpAttr, Error> {
        let mut pool = crate::PjPool::default_with_name(c"sdp_attr");
        let mut attr = unsafe { std::mem::zeroed() };
        let status = unsafe {
            pj::pjmedia_sdp_rtpmap_to_attr(pool.as_mut_ptr(), self.rtpmap.as_ptr(), &mut attr)
        };

        PjStatus::result_for_status(status).map(|_| unsafe {
            PjMediaSdpAttr::new(
                CStr::from_ptr((*attr).name.ptr),
                CStr::from_ptr((*attr).value.ptr),
            )
        })
    }
}

impl Drop for PjMediaSdpRtpMap {
    fn drop(&mut self) {
        unsafe {
            let rtpmap = Box::from_raw(self.as_mut_ptr());
            let pt = CString::from_raw(rtpmap.pt.ptr);
            let enc_name = CString::from_raw(rtpmap.enc_name.ptr);
            if !rtpmap.param.ptr.is_null() {
                let param = CString::from_raw(rtpmap.param.ptr);
                drop(param);
            }

            drop(pt);
            drop(enc_name);
            drop(rtpmap);
        }
    }
}

impl AsRef<PjMediaSdpRtpMapRef> for PjMediaSdpRtpMap {
    #[inline]
    fn as_ref(&self) -> &PjMediaSdpRtpMapRef {
        &self.rtpmap
    }
}

impl AsMut<PjMediaSdpRtpMapRef> for PjMediaSdpRtpMap {
    #[inline]
    fn as_mut(&mut self) -> &mut PjMediaSdpRtpMapRef {
        &mut self.rtpmap
    }
}

pub struct PjMediaSdpRtpMapRef {
    rtpmap: *const pj::pjmedia_sdp_rtpmap,
}

impl PjMediaSdpRtpMapRef {
    pub fn as_ptr(&self) -> *const pj::pjmedia_sdp_rtpmap {
        self.rtpmap
    }
}

impl From<&pj::pjmedia_sdp_rtpmap> for PjMediaSdpRtpMapRef {
    fn from(value: &pj::pjmedia_sdp_rtpmap) -> Self {
        Self {
            rtpmap: value as *const _,
        }
    }
}

impl From<*const pj::pjmedia_sdp_rtpmap> for PjMediaSdpRtpMapRef {
    fn from(value: *const pj::pjmedia_sdp_rtpmap) -> Self {
        Self { rtpmap: value }
    }
}

impl From<*mut pj::pjmedia_sdp_rtpmap> for PjMediaSdpRtpMapRef {
    fn from(value: *mut pj::pjmedia_sdp_rtpmap) -> Self {
        Self { rtpmap: value }
    }
}

pub struct PjMediaSdpAttr {
    attr: PjMediaSdpAttrRef,
    drop: bool,
}

impl PjMediaSdpAttr {
    pub fn new<S: AsRef<CStr>, T: AsRef<CStr>>(name: S, value: T) -> Self {
        let name = name.as_ref().to_owned();
        let value = value.as_ref().to_owned();

        let attr = Box::new(unsafe {
            pj::pjmedia_sdp_attr {
                name: pj::pj_str(name.into_raw()),
                value: pj::pj_str(value.into_raw()),
            }
        });

        Self {
            attr: PjMediaSdpAttrRef::from(Box::into_raw(attr)),
            drop: true,
        }
    }

    pub fn as_ptr(&self) -> *const pj::pjmedia_sdp_attr {
        self.attr.as_ptr()
    }

    pub fn as_mut_ptr(&self) -> *mut pj::pjmedia_sdp_attr {
        self.as_ptr() as *mut _
    }

    pub fn into_raw(self) -> *mut pj::pjmedia_sdp_attr {
        let mut s = self;
        s.drop = false;
        s.as_mut_ptr()
    }

    pub unsafe fn from_raw(s: *mut pj::pjmedia_sdp_attr) -> Self {
        Self {
            attr: PjMediaSdpAttrRef::from(s),
            drop: true,
        }
    }
}

impl Drop for PjMediaSdpAttr {
    fn drop(&mut self) {
        if self.drop {
            unsafe {
                let sdp_attr = Box::from_raw(self.as_mut_ptr());
                let name = CString::from_raw(sdp_attr.name.ptr);
                let value = CString::from_raw(sdp_attr.value.ptr);
                drop(name);
                drop(value);
                drop(sdp_attr);
            }
        }
    }
}

impl AsRef<PjMediaSdpAttrRef> for PjMediaSdpAttr {
    #[inline]
    fn as_ref(&self) -> &PjMediaSdpAttrRef {
        &self.attr
    }
}

impl AsMut<PjMediaSdpAttrRef> for PjMediaSdpAttr {
    #[inline]
    fn as_mut(&mut self) -> &mut PjMediaSdpAttrRef {
        &mut self.attr
    }
}

impl Display for PjMediaSdpAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

pub struct PjMediaSdpAttrRef {
    attr: *const pj::pjmedia_sdp_attr,
}

impl PjMediaSdpAttrRef {
    pub fn as_ptr(&self) -> *const pj::pjmedia_sdp_attr {
        self.attr
    }

    pub fn as_ref(&self) -> &pj::pjmedia_sdp_attr {
        unsafe { &*self.as_ptr() }
    }
}

impl From<&pj::pjmedia_sdp_attr> for PjMediaSdpAttrRef {
    fn from(value: &pj::pjmedia_sdp_attr) -> Self {
        Self {
            attr: value as *const _,
        }
    }
}

impl From<*const pj::pjmedia_sdp_attr> for PjMediaSdpAttrRef {
    fn from(value: *const pj::pjmedia_sdp_attr) -> Self {
        Self { attr: value }
    }
}

impl From<*mut pj::pjmedia_sdp_attr> for PjMediaSdpAttrRef {
    fn from(value: *mut pj::pjmedia_sdp_attr) -> Self {
        Self { attr: value }
    }
}

impl Display for PjMediaSdpAttrRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            write!(
                f,
                "name[\"{}\"] -> value[\"{}\"]",
                CStr::from_ptr(self.as_ref().name.ptr).to_string_lossy(),
                CStr::from_ptr(self.as_ref().value.ptr).to_string_lossy()
            )
        }
    }
}
