use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, DrawIndexedIndirectCommand, DrawIndirectCommand, RenderPassBeginInfo,
};
use vulkano::device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageUsage};
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::subpass::PipelineSubpassType;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::swapchain::{
    Swapchain, SwapchainCreateInfo, SwapchainPresentInfo, acquire_next_image,
};
use vulkano::sync::GpuFuture;
use vulkano::{Validated, VulkanError, single_pass_renderpass, sync};
use vulkano::{
    VulkanLibrary,
    device::{DeviceExtensions, QueueFlags, physical::PhysicalDevice},
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
    swapchain::Surface,
};
use vulkano::{
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
        graphics::GraphicsPipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
};
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::Window;

mod vs {
    vulkano_shaders::shader!(
        ty: "vertex",
        path: "src/engine/renderer/shaders/vertex_shader.glsl"
    );
}

mod fs {
    vulkano_shaders::shader!(
        ty: "fragment",
        path: "src/engine/renderer/shaders/fragment_shader.glsl"
    );
}
mod object_data;
mod prelude;
use prelude::*;

pub struct RenderData {
    window: Arc<Window>,
    surface: Arc<Surface>,
    swapchain: Arc<Swapchain>,
    swapchain_images: Vec<Arc<Image>>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    pipeline: Arc<GraphicsPipeline>,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    viewport: Viewport,
}
struct MeshBufferInfo {
    first_vertex: u32,
    vertex_count: u32,
}

pub struct Renderer {
    library: Arc<VulkanLibrary>,
    objects: HashMap<u32, ObjectData>,
    instance: Arc<Instance>,
    instances: Vec<InstanceData>,
    indirect_commands: Vec<DrawIndirectCommand>,
    max_indirect_commands: usize,
    max_instance_count: usize,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    vertex_buffer: Subbuffer<[VertexData]>,
    instance_buffer: Subbuffer<[InstanceData]>,
    indirect_buffer: Subbuffer<[DrawIndirectCommand]>,
    physical_device: Arc<PhysicalDevice>,
    device: Arc<Device>,
    queue_family_index: u32,
    queue: Arc<Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    render_data: Option<RenderData>,
    mesh_buffer_mapping: HashMap<u32, MeshBufferInfo>,
}
/*
 *
 */
impl Renderer {
    /*
     * add_instance object_id, Vec<InstanceDat>
     *
     */
    pub fn add_object_instance(&mut self, object_id: u32, instanz: InstanceData) {
        self.add_instance(instanz);
        let mesh_ids: Vec<u32> = self
            .objects
            .get(&object_id)
            .expect("Object ID not found")
            .meshes
            .iter()
            .map(|mesh| mesh.id)
            .collect();
        for mesh_id in mesh_ids {
            self.add_indirect_draw(mesh_id);
        }
    }

    pub fn add_indirect_draw(&mut self, mesh_id: u32) {
        let info = self.mesh_buffer_mapping.get(&mesh_id).unwrap();

        self.indirect_commands.push(DrawIndirectCommand {
            vertex_count: info.vertex_count,
            instance_count: 1,
            first_vertex: info.first_vertex,
            first_instance: (self.instances.len() - 1) as u32,
        });
        let count = self.indirect_commands.len();

        if count > self.max_indirect_commands {
            self.max_indirect_commands = count * 2 + 1;
            self.indirect_buffer = Buffer::from_iter(
                self.memory_allocator.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::INDIRECT_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                self.indirect_commands.clone(),
            )
            .unwrap();
        } else {
            if let Ok(mut mapping) = self.indirect_buffer.write() {
                mapping[..count].copy_from_slice(&self.indirect_commands);
            }
        }
    }

    pub fn add_instance(&mut self, instanz: InstanceData) {
        self.instances.push(instanz);
        let count = self.instances.len();

        if count > self.max_instance_count {
            self.max_instance_count = count * 2 + 1;
            self.instance_buffer = Buffer::from_iter(
                self.memory_allocator.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::VERTEX_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                self.instances.clone(),
            )
            .unwrap();
        } else {
            if let Ok(mut mapping) = self.instance_buffer.write() {
                mapping[..count].copy_from_slice(&self.instances);
            }
        }
    }

    pub fn new(event_loop: &impl HasDisplayHandle, objects: Vec<ObjectData>) -> Renderer {
        let library = VulkanLibrary::new().expect("no local Vulkan library");
        let required_extensions = Surface::required_extensions(&event_loop).unwrap();
        let instance = create_instance(library.clone(), required_extensions);
        let physical_device = create_physical_device(instance.clone());

        let queue_family_index = create_queue_family_index(physical_device.clone());

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            khr_draw_indirect_count: true,
            ..DeviceExtensions::empty()
        };

