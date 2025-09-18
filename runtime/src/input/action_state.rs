use openxr_sys as xr;

use crate::{error::to_xr_result, with_session};

pub extern "system" fn get_boolean(
    xr_session: xr::Session,
    info: *const xr::ActionStateGetInfo,
    state: *mut xr::ActionStateBoolean,
) -> xr::Result {
    if info.is_null() || state.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (info, _state) = unsafe { (&*info, &mut *state) };

    to_xr_result(with_session!(xr_session, |_session| {
        log::debug!("get_boolean {info:?}");
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
        Ok(())
    }))
}

pub extern "system" fn get_float(
    xr_session: xr::Session,
    info: *const xr::ActionStateGetInfo,
    state: *mut xr::ActionStateFloat,
) -> xr::Result {
    if info.is_null() || state.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (info, _state) = unsafe { (&*info, &mut *state) };

    to_xr_result(with_session!(xr_session, |_session| {
        log::debug!("get_float {info:?}");
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
        Ok(())
    }))
}

pub extern "system" fn get_vector2f(
    xr_session: xr::Session,
    info: *const xr::ActionStateGetInfo,
    state: *mut xr::ActionStateVector2f,
) -> xr::Result {
    if info.is_null() || state.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (info, _state) = unsafe { (&*info, &mut *state) };

    to_xr_result(with_session!(xr_session, |_session| {
        log::debug!("get_vector2f {info:?}");
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
        Ok(())
    }))
}

pub extern "system" fn get_pose(
    xr_session: xr::Session,
    info: *const xr::ActionStateGetInfo,
    state: *mut xr::ActionStatePose,
) -> xr::Result {
    if info.is_null() || state.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (info, _state) = unsafe { (&*info, &mut *state) };

    to_xr_result(with_session!(xr_session, |_session| {
        log::debug!("get_pose {info:?}");
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
        Ok(())
    }))
}

pub extern "system" fn sync_actions(
    xr_session: xr::Session,
    info: *const xr::ActionsSyncInfo,
) -> xr::Result {
    if info.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let info = unsafe { &*info };

    to_xr_result(with_session!(xr_session, |_session| {
        log::debug!("sync_actions {info:?}");
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
        Ok(())
    }))
}
