use std::{
    cell::UnsafeCell,
    collections::HashMap,
    sync::{LazyLock, Mutex, atomic},
};

use ash::vk::Handle;

use crate::{
    error::{Error, Result, to_xr_result},
    session::{GraphicsBinding, SimulatedSession},
    with_session,
};

#[macro_export]
macro_rules! with_swapchain {
    ($xr_obj:expr, |$instance:ident| $expr:expr) => {{
        match $crate::rendering::swapchain::get_simulated_swapchain_cell($xr_obj) {
            Ok(instance_ptr) => {
                let $instance = unsafe { &mut *instance_ptr };
                $expr
            }
            Err(err) => Err(err),
        }
    }};
}

const SUPPORTED_SWAPCHAIN_FORMATS: &[i64] = &[
    ash::vk::Format::R8G8B8_UNORM.as_raw() as i64,
    ash::vk::Format::R8G8B8_SNORM.as_raw() as i64,
    ash::vk::Format::R8G8B8_UINT.as_raw() as i64,
    ash::vk::Format::R8G8B8_SINT.as_raw() as i64,
    ash::vk::Format::R8G8B8_SRGB.as_raw() as i64,
    ash::vk::Format::R8G8B8A8_UNORM.as_raw() as i64,
    ash::vk::Format::R8G8B8A8_SNORM.as_raw() as i64,
    ash::vk::Format::R8G8B8A8_UINT.as_raw() as i64,
    ash::vk::Format::R8G8B8A8_SINT.as_raw() as i64,
    ash::vk::Format::R8G8B8A8_SRGB.as_raw() as i64,
];

pub extern "system" fn enumerate_formats(
    xr_session: xr::Session,
    capacity_in: u32,
    count_out: *mut u32,
    formats: *mut i64,
) -> xr::Result {
    let count_out = unsafe { &mut *count_out };

    log::debug!("enumerate formats: {:?}", capacity_in);

    to_xr_result(with_session!(xr_session, |_session| {
        if capacity_in == 0 {
            *count_out = SUPPORTED_SWAPCHAIN_FORMATS.len() as u32;
            return xr::Result::SUCCESS;
        }

        if *count_out != SUPPORTED_SWAPCHAIN_FORMATS.len() as u32 {
            return xr::Result::ERROR_SIZE_INSUFFICIENT;
        }

        if formats.is_null() {
            return xr::Result::ERROR_VALIDATION_FAILURE;
        }

        unsafe {
            for (i, format) in SUPPORTED_SWAPCHAIN_FORMATS.iter().enumerate() {
                *formats.add(i) = *format;
            }
        }

        Ok(())
    }))
}

pub extern "system" fn create(
    xr_session: xr::Session,
    create_info: *const xr::SwapchainCreateInfo,
    xr_swapchain: *mut xr::Swapchain,
) -> xr::Result {
    if create_info.is_null() || xr_swapchain.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (create_info, xr_swapchain) = unsafe { (&*create_info, &mut *xr_swapchain) };

    if create_info.ty != xr::StructureType::SWAPCHAIN_CREATE_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    to_xr_result(with_session!(xr_session, |session| {
        let mut swapchain_instances = INSTANCES.lock().unwrap();
        let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        swapchain_instances.insert(
            next_id,
            UnsafeCell::new(
                match SimulatedSwapchain::new(session, next_id, create_info) {
                    Ok(set) => set,
                    Err(err) => return err.into(),
                },
            ),
        );

        log::debug!("create swapchain: {:?}", unsafe {
            &*swapchain_instances[&next_id].get()
        });

        *xr_swapchain = xr::Swapchain::from_raw(next_id);

        session.add_swapchain(next_id)
    }))
}

pub extern "system" fn enumerate_images(
    xr_swapchain: xr::Swapchain,
    capacity_in: u32,
    count_out: *mut u32,
    images: *mut xr::SwapchainImageBaseHeader,
) -> xr::Result {
    let count_out = unsafe { &mut *count_out };

    to_xr_result(with_swapchain!(xr_swapchain, |swapchain| {
        if capacity_in == 0 {
            *count_out = swapchain.images.len() as u32;
            return xr::Result::SUCCESS;
        }

        if *count_out != swapchain.images.len() as u32 {
            return xr::Result::ERROR_SIZE_INSUFFICIENT;
        }

        if images.is_null()
            || !matches!(
                unsafe { *images }.ty,
                xr::StructureType::SWAPCHAIN_IMAGE_VULKAN_KHR
            )
        {
            return xr::Result::ERROR_VALIDATION_FAILURE;
        }

        log::debug!("enumerate images");

        #[allow(clippy::needless_range_loop)]
        for i in 0..swapchain.images.len() {
            let item = unsafe { &mut *(images as *mut xr::SwapchainImageVulkanKHR).add(i) };
            item.image = swapchain.images[i].image.as_raw();
            log::debug!("{item:?}");
        }

        Ok(())
    }))
}

