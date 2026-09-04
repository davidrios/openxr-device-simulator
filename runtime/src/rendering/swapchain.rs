use std::{
    cell::UnsafeCell,
    collections::{HashMap, VecDeque},
    sync::{Arc, LazyLock, Mutex, atomic},
};

use ash::vk::Handle;

use crate::{
    prelude::*,
    session::{GraphicsBinding, SimulatedSession, with_session},
    utils::with_obj_instance,
};

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

    with_session(xr_session.into_raw(), |_session| {
        if capacity_in == 0 {
            *count_out = SUPPORTED_SWAPCHAIN_FORMATS.len() as u32;
            return Ok(());
        }

        if *count_out != SUPPORTED_SWAPCHAIN_FORMATS.len() as u32 {
            return Err(xr::Result::ERROR_SIZE_INSUFFICIENT.into());
        }

        if formats.is_null() {
            return Err(xr::Result::ERROR_VALIDATION_FAILURE.into());
        }

        unsafe {
            for (i, format) in SUPPORTED_SWAPCHAIN_FORMATS.iter().enumerate() {
                *formats.add(i) = *format;
            }
        }

        Ok(())
    })
    .into_xr_result()
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

    with_session(xr_session.into_raw(), |session| {
        let mut swapchain_instances = INSTANCES.lock().unwrap();
        let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        swapchain_instances.insert(
            next_id,
            UnsafeCell::new(SimulatedSwapchain::new(session, next_id, create_info)?),
        );

        log::debug!("created: {:?}", unsafe {
            &*swapchain_instances[&next_id].get()
        });

        *xr_swapchain = xr::Swapchain::from_raw(next_id);

        session.add_swapchain(next_id)
    })
    .into_xr_result()
}

pub extern "system" fn enumerate_images(
    xr_swapchain: xr::Swapchain,
    capacity_in: u32,
    count_out: *mut u32,
    images: *mut xr::SwapchainImageBaseHeader,
) -> xr::Result {
    let count_out = unsafe { &mut *count_out };

    with_swapchain(xr_swapchain.into_raw(), |swapchain| {
        if capacity_in == 0 {
            *count_out = swapchain.images.len() as u32;
            return Ok(());
        }

        if *count_out != swapchain.images.len() as u32 {
            return Err(xr::Result::ERROR_SIZE_INSUFFICIENT.into());
        }

        if images.is_null()
            || !matches!(
                unsafe { *images }.ty,
                xr::StructureType::SWAPCHAIN_IMAGE_VULKAN_KHR
            )
        {
            return Err(xr::Result::ERROR_VALIDATION_FAILURE.into());
        }

        log::debug!("enumerate images");

        for i in 0..swapchain.images.len() {
            let item = unsafe { &mut *(images as *mut xr::SwapchainImageVulkanKHR).add(i) };
            item.image = swapchain.images[i].image.as_raw();
            log::debug!("{item:?}");
        }

        Ok(())
    })
    .into_xr_result()
}

#[allow(unreachable_code)]
pub extern "system" fn acquire_image(
    xr_swapchain: xr::Swapchain,
    info: *const xr::SwapchainImageAcquireInfo,
    index: *mut u32,
) -> xr::Result {
    let (info, index) = unsafe {
        (
            if info.is_null() { None } else { Some(&*info) },
            &mut *index,
        )
    };

    with_swapchain(xr_swapchain.into_raw(), |swapchain| {
        *index = swapchain.acquire_image(info)? as u32;
        Ok(())
    })
    .into_xr_result()
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

    with_swapchain(xr_swapchain.into_raw(), |swapchain| {
        swapchain.wait_image(info)
    })
    .into_xr_result()
}

#[allow(unreachable_code)]
pub extern "system" fn release_image(
    xr_swapchain: xr::Swapchain,
    info: *const xr::SwapchainImageReleaseInfo,
) -> xr::Result {
    let info = unsafe { if info.is_null() { None } else { Some(&*info) } };

    with_swapchain(xr_swapchain.into_raw(), |swapchain| {
        swapchain.release_image(info)
    })
    .into_xr_result()
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
    available_images: VecDeque<usize>,
    acquired_images: VecDeque<usize>,
    waited_images: VecDeque<usize>,
    released_images: VecDeque<usize>,
    to_read_images: VecDeque<usize>,
    // Vulkan handles kept here so Drop and readback don't need to re-lock the session
    device: Arc<ash::Device>,
    queue: ash::vk::Queue,
    queue_family_index: u32,
    phys_mem_props: ash::vk::PhysicalDeviceMemoryProperties,
}

