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

    #[error("VkResult error: {0}")]
    VkResult(ash::vk::Result),

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

impl From<ash::vk::Result> for Error {
    fn from(value: ash::vk::Result) -> Self {
        Self::VkResult(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait IntoXrSuccess {
    fn into_xr_success(self) -> xr::Result;
}

impl IntoXrSuccess for xr::Result {
    fn into_xr_success(self) -> xr::Result {
        self
    }
}

impl IntoXrSuccess for () {
    fn into_xr_success(self) -> xr::Result {
        xr::Result::SUCCESS
    }
}

pub trait IntoXrResult {
    fn into_xr_result(self) -> xr::Result;
}

impl<T: IntoXrSuccess> IntoXrResult for Result<T> {
    fn into_xr_result(self) -> xr::Result {
        match self {
            Ok(res) => res.into_xr_success(),
            Err(err) => err.into(),
        }
    }
}
