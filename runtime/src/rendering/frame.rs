use std::{
    collections::HashSet,
    mem::transmute,
    sync::{Arc, Condvar, Mutex, atomic},
    thread,
    time::Duration,
};

use crate::{
    loader::START_TIME,
    prelude::*,
    rendering::swapchain::with_swapchain,
    server,
    session::{SimulatedSession, with_session},
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

        let mut free_swapchains = HashSet::with_capacity(2);

        for layer in layers.unwrap_or_default() {
            match layer.ty {
                xr::StructureType::COMPOSITION_LAYER_PROJECTION => {
                    let layer = unsafe {
                        transmute::<
                            &&xr::CompositionLayerBaseHeader,
                            &&xr::CompositionLayerProjection,
                        >(layer)
                    };

                    if layer.view_count != 2 {
                        return Err(xr::Result::ERROR_VALIDATION_FAILURE.into());
                    }

                    let views = unsafe {
                        std::slice::from_raw_parts(layer.views, layer.view_count as usize)
                    };

                    log::debug!(
                        "[{}] end_frame, layer: {layer:?}, views: {views:?}",
                        session.id
                    );

                    for view in views {
                        free_swapchains.insert(view.sub_image.swapchain.into_raw());
                    }
                }
                _ => return Err(xr::Result::ERROR_RUNTIME_FAILURE.into()),
            }
        }

        for swapchain_id in &free_swapchains {
            with_swapchain(*swapchain_id, |swapchain| swapchain.free_image())?;
        }

        session.frame.end(free_swapchains.iter().copied().collect())
    })
    .into_xr_result()
}

#[derive(Debug)]
pub struct SessionFrame {
    pub(crate) waiting_begin: Arc<(Mutex<bool>, Condvar)>,
    pub(crate) is_waited: bool,
    pub(crate) is_began: bool,
    pub(crate) swapchains_to_read: Option<Vec<u64>>,
}

impl Default for SessionFrame {
    fn default() -> Self {
        Self {
            waiting_begin: Arc::new((Mutex::new(false), Condvar::new())),
            is_waited: false,
            is_began: false,
            swapchains_to_read: None,
        }
    }
}

impl SessionFrame {
    pub fn wait(
        &mut self,
        // session: &mut SimulatedSession,
        _info: Option<&xr::FrameWaitInfo>,
        frame_state: &mut xr::FrameState,
    ) -> Result<()> {
        {
            let (lock, cvar) = &*self.waiting_begin;
            let mut guard = lock.lock().unwrap();
            while *guard {
                guard = cvar.wait(guard).unwrap();
            }
        } // release lock before doing any work

        let start = START_TIME.elapsed();

        while server::process_server_message().is_some() {}

        if let Some(swapchains_to_read) = self.swapchains_to_read.as_ref() {
            let frame_number = FRAME_COUNTER.fetch_add(1, atomic::Ordering::Relaxed);
            server::send_frame(frame_number as usize);
            for swapchain_id in swapchains_to_read {
                with_swapchain(*swapchain_id, |swapchain| {
                    swapchain.dump_frame(frame_number);
                    Ok(())
                })?;
            }
        }

        // throttle to 2 fps
        thread::sleep(Duration::from_millis(3000) - (START_TIME.elapsed() - start));

        frame_state.predicted_display_time =
            MyTime::from(START_TIME.elapsed() + Duration::from_millis(1)).into();
        frame_state.predicted_display_period = Duration::from_millis(16).try_into().unwrap();
        frame_state.should_render = xr::TRUE;

        self.is_waited = true;
        let (lock, _) = &*self.waiting_begin;
        *lock.lock().unwrap() = true;

        Ok(())
    }

    pub fn begin(&mut self, _info: Option<&xr::FrameBeginInfo>) -> Result<xr::Result> {
        if !self.is_waited {
            return Err(xr::Result::ERROR_CALL_ORDER_INVALID.into());
        }

        let (lock, cvar) = &*self.waiting_begin;
        *lock.lock().unwrap() = false;
        cvar.notify_one();
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

    pub fn end(&mut self, swapchains_to_read: Vec<u64>) -> Result<()> {
        self.is_began = false;
        self.swapchains_to_read = Some(swapchains_to_read);
        Ok(())
    }
}

static FRAME_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(0);
