use std::{
    cell::UnsafeCell,
    collections::{HashMap, VecDeque},
    sync::{LazyLock, Mutex},
};

use crate::{
    error::{Error, Result, to_xr_result},
    utils::MyTime,
    with_instance,
};

#[macro_export]
macro_rules! with_event_queue {
    ($xr_obj:expr, |$instance:ident| $expr:expr) => {{
        match $crate::event::get_event_queue_cell($xr_obj) {
            Ok(instance_ptr) => {
                let $instance = unsafe { &mut *instance_ptr };
                $expr
            }
            Err(err) => Err(err),
        }
    }};
}

pub extern "system" fn poll(
    xr_instance: xr::Instance,
    event_data: *mut xr::EventDataBuffer,
) -> xr::Result {
    if event_data.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let event_data = unsafe { &mut *event_data };

    let res: Result<u64> = with_instance!(xr_instance, |_instance| Ok(xr_instance.into_raw()));
    let queue_id = match res {
        Ok(queue_id) => queue_id,
        Err(err) => return err.into(),
    };

    to_xr_result(with_event_queue!(queue_id, |queue| {
        if let Some(item) = queue.pop_front() {
            log::debug!("polled event {:?}", item.ty);
            event_data.ty = item.ty;
            let dest_slice = &mut event_data.varying[..item.buf.len()];
            dest_slice.copy_from_slice(&item.buf);
        }
        Ok(())
    }))
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

type SharedEventQueue = UnsafeCell<VecDeque<QueueItem>>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedEventQueue>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[inline]
pub fn get_event_queue_cell(id: u64) -> Result<*mut VecDeque<QueueItem>> {
    Ok(INSTANCES
        .lock()?
        .get(&id)
        .ok_or_else(|| Error::ExpectedSome("event queue does not exist".into()))?
        .get())
}

#[derive(Debug)]
pub enum Event {
    SessionStateChanged {
        session: xr::Session,
        state: xr::SessionState,
        time: MyTime,
    },
}

const SIZEOF_TY_NEXT: usize = std::mem::size_of::<xr::EventDataBaseHeader>();

pub fn schedule_event(queue_id: u64, event: &Event) -> Result<()> {
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

    with_event_queue!(queue_id, |queue| {
        queue.push_back(QueueItem {
            ty: unsafe { *(ptr as *const xr::StructureType) },
            buf: buf.into_boxed_slice(),
        });
        Ok(())
    })
}
