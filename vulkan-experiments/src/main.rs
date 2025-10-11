use std::sync::Arc;

mod vk {
    pub use vulkano::{
        Validated, VulkanError, VulkanLibrary,
        buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
        command_buffer::{
            AutoCommandBufferBuilder, CommandBufferExecFuture, CommandBufferUsage,
            PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents,
            allocator::StandardCommandBufferAllocator,
        },
        device::{
            Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
            physical::{PhysicalDevice, PhysicalDeviceType},
        },
        image::{Image, ImageUsage, view::ImageView},
        instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
        memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
        pipeline::{
            GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
            graphics::{
                GraphicsPipelineCreateInfo,
                color_blend::{ColorBlendAttachmentState, ColorBlendState},
                input_assembly::InputAssemblyState,
                multisample::MultisampleState,
                rasterization::RasterizationState,
                vertex_input::Vertex,
                viewport::{Viewport, ViewportState},
            },
            layout::PipelineDescriptorSetLayoutCreateInfo,
        },
        render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
        shader::ShaderModule,
        swapchain::{
            PresentFuture, PresentMode, Surface, Swapchain, SwapchainAcquireFuture,
            SwapchainCreateInfo, SwapchainPresentInfo, acquire_next_image,
        },
        sync::{
            self, GpuFuture,
            future::{FenceSignalFuture, JoinFuture},
        },
    };
}
use vulkano::{
    pipeline::graphics::vertex_input::{Vertex, VertexDefinition},
    sync::GpuFuture,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

#[derive(vk::BufferContents, vk::Vertex)]
#[repr(C)]
struct MyVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 460

            layout(location = 0) in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 460

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        ",
    }
}

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_LUNARG_standard_validation"];

#[cfg(debug_assertions)]
const ENABLE_VALIDATION_LAYERS: bool = true;
#[cfg(not(debug_assertions))]
const ENABLE_VALIDATION_LAYERS: bool = false;

struct SwapchainWithImages {
    swapchain: Arc<vk::Swapchain>,
    images: Vec<Arc<vk::Image>>,
}

struct VulkanWindowRenderer {
    swapchain: SwapchainWithImages,
    framebuffers: Vec<Arc<vk::Framebuffer>>,
    render_pass: Arc<vk::RenderPass>,
    pipeline: Arc<vk::GraphicsPipeline>,
    command_buffers: Vec<Arc<vk::PrimaryAutoCommandBuffer>>,
}

