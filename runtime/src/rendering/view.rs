use crate::{prelude::*, session::with_session};

/// Build a quaternion from yaw (Y-axis) then pitch (X-axis) rotations.
fn quat_yaw_pitch(yaw: f32, pitch: f32) -> xr::Quaternionf {
    let (sy, cy) = (yaw * 0.5).sin_cos();
    let (sp, cp) = (pitch * 0.5).sin_cos();
    // q = q_yaw * q_pitch
    xr::Quaternionf {
        x: cy * sp,
        y: sy * cp,
        z: -sy * sp,
        w: cy * cp,
    }
}

/// Rotate a vector by a unit quaternion: v' = q v q*
fn rotate_vec3(q: &xr::Quaternionf, v: xr::Vector3f) -> xr::Vector3f {
    // t = 2 * (q_xyz × v)
    let tx = 2.0 * (q.y * v.z - q.z * v.y);
    let ty = 2.0 * (q.z * v.x - q.x * v.z);
    let tz = 2.0 * (q.x * v.y - q.y * v.x);
    xr::Vector3f {
        x: v.x + q.w * tx + q.y * tz - q.z * ty,
        y: v.y + q.w * ty + q.z * tx - q.x * tz,
        z: v.z + q.w * tz + q.x * ty - q.y * tx,
    }
}

#[allow(unreachable_code)]
pub extern "system" fn locate_views(
    xr_session: xr::Session,
    info: *const xr::ViewLocateInfo,
    view_state: *mut xr::ViewState,
    capacity_in: u32,
    count_out: *mut u32,
    views: *mut xr::View,
) -> xr::Result {
    if info.is_null() || view_state.is_null() || count_out.is_null() || views.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let info = unsafe { &*info };

    if !matches!(
        info.view_configuration_type,
        xr::ViewConfigurationType::PRIMARY_STEREO
    ) {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    unsafe { *count_out = 2 }

    if capacity_in == 0 {
        return xr::Result::SUCCESS;
    }

    if capacity_in < 2 {
        return xr::Result::ERROR_SIZE_INSUFFICIENT;
    }

    if views.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    log::debug!("locate_views {info:?}");

    with_session(xr_session.into_raw(), |session| {
        if !session.space_ids.contains_key(&info.space.into_raw()) {
            return Err(xr::Result::ERROR_SESSION_LOST.into());
        }

        let view_state = unsafe { &mut *view_state };
        view_state.view_state_flags = xr::ViewStateFlags::from_raw(0b1111);

        let (yaw, pitch) = crate::server::get_head_look();
        let head_quat = quat_yaw_pitch(yaw, pitch);

        // IPD 64mm: left eye at -32mm, right eye at +32mm, rotated by head orientation
        let eye_x_offsets = [-0.032_f32, 0.032_f32];
        for i in 0..2 {
            let view = unsafe { &mut *(views.add(i)) };
            view.ty = xr::StructureType::VIEW;
            view.next = std::ptr::null_mut();
            view.pose = xr::Posef {
                orientation: head_quat,
                position: rotate_vec3(
                    &head_quat,
                    xr::Vector3f { x: eye_x_offsets[i], y: 0.0, z: 0.0 },
                ),
            };
            view.fov = xr::Fovf {
                angle_left: -std::f32::consts::FRAC_PI_4,
                angle_right: std::f32::consts::FRAC_PI_4,
                angle_up: std::f32::consts::FRAC_PI_4,
                angle_down: -std::f32::consts::FRAC_PI_4,
            };
        }

        Ok(())
    })
    .into_xr_result()
}
