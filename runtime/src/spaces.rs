use std::{
    cell::UnsafeCell,
    collections::HashMap,
    sync::{LazyLock, Mutex, atomic},
};

use openxr_sys as xr;

use crate::{
    error::{Error, Result},
    with_session,
};

pub extern "system" fn enumerate_reference_spaces(
    xr_session: xr::Session,
    capacity_in: u32,
    count_out: *mut u32,
    space_types: *mut xr::ReferenceSpaceType,
) -> xr::Result {
    let count_out = unsafe { &mut *count_out };

    with_session!(xr_session, |_session| {
        if capacity_in == 0 {
            *count_out = 3;
            return xr::Result::SUCCESS;
        }

        if *count_out != 3 {
            return xr::Result::ERROR_SIZE_INSUFFICIENT;
        }

        unsafe {
            *space_types = xr::ReferenceSpaceType::VIEW;
            *space_types.add(1) = xr::ReferenceSpaceType::LOCAL;
            *space_types.add(2) = xr::ReferenceSpaceType::LOCAL_FLOOR;
        }
    });

    xr::Result::SUCCESS
}

pub extern "system" fn create_reference_space(
    xr_session: xr::Session,
    create_info: *const xr::ReferenceSpaceCreateInfo,
    xr_space: *mut xr::Space,
) -> xr::Result {
    if create_info.is_null() || xr_space.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (create_info, xr_space) = unsafe { (&*create_info, &mut *xr_space) };

    if create_info.ty != xr::StructureType::REFERENCE_SPACE_CREATE_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    with_session!(xr_session, |session| {
        let mut reference_spaces = INSTANCES.lock().unwrap();
        let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        reference_spaces.insert(
            next_id,
            UnsafeCell::new(
                match SimulatedSpace::new(xr_session.into_raw(), next_id, create_info) {
                    Ok(set) => set,
                    Err(err) => match err {
                        Error::XrResult(res) => return res,
                        _ => {
                            log::error!("{err}");
                            return xr::Result::ERROR_RUNTIME_FAILURE;
                        }
                    },
                },
            ),
        );

        log::debug!("create reference space: {:?}", unsafe {
            &*reference_spaces[&next_id].get()
        });

        *xr_space = xr::Space::from_raw(next_id);

        match session.set_space(next_id) {
            Ok(_) => xr::Result::SUCCESS,
            Err(err) => match err {
                Error::XrResult(res) => res,
                _ => {
                    log::error!("{err}");
                    xr::Result::ERROR_RUNTIME_FAILURE
                }
            },
        }
    })
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
pub struct SimulatedSpace {
    session_id: u64,
    id: u64,
    ty: xr::ReferenceSpaceType,
    pose: xr::Posef,
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

#[inline]
pub fn get_simulated_space_cell(instance: xr::ActionSet) -> Result<*mut SimulatedSpace> {
    Ok(INSTANCES
        .lock()?
        .get(&instance.into_raw())
        .ok_or_else(|| Error::ExpectedSome("space does not exist".into()))?
        .get())
}

impl SimulatedSpace {
    pub fn new(
        session_id: u64,
        id: u64,
        create_info: &xr::ReferenceSpaceCreateInfo,
    ) -> Result<Self> {
        Ok(Self {
            session_id,
            id,
            ty: create_info.reference_space_type,
            pose: create_info.pose_in_reference_space,
        })
    }
}

type SharedSimulatedActionSet = UnsafeCell<SimulatedSpace>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedActionSet>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
