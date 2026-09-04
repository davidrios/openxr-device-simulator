use crate::{prelude::*, session::with_session};

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

    with_session(xr_session.into_raw(), |_session| {
        // No physical actuator to drive — accept the request as a no-op rather
        // than erroring, since apps commonly don't guard against a failed
        // haptics call and treat it as fatal.
        log::debug!("apply_feedback {info:?}, {header:?}");
        Ok(())
    })
    .into_xr_result()
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

    with_session(xr_session.into_raw(), |_session| {
        log::debug!("stop_feedback {info:?}");
        Ok(())
    })
    .into_xr_result()
}
