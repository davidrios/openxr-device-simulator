use std::{
    cell::UnsafeCell,
    collections::{HashMap, HashSet},
    sync::{LazyLock, Mutex, atomic},
};

use openxr_sys as xr;

use crate::{
    error::{Error, Result, to_xr_result},
    event::{Event, schedule_event},
    loader::START_TIME,
    system::HMD_SYSTEM_ID,
    with_instance,
};

#[macro_export]
macro_rules! with_session {
    ($xr_session:expr, |$instance:ident| $expr:expr) => {{
        match $crate::session::get_simulated_session_cell($xr_session) {
            Ok(instance_ptr) => {
                let $instance = unsafe { &mut *instance_ptr };
                $expr
            }
            Err(err) => {
                log::error!("error: {err}");
                Err(openxr_sys::Result::ERROR_SESSION_LOST.into())
            }
        }
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

    to_xr_result(with_instance!(xr_instance, |instance| {
        let mut session_instances = INSTANCES.lock().unwrap();
        let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        session_instances.insert(
            next_id,
            UnsafeCell::new(SimulatedSession::new(xr_instance.into_raw(), next_id)),
        );

        *xr_session = xr::Session::from_raw(next_id);

        log::debug!("create: {:?}", create_info);

        instance.set_session(next_id)
    }))
}

pub extern "system" fn destroy(xr_obj: xr::Session) -> xr::Result {
    if xr_obj == xr::Session::NULL {
        return xr::Result::ERROR_HANDLE_INVALID;
    }

    let instance_id = xr_obj.into_raw();

    log::debug!("destroyed session {instance_id} (todo)");
    xr::Result::SUCCESS
}

pub extern "system" fn attach_action_sets(
    xr_session: xr::Session,
    attach_info: *const xr::SessionActionSetsAttachInfo,
) -> xr::Result {
    if attach_info.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let attach_info = unsafe { &*attach_info };

    if attach_info.ty != xr::StructureType::SESSION_ACTION_SETS_ATTACH_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    if attach_info.count_action_sets == 0 || attach_info.action_sets.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    to_xr_result(with_session!(xr_session, |session| {
        for i in 0..attach_info.count_action_sets {
            let item = unsafe { &*attach_info.action_sets.add(i as usize) };

            if let Err(err) = session.attach_action_set(item.into_raw()) {
                return err.into();
            }
        }
        Ok(())
    }))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimulatedSessionSpace {
    Reference,
    Action,
}

#[derive(Debug)]
pub struct SimulatedSession {
    instance_id: u64,
    id: u64,
    space_ids: HashMap<SimulatedSessionSpace, u64>,
    action_set_ids: HashSet<u64>,
    swapchain_ids: HashSet<u64>,
    state: xr::SessionState,
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
            space_ids: HashMap::new(),
            action_set_ids: HashSet::new(),
            swapchain_ids: HashSet::new(),
            state: xr::SessionState::IDLE,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn check_ready(&mut self) -> Result<()> {
        if let xr::SessionState::IDLE = self.state {
            if !self.space_ids.is_empty()
                && !self.swapchain_ids.is_empty()
                && !self.action_set_ids.is_empty()
            {
                self.state = xr::SessionState::READY;
                schedule_event(
                    self.instance_id,
                    &Event::SessionStateChanged {
                        session: xr::Session::from_raw(self.id),
                        state: self.state,
                        time: START_TIME.elapsed().into(),
                    },
                )?;
            }
        }

        Ok(())
    }

    pub fn set_space(&mut self, space_type: SimulatedSessionSpace, space_id: u64) -> Result<()> {
        self.space_ids.insert(space_type, space_id);
        self.check_ready()?;

        Ok(())
    }

    pub fn attach_action_set(&mut self, action_set_id: u64) -> Result<()> {
        if !self.action_set_ids.insert(action_set_id) {
            Err(xr::Result::ERROR_ACTIONSETS_ALREADY_ATTACHED.into())
        } else {
            log::debug!("attached action set {action_set_id}");
            self.check_ready()?;

            Ok(())
        }
    }

    pub fn add_swapchain(&mut self, swapchain_id: u64) -> Result<()> {
        if !self.swapchain_ids.insert(swapchain_id) {
            Err(xr::Result::ERROR_ACTIONSETS_ALREADY_ATTACHED.into())
        } else {
            log::debug!("attached swapchain {swapchain_id}");
            self.check_ready()?;

            Ok(())
        }
    }
}

type SharedSimulatedSession = UnsafeCell<SimulatedSession>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
