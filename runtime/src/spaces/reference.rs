use openxr_sys as xr;

use crate::with_session;

pub extern "system" fn enumerate(
    xr_session: xr::Session,
    capacity_in: u32,
    count_out: *mut u32,
    space_types: *mut xr::ReferenceSpaceType,
) -> xr::Result {
    let count_out = unsafe { &mut *count_out };

    with_session!(xr_session, |_session| {
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
    });

    xr::Result::SUCCESS
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

    with_session!(xr_session, |session| {
        let space_id = match super::create(
            session,
            super::SimulatedSpaceType::Reference(SimulatedReferenceSpace {
                pose: create_info.pose_in_reference_space,
            }),
        ) {
            Ok(space_id) => space_id,
            Err(err) => match err {
                crate::error::Error::XrResult(res) => return res,
                _ => {
                    log::error!("{err}");
                    return xr::Result::ERROR_RUNTIME_FAILURE;
                }
            },
        };

        *xr_space = xr::Space::from_raw(space_id);

        xr::Result::SUCCESS
    })
}

#[derive(Debug)]
pub struct SimulatedReferenceSpace {
    pose: xr::Posef,
}
