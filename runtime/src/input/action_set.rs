use std::{
    cell::UnsafeCell,
    collections::HashMap,
    ffi::{CStr, CString},
    sync::{LazyLock, Mutex, atomic},
};

use crate::{instance::api::with_instance, prelude::*, utils::with_obj_instance};

pub extern "system" fn create(
    xr_instance: xr::Instance,
    create_info: *const xr::ActionSetCreateInfo,
    xr_action_set: *mut xr::ActionSet,
) -> xr::Result {
    if create_info.is_null() || xr_action_set.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (create_info, xr_action_set) = unsafe { (&*create_info, &mut *xr_action_set) };

    if create_info.ty != xr::StructureType::ACTION_SET_CREATE_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    with_instance(xr_instance.into_raw(), |instance| {
        let mut session_instances = INSTANCES.lock().unwrap();
        let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        session_instances.insert(
            next_id,
            UnsafeCell::new(SimulatedActionSet::new(
                xr_instance.into_raw(),
                next_id,
                create_info,
            )?),
        );

        log::debug!("created {:?}", unsafe {
            &*session_instances[&next_id].get()
        });

        *xr_action_set = xr::ActionSet::from_raw(next_id);

        instance.add_action_set(next_id)
    })
    .into_xr_result()
}

pub extern "system" fn destroy(xr_obj: xr::ActionSet) -> xr::Result {
    if xr_obj == xr::ActionSet::NULL {
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
pub struct SimulatedActionSet {
    instance_id: u64,
    id: u64,
    name: CString,
    localized_name: String,
    priority: u32,
    actions: Vec<u64>,
}

impl SimulatedActionSet {
    pub fn new(instance_id: u64, id: u64, create_info: &xr::ActionSetCreateInfo) -> Result<Self> {
        let name = unsafe { CStr::from_ptr(create_info.action_set_name.as_ptr()) };
        let localized_name =
            unsafe { CStr::from_ptr(create_info.localized_action_set_name.as_ptr()) };

        Ok(Self {
            instance_id,
            id,
            name: name.into(),
            localized_name: localized_name.to_str()?.into(),
            priority: create_info.priority,
            actions: Vec::new(),
        })
    }

    pub fn add_action(&mut self, action_id: u64) -> Result<()> {
        self.actions.push(action_id);
        Ok(())
    }
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

type SharedSimulatedActionSet = UnsafeCell<SimulatedActionSet>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedActionSet>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn with_action_set<T, F>(xr_obj_id: u64, f: F) -> Result<T>
where
    F: FnMut(&mut SimulatedActionSet) -> Result<T>,
{
    with_obj_instance(&INSTANCES, xr_obj_id, f)
}
