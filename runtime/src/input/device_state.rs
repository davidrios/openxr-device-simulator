use std::sync::{LazyLock, Mutex};

use crate::utils::create_identity_pose;

/// Index into `DeviceState::hands`.
pub const LEFT: usize = 0;
pub const RIGHT: usize = 1;

/// A literal-origin head pose sits at floor level in the STAGE space,
/// coincident with (or inside) any scene geometry placed near the origin —
/// apps look broken (e.g. bevy_oxr's 3d_scene renders solid black) until the
/// user manually raises the viewpoint. Default to a plausible standing eye
/// height instead, matching a real headset. Kept in sync with the web
/// client's own `position`/hand defaults in `MainPage.vue`, since those
/// overwrite this default the moment a client connects and starts streaming
/// input every frame.
const DEFAULT_STANDING_HEIGHT: f32 = 1.6;

#[derive(Debug, Clone, Copy, Default)]
pub struct ControllerButtons {
    pub trigger: f32,
    pub squeeze: f32,
    pub thumbstick: (f32, f32),
    pub thumbstick_click: bool,
    pub primary_click: bool,
    pub secondary_click: bool,
    pub menu_click: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct HandState {
    pub controller_pose: xr::Posef,
    pub buttons: ControllerButtons,
}

impl HandState {
    fn resting(x: f32) -> Self {
        Self {
            controller_pose: xr::Posef {
                orientation: xr::Quaternionf { x: 0.0, y: 0.0, z: 0.0, w: 1.0 },
                position: xr::Vector3f { x, y: DEFAULT_STANDING_HEIGHT - 0.30, z: -0.5 },
            },
            buttons: ControllerButtons::default(),
        }
    }
}

/// Live state of the simulated device, updated by the web client over the
/// server's socket connection and read by the rendering/input/space code.
#[derive(Debug, Clone, Copy)]
pub struct DeviceState {
    pub head: xr::Posef,
    pub hands: [HandState; 2],
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            head: xr::Posef {
                position: xr::Vector3f { x: 0.0, y: DEFAULT_STANDING_HEIGHT, z: 0.0 },
                ..create_identity_pose()
            },
            // Matches the resting wrist positions used by hand_tracking.rs.
            hands: [HandState::resting(-0.30), HandState::resting(0.30)],
        }
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

pub fn set_hand(index: usize, hand: HandState) {
    DEVICE_STATE.lock().unwrap().hands[index] = hand;
}
