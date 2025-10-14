use std::{
    cell::UnsafeCell,
    collections::HashMap,
    ffi::{CStr, CString, c_char},
    sync::{LazyLock, Mutex, atomic},
};

use crate::{
    input::action_set::with_action_set,
    prelude::*,
    session::with_session,
    utils::{create_identity_pose, with_obj_instance},
};

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

    with_action_set(xr_action_set.into_raw(), |action_set| {
        let mut action_instances = INSTANCES.lock().unwrap();
        let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        action_instances.insert(
            next_id,
            UnsafeCell::new(SimulatedAction::new(
                xr_action_set.into_raw(),
                next_id,
                create_info,
            )?),
        );

        log::debug!("created: {:?}", unsafe {
            &*action_instances[&next_id].get()
        });

        *xr_action = xr::Action::from_raw(next_id);

        action_set.add_action(next_id)
    })
    .into_xr_result()
}

pub extern "system" fn destroy(xr_obj: xr::Action) -> xr::Result {
    if xr_obj == xr::Action::NULL {
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

#[allow(unreachable_code)]
pub extern "system" fn enumerate_bound_sources(
    xr_session: xr::Session,
    info: *const xr::BoundSourcesForActionEnumerateInfo,
    _capacity_in: u32,
    count_out: *mut u32,
    _sources: *mut xr::Path,
) -> xr::Result {
    if info.is_null() || count_out.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (info, _count_out) = unsafe { (&*info, &mut *count_out) };

    with_session(xr_session.into_raw(), |_session| {
        log::debug!("enumerate_bound_sources {info:?}");
        return Err(xr::Result::ERROR_FUNCTION_UNSUPPORTED.into());
        Ok(())
    })
    .into_xr_result()
}

#[allow(unreachable_code)]
pub extern "system" fn get_input_source_localized_name(
    xr_session: xr::Session,
    info: *const xr::InputSourceLocalizedNameGetInfo,
    _capacity_in: u32,
    count_out: *mut u32,
    _buf: *mut c_char,
) -> xr::Result {
    if info.is_null() || count_out.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (info, _count_out) = unsafe { (&*info, &mut *count_out) };

    with_session(xr_session.into_raw(), |_session| {
        log::debug!("get_input_source_localized_name {info:?}");
        return Err(xr::Result::ERROR_FUNCTION_UNSUPPORTED.into());
        Ok(())
    })
    .into_xr_result()
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum SimulatedActionValue {
    Boolean(bool),
    Float(f32),
    Vector2f(xr::Vector2f),
    Pose(xr::Posef),
    Vibration(f32),
    Unknown(i32),
}

impl Default for SimulatedActionValue {
    fn default() -> Self {
        Self::Unknown(0)
    }
}

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct SimulatedActionCurrentValue {
    pub(crate) current: SimulatedActionValue,
    pub(crate) changed_since_last_sync: bool,
    pub(crate) last_change_time: u64,
    pub(crate) is_active: bool,
}

type PathId = u64;

#[allow(dead_code)]
#[derive(Debug)]
pub struct SimulatedAction {
    pub(crate) action_set_id: u64,
    pub(crate) id: u64,
    pub(crate) name: CString,
    pub(crate) localized_name: String,
    pub(crate) subaction_values: HashMap<PathId, SimulatedActionCurrentValue>,
}

impl SimulatedAction {
    pub fn new(action_set_id: u64, id: u64, create_info: &xr::ActionCreateInfo) -> Result<Self> {
        let name = unsafe { CStr::from_ptr(create_info.action_name.as_ptr()) };
        let localized_name = unsafe { CStr::from_ptr(create_info.localized_action_name.as_ptr()) };

        let mut subaction_values = HashMap::new();

        if create_info.count_subaction_paths > 0 {
            if create_info.subaction_paths.is_null() {
                log::error!("count subaction is > 0 but paths is null");
                return Err(xr::Result::ERROR_VALIDATION_FAILURE.into());
            }

            for i in 0..create_info.count_subaction_paths {
                let path = unsafe { *create_info.subaction_paths.add(i as usize) }.into_raw();

                subaction_values.insert(
                    path,
                    SimulatedActionCurrentValue {
                        current: match create_info.action_type {
                            xr::ActionType::BOOLEAN_INPUT => SimulatedActionValue::Boolean(false),
                            xr::ActionType::FLOAT_INPUT => SimulatedActionValue::Float(0.0),
                            xr::ActionType::VECTOR2F_INPUT => {
                                SimulatedActionValue::Vector2f(xr::Vector2f::default())
                            }
                            xr::ActionType::POSE_INPUT => {
                                SimulatedActionValue::Pose(create_identity_pose())
                            }
                            xr::ActionType::VIBRATION_OUTPUT => {
                                SimulatedActionValue::Vibration(0.0)
                            }
                            _ => SimulatedActionValue::Unknown(create_info.action_type.into_raw()),
                        },
                        ..Default::default()
                    },
                );
            }
        } else {
            subaction_values.insert(
                0,
                SimulatedActionCurrentValue {
                    current: match create_info.action_type {
                        xr::ActionType::BOOLEAN_INPUT => SimulatedActionValue::Boolean(false),
                        xr::ActionType::FLOAT_INPUT => SimulatedActionValue::Float(0.0),
                        xr::ActionType::VECTOR2F_INPUT => {
                            SimulatedActionValue::Vector2f(xr::Vector2f::default())
                        }
                        xr::ActionType::POSE_INPUT => {
                            SimulatedActionValue::Pose(create_identity_pose())
                        }
                        xr::ActionType::VIBRATION_OUTPUT => SimulatedActionValue::Vibration(0.0),
                        _ => SimulatedActionValue::Unknown(create_info.action_type.into_raw()),
                    },
                    ..Default::default()
                },
            );
        }

        Ok(Self {
            action_set_id,
            id,
            name: name.into(),
            localized_name: localized_name.to_str()?.into(),
            subaction_values,
        })
    }

    pub fn subaction_value(&self, path: u64) -> Result<&SimulatedActionCurrentValue> {
        match self.subaction_values.get(&path) {
            Some(value) => Ok(value),
            None => Err(xr::Result::ERROR_PATH_INVALID.into()),
        }
    }
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

type SharedSimulatedActionSet = UnsafeCell<SimulatedAction>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedActionSet>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn with_action<T, F>(xr_obj_id: u64, f: F) -> Result<T>
where
    F: FnMut(&mut SimulatedAction) -> Result<T>,
{
    with_obj_instance(&INSTANCES, xr_obj_id, f)
}
