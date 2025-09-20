use std::{
    ffi::{CStr, c_char},
    sync::{LazyLock, atomic},
    time::{Duration, Instant},
};

use crate::{
    bind_api_fn, event, haptics, input, instance, path, rendering, session, spaces, system, view,
    vulkan,
};

static LOGGING_INITED: atomic::AtomicBool = atomic::AtomicBool::new(false);
pub static START_TIME: LazyLock<Instant> =
    LazyLock::new(|| Instant::now() - Duration::from_secs(60 * 60 * 24));

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
                "xrResultToString" => Some(bind_api_fn!(
                    xr::pfn::ResultToString,
                    instance::api::result_to_string
                )),
                "xrStructureTypeToString" => Some(bind_api_fn!(
                    xr::pfn::StructureTypeToString,
                    instance::api::structure_type_to_string
                )),
                "xrStringToPath" => Some(bind_api_fn!(xr::pfn::StringToPath, path::string_to_path)),
                "xrPathToString" => Some(bind_api_fn!(xr::pfn::PathToString, path::path_to_string)),

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
                "xrAttachSessionActionSets" => Some(bind_api_fn!(
                    xr::pfn::AttachSessionActionSets,
                    session::attach_action_sets
                )),
                "xrBeginSession" => Some(bind_api_fn!(xr::pfn::BeginSession, session::begin)),
                "xrRequestExitSession" => Some(bind_api_fn!(
                    xr::pfn::RequestExitSession,
                    session::request_exit
                )),
                "xrEndSession" => Some(bind_api_fn!(xr::pfn::EndSession, session::end)),
                "xrDestroySession" => Some(bind_api_fn!(xr::pfn::DestroySession, session::destroy)),

                "xrEnumerateReferenceSpaces" => Some(bind_api_fn!(
                    xr::pfn::EnumerateReferenceSpaces,
                    spaces::reference::enumerate
                )),
                "xrCreateReferenceSpace" => Some(bind_api_fn!(
                    xr::pfn::CreateReferenceSpace,
                    spaces::reference::create
                )),
                "xrGetReferenceSpaceBoundsRect" => Some(bind_api_fn!(
                    xr::pfn::GetReferenceSpaceBoundsRect,
                    spaces::reference::get_bounds_rect
                )),
                "xrCreateActionSpace" => Some(bind_api_fn!(
                    xr::pfn::CreateActionSpace,
                    spaces::action::create
                )),
                "xrLocateSpace" => Some(bind_api_fn!(xr::pfn::LocateSpace, spaces::locate)),
                "xrDestroySpace" => Some(bind_api_fn!(xr::pfn::DestroySpace, spaces::destroy)),
                "xrLocateSpaces" => {
                    Some(bind_api_fn!(xr::pfn::LocateSpaces, spaces::locate_spaces))
                }

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
                "xrDestroyAction" => {
                    Some(bind_api_fn!(xr::pfn::DestroyAction, input::action::destroy))
                }
                "xrEnumerateBoundSourcesForAction" => Some(bind_api_fn!(
                    xr::pfn::EnumerateBoundSourcesForAction,
                    input::action::enumerate_bound_sources
                )),
                "xrGetInputSourceLocalizedName" => Some(bind_api_fn!(
                    xr::pfn::GetInputSourceLocalizedName,
                    input::action::get_input_source_localized_name
                )),
                "xrGetActionStateBoolean" => Some(bind_api_fn!(
                    xr::pfn::GetActionStateBoolean,
                    input::action_state::get_boolean
                )),
                "xrGetActionStateFloat" => Some(bind_api_fn!(
                    xr::pfn::GetActionStateFloat,
                    input::action_state::get_float
                )),
                "xrGetActionStateVector2f" => Some(bind_api_fn!(
                    xr::pfn::GetActionStateVector2f,
                    input::action_state::get_vector2f
                )),
                "xrGetActionStatePose" => Some(bind_api_fn!(
                    xr::pfn::GetActionStatePose,
                    input::action_state::get_pose
                )),
                "xrSyncActions" => Some(bind_api_fn!(
                    xr::pfn::SyncActions,
                    input::action_state::sync_actions
                )),

                "xrSuggestInteractionProfileBindings" => Some(bind_api_fn!(
                    xr::pfn::SuggestInteractionProfileBindings,
                    input::interaction_profile::suggest
                )),
                "xrGetCurrentInteractionProfile" => Some(bind_api_fn!(
                    xr::pfn::GetCurrentInteractionProfile,
                    input::interaction_profile::get_current
                )),

                "xrWaitFrame" => Some(bind_api_fn!(xr::pfn::WaitFrame, rendering::frame::wait)),
                "xrBeginFrame" => Some(bind_api_fn!(xr::pfn::BeginFrame, rendering::frame::begin)),
                "xrEndFrame" => Some(bind_api_fn!(xr::pfn::EndFrame, rendering::frame::end)),

                "xrEnumerateEnvironmentBlendModes" => Some(bind_api_fn!(
                    xr::pfn::EnumerateEnvironmentBlendModes,
                    rendering::enumerate_blend_modes
                )),

                "xrEnumerateSwapchainFormats" => Some(bind_api_fn!(
                    xr::pfn::EnumerateSwapchainFormats,
                    rendering::swapchain::enumerate_formats
                )),
                "xrCreateSwapchain" => Some(bind_api_fn!(
                    xr::pfn::CreateSwapchain,
                    rendering::swapchain::create
                )),
                "xrDestroySwapchain" => Some(bind_api_fn!(
                    xr::pfn::DestroySwapchain,
                    rendering::swapchain::destroy
                )),
                "xrEnumerateSwapchainImages" => Some(bind_api_fn!(
                    xr::pfn::EnumerateSwapchainImages,
                    rendering::swapchain::enumerate_images
                )),
                "xrAcquireSwapchainImage" => Some(bind_api_fn!(
                    xr::pfn::AcquireSwapchainImage,
                    rendering::swapchain::acquire_image
                )),
                "xrWaitSwapchainImage" => Some(bind_api_fn!(
                    xr::pfn::WaitSwapchainImage,
                    rendering::swapchain::wait_image
                )),
                "xrReleaseSwapchainImage" => Some(bind_api_fn!(
                    xr::pfn::ReleaseSwapchainImage,
                    rendering::swapchain::release_image
                )),

                "xrLocateViews" => Some(bind_api_fn!(
                    xr::pfn::LocateViews,
                    rendering::view::locate_views
                )),

                "xrPollEvent" => Some(bind_api_fn!(xr::pfn::PollEvent, event::poll)),

                "xrApplyHapticFeedback" => Some(bind_api_fn!(
                    xr::pfn::ApplyHapticFeedback,
                    haptics::apply_feedback
                )),
                "xrStopHapticFeedback" => Some(bind_api_fn!(
                    xr::pfn::StopHapticFeedback,
                    haptics::stop_feedback
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
