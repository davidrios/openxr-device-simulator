use std::ffi::c_char;

use ash::vk::{Handle, KHR_SURFACE_NAME, KHR_SWAPCHAIN_NAME, KHR_WAYLAND_SURFACE_NAME, QueueFlags};

use crate::{instance::api::with_instance, prelude::*, system::HMD_SYSTEM_ID, utils::ExtList};

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

    with_instance(xr_instance.into_raw(), |_instance| {
        requirements.min_api_version_supported = xr::Version::new(1, 0, 0);
        requirements.max_api_version_supported = xr::Version::new(1, 3, 0);
        Ok(())
    })
    .into_xr_result()
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

        let mut dev = None;

        for p in devs.iter() {
            let queue_families = vk_instance.get_physical_device_queue_family_properties(*p);
            for qf in queue_families.iter() {
                if !qf.queue_flags.contains(QueueFlags::GRAPHICS) {
                    continue;
                }

                dev = Some(p);
            }
        }

        if let Some(dev) = dev {
            dev.as_raw()
        } else {
            return xr::Result::ERROR_RUNTIME_FAILURE;
        }
    };

    with_instance(xr_instance.into_raw(), |_instance| {
        unsafe { *vk_physical_device = handle }
        log::debug!("returning graphics device {handle:x}");
        Ok(())
    })
    .into_xr_result()
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

    with_instance(xr_instance.into_raw(), |_instance| {
        let exts = ExtList::new(vec![
            KHR_SURFACE_NAME.to_bytes(),
            KHR_WAYLAND_SURFACE_NAME.to_bytes(),
        ]);
        let size = exts.len();

        if capacity_in == 0 {
            *count_out = size as u32;
            return Ok(());
        }

        if *count_out != size as u32 {
            return Err(xr::Result::ERROR_SIZE_INSUFFICIENT.into());
        }

        exts.copy_to_cchar_ptr(buffer);

        Ok(())
    })
    .into_xr_result()
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

    with_instance(xr_instance.into_raw(), |_instance| {
        let exts = ExtList::new(vec![KHR_SWAPCHAIN_NAME.to_bytes()]);
        let size = exts.len();

        if capacity_in == 0 {
            *count_out = size as u32;
            return Ok(());
        }

        if *count_out != size as u32 {
            return Err(xr::Result::ERROR_SIZE_INSUFFICIENT.into());
        }

        if buffer.is_null() {
            return Err(xr::Result::ERROR_VALIDATION_FAILURE.into());
        }

        exts.copy_to_cchar_ptr(buffer);

        Ok(())
    })
    .into_xr_result()
}
