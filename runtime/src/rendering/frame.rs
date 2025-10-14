use std::{thread, time::Duration};

use crate::{error::IntoXrResult, loader::START_TIME, session::with_session, utils::MyTime};

pub extern "system" fn wait(
    xr_session: xr::Session,
    _wait_info: *const xr::FrameWaitInfo,
    frame_state: *mut xr::FrameState,
) -> xr::Result {
    log::debug!("wait frame");
    if frame_state.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let frame_state = unsafe { &mut *frame_state };

    if frame_state.ty != xr::StructureType::FRAME_STATE {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    with_session(xr_session.into_raw(), |_session| {
        // throttle to 2 fps
        thread::sleep(Duration::from_millis(500));

        frame_state.predicted_display_time =
            MyTime::from(START_TIME.elapsed() + Duration::from_millis(1)).into();
        frame_state.predicted_display_period = Duration::from_millis(16).try_into().unwrap();
        frame_state.should_render = xr::TRUE;

        Ok(())
    })
    .into_xr_result()
}

pub extern "system" fn begin(
    xr_session: xr::Session,
    _begin_info: *const xr::FrameBeginInfo,
) -> xr::Result {
    log::debug!("begin frame");
    with_session(xr_session.into_raw(), |_session| Ok(())).into_xr_result()
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

    with_session(xr_session.into_raw(), |_session| Ok(())).into_xr_result()
}
