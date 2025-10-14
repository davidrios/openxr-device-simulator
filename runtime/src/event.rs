use std::{
    cell::UnsafeCell,
    collections::{HashMap, VecDeque},
    sync::{LazyLock, Mutex},
};

use crate::{
    error::{IntoXrResult, Result},
    instance::api::with_instance,
    utils::{MyTime, with_obj_instance},
};

pub extern "system" fn poll(
    xr_instance: xr::Instance,
    event_data: *mut xr::EventDataBuffer,
) -> xr::Result {
    if event_data.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let event_data = unsafe { &mut *event_data };

    let queue_id = match with_instance(xr_instance.into_raw(), |instance| Ok(instance.id)) {
        Ok(value) => value,
        Err(err) => return err.into(),
    };

    with_event_queue(queue_id, |queue| {
        if let Some(item) = queue.pop_front() {
            log::debug!("polled event {:?}", item.ty);
            event_data.ty = item.ty;
            let dest_slice = &mut event_data.varying[..item.buf.len()];
            dest_slice.copy_from_slice(&item.buf);
            Ok(xr::Result::SUCCESS)
        } else {
            Ok(xr::Result::EVENT_UNAVAILABLE)
        }
    })
    .into_xr_result()
}

pub fn create_queue(queue_id: u64) -> Result<()> {
    INSTANCES
        .lock()?
        .insert(queue_id, UnsafeCell::new(VecDeque::new()));
    Ok(())
}

#[derive(Debug)]
pub struct QueueItem {
    ty: xr::StructureType,
    buf: Box<[u8]>,
}

#[derive(Debug)]
pub enum Event {
    SessionStateChanged {
        session: xr::Session,
        state: xr::SessionState,
        time: MyTime,
    },
}

type SharedEventQueue = UnsafeCell<VecDeque<QueueItem>>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedEventQueue>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn with_event_queue<T, F>(queue_id: u64, f: F) -> Result<T>
where
    F: FnMut(&mut VecDeque<QueueItem>) -> Result<T>,
{
    with_obj_instance(&INSTANCES, queue_id, f)
}

const SIZEOF_TY_NEXT: usize = std::mem::size_of::<xr::EventDataBaseHeader>();

pub fn schedule_event(queue_id: u64, event: &Event) -> Result<()> {
    with_event_queue(queue_id, |queue| {
        let (ptr, size) = match event {
            Event::SessionStateChanged {
                session,
                state,
                time,
            } => {
                let ty = xr::StructureType::EVENT_DATA_SESSION_STATE_CHANGED;
                let xr_event = xr::EventDataSessionStateChanged {
                    ty,
                    next: std::ptr::null_mut(),
                    session: *session,
                    state: *state,
                    time: (*time).into(),
                };
                let size = std::mem::size_of::<xr::EventDataSessionStateChanged>();
                let ptr = &xr_event as *const _ as *const u8;
                (ptr, size)
            }
        };

        let slice: &[u8] =
            unsafe { std::slice::from_raw_parts(ptr.add(SIZEOF_TY_NEXT), size - SIZEOF_TY_NEXT) };

        let mut buf = vec![0_u8; size - SIZEOF_TY_NEXT];
        buf.copy_from_slice(slice);

        queue.push_back(QueueItem {
            ty: unsafe { *(ptr as *const xr::StructureType) },
            buf: buf.into_boxed_slice(),
        });
        Ok(())
    })
}