impl VulkanWindowRenderer {
    fn new(vw: &VulkanWindow, dimensions: PhysicalSize<u32>) -> Self {
        let caps = vw
            .physical_device
            .surface_capabilities(&vw.surface, Default::default())
            .expect("failed to get surface capabilities");

        let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
        let image_format = vw
            .physical_device
            .surface_formats(&vw.surface, Default::default())
            .unwrap()[0]
            .0;

        let create_info = vk::SwapchainCreateInfo {
            min_image_count: caps.min_image_count + 1,
            image_format,
            image_extent: dimensions.into(),
            image_usage: vk::ImageUsage::COLOR_ATTACHMENT,
            composite_alpha,
            present_mode: vk::PresentMode::Fifo,
            ..Default::default()
        };

        let swapchain_with_image = if let Some(renderer) = vw.renderer.as_ref() {
            renderer.swapchain.swapchain.recreate(create_info)
        } else {
            vk::Swapchain::new(vw.device.clone(), vw.surface.clone(), create_info)
        }
        .unwrap();

        let swapchain = SwapchainWithImages {
            swapchain: swapchain_with_image.0,
            images: swapchain_with_image.1,
        };

        let render_pass = vulkano::single_pass_renderpass!(
            vw.device.clone(),
            attachments: {
                color: {
                    // Set the format the same as the swapchain.
                    format: swapchain.swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {},
            },
        )
        .unwrap();

        let framebuffers = swapchain
            .images
            .iter()
            .map(|image| {
                let view = vk::ImageView::new_default(image.clone()).unwrap();
                vk::Framebuffer::new(
                    render_pass.clone(),
                    vk::FramebufferCreateInfo {
                        attachments: vec![view],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();

        let vs = vw.vs.entry_point("main").unwrap();
        let fs = vw.fs.entry_point("main").unwrap();

        let vertex_input_state = MyVertex::per_vertex().definition(&vs).unwrap();

        let stages = [
            vk::PipelineShaderStageCreateInfo::new(vs),
            vk::PipelineShaderStageCreateInfo::new(fs),
        ];

        let layout = vk::PipelineLayout::new(
            vw.device.clone(),
            vk::PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(vw.device.clone())
                .unwrap(),
        )
        .unwrap();

        let subpass = vk::Subpass::from(render_pass.clone(), 0).unwrap();

        let viewport = vk::Viewport {
            offset: [0.0, 0.0],
            extent: dimensions.into(),
            depth_range: 0.0..=1.0,
        };

        let pipeline = vk::GraphicsPipeline::new(
            vw.device.clone(),
            None,
            vk::GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(vk::InputAssemblyState::default()),
                viewport_state: Some(vk::ViewportState {
                    viewports: [viewport].into_iter().collect(),
                    ..Default::default()
                }),
                rasterization_state: Some(vk::RasterizationState::default()),
                multisample_state: Some(vk::MultisampleState::default()),
                color_blend_state: Some(vk::ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    vk::ColorBlendAttachmentState::default(),
                )),
                subpass: Some(subpass.into()),
                ..vk::GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap();

        let command_buffers = framebuffers
            .iter()
            .map(|framebuffer| {
                let mut builder = vk::AutoCommandBufferBuilder::primary(
                    vw.command_buffer_allocator.clone(),
                    vw.queue.queue_family_index(),
                    vk::CommandBufferUsage::MultipleSubmit,
                )
                .unwrap();

                builder
                    .begin_render_pass(
                        vk::RenderPassBeginInfo {
                            clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into())],
                            ..vk::RenderPassBeginInfo::framebuffer(framebuffer.clone())
                        },
                        vk::SubpassBeginInfo {
                            contents: vk::SubpassContents::Inline,
                            ..Default::default()
                        },
                    )
                    .unwrap()
                    .bind_pipeline_graphics(pipeline.clone())
                    .unwrap()
                    .bind_vertex_buffers(0, vw.vertex_buffer.clone())
                    .unwrap();

                unsafe {
                    builder
                        .draw(vw.vertex_buffer.len() as u32, 1, 0, 0)
                        .unwrap()
                };

                builder.end_render_pass(Default::default()).unwrap();

                builder.build().unwrap()
            })
            .collect();

        VulkanWindowRenderer {
            swapchain,
            framebuffers,
            render_pass,
            pipeline,
            command_buffers,
        }
    }
}

type MyFenceFuture = vk::FenceSignalFuture<
    vk::PresentFuture<
        vk::CommandBufferExecFuture<
            vk::JoinFuture<Box<dyn vk::GpuFuture>, vk::SwapchainAcquireFuture>,
        >,
    >,
>;

struct VulkanWindow {
    instance: Arc<vk::Instance>,
    physical_device: Arc<vk::PhysicalDevice>,
    device: Arc<vk::Device>,
    queue: Arc<vk::Queue>,
    surface: Arc<vk::Surface>,
    vertex_buffer: vk::Subbuffer<[MyVertex]>,
    vs: Arc<vk::ShaderModule>,
    fs: Arc<vk::ShaderModule>,
    renderer: Option<VulkanWindowRenderer>,
    window_resized: bool,
    recreate_swapchain: bool,
    fences: Vec<Option<Arc<MyFenceFuture>>>,
    previous_fence_i: u32,
    command_buffer_allocator: Arc<vk::StandardCommandBufferAllocator>,
    window: Arc<Window>,
}

impl VulkanWindow {
    pub fn new(event_loop: &ActiveEventLoop) -> Self {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(LogicalSize::new(WIDTH, HEIGHT))
                        .with_title("the title")
                        .with_decorations(true),
                )
                .unwrap(),
        );

        let library = vk::VulkanLibrary::new().expect("no local Vulkan library/DLL");
        let required_extensions = vk::Surface::required_extensions(&event_loop).unwrap();

        let instance = vk::Instance::new(
            library,
            vk::InstanceCreateInfo {
                flags: vk::InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        )
        .expect("failed to create instance");

        let surface = vk::Surface::from_window(instance.clone(), window.clone()).unwrap();

        let device_extensions = vk::DeviceExtensions {
            khr_swapchain: true,
            ..vk::DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .expect("could not enumerate devices")
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|q| (p, q as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                vk::PhysicalDeviceType::DiscreteGpu => 0,
                vk::PhysicalDeviceType::IntegratedGpu => 1,
                vk::PhysicalDeviceType::VirtualGpu => 2,
                vk::PhysicalDeviceType::Cpu => 3,
                _ => 4,
            })
            .expect("no device available");

        let (device, mut queues) = vk::Device::new(
            physical_device.clone(),
            vk::DeviceCreateInfo {
                queue_create_infos: vec![vk::QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let queue = queues.next().unwrap();

        let memory_allocator = Arc::new(vk::StandardMemoryAllocator::new_default(device.clone()));

        let vertex1 = MyVertex {
            position: [-0.5, -0.5],
        };
        let vertex2 = MyVertex {
            position: [0.0, 0.5],
        };
        let vertex3 = MyVertex {
            position: [0.5, -0.25],
        };
        let vertex_buffer = vk::Buffer::from_iter(
            memory_allocator,
            vk::BufferCreateInfo {
                usage: vk::BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            vk::AllocationCreateInfo {
                memory_type_filter: vk::MemoryTypeFilter::PREFER_DEVICE
                    | vk::MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vec![vertex1, vertex2, vertex3],
        )
        .unwrap();

        let vs = vs::load(device.clone()).expect("failed to create shader module");
        let fs = fs::load(device.clone()).expect("failed to create shader module");

        let caps = physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");

        let frames_in_flight = caps.min_image_count + 1;

        VulkanWindow {
            command_buffer_allocator: Arc::new(vk::StandardCommandBufferAllocator::new(
                device.clone(),
                Default::default(),
            )),
            instance,
            physical_device,
            device,
            queue,
            surface,
            vertex_buffer,
            vs,
            fs,
            renderer: None,
            window_resized: false,
            recreate_swapchain: false,
            fences: vec![None; frames_in_flight as usize],
            previous_fence_i: 0,
            window,
        }
    }

    fn update_renderer(&mut self) {
        if self.renderer.is_none() || self.window_resized || self.recreate_swapchain {
            log::debug!(
                "update renderer {}, {}",
                self.window_resized,
                self.recreate_swapchain
            );

            if self.renderer.is_some() && self.fences[self.previous_fence_i as usize].is_some() {
                let mut now = vk::sync::now(self.device.clone());
                now.cleanup_finished();
            }

            self.renderer = Some(VulkanWindowRenderer::new(self, self.window.inner_size()));
        }
    }

    pub fn set_window_resized(&mut self) {
        self.window_resized = true;
    }

    pub fn render(&mut self) {
        self.update_renderer();
        let renderer = self.renderer.as_ref().unwrap();

        let (image_i, suboptimal, acquire_future) =
            match vk::acquire_next_image(renderer.swapchain.swapchain.clone(), None)
                .map_err(vk::Validated::unwrap)
            {
                Ok(r) => r,
                Err(vk::VulkanError::OutOfDate) => {
                    log::debug!("set recreate_swapchain");
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        if let Some(image_fence) = &self.fences[image_i as usize] {
            image_fence.wait(None).unwrap();
        }

        let previous_future = match self.fences[self.previous_fence_i as usize].clone() {
            // Create a NowFuture
            None => {
                let mut now = vk::sync::now(self.device.clone());
                now.cleanup_finished();

                now.boxed()
            }
            // Use the existing FenceSignalFuture
            Some(fence) => fence.boxed(),
        };

        let future = previous_future
            .join(acquire_future)
            .then_execute(
                self.queue.clone(),
                renderer.command_buffers[image_i as usize].clone(),
            )
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                vk::SwapchainPresentInfo::swapchain_image_index(
                    renderer.swapchain.swapchain.clone(),
                    image_i,
                ),
            )
            .then_signal_fence_and_flush();

        self.fences[image_i as usize] = match future.map_err(vk::Validated::unwrap) {
            Ok(value) => Some(Arc::new(value)),
            Err(vk::VulkanError::OutOfDate) => {
                log::debug!("VulkanError::OutOfDate");
                self.recreate_swapchain = true;
                None
            }
            Err(e) => {
                log::error!("failed to flush future: {}", e);
                None
            }
        };

        self.previous_fence_i = image_i;
    }
}

#[derive(Default)]
struct HelloTriangleApplication {
    vulkan_window: Option<VulkanWindow>,
}

impl ApplicationHandler for HelloTriangleApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("resumed");
        self.vulkan_window = Some(VulkanWindow::new(event_loop));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                log::debug!("resized event");
                self.vulkan_window.as_mut().unwrap().set_window_resized();
            }
            WindowEvent::RedrawRequested => {
                self.vulkan_window.as_mut().unwrap().render();
            }
            _ => (),
        }
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = HelloTriangleApplication::default();
    event_loop.run_app(&mut app).unwrap();
}
