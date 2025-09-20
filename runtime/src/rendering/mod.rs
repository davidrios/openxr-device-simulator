use crate::{error::to_xr_result, system::HMD_SYSTEM_ID, with_instance};

pub mod frame;
pub mod swapchain;
pub mod view;

pub extern "system" fn enumerate_blend_modes(
    xr_instance: xr::Instance,
    system_id: xr::SystemId,
    view_configuration_type: xr::ViewConfigurationType,
    capacity_in: u32,
    count_out: *mut u32,
    blend_mode: *mut xr::EnvironmentBlendMode,
) -> xr::Result {
    if system_id.into_raw() != HMD_SYSTEM_ID {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    log::debug!("enumerate_blend_modes: {:?}", view_configuration_type);

    if view_configuration_type != xr::ViewConfigurationType::PRIMARY_STEREO {
        return xr::Result::ERROR_VIEW_CONFIGURATION_TYPE_UNSUPPORTED;
    }

    let count_out = unsafe { &mut *count_out };

    to_xr_result(with_instance!(xr_instance, |_instance| {
        if capacity_in == 0 {
            *count_out = 1;
            return xr::Result::SUCCESS;
        }

        if *count_out != 1 {
            return xr::Result::ERROR_SIZE_INSUFFICIENT;
        }

        if blend_mode.is_null() {
            return xr::Result::ERROR_VALIDATION_FAILURE;
        }

        unsafe { *blend_mode = xr::EnvironmentBlendMode::OPAQUE }

        Ok(())
    }))
}
