use std::{
    cell::UnsafeCell,
    collections::HashMap,
    ffi::{CStr, CString},
    sync::{LazyLock, Mutex, atomic},
};

use openxr_sys as xr;

use crate::{
    error::{Error, Result, to_xr_result},
    utils::create_identity_pose,
    with_action_set,
};

#[macro_export]
macro_rules! with_action {
    ($xr_obj:expr, |$instance:ident| $expr:expr) => {{
        match $crate::input::action::get_simulated_action_cell($xr_obj) {
            Ok(instance_ptr) => {
                let $instance = unsafe { &mut *instance_ptr };
                $expr
            }
            Err(err) => Err(err),
        }
    }};
}

pub extern "system" fn create(
    xr_action_set: xr::ActionSet,
    create_info: *const xr::ActionCreateInfo,
    xr_action: *mut xr::Action,
) -> xr::Result {
    if create_info.is_null() || xr_action.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (create_info, xr_action) = unsafe { (&*create_info, &mut *xr_action) };

    if create_info.ty != xr::StructureType::ACTION_CREATE_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    to_xr_result(with_action_set!(xr_action_set, |action_set| {
        let mut action_instances = INSTANCES.lock().unwrap();
        let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        action_instances.insert(
            next_id,
            UnsafeCell::new(
                match SimulatedAction::new(xr_action.into_raw(), next_id, create_info) {
                    Ok(set) => set,
                    Err(err) => return err.into(),
                },
            ),
        );

        log::debug!("create action: {:?}", unsafe {
            &*action_instances[&next_id].get()
        });

        *xr_action = xr::Action::from_raw(next_id);

        action_set.add_action(next_id)
    }))
}

#[derive(Debug)]
pub enum SimulatedActionValue {
    Boolean(bool),
    Float(f32),
    Vector2f(xr::Vector2f),
    Pose(xr::Posef),
    Vibration(f32),
    Unknown(i32),
}

#[derive(Debug)]
pub struct SimulatedAction {
    action_set_id: u64,
    id: u64,
    name: CString,
    localized_name: String,
    subaction_paths: Vec<u64>,
    input_value: SimulatedActionValue,
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

#[inline]
pub fn get_simulated_action_cell(instance: xr::Session) -> Result<*mut SimulatedAction> {
    Ok(INSTANCES
        .lock()?
        .get(&instance.into_raw())
        .ok_or_else(|| Error::ExpectedSome("action does not exist".into()))?
        .get())
}

impl SimulatedAction {
    pub fn new(action_set_id: u64, id: u64, create_info: &xr::ActionCreateInfo) -> Result<Self> {
        let name = unsafe { CStr::from_ptr(create_info.action_name.as_ptr()) };
        let localized_name = unsafe { CStr::from_ptr(create_info.localized_action_name.as_ptr()) };

        let mut subaction_paths = Vec::new();
        if create_info.count_subaction_paths > 0 {
            if create_info.subaction_paths.is_null() {
                log::error!("count subaction is > 0 but paths is null");
                return Err(xr::Result::ERROR_VALIDATION_FAILURE.into());
            }

            for i in 0..create_info.count_subaction_paths {
                subaction_paths
                    .push(unsafe { *create_info.subaction_paths.add(i as usize) }.into_raw());
            }
        }

        Ok(Self {
            action_set_id,
            id,
            name: name.into(),
            localized_name: localized_name.to_str()?.into(),
            subaction_paths,
            input_value: match create_info.action_type {
                xr::ActionType::BOOLEAN_INPUT => SimulatedActionValue::Boolean(false),
                xr::ActionType::FLOAT_INPUT => SimulatedActionValue::Float(0.0),
                xr::ActionType::VECTOR2F_INPUT => {
                    SimulatedActionValue::Vector2f(xr::Vector2f::default())
                }
                xr::ActionType::POSE_INPUT => SimulatedActionValue::Pose(create_identity_pose()),
                xr::ActionType::VIBRATION_OUTPUT => SimulatedActionValue::Vibration(0.0),
                _ => SimulatedActionValue::Unknown(create_info.action_type.into_raw()),
            },
        })
    }
}

type SharedSimulatedActionSet = UnsafeCell<SimulatedAction>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedActionSet>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
