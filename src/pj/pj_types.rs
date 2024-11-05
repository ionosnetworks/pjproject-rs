use std::ops::{Add, Sub};

use pjproject_sys as pj;

use crate::{Error, PjStatus};

#[derive(Debug, Copy, Clone)]
pub struct PjTimeVal(pub(crate) pj::pj_time_val);

impl PjTimeVal {
    pub fn new(sec: i64, msec: i64) -> Self {
        Self(pj::pj_time_val { sec, msec })
    }
    pub fn sec(&self) -> i64 {
        self.0.sec
    }

    pub fn msec(&self) -> i64 {
        self.0.msec
    }

    pub fn set(&mut self, sec: i64, msec: i64) {
        self.0.sec = sec;
        self.0.msec = msec;
        self.normalize();
    }

    pub fn set_sec(&mut self, sec: i64) {
        self.0.sec = sec;
        self.normalize();
    }

    pub fn set_msec(&mut self, msec: i64) {
        self.0.msec = msec;
        self.normalize();
    }

    pub fn clear(&mut self) {
        self.0.msec = 0;
        self.0.sec = 0;
    }

    pub fn in_sec(&self) -> i64 {
        self.sec() + self.msec() / 1000
    }

    pub fn in_msec(&self) -> i64 {
        self.sec() * 1000 + self.msec()
    }

    pub fn timeofday() -> Result<Self, Error> {
        let mut tv = unsafe { std::mem::zeroed::<pj::pj_time_val>() };

        let status = unsafe { pj::pj_gettimeofday(&mut tv) };

        PjStatus::result_for_status(status).map(|_| Self(tv))
    }

    pub fn normalize(&mut self) {
        unsafe {
            pj::pj_time_val_normalize(&mut self.0);
        };
    }
}

impl Default for PjTimeVal {
    fn default() -> Self {
        Self(pj::pj_time_val {
            sec: Default::default(),
            msec: Default::default(),
        })
    }
}

impl Add for PjTimeVal {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        let mut t = pj::pj_time_val {
            sec: self.sec() + other.sec(),
            msec: self.msec() + other.msec(),
        };
        unsafe {
            pj::pj_time_val_normalize(&mut t);
        };

        Self(t)
    }
}

impl Sub for PjTimeVal {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        let mut t = pj::pj_time_val {
            sec: self.sec() - other.sec(),
            msec: self.msec() - other.msec(),
        };
        unsafe {
            pj::pj_time_val_normalize(&mut t);
        };

        Self(t)
    }
}
