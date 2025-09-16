use openxr_sys as xr;
use std::sync::PoisonError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Generic error: {0}")]
    Generic(String),

    #[error("Sync error: {0}")]
    SyncError(String),

    #[error("ExpectedSome error: {0}")]
    ExpectedSome(String),

    #[error("XrResult error: {0}")]
    XrResult(xr::Result),

    #[error("Utf8Error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self::Generic(value.into())
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(value: PoisonError<T>) -> Self {
        Self::SyncError(format!("Poison error: {value}"))
    }
}

impl From<xr::Result> for Error {
    fn from(value: xr::Result) -> Self {
        Self::XrResult(value)
    }
}

impl From<Error> for xr::Result {
    fn from(err: Error) -> Self {
        log::error!("error: {err}");
        match err {
            Error::XrResult(res) => res,
            _ => Self::ERROR_RUNTIME_FAILURE,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn to_xr_result<T>(value: Result<T>) -> xr::Result {
    match value {
        Ok(_) => xr::Result::SUCCESS,
        Err(err) => err.into(),
    }
}