#[allow(unreachable_code)]
pub extern "system" fn acquire_image(
    xr_swapchain: xr::Swapchain,
    info: *const xr::SwapchainImageAcquireInfo,
    index: *mut u32,
) -> xr::Result {
    let (info, _index) = unsafe { (&*info, &mut *index) };
    to_xr_result(with_swapchain!(xr_swapchain, |_swapchain| {
        log::debug!("[{xr_swapchain:?}] acquire_image {info:?}");
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
        Ok(())
    }))
}

#[allow(unreachable_code)]
pub extern "system" fn wait_image(
    xr_swapchain: xr::Swapchain,
    info: *const xr::SwapchainImageWaitInfo,
) -> xr::Result {
    if info.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let info = unsafe { &*info };
    to_xr_result(with_swapchain!(xr_swapchain, |_swapchain| {
        log::debug!("[{xr_swapchain:?}] wait_image {info:?}");
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
        Ok(())
    }))
}

#[allow(unreachable_code)]
pub extern "system" fn release_image(
    xr_swapchain: xr::Swapchain,
    info: *const xr::SwapchainImageReleaseInfo,
) -> xr::Result {
    if info.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let info = unsafe { &*info };
    to_xr_result(with_swapchain!(xr_swapchain, |_swapchain| {
        log::debug!("[{xr_swapchain:?}] release_image {info:?}");
        return xr::Result::ERROR_FUNCTION_UNSUPPORTED;
        Ok(())
    }))
}

