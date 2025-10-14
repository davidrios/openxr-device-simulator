use std::{mem::transmute, thread, time::Duration};

use crate::{
    error::{IntoXrResult, Result},
    loader::START_TIME,
    session::with_session,
    utils::MyTime,
};

pub extern "system" fn wait(
    xr_session: xr::Session,
    info: *const xr::FrameWaitInfo,
    frame_state: *mut xr::FrameState,
) -> xr::Result {
    if frame_state.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let frame_state = unsafe { &mut *frame_state };

    if frame_state.ty != xr::StructureType::FRAME_STATE {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let info = unsafe { if info.is_null() { None } else { Some(&*info) } };
    if let Some(info) = info {
        if info.ty != xr::StructureType::FRAME_WAIT_INFO {
            return xr::Result::ERROR_VALIDATION_FAILURE;
        }
    }

    with_session(xr_session.into_raw(), |session| {
        if !session.is_running {
            return Err(xr::Result::ERROR_SESSION_NOT_RUNNING.into());
        }
        log::debug!("[{}] wait_frame ({info:?})", session.id);
        session.synchronize()?;
        session.frame.wait(info, frame_state)
    })
    .into_xr_result()
}

pub extern "system" fn begin(
    xr_session: xr::Session,
    info: *const xr::FrameBeginInfo,
) -> xr::Result {
    let info = unsafe { if info.is_null() { None } else { Some(&*info) } };
    if let Some(info) = info {
        if info.ty != xr::StructureType::FRAME_BEGIN_INFO {
            return xr::Result::ERROR_VALIDATION_FAILURE;
        }
    }
    with_session(xr_session.into_raw(), |session| {
        if !session.is_running {
            return Err(xr::Result::ERROR_SESSION_NOT_RUNNING.into());
        }
        log::debug!("[{}] begin_frame ({info:?})", session.id);
        session.frame.begin(info)
    })
    .into_xr_result()
}

pub extern "system" fn end(xr_session: xr::Session, info: *const xr::FrameEndInfo) -> xr::Result {
    if info.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let info = unsafe { &*info };
    if info.ty != xr::StructureType::FRAME_END_INFO || info.layers.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    with_session(xr_session.into_raw(), |session| {
        if !session.is_running {
            return Err(xr::Result::ERROR_SESSION_NOT_RUNNING.into());
        }
        if !session.frame.can_end() {
            return Err(xr::Result::ERROR_CALL_ORDER_INVALID.into());
        }

        let layers: Option<&[&xr::CompositionLayerBaseHeader]> = if info.layer_count > 0 {
            unsafe {
                Some(std::slice::from_raw_parts(
                    info.layers as *const &xr::CompositionLayerBaseHeader,
                    info.layer_count as usize,
                ))
            }
        } else {
            None
        };

        log::debug!("[{}] end_frame ({info:?})", session.id);

        for layer in layers.unwrap_or_default() {
            match layer.ty {
                xr::StructureType::COMPOSITION_LAYER_PROJECTION => {
                    let layer = unsafe {
                        transmute::<
                            &&xr::CompositionLayerBaseHeader,
                            &&xr::CompositionLayerProjection,
                        >(layer)
                    };
                    log::debug!("[{}] end_frame, layer: {layer:?}", session.id);
                }
                _ => return Err(xr::Result::ERROR_RUNTIME_FAILURE.into()),
            }
        }

        session.frame.end()
    })
    .into_xr_result()
}

#[derive(Debug, Default)]
pub struct SessionFrame {
    pub(crate) waiting_begin: bool,
    pub(crate) is_waited: bool,
    pub(crate) is_began: bool,
}

impl SessionFrame {
    pub fn wait(
        &mut self,
        _info: Option<&xr::FrameWaitInfo>,
        frame_state: &mut xr::FrameState,
    ) -> Result<()> {
        #[allow(clippy::while_immutable_condition)]
        while self.waiting_begin {}

        // throttle to 2 fps
        thread::sleep(Duration::from_millis(500));

        frame_state.predicted_display_time =
            MyTime::from(START_TIME.elapsed() + Duration::from_millis(1)).into();
        frame_state.predicted_display_period = Duration::from_millis(16).try_into().unwrap();
        frame_state.should_render = xr::TRUE;

        self.is_waited = true;
        self.waiting_begin = true;

        Ok(())
    }

    pub fn begin(&mut self, _info: Option<&xr::FrameBeginInfo>) -> Result<xr::Result> {
        if !self.is_waited {
            return Err(xr::Result::ERROR_CALL_ORDER_INVALID.into());
        }

        self.waiting_begin = false;
        self.is_waited = false;

        if self.is_began {
            return Ok(xr::Result::FRAME_DISCARDED);
        }

        self.is_began = true;

        Ok(xr::Result::SUCCESS)
    }

    pub fn can_end(&mut self) -> bool {
        self.is_began
    }

    pub fn end(&mut self) -> Result<()> {
        self.is_began = false;
        Ok(())
    }
}
