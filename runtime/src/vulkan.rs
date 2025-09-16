use std::ffi::c_char;

use ash::vk::Handle;
use openxr_sys as xr;

use crate::{error::to_xr_result, system::HMD_SYSTEM_ID, with_instance};

pub extern "system" fn get_graphics_requirements(
    xr_instance: xr::Instance,
    system_id: xr::SystemId,
    requirements: *mut xr::GraphicsRequirementsVulkanKHR,
) -> xr::Result {
    if system_id.into_raw() != HMD_SYSTEM_ID {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let requirements = unsafe { &mut *requirements };

    if requirements.ty != xr::StructureType::GRAPHICS_REQUIREMENTS_VULKAN_KHR {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    to_xr_result(with_instance!(xr_instance, |_instance| {
        requirements.min_api_version_supported = xr::Version::new(1, 0, 0);
        requirements.max_api_version_supported = xr::Version::new(1, 3, 0);
        Ok(())
    }))
}

pub extern "system" fn get_graphics_device(
    xr_instance: xr::Instance,
    system_id: xr::SystemId,
    vk_instance: u64,
    vk_physical_device: *mut u64,
) -> xr::Result {
    if system_id.into_raw() != HMD_SYSTEM_ID {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }
    if vk_instance == 0 {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let entry = match unsafe { ash::Entry::load() } {
        Ok(e) => e,
        Err(e) => {
            log::error!("failed to load Vulkan loader: {}", e);
            return xr::Result::ERROR_RUNTIME_FAILURE;
        }
    };

    let vk_instance = ash::vk::Instance::from_raw(vk_instance);
    let handle = unsafe {
        let vk_instance = ash::Instance::load(entry.static_fn(), vk_instance);
        let devs = match vk_instance.enumerate_physical_devices() {
            Ok(e) => e,
            Err(e) => {
                log::error!("failed to enumerate Vulkan devices: {}", e);
                return xr::Result::ERROR_RUNTIME_FAILURE;
            }
        };

        if let Some(dev) = devs.first() {
            dev.as_raw()
        } else {
            return xr::Result::ERROR_RUNTIME_FAILURE;
        }
    };

    to_xr_result(with_instance!(xr_instance, |_instance| {
        unsafe { *vk_physical_device = handle }
        log::debug!("returning graphics device {handle:x}");
        Ok(())
    }))
}

pub extern "system" fn get_instance_extensions(
    xr_instance: xr::Instance,
    system_id: xr::SystemId,
    capacity_in: u32,
    count_out: *mut u32,
    buffer: *mut c_char,
) -> xr::Result {
    if system_id.into_raw() != HMD_SYSTEM_ID {
        return xr::Result::ERROR_VALIDATION_FAILURE;
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

        unsafe { *buffer = 0 }
        Ok(())
    }))
}

pub extern "system" fn get_device_extensions(
    xr_instance: xr::Instance,
    system_id: xr::SystemId,
    capacity_in: u32,
    count_out: *mut u32,
    buffer: *mut c_char,
) -> xr::Result {
    if system_id.into_raw() != HMD_SYSTEM_ID {
        return xr::Result::ERROR_VALIDATION_FAILURE;
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

        unsafe { *buffer = 0 }
        Ok(())
    }))
}
