use crate::{error::IntoXrResult, instance::api::with_instance, system::HMD_SYSTEM_ID};

pub extern "system" fn enumerate_configurations(
    xr_instance: xr::Instance,
    system_id: xr::SystemId,
    capacity_in: u32,
    count_out: *mut u32,
    configuration_types: *mut xr::ViewConfigurationType,
) -> xr::Result {
    if system_id.into_raw() != HMD_SYSTEM_ID {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let count_out = unsafe { &mut *count_out };

    with_instance(xr_instance.into_raw(), |_instance| {
        if capacity_in == 0 {
            *count_out = 1;
            return Ok(());
        }

        if *count_out != 1 {
            return Err(xr::Result::ERROR_SIZE_INSUFFICIENT.into());
        }

        if configuration_types.is_null() {
            return Err(xr::Result::ERROR_VALIDATION_FAILURE.into());
        }

        unsafe { *configuration_types = xr::ViewConfigurationType::PRIMARY_STEREO }
        Ok(())
    })
    .into_xr_result()
}

pub extern "system" fn get_configuration_properties(
    xr_instance: xr::Instance,
    system_id: xr::SystemId,
    configuration_type: xr::ViewConfigurationType,
    properties: *mut xr::ViewConfigurationProperties,
) -> xr::Result {
    if system_id.into_raw() != HMD_SYSTEM_ID {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    if configuration_type != xr::ViewConfigurationType::PRIMARY_STEREO {
        return xr::Result::ERROR_VIEW_CONFIGURATION_TYPE_UNSUPPORTED;
    }

    let properties = unsafe { &mut *properties };

    if properties.ty != xr::StructureType::VIEW_CONFIGURATION_PROPERTIES {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    with_instance(xr_instance.into_raw(), |_instance| {
        properties.view_configuration_type = xr::ViewConfigurationType::PRIMARY_STEREO;
        properties.fov_mutable = xr::FALSE;
        Ok(())
    })
    .into_xr_result()
}

pub extern "system" fn enumerate_configuration_views(
    xr_instance: xr::Instance,
    system_id: xr::SystemId,
    configuration_type: xr::ViewConfigurationType,
    capacity_in: u32,
    count_out: *mut u32,
    views: *mut xr::ViewConfigurationView,
) -> xr::Result {
    if system_id.into_raw() != HMD_SYSTEM_ID {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    if configuration_type != xr::ViewConfigurationType::PRIMARY_STEREO {
        return xr::Result::ERROR_VIEW_CONFIGURATION_TYPE_UNSUPPORTED;
    }

    let count_out = unsafe { &mut *count_out };

    with_instance(xr_instance.into_raw(), |_instance| {
        if capacity_in == 0 {
            *count_out = 2;
            return Ok(());
        }

        if *count_out != 2 {
            return Err(xr::Result::ERROR_SIZE_INSUFFICIENT.into());
        }

        if views.is_null() {
            return Err(xr::Result::ERROR_VALIDATION_FAILURE.into());
        }

        unsafe {
            // for left and right eyes
            for i in 0..2 {
                *views.add(i) = xr::ViewConfigurationView {
                    ty: (*views).ty,
                    next: std::ptr::null_mut(),
                    recommended_image_rect_width: 1024,
                    max_image_rect_width: 1024,
                    recommended_image_rect_height: 1024,
                    max_image_rect_height: 1024,
                    recommended_swapchain_sample_count: 1,
                    max_swapchain_sample_count: 1,
                };
            }
        };

        Ok(())
    })
    .into_xr_result()
}
