use std::{ffi::c_char, sync::atomic};

use ash::vk::{Handle, KHR_SURFACE_NAME, KHR_SWAPCHAIN_NAME, KHR_WAYLAND_SURFACE_NAME, QueueFlags};
use xr::platform::{VkDevice, VkInstance, VkPhysicalDevice, VkResult};

use crate::{instance::api::with_instance, prelude::*, system::HMD_SYSTEM_ID, utils::ExtList};

static CREATED_VK_INSTANCE: atomic::AtomicU64 = atomic::AtomicU64::new(0);

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

// ---------------------------------------------------------------------------
// XR_KHR_vulkan_enable2
// ---------------------------------------------------------------------------

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

pub extern "system" fn get_graphics_requirements2(
    xr_instance: xr::Instance,
    system_id: xr::SystemId,
    requirements: *mut xr::GraphicsRequirementsVulkan2KHR,
) -> xr::Result {
    if system_id.into_raw() != HMD_SYSTEM_ID {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }
    if requirements.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }
    let requirements = unsafe { &mut *requirements };
    if requirements.ty != xr::StructureType::GRAPHICS_REQUIREMENTS_VULKAN2_KHR {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }
    with_instance(xr_instance.into_raw(), |_instance| {
        requirements.min_api_version_supported = xr::Version::new(1, 0, 0);
        requirements.max_api_version_supported = xr::Version::new(1, 3, 0);
        Ok(())
    })
    .into_xr_result()
}

pub extern "system" fn get_graphics_device2(
    xr_instance: xr::Instance,
    get_info: *const xr::VulkanGraphicsDeviceGetInfoKHR,
    vk_physical_device: *mut VkPhysicalDevice,
) -> xr::Result {
    if get_info.is_null() || vk_physical_device.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }
    let get_info = unsafe { &*get_info };
    if get_info.ty != xr::StructureType::VULKAN_GRAPHICS_DEVICE_GET_INFO_KHR {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }
    if get_info.system_id.into_raw() != HMD_SYSTEM_ID {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let entry = match unsafe { ash::Entry::load() } {
        Ok(e) => e,
        Err(e) => {
            log::error!("failed to load Vulkan loader: {e}");
            return xr::Result::ERROR_RUNTIME_FAILURE;
        }
    };

    let vk_instance = ash::vk::Instance::from_raw(get_info.vulkan_instance as usize as u64);
    let handle = unsafe {
        let ash_instance = ash::Instance::load(entry.static_fn(), vk_instance);
        let devs = match ash_instance.enumerate_physical_devices() {
            Ok(e) => e,
            Err(e) => {
                log::error!("failed to enumerate Vulkan devices: {e}");
                return xr::Result::ERROR_RUNTIME_FAILURE;
            }
        };

        let mut dev = None;
        'outer: for p in devs.iter() {
            let queue_families = ash_instance.get_physical_device_queue_family_properties(*p);
            for qf in queue_families.iter() {
                if qf.queue_flags.contains(QueueFlags::GRAPHICS) {
                    dev = Some(*p);
                    break 'outer;
                }
            }
        }

        match dev {
            Some(d) => d.as_raw() as usize as VkPhysicalDevice,
            None => return xr::Result::ERROR_RUNTIME_FAILURE,
        }
    };

    with_instance(xr_instance.into_raw(), |_instance| {
        unsafe { *vk_physical_device = handle }
        log::debug!("get_graphics_device2 -> {handle:?}");
        Ok(())
    })
    .into_xr_result()
}

pub extern "system" fn create_vulkan_instance(
    xr_instance: xr::Instance,
    create_info: *const xr::VulkanInstanceCreateInfoKHR,
    vulkan_instance: *mut VkInstance,
    vulkan_result: *mut VkResult,
) -> xr::Result {
    if create_info.is_null() || vulkan_instance.is_null() || vulkan_result.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }
    let create_info = unsafe { &*create_info };
    if create_info.ty != xr::StructureType::VULKAN_INSTANCE_CREATE_INFO_KHR {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let entry = match unsafe { ash::Entry::load() } {
        Ok(e) => e,
        Err(e) => {
            log::error!("failed to load Vulkan loader: {e}");
            return xr::Result::ERROR_RUNTIME_FAILURE;
        }
    };

    let mut vk_instance_raw = ash::vk::Instance::null();
    let vk_result = unsafe {
        (entry.fp_v1_0().create_instance)(
            create_info.vulkan_create_info as *const ash::vk::InstanceCreateInfo,
            create_info.vulkan_allocator as *const ash::vk::AllocationCallbacks,
            &mut vk_instance_raw,
        )
    };

    CREATED_VK_INSTANCE.store(vk_instance_raw.as_raw(), atomic::Ordering::SeqCst);

    with_instance(xr_instance.into_raw(), |_instance| {
        unsafe {
            *vulkan_instance = vk_instance_raw.as_raw() as usize as VkInstance;
            *vulkan_result = vk_result.as_raw();
        }
        log::debug!("create_vulkan_instance -> {vk_instance_raw:?}, result={vk_result:?}");
        Ok(())
    })
    .into_xr_result()
}

pub extern "system" fn create_vulkan_device(
    xr_instance: xr::Instance,
    create_info: *const xr::VulkanDeviceCreateInfoKHR,
    vulkan_device: *mut VkDevice,
    vulkan_result: *mut VkResult,
) -> xr::Result {
    if create_info.is_null() || vulkan_device.is_null() || vulkan_result.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }
    let create_info = unsafe { &*create_info };
    if create_info.ty != xr::StructureType::VULKAN_DEVICE_CREATE_INFO_KHR {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let entry = match unsafe { ash::Entry::load() } {
        Ok(e) => e,
        Err(e) => {
            log::error!("failed to load Vulkan loader: {e}");
            return xr::Result::ERROR_RUNTIME_FAILURE;
        }
    };

    let cached = CREATED_VK_INSTANCE.load(atomic::Ordering::SeqCst);
    if cached == 0 {
        log::error!("create_vulkan_device called before create_vulkan_instance");
        return xr::Result::ERROR_RUNTIME_FAILURE;
    }

    let vk_instance = ash::vk::Instance::from_raw(cached);
    let ash_instance = unsafe { ash::Instance::load(entry.static_fn(), vk_instance) };
    let physical_device =
        ash::vk::PhysicalDevice::from_raw(create_info.vulkan_physical_device as usize as u64);

    let mut vk_device_raw = ash::vk::Device::null();
    let vk_result = unsafe {
        (ash_instance.fp_v1_0().create_device)(
            physical_device,
            create_info.vulkan_create_info as *const ash::vk::DeviceCreateInfo,
            create_info.vulkan_allocator as *const ash::vk::AllocationCallbacks,
            &mut vk_device_raw,
        )
    };

    with_instance(xr_instance.into_raw(), |_instance| {
        unsafe {
            *vulkan_device = vk_device_raw.as_raw() as usize as VkDevice;
            *vulkan_result = vk_result.as_raw();
        }
        log::debug!("create_vulkan_device -> {vk_device_raw:?}, result={vk_result:?}");
        Ok(())
    })
    .into_xr_result()
}