pub extern "system" fn destroy(xr_obj: xr::Swapchain) -> xr::Result {
    if xr_obj == xr::Swapchain::NULL {
        return xr::Result::ERROR_HANDLE_INVALID;
    }

    let instance_id = xr_obj.into_raw();

    if INSTANCES
        .lock()
        .expect("couldn't acquire instances")
        .remove(&instance_id)
        .is_some()
    {
        log::debug!("destroyed {instance_id}");
    } else {
        log::debug!("instance {instance_id} not found");
    }

    xr::Result::SUCCESS
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SimulatedSwapchain {
    session_id: u64,
    id: u64,
    create_flags: xr::SwapchainCreateFlags,
    usage_flags: xr::SwapchainUsageFlags,
    format: ash::vk::Format,
    sample_count: u32,
    width: u32,
    height: u32,
    face_count: u32,
    array_size: u32,
    mip_count: u32,
    images: Vec<OffscreenImage>,
    current_image: u32,
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

impl SimulatedSwapchain {
    pub fn new(
        session: &SimulatedSession,
        id: u64,
        create_info: &xr::SwapchainCreateInfo,
    ) -> Result<Self> {
        if !SUPPORTED_SWAPCHAIN_FORMATS.contains(&create_info.format) {
            return Err(xr::Result::ERROR_SWAPCHAIN_FORMAT_UNSUPPORTED.into());
        }

        let format = ash::vk::Format::from_raw(create_info.format as i32);

        let num_images = if create_info
            .create_flags
            .contains(xr::SwapchainCreateFlags::STATIC_IMAGE)
        {
            1
        } else {
            3
        };

        let mut images = Vec::with_capacity(num_images);
        for _ in 0..num_images {
            images.push(OffscreenImage::new(&session.graphics_binding, create_info)?);
        }

        Ok(Self {
            session_id: session.id,
            id,
            create_flags: create_info.create_flags,
            usage_flags: create_info.usage_flags,
            format,
            sample_count: create_info.sample_count,
            width: create_info.width,
            height: create_info.height,
            face_count: create_info.face_count,
            array_size: create_info.array_size,
            mip_count: create_info.mip_count,
            images,
            current_image: 0,
        })
    }
}

impl Drop for SimulatedSwapchain {
    fn drop(&mut self) {
        let _res: Result<()> = with_session!(xr::Session::from_raw(self.session_id), |session| {
            for image in self.images.iter() {
                image.cleanup(session.graphics_binding.device.as_ref());
            }
            Ok(())
        });
    }
}

type SharedSimulatedSwapchain = UnsafeCell<SimulatedSwapchain>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedSwapchain>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[inline]
pub fn get_simulated_swapchain_cell(instance: xr::Swapchain) -> Result<*mut SimulatedSwapchain> {
    Ok(INSTANCES
        .lock()?
        .get(&instance.into_raw())
        .ok_or_else(|| Error::ExpectedSome("swapchain does not exist".into()))?
        .get())
}

pub fn find_memory_type_index(
    memory_req: &ash::vk::MemoryRequirements,
    memory_properties: &ash::vk::PhysicalDeviceMemoryProperties,
    flags: ash::vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_properties.memory_types[..memory_properties.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags.contains(flags)
        })
        .map(|(index, _memory_type)| index as u32)
}

const USAGE_FLAGS_MAP: &[(xr::SwapchainUsageFlags, u32)] = &[
    (
        xr::SwapchainUsageFlags::COLOR_ATTACHMENT,
        ash::vk::ImageUsageFlags::COLOR_ATTACHMENT.as_raw(),
    ),
    (
        xr::SwapchainUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        ash::vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT.as_raw(),
    ),
    (
        xr::SwapchainUsageFlags::UNORDERED_ACCESS,
        ash::vk::ImageUsageFlags::STORAGE.as_raw(),
    ),
    (
        xr::SwapchainUsageFlags::TRANSFER_SRC,
        ash::vk::ImageUsageFlags::TRANSFER_SRC.as_raw(),
    ),
    (
        xr::SwapchainUsageFlags::TRANSFER_DST,
        ash::vk::ImageUsageFlags::TRANSFER_DST.as_raw(),
    ),
    (
        xr::SwapchainUsageFlags::SAMPLED,
        ash::vk::ImageUsageFlags::SAMPLED.as_raw(),
    ),
    (
        xr::SwapchainUsageFlags::INPUT_ATTACHMENT,
        ash::vk::ImageUsageFlags::INPUT_ATTACHMENT.as_raw(),
    ),
];

#[allow(dead_code)]
#[derive(Debug)]
pub struct OffscreenImage {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: ash::vk::Format,
    pub(crate) image: ash::vk::Image,
    pub(crate) image_memory: ash::vk::DeviceMemory,
    pub(crate) image_view: ash::vk::ImageView,
}

impl OffscreenImage {
    pub fn cleanup(&self, device: &ash::Device) {
        unsafe {
            device.destroy_image_view(self.image_view, None);
            device.free_memory(self.image_memory, None);
            device.destroy_image(self.image, None);
        }
    }

    pub fn new(
        graphics_binding: &GraphicsBinding,
        create_info: &xr::SwapchainCreateInfo,
    ) -> Result<Self> {
        let format = ash::vk::Format::from_raw(create_info.format as i32);

        let physical_device_memory_properties = unsafe {
            graphics_binding
                .instance
                .get_physical_device_memory_properties(graphics_binding.physical_device)
        };

        let (color_image, color_image_memory, color_image_view) = {
            let mut usage = ash::vk::ImageUsageFlags::from_raw(0);
            for (from, to) in USAGE_FLAGS_MAP {
                if create_info
                    .usage_flags
                    .contains(xr::SwapchainUsageFlags::from_raw(from.into_raw()))
                {
                    usage |= ash::vk::ImageUsageFlags::from_raw(*to);
                }
            }

            let image_create_info = ash::vk::ImageCreateInfo {
                image_type: ash::vk::ImageType::TYPE_2D,
                format,
                extent: ash::vk::Extent3D {
                    width: create_info.width,
                    height: create_info.height,
                    depth: 1,
                },
                mip_levels: create_info.mip_count,
                array_layers: 1,
                samples: ash::vk::SampleCountFlags::TYPE_1,
                usage,
                ..Default::default()
            };

            let image = match unsafe {
                graphics_binding
                    .device
                    .create_image(&image_create_info, None)
            } {
                Ok(image) => image,
                Err(err) => return Err(err.into()),
            };

            let mem_req = unsafe { graphics_binding.device.get_image_memory_requirements(image) };
            let mem_type_index = find_memory_type_index(
                &mem_req,
                &physical_device_memory_properties,
                ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .expect("Failed to find suitable memory type for color image.");

            let alloc_info = ash::vk::MemoryAllocateInfo {
                allocation_size: mem_req.size,
                memory_type_index: mem_type_index,
                ..Default::default()
            };

            let memory = unsafe { graphics_binding.device.allocate_memory(&alloc_info, None)? };
            unsafe {
                graphics_binding
                    .device
                    .bind_image_memory(image, memory, 0)?
            };

            let view_create_info = ash::vk::ImageViewCreateInfo {
                image,
                view_type: ash::vk::ImageViewType::TYPE_2D,
                format,
                subresource_range: ash::vk::ImageSubresourceRange {
                    aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };

            let view = unsafe {
                graphics_binding
                    .device
                    .create_image_view(&view_create_info, None)?
            };
            (image, memory, view)
        };

        Ok(Self {
            width: create_info.width,
            height: create_info.height,
            format,
            image: color_image,
            image_memory: color_image_memory,
            image_view: color_image_view,
        })
    }
}
