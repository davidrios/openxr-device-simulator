use crate::{
    input::action_state::TARGET_INTERACTION_PROFILE,
    instance::{api::with_instance, obj::ActionBinding},
    prelude::*,
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

pub extern "system" fn get_current(
    xr_session: xr::Session,
    top_level_user_path: xr::Path,
    interaction_profile: *mut xr::InteractionProfileState,
) -> xr::Result {
    if interaction_profile.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let interaction_profile = unsafe { &mut *interaction_profile };

    with_session(xr_session.into_raw(), |session| {
        log::debug!("get_current {top_level_user_path:?}");

        // Per spec this reports XR_NULL_PATH (not an error) whenever no
        // profile is bound for the given top-level user path — apps like
        // Godot rely on this call succeeding (with a real profile) before
        // they'll start polling button/trigger action states at all, so
        // unconditionally erroring here (as this used to) silently starved
        // every action after pose actions of ever being read.
        let user_path = with_instance(session.instance_id, |instance| {
            Ok(instance
                .get_path_string(top_level_user_path.into_raw())?
                .clone())
        })?;

        let bound_profile = if user_path == "/user/hand/left" || user_path == "/user/hand/right" {
            with_instance(session.instance_id, |instance| {
                let profile_id = instance
                    .paths
                    .iter()
                    .find(|(_, path)| path.as_str() == TARGET_INTERACTION_PROFILE)
                    .map(|(id, _)| *id);

                Ok(profile_id.filter(|id| instance.interaction_profile_bindings.contains_key(id)))
            })?
        } else {
            None
        };

        interaction_profile.interaction_profile = xr::Path::from_raw(bound_profile.unwrap_or(0));

        Ok(())
    })
    .into_xr_result()
}
