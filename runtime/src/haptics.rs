use openxr_sys as xr;

use crate::{error::to_xr_result, with_session};

pub extern "system" fn apply_feedback(
    xr_session: xr::Session,
    info: *const xr::HapticActionInfo,
    header: *const xr::HapticBaseHeader,
) -> xr::Result {
    if info.is_null() || header.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (info, header) = unsafe { (&*info, &*header) };

    if info.ty != xr::StructureType::HAPTIC_ACTION_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    to_xr_result(with_session!(xr_session, |_session| {
        log::debug!("apply_feedback {info:?}, {header:?}");
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
        Ok(())
    }))
}

pub extern "system" fn stop_feedback(
    xr_session: xr::Session,
    info: *const xr::HapticActionInfo,
) -> xr::Result {
    if info.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let info = unsafe { &*info };

    if info.ty != xr::StructureType::HAPTIC_ACTION_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    to_xr_result(with_session!(xr_session, |_session| {
        log::debug!("stop_feedback {info:?}");
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
        Ok(())
    }))
}