impl std::fmt::Debug for SimulatedSwapchain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimulatedSwapchain")
            .field("session_id", &self.session_id)
            .field("id", &self.id)
            .field("format", &self.format)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("images", &self.images)
            .field("available_images", &self.available_images)
            .field("acquired_images", &self.acquired_images)
            .field("waited_images", &self.waited_images)
            .field("released_images", &self.released_images)
            .field("to_read_images", &self.to_read_images)
            .finish()
    }
}

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

        let gb = &session.graphics_binding;
        let device = gb.device.clone();
        let queue = unsafe { gb.device.get_device_queue(gb.queue_family_index, gb.queue_index) };
        let queue_family_index = gb.queue_family_index;
        let phys_mem_props = unsafe {
            gb.instance.get_physical_device_memory_properties(gb.physical_device)
        };

        let num_images = if create_info
            .create_flags
            .contains(xr::SwapchainCreateFlags::STATIC_IMAGE)
        {
            1
        } else {
            3
        };

        let mut available_images = VecDeque::with_capacity(num_images);
        let mut images = Vec::with_capacity(num_images);
        for i in 0..num_images {
            images.push(OffscreenImage::new(gb, create_info)?);
            available_images.push_back(i);
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
            available_images,
            acquired_images: VecDeque::with_capacity(num_images),
            waited_images: VecDeque::with_capacity(num_images),
            released_images: VecDeque::with_capacity(num_images),
            to_read_images: VecDeque::with_capacity(num_images),
            device,
            queue,
            queue_family_index,
            phys_mem_props,
        })
    }

    pub fn acquire_image(&mut self, info: Option<&xr::SwapchainImageAcquireInfo>) -> Result<usize> {
        let Some(index) = self.available_images.pop_front() else {
            return Err(xr::Result::ERROR_CALL_ORDER_INVALID.into());
        };

        self.acquired_images.push_back(index);
        log::debug!("[{}] acquire_image ({info:?}) -> {index}", self.id);

        Ok(index)
    }

    pub fn wait_image(&mut self, info: &xr::SwapchainImageWaitInfo) -> Result<xr::Result> {
        log::debug!("[{}] wait_image ({info:?})", self.id);
        if self.acquired_images.front().is_none() {
            return Err(xr::Result::ERROR_CALL_ORDER_INVALID.into());
        };

        let index = self.acquired_images.pop_front().unwrap();
        self.waited_images.push_back(index);

        log::debug!("[{}] wait_image -> {index}", self.id);
        Ok(xr::Result::SUCCESS)
    }

    pub fn release_image(
        &mut self,
        info: Option<&xr::SwapchainImageReleaseInfo>,
    ) -> Result<xr::Result> {
        let Some(index) = self.waited_images.pop_front() else {
            return Err(xr::Result::ERROR_CALL_ORDER_INVALID.into());
        };

        self.released_images.push_back(index);

        log::debug!("[{}] release_image ({info:?}) -> {index}", self.id);
        Ok(xr::Result::SUCCESS)
    }

    pub fn free_image(&mut self) -> Result<()> {
        let Some(index) = self.released_images.pop_front() else {
            return Err(xr::Result::ERROR_CALL_ORDER_INVALID.into());
        };

        self.to_read_images.push_back(index);

        Ok(())
    }

    pub fn dump_frame(&mut self, frame_number: u64) {
        let Some(index) = self.to_read_images.pop_front() else {
            return;
        };

        let offscreen = &self.images[index];
        if let Some(pixels) = offscreen.read_pixels(
            &self.device,
            &self.phys_mem_props,
            self.queue,
            self.queue_family_index,
        ) {
            let bytes_per_pixel = pixels.len() / (offscreen.array_layers as usize * self.width as usize * self.height as usize);
            let layer_size = self.width as usize * self.height as usize * bytes_per_pixel;

            // Array-layer swapchains (e.g. multiview stereo rendering) pack every
            // eye into one image, one layer each — encode and send them separately.
            for (layer, layer_pixels) in pixels.chunks_exact(layer_size).enumerate() {
                // JPEG doesn't support alpha — strip it for RGBA formats
                let (rgb, color_type) = match self.format {
                    ash::vk::Format::R8G8B8_UNORM
                    | ash::vk::Format::R8G8B8_SNORM
                    | ash::vk::Format::R8G8B8_UINT
                    | ash::vk::Format::R8G8B8_SINT
                    | ash::vk::Format::R8G8B8_SRGB => {
                        (layer_pixels.to_vec(), ::image::ExtendedColorType::Rgb8)
                    }
                    _ => {
                        let rgb: Vec<u8> = layer_pixels
                            .chunks_exact(4)
                            .flat_map(|p| [p[0], p[1], p[2]])
                            .collect();
                        (rgb, ::image::ExtendedColorType::Rgb8)
                    }
                };

                let mut jpeg_bytes = Vec::new();
                let mut enc = ::image::codecs::jpeg::JpegEncoder::new_with_quality(
                    std::io::Cursor::new(&mut jpeg_bytes),
                    80,
                );
                match enc.encode(&rgb, self.width, self.height, color_type) {
                    Ok(()) => {
                        use base64::Engine as _;
                        let jpeg_b64 =
                            base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes);
                        crate::server::send_frame(frame_number, self.id, layer as u32, jpeg_b64);
                        log::debug!(
                            "sent frame {frame_number} sc{} layer{layer} ({} JPEG bytes)",
                            self.id,
                            jpeg_bytes.len()
                        );
                    }
                    Err(e) => log::error!("JPEG encode failed: {e}"),
                }
            }
        }

        self.available_images.push_back(index);
    }
}

