use crate::{input::device_state::get_device_state, prelude::*, session::with_session, utils::rotate_vec3};

#[allow(unreachable_code)]
pub extern "system" fn locate_views(
    xr_session: xr::Session,
    info: *const xr::ViewLocateInfo,
    view_state: *mut xr::ViewState,
    capacity_in: u32,
    count_out: *mut u32,
    views: *mut xr::View,
) -> xr::Result {
    // `views` is legitimately null on the standard two-call idiom: apps first
    // query the count with capacity_in=0, then call again with a real buffer.
    if info.is_null() || view_state.is_null() || count_out.is_null() {
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

        let head = get_device_state().head;

        // IPD 64mm: left eye at -32mm, right eye at +32mm, rotated by head orientation
        let eye_x_offsets = [-0.032_f32, 0.032_f32];
        for i in 0..2 {
            let view = unsafe { &mut *(views.add(i)) };
            view.ty = xr::StructureType::VIEW;
            view.next = std::ptr::null_mut();
            let eye_offset =
                rotate_vec3(&head.orientation, xr::Vector3f { x: eye_x_offsets[i], y: 0.0, z: 0.0 });
            view.pose = xr::Posef {
                orientation: head.orientation,
                position: xr::Vector3f {
                    x: head.position.x + eye_offset.x,
                    y: head.position.y + eye_offset.y,
                    z: head.position.z + eye_offset.z,
                },
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