        let (device, mut queues) = create_device(
            physical_device.clone(),
            queue_family_index,
            device_extensions,
        );

        let queue = queues.next().unwrap();

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let mut objs = HashMap::new();
        let mut vertices = Vec::new();
        let mut mesh_buffer_mapping = HashMap::new();
        let mut pos = 0;
        for mut obj in objects {
            for mut mesh in obj.meshes.drain(..) {
                let len = mesh.vertices.len();
                let info = MeshBufferInfo {
                    first_vertex: pos,
                    vertex_count: len as u32,
                };
                pos += len as u32;
                vertices.append(&mut mesh.vertices);
                mesh_buffer_mapping.insert(mesh.id, info);
            }
            objs.insert(obj.id, obj);
        }

        let vertex_buffer = Buffer::new_slice(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            1024,
        )
        .unwrap();
        let mut instances = vec![];

        let indirect_commands = vec![];

        let indirect_buffer = Buffer::new_slice(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDIRECT_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            1024,
        )
        .unwrap();

        let instance_buffer = Buffer::new_slice(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            1024,
            //TODO: alle objekte hochladen
        )
        .unwrap();

        return Renderer {
            library,
            instance,
            physical_device: physical_device,
            queue_family_index: queue_family_index,
            queue: queue,
            device: device,
            command_buffer_allocator: command_buffer_allocator,
            memory_allocator: memory_allocator,
            instance_buffer: instance_buffer,
            vertex_buffer: vertex_buffer,
            render_data: None,
            indirect_buffer,
            instances: instances,
            indirect_commands: indirect_commands,
            max_indirect_commands: 0,
            max_instance_count: 0,
            objects: objs,
            mesh_buffer_mapping: mesh_buffer_mapping,
        };
    }

    fn create_pipeline(&mut self, render_pass: &Arc<RenderPass>) -> Arc<GraphicsPipeline> {
        let pipeline = {
            let vs = vs::load(self.device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap();
            let fs = fs::load(self.device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap();

            let vertex_input_state = [VertexData::per_vertex(), InstanceData::per_instance()]
                .definition(&vs)
                .unwrap();

            let stages = vec![
                PipelineShaderStageCreateInfo::new(vs),
                PipelineShaderStageCreateInfo::new(fs),
            ];

            let layout_create_info = PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(self.device.clone())
                .expect("Failed to create pipeline layout info");
            let layout = PipelineLayout::new(self.device.clone(), layout_create_info)
                .expect("Failed to create pipeline layout");

            let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

            GraphicsPipeline::new(
                self.device.clone(),
                None, // No pipeline cache
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(), // Ensure it's a SmallVec/Vec of stages
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState {
                        // If dynamic, we usually provide an empty viewports list
                        viewports: [Viewport::default()].into_iter().collect(),
                        ..Default::default()
                    }),
                    rasterization_state: Some(RasterizationState::default()),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        subpass.num_color_attachments(),
                        ColorBlendAttachmentState::default(),
                    )),
                    // Dynamic state is now usually a set of enum values
                    dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                    subpass: Some(PipelineSubpassType::BeginRenderPass(subpass.clone())),
                    ..GraphicsPipelineCreateInfo::layout(layout.clone()) // Pass layout here
                },
            )
            .expect("failed to create graphics pipeline")
        };
        pipeline
    }

    pub fn update_screen_size(&mut self) {
        let data = self.render_data.as_mut().unwrap();
        let window_size = data.window.as_ref().inner_size();
        let (new_swapchain, new_images) = data
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: window_size.into(),
                ..data.swapchain.as_ref().create_info()
            })
            .expect("failed to recreate swapchain");
        data.swapchain = new_swapchain;
        data.framebuffers = generate_framebuffers(new_images, data.render_pass.clone());
        data.viewport.extent = window_size.into();
        data.recreate_swapchain = false;
    }

    pub fn render(&mut self) {
        let data = self.render_data.as_mut().unwrap();
        let window_size = data.window.as_ref().inner_size();

        if window_size.width == 0 || window_size.height == 0 {
            return;
        }

        data.previous_frame_end.as_mut().unwrap().cleanup_finished();

        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(data.swapchain.clone(), None).map_err(Validated::unwrap) {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    data.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        if suboptimal {
            data.recreate_swapchain = true;
        }

        let mut builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.queue.queue_family_index(),
            vulkano::command_buffer::CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer(
                        data.framebuffers[image_index as usize].clone(),
                    )
                },
                Default::default(),
            )
            .unwrap()
            .set_viewport(0, [data.viewport.clone()].into_iter().collect())
            .unwrap()
            .bind_pipeline_graphics(data.pipeline.clone())
            .unwrap()
            // We pass both our lists of vertices here.
            .bind_vertex_buffers(
                0,
                (self.vertex_buffer.clone(), self.instance_buffer.clone()),
            )
            .unwrap();
        let command_count = self.indirect_commands.len() as u64;

        if command_count > 0 {
            let buffer_slice = self.indirect_buffer.clone().slice(0..command_count);
            unsafe { builder.draw_indirect(buffer_slice) }.unwrap();
        } else {
        }

        builder.end_render_pass(Default::default()).unwrap();

        let command_buffer = builder.build().unwrap();
        let future = data
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(data.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush();

        match future.map_err(Validated::unwrap) {
            Ok(future) => {
                data.previous_frame_end = Some(future.boxed());
            }
            Err(VulkanError::OutOfDate) => {
                data.recreate_swapchain = true;
                data.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                data.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
        }
    }

    pub fn resize(&mut self, window: Arc<Window>) {
        let surface = Surface::from_window(self.instance.clone(), window.clone()).unwrap();

        let (mut swapchain, images) = {
            let caps = self
                .physical_device
                .surface_capabilities(&surface, Default::default())
                .expect("failed to get surface capabilities");

            let dimensions = window.inner_size();
            let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
            let image_format = self
                .physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0;

            Swapchain::new(
                self.device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: caps.min_image_count,
                    image_format,
                    image_extent: dimensions.into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha,
                    ..Default::default()
                },
            )
            .unwrap()
        };

        let render_pass = single_pass_renderpass!(
            self.device.clone(),
            attachments: {
                color: {
                    format: swapchain.image_format(),
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

        let pipeline = self.create_pipeline(&render_pass);
        let window_size = window.inner_size();
        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: window_size.into(),
            depth_range: RangeInclusive::new(0.0, 1.0),
        };
        let previous_frame_end = Some(sync::now(self.device.clone()).boxed());
        self.render_data = Some(RenderData {
            window,
            surface,
            swapchain,
            swapchain_images: images.clone(),
            render_pass: render_pass.clone(),
            framebuffers: generate_framebuffers(images, render_pass),
            pipeline,
            recreate_swapchain: false,
            previous_frame_end,
            viewport,
        })
    }
}

fn create_device(
    physical_device: Arc<PhysicalDevice>,
    queue_family_index: u32,
    device_extensions: DeviceExtensions,
) -> (Arc<Device>, impl ExactSizeIterator<Item = Arc<Queue>>) {
    let (device, mut queues) = Device::new(
        physical_device.clone(),
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            enabled_extensions: device_extensions, // new
            ..Default::default()
        },
    )
    .expect("failed to create device");
    (device, queues)
}

