use crate::{
    input::action::{SimulatedActionValue, with_action},
    instance::api::with_instance,
    prelude::*,
    session::with_session,
};

fn check_path_is_valid(instance_id: u64, path_id: u64) -> Result<()> {
    if path_id == 0 {
        return Ok(());
    }

    with_instance(instance_id, |instance| {
        let path = instance.get_path_string(path_id)?;
        if path.starts_with("/user/head")
            || path.starts_with("/user/hand/left")
            || path.starts_with("/user/hand/right")
            || path.starts_with("/user/gamepad")
        {
            Ok(())
        } else {
            Err(xr::Result::ERROR_PATH_UNSUPPORTED.into())
        }
    })
}

macro_rules! get_action_value {
    ($xr_session: ident, $info: ident, $state: ident, |$value: ident| $value_enum:pat) => {{
        if $info.is_null() || $state.is_null() {
            return xr::Result::ERROR_VALIDATION_FAILURE;
        }

        let (info, state) = unsafe { (&*$info, &mut *$state) };

        with_action(info.action.into_raw(), |action| {
            let instance_id = with_session($xr_session, |session| {
                if !session.has_attached_action_set(action.action_set_id) {
                    return Err(xr::Result::ERROR_ACTIONSET_NOT_ATTACHED.into());
                }
                Ok(session.instance_id)
            })?;

            log::debug!("get value {info:?}");

            check_path_is_valid(instance_id, info.subaction_path.into_raw())?;

            let value = action.subaction_value(info.subaction_path.into_raw())?;
            match value.current {
                $value_enum => {
                    state.current_state = $value.into();
                }
                _ => return Err(xr::Result::ERROR_ACTION_TYPE_MISMATCH.into()),
            }

            state.is_active = value.is_active.into();
            state.changed_since_last_sync = value.changed_since_last_sync.into();
            Ok(())
        })
        .into_xr_result()
    }};
}

#[allow(unreachable_code)]
pub extern "system" fn get_boolean(
    xr_session: xr::Session,
    info: *const xr::ActionStateGetInfo,
    state: *mut xr::ActionStateBoolean,
) -> xr::Result {
    let xr_obj_id = xr_session.into_raw();
    get_action_value!(
        xr_obj_id,
        info,
        state,
        // needs to be a single line for the macro
        |value| SimulatedActionValue::Boolean(value)
    )
}

#[allow(unreachable_code)]
pub extern "system" fn get_float(
    xr_session: xr::Session,
    info: *const xr::ActionStateGetInfo,
    state: *mut xr::ActionStateFloat,
) -> xr::Result {
    let xr_obj_id = xr_session.into_raw();
    get_action_value!(
        xr_obj_id,
        info,
        state,
        // needs to be a single line for the macro
        |value| SimulatedActionValue::Float(value)
    )
}

#[allow(unreachable_code)]
pub extern "system" fn get_vector2f(
    xr_session: xr::Session,
    info: *const xr::ActionStateGetInfo,
    state: *mut xr::ActionStateVector2f,
) -> xr::Result {
    let xr_obj_id = xr_session.into_raw();
    get_action_value!(
        xr_obj_id,
        info,
        state,
        // needs to be a single line for the macro
        |value| SimulatedActionValue::Vector2f(value)
    )
}

#[allow(unreachable_code)]
pub extern "system" fn get_pose(
    xr_session: xr::Session,
    info: *const xr::ActionStateGetInfo,
    state: *mut xr::ActionStatePose,
) -> xr::Result {
    if info.is_null() || state.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (info, state) = unsafe { (&*info, &mut *state) };

    with_action(info.action.into_raw(), |action| {
        let instance_id = with_session(xr_session.into_raw(), |session| {
            if !session.has_attached_action_set(action.action_set_id) {
                return Err(xr::Result::ERROR_ACTIONSET_NOT_ATTACHED.into());
            }
            Ok(session.instance_id)
        })?;

        log::debug!("get_pose {info:?}");

        check_path_is_valid(instance_id, info.subaction_path.into_raw())?;

        let value = action.subaction_value(info.subaction_path.into_raw())?;
        match value.current {
            SimulatedActionValue::Pose(_) => {
                state.is_active = value.is_active.into();
                Ok(())
            }
            _ => Err(xr::Result::ERROR_ACTION_TYPE_MISMATCH.into()),
        }
    })
    .into_xr_result()
}

#[allow(unreachable_code)]
pub extern "system" fn sync_actions(
    xr_session: xr::Session,
    info: *const xr::ActionsSyncInfo,
) -> xr::Result {
    if info.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let info = unsafe { &*info };

    if info.active_action_sets.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let active_action_sets: &[xr::ActiveActionSet] = unsafe {
        std::slice::from_raw_parts(
            info.active_action_sets as *const _,
            info.count_active_action_sets as usize,
        )
    };

    with_session(xr_session.into_raw(), |session| {
        if !session.is_focused() {
            return Ok(xr::Result::SESSION_NOT_FOCUSED);
        }

        log::debug!("sync_actions {active_action_sets:?}");

        for active_action_set in active_action_sets {
            if !session.has_attached_action_set(active_action_set.action_set.into_raw()) {
                return Err(xr::Result::ERROR_ACTIONSET_NOT_ATTACHED.into());
            }
        }

        Ok(xr::Result::SUCCESS)
    })
    .into_xr_result()
}
