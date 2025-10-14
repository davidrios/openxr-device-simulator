use std::{
    cell::UnsafeCell,
    collections::HashMap,
    sync::{LazyLock, Mutex, atomic},
};

use crate::{
    error::{Error, Result, to_xr_result},
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
        UnsafeCell::new(SimulatedSpace::new(session.id, next_id, space)?),
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
    space_location: *mut xr::SpaceLocation,
) -> xr::Result {
    if space_location.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    to_xr_result(get_simulated_space_cells(
        &[xr_space.into_raw(), xr_base_space.into_raw()],
        |spaces| {
            let space_location = unsafe { &mut *space_location };
            let (space, _base_space) = (&*spaces[0], &*spaces[1]);

            space_location.location_flags = xr::SpaceLocationFlags::from_raw(0b1111);
            space_location.pose = match &space.space {
                SimulatedSpaceType::Reference(simulated_reference_space) => {
                    simulated_reference_space.pose
                }
                SimulatedSpaceType::Action(simulated_action_space) => simulated_action_space.pose,
            };

            log::debug!("locate: {xr_time:?}, {space_location:?}",);

            Ok(())
        },
    ))
}

pub extern "system" fn destroy(xr_obj: xr::Space) -> xr::Result {
    if xr_obj == xr::Space::NULL {
        return xr::Result::ERROR_HANDLE_INVALID;
    }

    let instance_id = xr_obj.into_raw();

    if INSTANCES
        .lock()
        .expect("couldn't acquire instances")
        .remove(&instance_id)
        .is_some()
    {
        log::debug!("destroyed {instance_id}");
    } else {
        log::debug!("instance {instance_id} not found");
    }

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

#[inline]
pub fn get_simulated_space_cells<const MAX: usize, T: Fn(&[&mut SimulatedSpace]) -> Result<()>>(
    ids: &[u64; MAX],
    f: T,
) -> Result<()> {
    let instances = INSTANCES.lock()?;
    let mut res = Vec::with_capacity(MAX);
    let slice = 0..MAX;
    for i in slice {
        let id = ids[i];
        res.push(unsafe {
            &mut *instances
                .get(&id)
                .ok_or_else(|| Error::ExpectedSome(format!("space {id} does not exist")))?
                .get()
        });
    }

    f(res.as_ref())
}