fn create_queue_family_index(physical_device: Arc<PhysicalDevice>) -> u32 {
    let queue_family_index = physical_device
        .queue_family_properties()
        .iter()
        .position(|queue_family_properties| {
            queue_family_properties
                .queue_flags
                .contains(QueueFlags::GRAPHICS)
        })
        .expect("couldn't find a graphical queue family") as u32;
    queue_family_index
}

fn create_physical_device(instance: Arc<Instance>) -> Arc<PhysicalDevice> {
    let physical_device = instance
        .enumerate_physical_devices()
        .expect("could not enumerate enumerate devices")
        .next()
        .expect("no devices available");
    physical_device
}

fn create_instance(
    library: Arc<VulkanLibrary>,
    required_extensions: vulkano::instance::InstanceExtensions,
) -> Arc<Instance> {
    let instance = Instance::new(
        library.clone(),
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            enabled_extensions: required_extensions,
            ..Default::default()
        },
    )
    .expect("failed to create instance");
    instance
}

fn generate_framebuffers(
    swapchain_images: Vec<Arc<Image>>,
    render_pass: Arc<RenderPass>,
) -> Vec<Arc<Framebuffer>> {
    swapchain_images
        .iter()
        .map(|img| {
            let view = ImageView::new_default(img.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view.clone()],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}
