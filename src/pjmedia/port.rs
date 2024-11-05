use pjproject_sys as pj;

use crate::{Error, PjPool, PjStatus};

use super::PjMediaStream;

#[derive(Clone)]
pub struct PjMediaPort {
    port: *mut pj::pjmedia_port,
}

impl PjMediaPort {
    pub fn from_stream(stream: &PjMediaStream) -> Result<Self, Error> {
        let mut port = unsafe { std::mem::zeroed() };
        let status = unsafe { pj::pjmedia_stream_get_port(stream.as_mut_ptr(), &mut port) };

        PjStatus::result_for_status(status).map(|_| Self { port })
    }

    pub fn as_ptr(&self) -> *const pj::pjmedia_port {
        self.port
    }

    pub fn as_mut_ptr(&mut self) -> *mut pj::pjmedia_port {
        self.port
    }

    pub fn as_ref(&self) -> &pj::pjmedia_port {
        unsafe { &*self.as_ptr() }
    }

    pub fn as_mut(&mut self) -> &mut pj::pjmedia_port {
        unsafe { &mut *self.as_mut_ptr() }
    }

    pub fn clock_rate(&self) -> u32 {
        unsafe { self.as_ref().info.fmt.det.aud.clock_rate }
    }

    pub fn channel_count(&self) -> u32 {
        unsafe { self.as_ref().info.fmt.det.aud.channel_count }
    }

    pub fn bits_per_sample(&self) -> u32 {
        unsafe { self.as_ref().info.fmt.det.aud.bits_per_sample }
    }

    pub fn frame_time_usec(&self) -> u32 {
        unsafe { self.as_ref().info.fmt.det.aud.frame_time_usec }
    }

    pub fn stereo(
        pool: &mut PjPool,
        dn_port: &mut PjMediaPort,
        channel_cnt: u32,
        options: u32,
    ) -> Result<Self, Error> {
        let mut port = unsafe { std::mem::zeroed() };
        let status = unsafe {
            pj::pjmedia_stereo_port_create(
                pool.as_mut_ptr(),
                dn_port.as_mut_ptr(),
                channel_cnt,
                options,
                &mut port,
            )
        };

        PjStatus::result_for_status(status).map(|_| Self { port })
    }

    pub fn resample(
        pool: &mut PjPool,
        dn_port: &mut PjMediaPort,
        clock_rate: u32,
        options: u32,
    ) -> Result<Self, Error> {
        let mut port = unsafe { std::mem::zeroed() };
        let status = unsafe {
            pj::pjmedia_resample_port_create(
                pool.as_mut_ptr(),
                dn_port.as_mut_ptr(),
                clock_rate,
                options,
                &mut port,
            )
        };

        PjStatus::result_for_status(status).map(|_| Self { port })
    }
}

impl From<*mut pj::pjmedia_port> for PjMediaPort {
    fn from(value: *mut pj::pjmedia_port) -> Self {
        Self { port: value }
    }
}
