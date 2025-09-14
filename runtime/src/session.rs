use std::{
    cell::UnsafeCell,
    collections::HashMap,
    sync::{LazyLock, Mutex, atomic},
};

use openxr_sys as xr;

use crate::{
    error::{Error, Result},
    system::HMD_SYSTEM_ID,
    with_instance,
};

#[macro_export]
macro_rules! with_session {
    ($xr_session:expr, |$instance:ident| $expr:expr) => {{
        let instance_ptr = match $crate::session::get_simulated_session_cell($xr_session) {
            Ok(instance_ptr) => instance_ptr,
            Err(err) => {
                log::error!("error: {err}");
                return openxr_sys::Result::ERROR_SESSION_LOST;
            }
        };

        let $instance = unsafe { &mut *instance_ptr };
        $expr
    }};
}

pub extern "system" fn create(
    xr_instance: xr::Instance,
    create_info: *const xr::SessionCreateInfo,
    xr_session: *mut xr::Session,
) -> xr::Result {
    if create_info.is_null() || xr_session.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (create_info, xr_session) = unsafe { (&*create_info, &mut *xr_session) };

    if create_info.ty != xr::StructureType::SESSION_CREATE_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    if create_info.system_id != xr::SystemId::from_raw(HMD_SYSTEM_ID) {
        return xr::Result::ERROR_SYSTEM_INVALID;
    }

    with_instance!(xr_instance, |instance| {
        let mut session_instances = INSTANCES.lock().unwrap();
        let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        session_instances.insert(
            next_id,
            UnsafeCell::new(SimulatedSession::new(xr_instance.into_raw(), next_id)),
        );

        *xr_session = xr::Session::from_raw(next_id);

        log::debug!("create: {:?}", create_info);

        instance.set_session(next_id)
    })
}

pub extern "system" fn destroy(xr_obj: xr::Session) -> xr::Result {
    if xr_obj == xr::Session::NULL {
        return xr::Result::ERROR_HANDLE_INVALID;
    }

    let instance_id = xr_obj.into_raw();

    log::debug!("destroyed session {instance_id} (todo)");
    xr::Result::SUCCESS
}

#[derive(Debug)]
pub struct SimulatedSession {
    instance_id: u64,
    id: u64,
    space: Option<u64>,
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

#[inline]
pub fn get_simulated_session_cell(instance: xr::Session) -> Result<*mut SimulatedSession> {
    Ok(INSTANCES
        .lock()?
        .get(&instance.into_raw())
        .ok_or_else(|| Error::ExpectedSome("session does not exist".into()))?
        .get())
}

impl SimulatedSession {
    pub fn new(instance_id: u64, id: u64) -> Self {
        Self {
            instance_id,
            id,
            space: None,
        }
    }

    pub fn set_space(&mut self, space_id: u64) -> Result<()> {
        if self.space.is_some() {
            return Err(xr::Result::ERROR_RUNTIME_FAILURE.into());
        }

        self.space = Some(space_id);
        Ok(())
    }
}

type SharedSimulatedSession = UnsafeCell<SimulatedSession>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
