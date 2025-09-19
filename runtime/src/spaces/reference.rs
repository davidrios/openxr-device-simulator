use openxr_sys as xr;

use crate::{error::to_xr_result, with_session};

pub extern "system" fn enumerate(
    xr_session: xr::Session,
    capacity_in: u32,
    count_out: *mut u32,
    space_types: *mut xr::ReferenceSpaceType,
) -> xr::Result {
    let count_out = unsafe { &mut *count_out };

    to_xr_result(with_session!(xr_session, |_session| {
        if capacity_in == 0 {
            *count_out = 3;
            return xr::Result::SUCCESS;
        }

        if *count_out != 3 {
            return xr::Result::ERROR_SIZE_INSUFFICIENT;
        }

        unsafe {
            *space_types = xr::ReferenceSpaceType::VIEW;
            *space_types.add(1) = xr::ReferenceSpaceType::LOCAL;
            *space_types.add(2) = xr::ReferenceSpaceType::LOCAL_FLOOR;
        }

        Ok(())
    }))
}

pub extern "system" fn create(
    xr_session: xr::Session,
    create_info: *const xr::ReferenceSpaceCreateInfo,
    xr_space: *mut xr::Space,
) -> xr::Result {
    if create_info.is_null() || xr_space.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (create_info, xr_space) = unsafe { (&*create_info, &mut *xr_space) };

    if create_info.ty != xr::StructureType::REFERENCE_SPACE_CREATE_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    to_xr_result(with_session!(xr_session, |session| {
        match super::create(
            session,
            super::SimulatedSpaceType::Reference(SimulatedReferenceSpace {
                pose: create_info.pose_in_reference_space,
            }),
        ) {
            Ok(space_id) => {
                *xr_space = xr::Space::from_raw(space_id);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }))
}

#[allow(unreachable_code)]
pub extern "system" fn get_bounds_rect(
    xr_session: xr::Session,
    ref_space_type: xr::ReferenceSpaceType,
    bounds: *mut xr::Extent2Df,
) -> xr::Result {
    if bounds.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let bounds = unsafe { &mut *bounds };

    log::debug!("get_bounds_rect {ref_space_type:?}");

    to_xr_result(with_session!(xr_session, |_session| {
        bounds.width = 0.0;
        bounds.height = 0.0;
        return xr::Result::SPACE_BOUNDS_UNAVAILABLE;
        Ok(())
    }))
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SimulatedReferenceSpace {
    pose: xr::Posef,
}
