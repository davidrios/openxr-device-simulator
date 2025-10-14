use crate::{error::IntoXrResult, session::with_session};

pub extern "system" fn create(
    xr_session: xr::Session,
    create_info: *const xr::ActionSpaceCreateInfo,
    xr_space: *mut xr::Space,
) -> xr::Result {
    if create_info.is_null() || xr_space.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (create_info, xr_space) = unsafe { (&*create_info, &mut *xr_space) };

    if create_info.ty != xr::StructureType::ACTION_SPACE_CREATE_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    with_session(xr_session.into_raw(), |session| {
        let space_id = super::create(
            session,
            super::SimulatedSpaceType::Action(SimulatedActionSpace {
                action: create_info.action.into_raw(),
                subaction_path: create_info.subaction_path.into_raw(),
                pose: create_info.pose_in_action_space,
            }),
        )?;
        *xr_space = xr::Space::from_raw(space_id);
        Ok(())
    })
    .into_xr_result()
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SimulatedActionSpace {
    pub(crate) action: u64,
    pub(crate) subaction_path: u64,
    pub(crate) pose: xr::Posef,
}