impl Drop for SimulatedSwapchain {
    fn drop(&mut self) {
        for image in self.images.iter() {
            image.cleanup(&self.device);
        }
    }
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
    pub(crate) array_layers: u32,
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

    /// Copy the GPU image to a host-visible staging buffer and return the raw pixel bytes.
    pub fn read_pixels(
        &self,
        device: &ash::Device,
        phys_mem_props: &ash::vk::PhysicalDeviceMemoryProperties,
        queue: ash::vk::Queue,
        queue_family_index: u32,
    ) -> Option<Vec<u8>> {
        let bytes_per_pixel: u64 = match self.format {
            ash::vk::Format::R8G8B8_UNORM
            | ash::vk::Format::R8G8B8_SNORM
            | ash::vk::Format::R8G8B8_UINT
            | ash::vk::Format::R8G8B8_SINT
            | ash::vk::Format::R8G8B8_SRGB => 3,
            ash::vk::Format::R8G8B8A8_UNORM
            | ash::vk::Format::R8G8B8A8_SNORM
            | ash::vk::Format::R8G8B8A8_UINT
            | ash::vk::Format::R8G8B8A8_SINT
            | ash::vk::Format::R8G8B8A8_SRGB => 4,
            _ => {
                log::error!("read_pixels: unsupported format {:?}", self.format);
                return None;
            }
        };
        let buffer_size =
            self.width as u64 * self.height as u64 * bytes_per_pixel * self.array_layers as u64;

        unsafe {
            // --- staging buffer ---
            let buf_info = ash::vk::BufferCreateInfo {
                size: buffer_size,
                usage: ash::vk::BufferUsageFlags::TRANSFER_DST,
                sharing_mode: ash::vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            let staging_buf = device.create_buffer(&buf_info, None).ok()?;
            let mem_req = device.get_buffer_memory_requirements(staging_buf);
            let mem_type = find_memory_type_index(
                &mem_req,
                phys_mem_props,
                ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                    | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            let staging_mem = device
                .allocate_memory(
                    &ash::vk::MemoryAllocateInfo {
                        allocation_size: mem_req.size,
                        memory_type_index: mem_type,
                        ..Default::default()
                    },
                    None,
                )
                .ok()?;
            device.bind_buffer_memory(staging_buf, staging_mem, 0).ok()?;

            // --- one-shot command buffer ---
            let cmd_pool = device
                .create_command_pool(
                    &ash::vk::CommandPoolCreateInfo {
                        flags: ash::vk::CommandPoolCreateFlags::TRANSIENT,
                        queue_family_index,
                        ..Default::default()
                    },
                    None,
                )
                .ok()?;
            let cmd = device
                .allocate_command_buffers(&ash::vk::CommandBufferAllocateInfo {
                    command_pool: cmd_pool,
                    level: ash::vk::CommandBufferLevel::PRIMARY,
                    command_buffer_count: 1,
                    ..Default::default()
                })
                .ok()?[0];

            device
                .begin_command_buffer(
                    cmd,
                    &ash::vk::CommandBufferBeginInfo {
                        flags: ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                        ..Default::default()
                    },
                )
                .ok()?;

            let subresource_range = ash::vk::ImageSubresourceRange {
                aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            };

            // COLOR_ATTACHMENT_OPTIMAL -> TRANSFER_SRC_OPTIMAL
            device.cmd_pipeline_barrier(
                cmd,
                ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ash::vk::PipelineStageFlags::TRANSFER,
                ash::vk::DependencyFlags::empty(),
                &[],
                &[],
                &[ash::vk::ImageMemoryBarrier {
                    old_layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    new_layout: ash::vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    src_queue_family_index: ash::vk::QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: ash::vk::QUEUE_FAMILY_IGNORED,
                    image: self.image,
                    subresource_range,
                    src_access_mask: ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    dst_access_mask: ash::vk::AccessFlags::TRANSFER_READ,
                    ..Default::default()
                }],
            );

            device.cmd_copy_image_to_buffer(
                cmd,
                self.image,
                ash::vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                staging_buf,
                &[ash::vk::BufferImageCopy {
                    buffer_offset: 0,
                    buffer_row_length: 0,
                    buffer_image_height: 0,
                    image_subresource: ash::vk::ImageSubresourceLayers {
                        aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    image_offset: ash::vk::Offset3D::default(),
                    image_extent: ash::vk::Extent3D {
                        width: self.width,
                        height: self.height,
                        depth: 1,
                    },
                }],
            );

            // TRANSFER_SRC_OPTIMAL -> COLOR_ATTACHMENT_OPTIMAL (restore for next frame)
            device.cmd_pipeline_barrier(
                cmd,
                ash::vk::PipelineStageFlags::TRANSFER,
                ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ash::vk::DependencyFlags::empty(),
                &[],
                &[],
                &[ash::vk::ImageMemoryBarrier {
                    old_layout: ash::vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    new_layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    src_queue_family_index: ash::vk::QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: ash::vk::QUEUE_FAMILY_IGNORED,
                    image: self.image,
                    subresource_range,
                    src_access_mask: ash::vk::AccessFlags::TRANSFER_READ,
                    dst_access_mask: ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    ..Default::default()
                }],
            );

            device.end_command_buffer(cmd).ok()?;

            device
                .queue_submit(
                    queue,
                    &[ash::vk::SubmitInfo {
                        command_buffer_count: 1,
                        p_command_buffers: &cmd,
                        ..Default::default()
                    }],
                    ash::vk::Fence::null(),
                )
                .ok()?;
            device.queue_wait_idle(queue).ok()?;

            // map and copy
            let ptr = device
                .map_memory(staging_mem, 0, buffer_size, ash::vk::MemoryMapFlags::empty())
                .ok()? as *const u8;
            let pixels = std::slice::from_raw_parts(ptr, buffer_size as usize).to_vec();
            device.unmap_memory(staging_mem);

            // cleanup
            device.destroy_command_pool(cmd_pool, None);
            device.free_memory(staging_mem, None);
            device.destroy_buffer(staging_buf, None);

            Some(pixels)
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

        let (color_image, color_image_memory, color_image_view, color_image_array_layers) = {
            // Always include TRANSFER_SRC so we can read back rendered frames
            let mut usage = ash::vk::ImageUsageFlags::TRANSFER_SRC;
            for (from, to) in USAGE_FLAGS_MAP {
                if create_info
                    .usage_flags
                    .contains(xr::SwapchainUsageFlags::from_raw(from.into_raw()))
                {
                    usage |= ash::vk::ImageUsageFlags::from_raw(*to);
                }
            }

            let array_layers = create_info.array_size.max(1);

            let image_create_info = ash::vk::ImageCreateInfo {
                image_type: ash::vk::ImageType::TYPE_2D,
                format,
                extent: ash::vk::Extent3D {
                    width: create_info.width,
                    height: create_info.height,
                    depth: 1,
                },
                mip_levels: create_info.mip_count,
                array_layers,
                samples: ash::vk::SampleCountFlags::TYPE_1,
                usage,
                ..Default::default()
            };

            let image = unsafe {
                graphics_binding
                    .device
                    .create_image(&image_create_info, None)?
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
                view_type: if array_layers > 1 {
                    ash::vk::ImageViewType::TYPE_2D_ARRAY
                } else {
                    ash::vk::ImageViewType::TYPE_2D
                },
                format,
                subresource_range: ash::vk::ImageSubresourceRange {
                    aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: array_layers,
                },
                ..Default::default()
            };

            let view = unsafe {
                graphics_binding
                    .device
                    .create_image_view(&view_create_info, None)?
            };
            (image, memory, view, array_layers)
        };

        Ok(Self {
            width: create_info.width,
            height: create_info.height,
            format,
            array_layers: color_image_array_layers,
            image: color_image,
            image_memory: color_image_memory,
            image_view: color_image_view,
        })
    }
}

pub struct GpuImageCopy {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ash::vk::Format,
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

type SharedSimulatedSwapchain = UnsafeCell<SimulatedSwapchain>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedSwapchain>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn with_swapchain<T, F>(obj_id: u64, f: F) -> Result<T>
where
    F: FnMut(&mut SimulatedSwapchain) -> Result<T>,
{
    with_obj_instance(&INSTANCES, obj_id, f)
}
