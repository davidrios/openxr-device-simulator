use crate::{
    error::{Result, to_xr_result},
    input::action::SimulatedActionValue,
    with_action, with_instance, with_session,
};

fn check_path_is_valid(instance_id: Result<u64>, path_id: u64) -> Result<()> {
    if path_id == 0 {
        return Ok(());
    }

    match match instance_id {
        Ok(instance_id) => {
            with_instance!(xr::Instance::from_raw(instance_id), |instance| {
                match instance.get_path_string(path_id) {
                    Ok(path) => Ok(path.starts_with("/user/head")
                        || path.starts_with("/user/hand/left")
                        || path.starts_with("/user/hand/right")
                        || path.starts_with("/user/gamepad")),
                    Err(err) => Err(err),
                }
            })
        }
        Err(err) => return Err(err),
    } {
        Ok(is_valid_path) => {
            if is_valid_path {
                Ok(())
            } else {
                Err(xr::Result::ERROR_PATH_UNSUPPORTED.into())
            }
        }
        Err(err) => Err(err),
    }
}

macro_rules! get_action_value {
    ($xr_session: ident, $info: ident, $state: ident, |$value: ident| $value_enum:pat) => {{
        if $info.is_null() || $state.is_null() {
            return xr::Result::ERROR_VALIDATION_FAILURE;
        }

        let (info, state) = unsafe { (&*$info, &mut *$state) };

        to_xr_result(with_action!(info.action, |action| {
            let instance_id_res: Result<u64> = with_session!($xr_session, |session| {
                if !session.has_attached_action_set(action.action_set_id) {
                    return xr::Result::ERROR_ACTIONSET_NOT_ATTACHED;
                }
                Ok(session.instance_id)
            });

            log::debug!("get value {info:?}");

            if let Err(err) = check_path_is_valid(instance_id_res, info.subaction_path.into_raw()) {
                return err.into();
            }

            match action.subaction_value(info.subaction_path.into_raw()) {
                Ok(value) => {
                    match value.current {
                        $value_enum => {
                            state.current_state = $value.into();
                        }
                        _ => return xr::Result::ERROR_ACTION_TYPE_MISMATCH,
                    }

                    state.is_active = value.is_active.into();
                    state.changed_since_last_sync = value.changed_since_last_sync.into();
                }
                Err(err) => return err.into(),
            }

            Ok(())
        }))
    }};
}

#[allow(unreachable_code)]
pub extern "system" fn get_boolean(
    xr_session: xr::Session,
    info: *const xr::ActionStateGetInfo,
    state: *mut xr::ActionStateBoolean,
) -> xr::Result {
    get_action_value!(
        xr_session,
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
    get_action_value!(
        xr_session,
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
    get_action_value!(
        xr_session,
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

    to_xr_result(with_action!(info.action, |action| {
        let instance_id_res: Result<u64> = with_session!(xr_session, |session| {
            if !session.has_attached_action_set(action.action_set_id) {
                return xr::Result::ERROR_ACTIONSET_NOT_ATTACHED;
            }
            Ok(session.instance_id)
        });

        log::debug!("get_pose {info:?}");

        if let Err(err) = check_path_is_valid(instance_id_res, info.subaction_path.into_raw()) {
            return err.into();
        }

        match action.subaction_value(info.subaction_path.into_raw()) {
            Ok(value) => {
                match value.current {
                    SimulatedActionValue::Pose(_) => {}
                    _ => return xr::Result::ERROR_ACTION_TYPE_MISMATCH,
                }

                state.is_active = value.is_active.into();
            }
            Err(err) => return err.into(),
        }

        Ok(())
    }))
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

    to_xr_result(with_session!(xr_session, |session| {
        if !session.is_focused() {
            return xr::Result::SESSION_NOT_FOCUSED;
        }

        log::debug!("sync_actions {active_action_sets:?}");

        for active_action_set in active_action_sets {
            if !session.has_attached_action_set(active_action_set.action_set.into_raw()) {
                return xr::Result::ERROR_ACTIONSET_NOT_ATTACHED;
            }
        }

        Ok(())
    }))
}
