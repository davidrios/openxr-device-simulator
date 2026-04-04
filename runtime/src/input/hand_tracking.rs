use std::{
    cell::UnsafeCell,
    collections::HashMap,
    sync::{LazyLock, Mutex, atomic},
};

use crate::{prelude::*, session::with_session, utils::with_obj_instance};

// ---------------------------------------------------------------------------
// Static mock joint poses (open-palm, fingers extended toward -Z)
// Each entry is (x, y, z) offset from the wrist in hand-local space.
// For the right hand positive-X is away from the body center; left hand mirrors X.
// ---------------------------------------------------------------------------
const JOINT_OFFSETS: [(f32, f32, f32); xr::HAND_JOINT_COUNT_EXT as usize] = [
    // 0  PALM
    (0.00, 0.00, -0.02),
    // 1  WRIST
    (0.00, 0.00, 0.00),
    // 2  THUMB_METACARPAL
    (-0.02, 0.00, -0.01),
    // 3  THUMB_PROXIMAL
    (-0.04, 0.01, -0.03),
    // 4  THUMB_DISTAL
    (-0.05, 0.01, -0.05),
    // 5  THUMB_TIP
    (-0.06, 0.01, -0.07),
    // 6  INDEX_METACARPAL
    (-0.01, 0.00, -0.04),
    // 7  INDEX_PROXIMAL
    (-0.01, 0.00, -0.08),
    // 8  INDEX_INTERMEDIATE
    (-0.01, 0.00, -0.11),
    // 9  INDEX_DISTAL
    (-0.01, 0.00, -0.13),
    // 10 INDEX_TIP
    (-0.01, 0.00, -0.15),
    // 11 MIDDLE_METACARPAL
    (0.00, 0.00, -0.04),
    // 12 MIDDLE_PROXIMAL
    (0.00, 0.00, -0.09),
    // 13 MIDDLE_INTERMEDIATE
    (0.00, 0.00, -0.12),
    // 14 MIDDLE_DISTAL
    (0.00, 0.00, -0.14),
    // 15 MIDDLE_TIP
    (0.00, 0.00, -0.16),
    // 16 RING_METACARPAL
    (0.01, 0.00, -0.04),
    // 17 RING_PROXIMAL
    (0.01, 0.00, -0.08),
    // 18 RING_INTERMEDIATE
    (0.01, 0.00, -0.11),
    // 19 RING_DISTAL
    (0.01, 0.00, -0.13),
    // 20 RING_TIP
    (0.01, 0.00, -0.14),
    // 21 LITTLE_METACARPAL
    (0.02, 0.00, -0.03),
    // 22 LITTLE_PROXIMAL
    (0.02, 0.00, -0.07),
    // 23 LITTLE_INTERMEDIATE
    (0.02, 0.00, -0.09),
    // 24 LITTLE_DISTAL
    (0.02, 0.00, -0.11),
    // 25 LITTLE_TIP
    (0.02, 0.00, -0.12),
];

// Wrist world positions for each hand.
const LEFT_WRIST: (f32, f32, f32) = (-0.30, -0.30, -0.5);
const RIGHT_WRIST: (f32, f32, f32) = (0.30, -0.30, -0.5);

fn joint_pose(hand: xr::HandEXT, joint_idx: usize) -> xr::Posef {
    let (wx, wy, wz) = if hand == xr::HandEXT::LEFT { LEFT_WRIST } else { RIGHT_WRIST };
    let (dx, dy, dz) = JOINT_OFFSETS[joint_idx];
    // Mirror X offset for left hand
    let x_sign = if hand == xr::HandEXT::LEFT { -1.0_f32 } else { 1.0_f32 };
    xr::Posef {
        orientation: xr::Quaternionf { x: 0.0, y: 0.0, z: 0.0, w: 1.0 },
        position: xr::Vector3f {
            x: wx + x_sign * dx,
            y: wy + dy,
            z: wz + dz,
        },
    }
}

// ---------------------------------------------------------------------------
// Tracker object
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct SimulatedHandTracker {
    session_id: u64,
    id: u64,
    hand: xr::HandEXT,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

pub extern "system" fn create(
    xr_session: xr::Session,
    create_info: *const xr::HandTrackerCreateInfoEXT,
    hand_tracker: *mut xr::HandTrackerEXT,
) -> xr::Result {
    if create_info.is_null() || hand_tracker.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (create_info, hand_tracker) = unsafe { (&*create_info, &mut *hand_tracker) };

    if create_info.ty != xr::StructureType::HAND_TRACKER_CREATE_INFO_EXT {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    with_session(xr_session.into_raw(), |session| {
        let mut instances = INSTANCES.lock().unwrap();
        let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        instances.insert(
            next_id,
            UnsafeCell::new(SimulatedHandTracker {
                session_id: session.id,
                id: next_id,
                hand: create_info.hand,
            }),
        );
        *hand_tracker = xr::HandTrackerEXT::from_raw(next_id);
        log::debug!("created hand tracker {next_id} for {:?}", create_info.hand);
        Ok(())
    })
    .into_xr_result()
}

pub extern "system" fn destroy(xr_hand_tracker: xr::HandTrackerEXT) -> xr::Result {
    if xr_hand_tracker == xr::HandTrackerEXT::NULL {
        return xr::Result::ERROR_HANDLE_INVALID;
    }

    INSTANCES.lock().unwrap().remove(&xr_hand_tracker.into_raw());
    xr::Result::SUCCESS
}

pub extern "system" fn locate_hand_joints(
    xr_hand_tracker: xr::HandTrackerEXT,
    locate_info: *const xr::HandJointsLocateInfoEXT,
    locations: *mut xr::HandJointLocationsEXT,
) -> xr::Result {
    if locate_info.is_null() || locations.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let locations = unsafe { &mut *locations };

    if locations.ty != xr::StructureType::HAND_JOINT_LOCATIONS_EXT {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    if locations.joint_locations.is_null()
        || locations.joint_count < xr::HAND_JOINT_COUNT_EXT
    {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    with_hand_tracker(xr_hand_tracker.into_raw(), |tracker| {
        locations.is_active = xr::TRUE;
        locations.joint_count = xr::HAND_JOINT_COUNT_EXT;

        for i in 0..xr::HAND_JOINT_COUNT_EXT as usize {
            let joint = unsafe { &mut *locations.joint_locations.add(i) };
            joint.location_flags = xr::SpaceLocationFlags::from_raw(0b1111);
            joint.pose = joint_pose(tracker.hand, i);
            joint.radius = 0.01;
        }

        log::debug!("locate_hand_joints {:?}", tracker.hand);
        Ok(())
    })
    .into_xr_result()
}

// ---------------------------------------------------------------------------
// Storage
// ---------------------------------------------------------------------------

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);
type SharedTracker = UnsafeCell<SimulatedHandTracker>;
static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedTracker>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn with_hand_tracker<T, F>(id: u64, f: F) -> Result<T>
where
    F: FnMut(&mut SimulatedHandTracker) -> Result<T>,
{
    with_obj_instance(&INSTANCES, id, f)
}
