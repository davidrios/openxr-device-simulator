use std::{thread, time::Duration};

use openxr_sys as xr;

use crate::{error::to_xr_result, loader::START_TIME, utils::MyTime, with_session};

pub extern "system" fn wait(
    xr_session: xr::Session,
    _wait_info: *const xr::FrameWaitInfo,
    frame_state: *mut xr::FrameState,
) -> xr::Result {
    if frame_state.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let frame_state = unsafe { &mut *frame_state };

    if frame_state.ty != xr::StructureType::FRAME_STATE {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    to_xr_result(with_session!(xr_session, |_session| {
        log::debug!("wait frame");
        // throttle to 1 fps
        thread::sleep(Duration::from_secs(1));

        frame_state.predicted_display_time =
            MyTime::from(START_TIME.elapsed() + Duration::from_millis(1)).into();
        frame_state.predicted_display_period = Duration::from_millis(16).try_into().unwrap();
        frame_state.should_render = xr::TRUE;

        Ok(())
    }))
}

pub extern "system" fn begin(
    xr_session: xr::Session,
    _begin_info: *const xr::FrameBeginInfo,
) -> xr::Result {
    log::debug!("begin frame");
    to_xr_result(with_session!(xr_session, |_session| Ok(())))
}

pub extern "system" fn end(
    xr_session: xr::Session,
    end_info: *const xr::FrameEndInfo,
) -> xr::Result {
    if end_info.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let end_info = unsafe { &*end_info };
    log::debug!("end frame info: {:?}", end_info);

    to_xr_result(with_session!(xr_session, |_session| Ok(())))
}
