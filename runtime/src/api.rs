use openxr_sys as xr;

#[unsafe(no_mangle)]
pub extern "C" fn xr_get_system(
    _instance: xr::Instance,
    get_info: *const xr::SystemGetInfo,
    system_id: *mut xr::SystemId,
) -> xr::Result {
    // if (*get_info).ty != xr::StructureType::SYSTEM_GET_INFO
    //     || (*get_info).form_factor != xr::FormFactor::HEAD_MOUNTED_DISPLAY
    // {
    //     return xr::Result::ERROR_VALIDATION_FAILURE;
    // }
    //
    // *system_id = SIMULATED_SYSTEM_ID;
    xr::Result::ERROR_FUNCTION_UNSUPPORTED
}

// --- Space and Pose ---

#[unsafe(no_mangle)]
pub extern "C" fn xr_locate_space(
    space: xr::Space,
    base_space: xr::Space,
    _time: xr::Time,
    location: *mut xr::SpaceLocation,
) -> xr::Result {
    // if (*location).ty != xr::StructureType::SPACE_LOCATION {
    //     return xr::Result::ERROR_VALIDATION_FAILURE;
    // }

    // // As before, we use simple integer handles for spaces for this simulation.
    // const VIEW_SPACE_HANDLE: xr::Space = xr::Space::from_raw(1);
    // const GRIP_SPACE_HANDLE: xr::Space = xr::Space::from_raw(2);
    // const STAGE_SPACE_HANDLE: xr::Space = xr::Space::from_raw(3);
    //
    // let mut pose = create_identity_pose();
    // let mut flags =
    //     xr::SpaceLocationFlags::ORIENTATION_VALID | xr::SpaceLocationFlags::POSITION_VALID;
    //
    // if space == VIEW_SPACE_HANDLE && base_space == STAGE_SPACE_HANDLE {
    //     // HMD is 1.6m high
    //     pose.position.y = 1.6;
    // } else if space == GRIP_SPACE_HANDLE && base_space == STAGE_SPACE_HANDLE {
    //     // Controller is to the right and forward
    //     pose.position.x = 0.4;
    //     pose.position.y = 1.2;
    //     pose.position.z = -0.3;
    // } else {
    //     // All other combinations are identity
    // }
    //
    // (*location).pose = pose;
    // (*location).location_flags = flags;
    xr::Result::ERROR_FUNCTION_UNSUPPORTED
}

// --- Input and Actions ---

#[unsafe(no_mangle)]
pub extern "C" fn xr_sync_actions(
    _session: xr::Session,
    _sync_info: *const xr::ActionsSyncInfo,
) -> xr::Result {
    // In a real app, you'd poll hardware. Here, we could check for a key press
    // to toggle our simulated trigger state for testing.
    // For now, we'll just leave it as is.
    // *IS_TRIGGER_PRESSED.lock().unwrap() = true; // Example: Force trigger press
    xr::Result::ERROR_FUNCTION_UNSUPPORTED
}

#[unsafe(no_mangle)]
pub extern "C" fn xr_get_action_state_boolean(
    _session: xr::Session,
    _get_info: *const xr::ActionStateGetInfo,
    state: *mut xr::ActionStateBoolean,
) -> xr::Result {
    // if (*state).ty != xr::StructureType::ACTION_STATE_BOOLEAN {
    //     return xr::Result::ERROR_VALIDATION_FAILURE;
    // }
    //
    // (*state).current_state = (*IS_TRIGGER_PRESSED.lock().unwrap()).into();
    // (*state).changed_since_last_sync = xr::TRUE;
    // (*state).is_active = xr::TRUE;

    xr::Result::ERROR_FUNCTION_UNSUPPORTED
}

#[unsafe(no_mangle)]
pub extern "C" fn xr_get_action_state_pose(
    _session: xr::Session,
    _get_info: *const xr::ActionStateGetInfo,
    state: *mut xr::ActionStatePose,
) -> xr::Result {
    // if (*state).ty != xr::StructureType::ACTION_STATE_POSE {
    //     return xr::Result::ERROR_VALIDATION_FAILURE;
    // }
    //
    // (*state).is_active = xr::TRUE;
    xr::Result::ERROR_FUNCTION_UNSUPPORTED
}
