use std::ffi;

use thiserror::Error;

use crate::PjStatus;

#[derive(Debug, Error, Clone)]
pub enum Error {
    #[error("{0}")]
    PjError(PjStatus),
    #[error("{0}")]
    CStringNul(#[from] ffi::NulError),
    #[error("{0}")]
    Validation(String),
}
