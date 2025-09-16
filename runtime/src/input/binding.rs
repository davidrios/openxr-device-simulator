use openxr_sys as xr;

use crate::{error::to_xr_result, instance::obj::ActionBinding, with_instance};

pub extern "system" fn suggest_interaction_profile(
    xr_instance: xr::Instance,
    suggestion: *const xr::InteractionProfileSuggestedBinding,
) -> xr::Result {
    if suggestion.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let suggestion = unsafe { &*suggestion };

    log::debug!("suggest interaction profile: {:?}", suggestion);

    to_xr_result(with_instance!(xr_instance, |instance| {
        if suggestion.count_suggested_bindings == 0 || suggestion.suggested_bindings.is_null() {
            return xr::Result::ERROR_VALIDATION_FAILURE;
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
    }))
}
