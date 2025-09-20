use crate::{error::to_xr_result, utils::copy_str_to_cchar_arr, with_instance};

pub const HMD_SYSTEM_ID: u64 = 1;

pub extern "system" fn get_system(
    xr_instance: xr::Instance,
    info: *const xr::SystemGetInfo,
    system_id: *mut xr::SystemId,
) -> xr::Result {
    if info.is_null() || system_id.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }
    if (unsafe { *info }).ty != xr::StructureType::SYSTEM_GET_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (info, system_id) = unsafe { (&*info, &mut *system_id) };

    log::debug!("get_system: {:?}", info);

    to_xr_result(with_instance!(xr_instance, |_instance| {
        if info.form_factor == xr::FormFactor::HEAD_MOUNTED_DISPLAY {
            *system_id = xr::SystemId::from_raw(HMD_SYSTEM_ID);
            Ok(())
        } else {
            Err(xr::Result::ERROR_FORM_FACTOR_UNSUPPORTED.into())
        }
    }))
}

pub extern "system" fn get_properties(
    xr_instance: xr::Instance,
    system_id: xr::SystemId,
    properties: *mut xr::SystemProperties,
) -> xr::Result {
    if system_id.into_raw() != HMD_SYSTEM_ID || properties.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let properties = unsafe { &mut *properties };

    if properties.ty != xr::StructureType::SYSTEM_PROPERTIES {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    to_xr_result(with_instance!(xr_instance, |_instance| {
        properties.system_id = system_id;
        properties.vendor_id = 0x079c98d4;
        copy_str_to_cchar_arr("openxr-device-simulator", &mut properties.system_name);
        properties.graphics_properties = xr::SystemGraphicsProperties {
            max_swapchain_image_height: 1024,
            max_swapchain_image_width: 1024,
            max_layer_count: xr::MIN_COMPOSITION_LAYERS_SUPPORTED as u32,
        };
        properties.tracking_properties.orientation_tracking = xr::TRUE;
        properties.tracking_properties.position_tracking = xr::TRUE;

        log::debug!("get_properties({:?}): {:?}", system_id, &properties);
        Ok(())
    }))
}
