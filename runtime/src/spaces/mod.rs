use std::{
    cell::UnsafeCell,
    collections::HashMap,
    sync::{LazyLock, Mutex, atomic},
};

use openxr_sys as xr;

use crate::{
    error::{Error, Result},
    session::{SimulatedSession, SimulatedSessionSpace},
};

pub mod action;
pub mod reference;

pub fn create(session: &mut SimulatedSession, space: SimulatedSpaceType) -> Result<u64> {
    let mut spaces = INSTANCES.lock().unwrap();
    let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
    let session_space = match &space {
        SimulatedSpaceType::Reference(_) => SimulatedSessionSpace::Reference,
        SimulatedSpaceType::Action(_) => SimulatedSessionSpace::Action,
    };

    spaces.insert(
        next_id,
        UnsafeCell::new(match SimulatedSpace::new(session.id(), next_id, space) {
            Ok(set) => set,
            Err(err) => match err {
                Error::XrResult(res) => return Err(res.into()),
                _ => {
                    log::error!("{err}");
                    return Err(xr::Result::ERROR_RUNTIME_FAILURE.into());
                }
            },
        }),
    );

    log::debug!("create space: {:?}", unsafe { &*spaces[&next_id].get() });

    if let Err(err) = session.set_space(session_space, next_id) {
        match err {
            Error::XrResult(res) => return Err(res.into()),
            _ => {
                log::error!("{err}");
                return Err(xr::Result::ERROR_RUNTIME_FAILURE.into());
            }
        }
    }

    Ok(next_id)
}

pub extern "system" fn destroy(xr_obj: xr::Space) -> xr::Result {
    if xr_obj == xr::Space::NULL {
        return xr::Result::ERROR_HANDLE_INVALID;
    }

    let instance_id = xr_obj.into_raw();

    log::debug!("destroyed space {instance_id} (todo)");
    xr::Result::SUCCESS
}

#[derive(Debug)]
pub enum SimulatedSpaceType {
    Reference(reference::SimulatedReferenceSpace),
    Action(action::SimulatedActionSpace),
}

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
