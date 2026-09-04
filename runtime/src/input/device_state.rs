use std::sync::{LazyLock, Mutex};

use crate::utils::create_identity_pose;

/// Live state of the simulated device, updated by the web client over the
/// server's socket connection and read by the rendering/space code.
#[derive(Debug, Clone, Copy)]
pub struct DeviceState {
    pub head: xr::Posef,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self { head: create_identity_pose() }
    }
}

static DEVICE_STATE: LazyLock<Mutex<DeviceState>> =
    LazyLock::new(|| Mutex::new(DeviceState::default()));

pub fn get_device_state() -> DeviceState {
    *DEVICE_STATE.lock().unwrap()
}

pub fn set_head_pose(pose: xr::Posef) {
    DEVICE_STATE.lock().unwrap().head = pose;
}
