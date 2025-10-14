use crate::{prelude::*, session::with_session};

#[allow(unreachable_code)]
pub extern "system" fn locate_views(
    xr_session: xr::Session,
    info: *const xr::ViewLocateInfo,
    view_state: *mut xr::ViewState,
    capacity_in: u32,
    count_out: *mut u32,
    views: *mut xr::View,
) -> xr::Result {
    if info.is_null() || view_state.is_null() || count_out.is_null() || views.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let info = unsafe { &*info };

    if !matches!(
        info.view_configuration_type,
        xr::ViewConfigurationType::PRIMARY_STEREO
    ) {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    if capacity_in != 2 {
        return xr::Result::ERROR_SIZE_INSUFFICIENT;
    }

    if views.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    unsafe { *count_out = 2 }

    log::debug!("locate_views {info:?}");

    with_session(xr_session.into_raw(), |session| {
        if !session.space_ids.contains_key(&info.space.into_raw()) {
            return Err(xr::Result::ERROR_SESSION_LOST.into());
        }

        let view_state = unsafe { &mut *view_state };
        view_state.view_state_flags = xr::ViewStateFlags::from_raw(0b1111);
        for i in 0..2 {
            let _view = unsafe { &mut *(views.add(i)) };
        }

        Ok(())
    })
    .into_xr_result()
}
