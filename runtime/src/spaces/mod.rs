use std::{
    cell::UnsafeCell,
    collections::HashMap,
    sync::{LazyLock, Mutex, atomic},
};

use openxr_sys as xr;

use crate::{
    error::{Result, to_xr_result},
    session::{SimulatedSession, SimulatedSessionSpace},
    with_session,
};

pub mod action;
pub mod reference;

pub fn create(session: &mut SimulatedSession, space: SimulatedSpaceType) -> Result<u64> {
    let mut spaces = INSTANCES.lock()?;
    let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
    let session_space = match &space {
        SimulatedSpaceType::Reference(_) => SimulatedSessionSpace::Reference,
        SimulatedSpaceType::Action(_) => SimulatedSessionSpace::Action,
    };

    spaces.insert(
        next_id,
        UnsafeCell::new(SimulatedSpace::new(session.id(), next_id, space)?),
    );

    log::debug!("create space: {:?}", unsafe { &*spaces[&next_id].get() });

    session.set_space(session_space, next_id)?;
    Ok(next_id)
}

#[allow(unreachable_code)]
pub extern "system" fn locate_spaces(
    xr_session: xr::Session,
    info: *const xr::SpacesLocateInfo,
    locations: *mut xr::SpaceLocations,
) -> xr::Result {
    if info.is_null() || locations.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (info, _locations) = unsafe { (&*info, &mut *locations) };

    to_xr_result(with_session!(xr_session, |_session| {
        log::debug!("locate_spaces: {info:?}");
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
        Ok(())
    }))
}

pub extern "system" fn locate(
    xr_space: xr::Space,
    xr_base_space: xr::Space,
    xr_time: xr::Time,
    xr_location: *mut xr::SpaceLocation,
) -> xr::Result {
    if xr_location.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    log::debug!("locate: {xr_space:?}, {xr_base_space:?}, {xr_time:?}");
    xr::Result::ERROR_FUNCTION_UNSUPPORTED
}

pub extern "system" fn destroy(xr_obj: xr::Space) -> xr::Result {
    if xr_obj == xr::Space::NULL {
        return xr::Result::ERROR_HANDLE_INVALID;
    }

    let instance_id = xr_obj.into_raw();

    log::debug!("destroyed space {instance_id} (todo)");
    xr::Result::SUCCESS
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum SimulatedSpaceType {
    Reference(reference::SimulatedReferenceSpace),
    Action(action::SimulatedActionSpace),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SimulatedSpace {
    session_id: u64,
    id: u64,
    space: SimulatedSpaceType,
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

impl SimulatedSpace {
    pub fn new(session_id: u64, id: u64, space: SimulatedSpaceType) -> Result<Self> {
        Ok(Self {
            session_id,
            id,
            space,
        })
    }
}

type SharedSimulatedSpace = UnsafeCell<SimulatedSpace>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedSpace>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
