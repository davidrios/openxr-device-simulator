use crate::{prelude::*, session::with_session};

pub extern "system" fn enumerate(
    xr_session: xr::Session,
    capacity_in: u32,
    count_out: *mut u32,
    space_types: *mut xr::ReferenceSpaceType,
) -> xr::Result {
    let count_out = unsafe { &mut *count_out };

    with_session(xr_session.into_raw(), |_session| {
        if capacity_in == 0 {
            *count_out = 3;
            return Ok(());
        }

        if *count_out != 3 {
            return Err(xr::Result::ERROR_SIZE_INSUFFICIENT.into());
        }

        unsafe {
            *space_types = xr::ReferenceSpaceType::VIEW;
            *space_types.add(1) = xr::ReferenceSpaceType::LOCAL;
            *space_types.add(2) = xr::ReferenceSpaceType::LOCAL_FLOOR;
        }

        Ok(())
    })
    .into_xr_result()
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

    with_session(xr_session.into_raw(), |session| {
        let space_id = super::create(
            session,
            super::SimulatedSpaceType::Reference(SimulatedReferenceSpace {
                pose: create_info.pose_in_reference_space,
            }),
        )?;
        *xr_space = xr::Space::from_raw(space_id);
        Ok(())
    })
    .into_xr_result()
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

    with_session(xr_session.into_raw(), |_session| {
        bounds.width = 0.0;
        bounds.height = 0.0;
        return Err(xr::Result::SPACE_BOUNDS_UNAVAILABLE.into());
        Ok(())
    })
    .into_xr_result()
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SimulatedReferenceSpace {
    pub(crate) pose: xr::Posef,
}
