use crate::{
    error::IntoXrResult,
    instance::{api::with_instance, obj::ActionBinding},
    session::with_session,
};

pub extern "system" fn suggest(
    xr_instance: xr::Instance,
    suggestion: *const xr::InteractionProfileSuggestedBinding,
) -> xr::Result {
    if suggestion.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let suggestion = unsafe { &*suggestion };

    log::debug!("suggest interaction profile: {:?}", suggestion);

    with_instance(xr_instance.into_raw(), |instance| {
        if suggestion.count_suggested_bindings == 0 || suggestion.suggested_bindings.is_null() {
            return Err(xr::Result::ERROR_VALIDATION_FAILURE.into());
        }

        let mut bindings = Vec::new();

        for i in 0..suggestion.count_suggested_bindings {
            let binding = unsafe { &(*suggestion.suggested_bindings.add(i as usize)) };
            bindings.push(ActionBinding::new(
                binding.action.into_raw(),
                binding.binding.into_raw(),
            ))
        }

        instance
            .set_interaction_profile_bindings(suggestion.interaction_profile.into_raw(), bindings)
    })
    .into_xr_result()
}

#[allow(unreachable_code)]
pub extern "system" fn get_current(
    xr_session: xr::Session,
    top_level_user_path: xr::Path,
    interaction_profile: *mut xr::InteractionProfileState,
) -> xr::Result {
    if interaction_profile.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let _interaction_profile = unsafe { &mut *interaction_profile };

    with_session(xr_session.into_raw(), |_session| {
        log::debug!("get_current {top_level_user_path:?}");
        return Err(xr::Result::ERROR_FUNCTION_UNSUPPORTED.into());
        Ok(())
    })
    .into_xr_result()
}
