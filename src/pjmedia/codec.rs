use std::{ffi::CStr, fmt::Debug, marker::PhantomData};

use pjproject_sys as pj;

pub struct PjMediaCodec {
    codec: PjMediaCodecRef<'static>,
}

impl PjMediaCodec {
    pub fn as_ptr(&mut self) -> *const pj::pjmedia_codec_info {
        self.codec.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut pj::pjmedia_codec_info {
        self.as_ptr() as *mut _
    }
}

impl<'a> AsRef<PjMediaCodecRef<'a>> for PjMediaCodec {
    #[inline]
    fn as_ref(&self) -> &PjMediaCodecRef<'a> {
        &self.codec
    }
}

impl Debug for PjMediaCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.codec)
    }
}

pub struct PjMediaCodecRef<'a> {
    codec: *const pj::pjmedia_codec_info,
    phantom: PhantomData<&'a ()>,
}

impl<'a> PjMediaCodecRef<'a> {
    pub fn as_ptr(&self) -> *const pj::pjmedia_codec_info {
        self.codec
    }

    pub fn as_ref(&self) -> &pj::pjmedia_codec_info {
        unsafe { &*self.as_ptr() }
    }

    pub fn type_(&self) -> u32 {
        self.as_ref().type_
    }

    pub fn pt(&self) -> u32 {
        self.as_ref().pt
    }

    pub fn encoding_name(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.as_ref().encoding_name.ptr) }
    }

    pub fn clock_rate(&self) -> u32 {
        self.as_ref().clock_rate
    }

    pub fn channel(&self) -> u32 {
        self.as_ref().channel_cnt
    }
}

impl<'a> Debug for PjMediaCodecRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PjMediaCodecRef")
            .field("type_", &self.type_())
            .field("pt", &self.pt())
            .field("encoding_name", &self.encoding_name().to_string_lossy())
            .field("clock_rate", &self.clock_rate())
            .field("channel", &self.channel())
            .finish()
    }
}

impl<'a> From<&'a pj::pjmedia_codec_info> for PjMediaCodecRef<'a> {
    fn from(value: &pj::pjmedia_codec_info) -> Self {
        Self {
            codec: value as *const _,
            phantom: PhantomData,
        }
    }
}

impl<'a> From<*const pj::pjmedia_codec_info> for PjMediaCodecRef<'a> {
    fn from(value: *const pj::pjmedia_codec_info) -> Self {
        Self {
            codec: value,
            phantom: PhantomData,
        }
    }
}

impl<'a> From<*mut pj::pjmedia_codec_info> for PjMediaCodecRef<'a> {
    fn from(value: *mut pj::pjmedia_codec_info) -> Self {
        Self {
            codec: value,
            phantom: PhantomData,
        }
    }
}
