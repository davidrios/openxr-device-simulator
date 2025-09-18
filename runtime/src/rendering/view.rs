use openxr_sys as xr;

use crate::{error::to_xr_result, with_session};

pub extern "system" fn locate_views(
    xr_session: xr::Session,
    info: *const xr::ViewLocateInfo,
    view_state: *mut xr::ViewState,
    _capacity_in: u32,
    count_out: *mut u32,
    views: *mut xr::View,
) -> xr::Result {
    log::debug!("enumerate_blend_modes: {:?}", info);

    if info.is_null() || view_state.is_null() || count_out.is_null() || views.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let info = unsafe { &*info };

    to_xr_result(with_session!(xr_session, |_session| {
        log::debug!("locate_views {info:?}");
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
        Ok(())
    }))
}
