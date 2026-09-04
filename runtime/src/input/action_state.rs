use crate::{
    input::{
        action::{SimulatedActionValue, with_action},
        action_set::with_action_set,
        device_state::{DeviceState, HandState},
    },
    instance::api::with_instance,
    prelude::*,
    session::with_session,
};

/// The interaction profile we prefer to resolve bindings against when an app
/// suggests several (see `sync_one_action`), and the only one we ever report
/// as "current" via `xrGetCurrentInteractionProfile` (see
/// `interaction_profile::get_current`).
pub(crate) const TARGET_INTERACTION_PROFILE: &str = "/interaction_profiles/oculus/touch_controller";

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

        let instance_id = session.instance_id;
        let device_state = crate::input::device_state::get_device_state();

        for active_action_set in active_action_sets {
            let action_ids: Vec<u64> =
                with_action_set(active_action_set.action_set.into_raw(), |set| {
                    Ok(set.actions().to_vec())
                })?;
            for action_id in action_ids {
                sync_one_action(instance_id, action_id, &device_state)?;
            }
        }

        Ok(xr::Result::SUCCESS)
    })
    .into_xr_result()
}

/// Resolve the live device value for one input suffix (e.g. "/input/trigger/value"),
/// matching the type of `current` so the action's declared type never changes.
fn resolve_value(
    current: &SimulatedActionValue,
    hand: &HandState,
    suffix: &str,
) -> Option<SimulatedActionValue> {
    use SimulatedActionValue::*;
    match (current, suffix) {
        // Per the OpenXR spec, a Boolean action bound to a float input path
        // is a valid, automatic conversion (true above a ~0.5 threshold) —
        // not every controller profile has a dedicated .../click path for
        // every button (Oculus Touch's trigger is analog-only, for
        // instance), so apps routinely bind boolean actions straight to
        // .../value instead.
        (
            Boolean(_),
            "/input/trigger/click"
            | "/input/select/click"
            | "/input/trigger/value"
            | "/input/select/value",
        ) => Some(Boolean(hand.buttons.trigger > 0.5)),
        (Boolean(_), "/input/squeeze/click" | "/input/squeeze/value" | "/input/squeeze/force") => {
            Some(Boolean(hand.buttons.squeeze > 0.5))
        }
        (Boolean(_), "/input/a/click" | "/input/x/click") => {
            Some(Boolean(hand.buttons.primary_click))
        }
        (Boolean(_), "/input/b/click" | "/input/y/click") => {
            Some(Boolean(hand.buttons.secondary_click))
        }
        (Boolean(_), "/input/menu/click") => Some(Boolean(hand.buttons.menu_click)),
        (Boolean(_), "/input/thumbstick/click") => Some(Boolean(hand.buttons.thumbstick_click)),
        (Float(_), "/input/trigger/value" | "/input/select/value") => {
            Some(Float(hand.buttons.trigger))
        }
        (Float(_), "/input/squeeze/value" | "/input/squeeze/force") => {
            Some(Float(hand.buttons.squeeze))
        }
        (Float(_), "/input/thumbstick/x") => Some(Float(hand.buttons.thumbstick.0)),
        (Float(_), "/input/thumbstick/y") => Some(Float(hand.buttons.thumbstick.1)),
        (Vector2f(_), "/input/thumbstick") => Some(Vector2f(xr::Vector2f {
            x: hand.buttons.thumbstick.0,
            y: hand.buttons.thumbstick.1,
        })),
        (Pose(_), "/input/grip/pose" | "/input/aim/pose") => Some(Pose(hand.controller_pose)),
        _ => None,
    }
}

/// Sync one action's subaction values from the live `DeviceState`, by resolving
/// each declared subaction path against the bindings suggested for it.
fn sync_one_action(instance_id: u64, action_id: u64, device_state: &DeviceState) -> Result<()> {
    let keys: Vec<u64> = with_action(action_id, |a| {
        Ok(a.subaction_values.keys().copied().collect())
    })?;

    let mut resolutions: Vec<(u64, Option<(usize, String)>)> = Vec::new();
    with_instance(instance_id, |instance| {
        // Apps commonly suggest bindings for several interaction profiles at once
        // (e.g. KHR simple + Oculus touch) for compatibility. Prefer bindings
        // suggested for our target profile so we don't pick up an incompatible
        // path (like a boolean "/input/select/click" for a float action) from a
        // profile we don't otherwise emulate; fall back to any profile if the
        // app never suggested ours.
        let target_profile_id = instance
            .paths
            .iter()
            .find(|(_, path)| path.as_str() == TARGET_INTERACTION_PROFILE)
            .map(|(id, _)| *id);

        let profile_bindings = target_profile_id
            .and_then(|id| instance.interaction_profile_bindings.get(&id))
            .filter(|bindings| bindings.iter().any(|b| b.action == action_id));

        let binding_paths: Vec<u64> = match profile_bindings {
            Some(bindings) => bindings
                .iter()
                .filter(|b| b.action == action_id)
                .map(|b| b.binding)
                .collect(),
            None => instance
                .interaction_profile_bindings
                .values()
                .flat_map(|bindings| bindings.iter())
                .filter(|b| b.action == action_id)
                .map(|b| b.binding)
                .collect(),
        };

        let mut candidates: Vec<(usize, String)> = Vec::new();
        for path_id in binding_paths {
            let path_str = instance.get_path_string(path_id)?.clone();
            if let Some(rest) = path_str.strip_prefix("/user/hand/left") {
                candidates.push((0, rest.to_string()));
            } else if let Some(rest) = path_str.strip_prefix("/user/hand/right") {
                candidates.push((1, rest.to_string()));
            }
        }

        for &key in &keys {
            let hand_filter = if key == 0 {
                None
            } else {
                let key_path = instance.get_path_string(key)?;
                if key_path.starts_with("/user/hand/left") {
                    Some(0)
                } else if key_path.starts_with("/user/hand/right") {
                    Some(1)
                } else {
                    None
                }
            };
            let found = match hand_filter {
                Some(hand) => candidates.iter().find(|(h, _)| *h == hand).cloned(),
                None => candidates.first().cloned(),
            };
            resolutions.push((key, found));
        }

        Ok(())
    })?;

    with_action(action_id, |action| {
        for (key, resolution) in &resolutions {
            let Some((hand_idx, suffix)) = resolution else {
                continue;
            };
            let hand = &device_state.hands[*hand_idx];
            if let Some(current) = action.subaction_values.get_mut(key)
                && let Some(new_value) = resolve_value(&current.current, hand, suffix)
            {
                current.changed_since_last_sync = current.current != new_value;
                if current.changed_since_last_sync {
                    log::debug!("[action {action_id}] hand={hand_idx} {suffix} -> {new_value:?}");
                    current.last_change_time =
                        crate::loader::START_TIME.elapsed().as_nanos() as u64;
                }
                current.current = new_value;
                current.is_active = true;
            }
        }
        Ok(())
    })?;

    Ok(())
}
