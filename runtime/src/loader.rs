use std::{
    ffi::{CStr, c_char},
    sync::atomic,
};

use openxr_sys as xr;

use crate::{bind_api_fn, input, path, session, spaces, system, view, vulkan};
use crate::{instance, rendering};

static LOGGING_INITED: atomic::AtomicBool = atomic::AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn xrNegotiateLoaderRuntimeInterface(
    loader_info: *const xr::NegotiateLoaderInfo,
    runtime_request: *mut xr::NegotiateRuntimeRequest,
) -> xr::Result {
    if !LOGGING_INITED.fetch_or(true, atomic::Ordering::SeqCst) {
        env_logger::init();
    }

    unsafe { log::debug!("negotiate with loader_info: {:?}", *loader_info) };

    if loader_info.is_null() || runtime_request.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let loader_info = unsafe { &*loader_info };
    let runtime_request = unsafe { &mut *runtime_request };

    if loader_info.min_interface_version > 1 || loader_info.max_interface_version < 1 {
        return xr::Result::ERROR_INITIALIZATION_FAILED;
    }

    runtime_request.runtime_interface_version = 1;
    runtime_request.runtime_api_version = xr::CURRENT_API_VERSION;

    runtime_request.get_instance_proc_addr = Some(xr_get_instance_proc_addr);

    log::debug!("negotiation success");

    xr::Result::SUCCESS
}

extern "system" fn xr_get_instance_proc_addr(
    xr_instance: xr::Instance,
    name: *const c_char,
    function: *mut Option<xr::pfn::VoidFunction>,
) -> xr::Result {
    if name.is_null() || function.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let name_str = unsafe { CStr::from_ptr(name).to_str().unwrap_or("") };

    log::debug!("get_instance_proc_addr({:?}), {name_str}", xr_instance);

    if xr_instance == xr::Instance::NULL {
        unsafe {
            *function = match name_str {
                "xrEnumerateInstanceExtensionProperties" => Some(bind_api_fn!(
                    xr::pfn::EnumerateInstanceExtensionProperties,
                    instance::api::enumerate_extension_properties
                )),
                "xrCreateInstance" => {
                    Some(bind_api_fn!(xr::pfn::CreateInstance, instance::api::create))
                }
                _ => None,
            }
        };
    } else {
        unsafe {
            // Match the function name and return a pointer to our implementation
            *function = match name_str {
                "xrCreateInstance" => Some(bind_api_fn!(
                    xr::pfn::CreateInstance,
                    instance::api::create_from_instance
                )),
                "xrDestroyInstance" => Some(bind_api_fn!(
                    xr::pfn::DestroyInstance,
                    instance::api::destroy
                )),
                "xrGetInstanceProperties" => Some(bind_api_fn!(
                    xr::pfn::GetInstanceProperties,
                    instance::api::get_properties
                )),
                "xrStringToPath" => Some(bind_api_fn!(xr::pfn::StringToPath, path::string_to_path)),

                "xrGetSystem" => Some(bind_api_fn!(xr::pfn::GetSystem, system::get_system)),
                "xrGetSystemProperties" => Some(bind_api_fn!(
                    xr::pfn::GetSystemProperties,
                    system::get_properties
                )),

                "xrEnumerateViewConfigurations" => Some(bind_api_fn!(
                    xr::pfn::EnumerateViewConfigurations,
                    view::enumerate_configurations
                )),
                "xrGetViewConfigurationProperties" => Some(bind_api_fn!(
                    xr::pfn::GetViewConfigurationProperties,
                    view::get_configuration_properties
                )),
                "xrEnumerateViewConfigurationViews" => Some(bind_api_fn!(
                    xr::pfn::EnumerateViewConfigurationViews,
                    view::enumerate_configuration_views
                )),

                "xrGetVulkanGraphicsRequirementsKHR" => Some(bind_api_fn!(
                    xr::pfn::GetVulkanGraphicsRequirementsKHR,
                    vulkan::get_graphics_requirements
                )),
                "xrGetVulkanGraphicsDeviceKHR" => {
                    Some(bind_api_fn!(*const (), vulkan::get_graphics_device as _))
                }
                "xrGetVulkanInstanceExtensionsKHR" => Some(bind_api_fn!(
                    xr::pfn::GetVulkanInstanceExtensionsKHR,
                    vulkan::get_instance_extensions
                )),
                "xrGetVulkanDeviceExtensionsKHR" => Some(bind_api_fn!(
                    xr::pfn::GetVulkanDeviceExtensionsKHR,
                    vulkan::get_device_extensions
                )),

                "xrCreateSession" => Some(bind_api_fn!(xr::pfn::CreateSession, session::create)),
                "xrDestroySession" => Some(bind_api_fn!(xr::pfn::DestroySession, session::destroy)),

                "xrEnumerateReferenceSpaces" => Some(bind_api_fn!(
                    xr::pfn::EnumerateReferenceSpaces,
                    spaces::enumerate_reference_spaces
                )),
                "xrCreateReferenceSpace" => Some(bind_api_fn!(
                    xr::pfn::CreateReferenceSpace,
                    spaces::create_reference_space
                )),
                "xrDestroySpace" => Some(bind_api_fn!(xr::pfn::DestroySpace, spaces::destroy)),

                "xrCreateActionSet" => Some(bind_api_fn!(
                    xr::pfn::CreateActionSet,
                    input::action_set::create
                )),
                "xrDestroyActionSet" => Some(bind_api_fn!(
                    xr::pfn::DestroyActionSet,
                    input::action_set::destroy
                )),

                "xrCreateAction" => {
                    Some(bind_api_fn!(xr::pfn::CreateAction, input::action::create))
                }

                "xrSuggestInteractionProfileBindings" => Some(bind_api_fn!(
                    xr::pfn::SuggestInteractionProfileBindings,
                    input::binding::suggest_interaction_profile
                )),

                "xrEnumerateEnvironmentBlendModes" => Some(bind_api_fn!(
                    xr::pfn::EnumerateEnvironmentBlendModes,
                    rendering::enumerate_blend_modes
                )),
                _ => None,
            }
        };
    }

    if unsafe { (*function).is_none() } {
        log::error!("could not get fn named {}", name_str);
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
    }

    xr::Result::SUCCESS
}
