use cgmath::{Deg, Point3, Rad};
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, DrawIndexedIndirectCommand, RenderPassBeginInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo};
use vulkano::format::Format;
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
    image::view::ImageView,
    pipeline::{
        Pipeline,
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            depth_stencil::{DepthState, DepthStencilState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            viewport::{Viewport, ViewportState},
        },
    },
};
use vulkano::{
    image::{Image, ImageCreateInfo, ImageUsage},
    pipeline::graphics::{
        subpass::PipelineSubpassType,
        vertex_input::{Vertex, VertexDefinition},
    },
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
mod camera;
pub mod prelude;
use prelude::*;

use crate::engine::renderer::camera::{Camera, CameraController, Projection};

pub struct RenderData {
    pub window: Arc<Window>,
    surface: Arc<Surface>,
    swapchain: Arc<Swapchain>,
    swapchain_images: Vec<Arc<Image>>,
    viewport: Viewport,
    framebuffers: Vec<Arc<Framebuffer>>,

    render_pass: Arc<RenderPass>,

    pipeline: Arc<GraphicsPipeline>,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,

    pub camera_uniform_descriptor_set: Arc<DescriptorSet>,

    depth_image: Arc<Image>,
    depth_view: Arc<ImageView>,
}
struct MeshBufferInfo {
    first_vertex: u32,
    index_count: u32,
    index_start: u32,
}

pub struct Renderer {
    library: Arc<VulkanLibrary>,
    objects: HashMap<u32, ObjectData>,
    instance: Arc<Instance>,

    instances: Vec<GPUInstance>,
    max_instance_count: usize,
    instance_buffer: Subbuffer<[InstanceData]>,

    indirect_commands: Vec<DrawIndexedIndirectCommand>,
    max_indirect_commands: usize,

    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,

    vertex_buffer: Subbuffer<[VertexData]>,
    index_buffer: Subbuffer<[u32]>,
    indirect_buffer: Subbuffer<[DrawIndexedIndirectCommand]>,

    physical_device: Arc<PhysicalDevice>,
    device: Arc<Device>,

    queue_family_index: u32,
    queue: Arc<Queue>,

    mesh_buffer_mapping: HashMap<u32, MeshBufferInfo>,

    camera: Camera,
    pub camera_controller: CameraController,
    camera_buffer: Subbuffer<vs::Camera>,

    pub render_data: Option<RenderData>,
}
/*
 *
 */
impl Renderer {
    /*
     * add_instance object_id, Vec<InstanceDat>
     *
     */
    pub fn add_object_instance(&mut self, object_id: u32, instanz: GPUInstance) {
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
    /*
     * update a allready present instance and change their transforms
     * use this to set new positions from simulation
     * TODO: Batch operation
     */
    pub fn update_object_instance(&mut self, instanz_id: u32, instanz: InstanceData) {
        let ind = self
            .instances
            .iter()
            .position(|i| i.instance_id == instanz_id)
            .unwrap();
        self.instances[ind].instance = instanz;

        if let Ok(mut mapping) = self.instance_buffer.write() {
            mapping[ind] = instanz;
            println!("A Object update happened");
        }
    }

    pub fn add_indirect_draw(&mut self, mesh_id: u32) {
        let info = self.mesh_buffer_mapping.get(&mesh_id).unwrap();

        self.indirect_commands.push(DrawIndexedIndirectCommand {
            index_count: info.index_count,
            instance_count: 1,
            first_index: info.first_vertex,
            vertex_offset: 0,
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
        } else if let Ok(mut mapping) = self.indirect_buffer.write() {
            mapping[..count].copy_from_slice(&self.indirect_commands);
        }
    }

    pub fn add_instance(&mut self, instanz: GPUInstance) {
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
                self.instances.iter().map(|i| i.instance),
            )
            .unwrap();
        } else if let Ok(mut mapping) = self.instance_buffer.write() {
            for (i, inst) in self.instances.iter().take(count).enumerate() {
                mapping[i] = inst.instance;
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

        let supported_features = physical_device.supported_features();
        if !supported_features.multi_draw_indirect {
            panic!("Selected GPU does not support multi_draw_indirect");
        }

        let queue = queues.next().unwrap();

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let mut objs = HashMap::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut mesh_buffer_mapping = HashMap::new();
        let mut vertex_pos = 0;
        let mut index_pos = 0;
        for obj in objects {
            for mut mesh in obj.clone().meshes.drain(..) {
                let info = MeshBufferInfo {
                    first_vertex: vertex_pos,
                    index_start: index_pos,
                    index_count: mesh.indices.len() as u32,
                };
                vertex_pos += mesh.vertices.len() as u32;
                index_pos += mesh.indices.len() as u32;
                vertices.append(&mut mesh.vertices);
                indices.append(&mut mesh.indices);
                mesh_buffer_mapping.insert(mesh.id, info);
            }
            objs.insert(obj.id, obj.clone());
        }
        if vertices.is_empty() {
            unreachable!("Empty vertex array given to renderer");
        }

        let instances = vec![];
        let indirect_commands = vec![];

        let vertex_buffer = create_vertex_buffer(&memory_allocator, vertices);
        let index_buffer = create_index_buffer(&memory_allocator, indices);
        let indirect_buffer = create_indirect_buffer(&memory_allocator);
        let instance_buffer = create_instance_buffer(&memory_allocator);

        let mut camera = Camera::new(
            Point3::new(0.0, 0.0, 0.0),
            Rad::from(Deg(90.0)),
            Rad::from(Deg(0.0)),
            Projection::new(10, 10, Rad::from(Deg(90.0)), 0.1, 10.0),
        );
        let camera_uniform = vs::Camera {
            view_position: [camera.position.x, camera.position.y, camera.position.z, 0.0],
            view_proj: camera.calc_matrix().into(),
        };

        let camera_buffer = Buffer::from_data(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            camera_uniform,
        )
        .expect("Failed to create buffer");

        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        Renderer {
            library,
            instance,
            physical_device,
            queue_family_index,
            queue,
            device,
            command_buffer_allocator,
            memory_allocator,
            instance_buffer,
            vertex_buffer,
            render_data: None,
            indirect_buffer,
            instances,
            indirect_commands,
            max_indirect_commands: 1024,
            max_instance_count: 1024,
            objects: objs,
            mesh_buffer_mapping,
            index_buffer,
            camera,
            camera_buffer,
            descriptor_set_allocator,
            camera_controller: CameraController::new(1.0, 2.0),
        }
    }

    fn create_pipeline(&mut self, render_pass: &Arc<RenderPass>) -> Arc<GraphicsPipeline> {
        {
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
                    depth_stencil_state: Some(DepthStencilState {
                        depth: Some(DepthState {
                            write_enable: true,
                            compare_op: vulkano::pipeline::graphics::depth_stencil::CompareOp::Less,
                        }),
                        ..Default::default()
                    }),
                    ..GraphicsPipelineCreateInfo::layout(layout.clone())
                },
            )
            .expect("failed to create graphics pipeline")
        }
    }

    pub fn update_screen_size(&mut self) {
        let (new_swapchain, new_images, viewport_extent) = {
            let data = self.render_data.as_mut().unwrap();
            let window_size = data.window.inner_size();

            let (new_swapchain, new_images) = data
                .swapchain
                .recreate(SwapchainCreateInfo {
                    image_extent: window_size.into(),
                    ..data.swapchain.create_info()
                })
                .expect("failed to recreate swapchain");

            (new_swapchain, new_images, window_size.into())
        };

        let depth_image = self.create_depth_buffer(&new_swapchain);
        let depth_view = ImageView::new_default(depth_image).unwrap();

        let data = self.render_data.as_mut().unwrap();
        data.swapchain = new_swapchain;
        data.framebuffers = generate_framebuffers(new_images, depth_view, data.render_pass.clone());
        data.viewport.extent = viewport_extent;
        data.recreate_swapchain = false;
    }

    pub fn render(&mut self) {
        self.update_camera();

        let data = self.render_data.as_mut().unwrap();
        let _layout = data.pipeline.layout().set_layouts().first().unwrap();
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
                    clear_values: vec![Some([0.0, 0.0, 0.0, 1.0].into()), Some(1.0.into())],
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
            .unwrap()
            .bind_index_buffer(self.index_buffer.clone())
            .unwrap()
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Graphics,
                data.pipeline.layout().clone(),
                0,                                          // Set Index 0
                data.camera_uniform_descriptor_set.clone(), // Hier kommt das Set rein
            )
            .unwrap();
        let command_count = self.indirect_commands.len() as u64;

        if command_count > 0 {
            let buffer_slice = self.indirect_buffer.clone().slice(0..command_count);
            unsafe { builder.draw_indexed_indirect(buffer_slice) }.unwrap();
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

    fn update_camera(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        let proj = self.camera.calc_matrix().into();
        let camera_uniform = vs::Camera {
            view_position: [
                self.camera.position.x,
                self.camera.position.y,
                self.camera.position.z,
                0.0,
            ],
            view_proj: proj,
        };

        let camera_buffer = Buffer::from_data(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            camera_uniform,
        )
        .expect("Fehler beim Erstellen des Camera Buffers");
        let data = self.render_data.as_mut().unwrap();
        let window_size = data.window.as_ref().inner_size();
        if window_size.width == 0 || window_size.height == 0 {
            return;
        }
        self.camera_buffer = camera_buffer.clone();

        let layout = data.pipeline.layout().set_layouts().first().unwrap();
        let camera_descriptor_set = DescriptorSet::new(
            self.descriptor_set_allocator.clone(),
            layout.clone(),
            [WriteDescriptorSet::buffer(0, camera_buffer)],
            [],
        )
        .expect("Fehler beim Erstellen des Descriptor Sets");
        self.render_data
            .as_mut()
            .unwrap()
            .camera_uniform_descriptor_set = camera_descriptor_set;
    }

    pub fn resize(&mut self, window: Arc<Window>) {
        let surface = Surface::from_window(self.instance.clone(), window.clone()).unwrap();

        let (swapchain, images) = {
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

            self.camera
                .projection
                .resize(dimensions.width, dimensions.height);

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
                depth: {
                            format: Format::D32_SFLOAT,
                            samples: 1,
                            load_op: Clear,
                            store_op: DontCare,
                        }
            },
            pass: {
                color: [color],
                depth_stencil: {depth},
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

        let layout = pipeline.layout().set_layouts().first().unwrap();

        let camera_uniform_descriptor_set = DescriptorSet::new(
            self.descriptor_set_allocator.clone(),
            layout.clone(),
            [WriteDescriptorSet::buffer(0, self.camera_buffer.clone())],
            [],
        )
        .expect("failed to create camera uniform descriptor set");

        let depth_image = self.create_depth_buffer(&swapchain);
        let depth_view = ImageView::new_default(depth_image.clone()).unwrap();

        self.render_data = Some(RenderData {
            window,
            surface,
            swapchain,
            swapchain_images: images.clone(),
            render_pass: render_pass.clone(),
            framebuffers: generate_framebuffers(images, depth_view.clone(), render_pass),
            pipeline,
            recreate_swapchain: false,
            previous_frame_end,
            viewport,
            camera_uniform_descriptor_set,
            depth_image,
            depth_view,
        })
    }

    fn create_depth_buffer(&mut self, swapchain: &Arc<Swapchain>) -> Arc<Image> {
        let extend = swapchain.image_extent();
        let mut e = [1; 3];
        e[0] = extend[0];
        e[1] = extend[1];

        Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
                format: Format::D32_SFLOAT,
                extent: e,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap()
    }
}

fn create_instance_buffer(
    memory_allocator: &Arc<
        vulkano::memory::allocator::GenericMemoryAllocator<
            vulkano::memory::allocator::FreeListAllocator,
        >,
    >,
) -> Subbuffer<[InstanceData]> {
    Buffer::new_slice(
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
    .unwrap()
}

fn create_indirect_buffer(
    memory_allocator: &Arc<
        vulkano::memory::allocator::GenericMemoryAllocator<
            vulkano::memory::allocator::FreeListAllocator,
        >,
    >,
) -> Subbuffer<[DrawIndexedIndirectCommand]> {
    Buffer::new_slice(
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
    .unwrap()
}

fn create_index_buffer(
    memory_allocator: &Arc<
        vulkano::memory::allocator::GenericMemoryAllocator<
            vulkano::memory::allocator::FreeListAllocator,
        >,
    >,
    indices: Vec<u32>,
) -> Subbuffer<[u32]> {
    Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::INDEX_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        indices,
    )
    .expect("failed to create index buffer")
}

fn create_vertex_buffer(
    memory_allocator: &Arc<
        vulkano::memory::allocator::GenericMemoryAllocator<
            vulkano::memory::allocator::FreeListAllocator,
        >,
    >,
    vertices: Vec<VertexData>,
) -> Subbuffer<[VertexData]> {
    Buffer::from_iter(
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
        vertices,
    )
    .unwrap()
}

fn create_device(
    physical_device: Arc<PhysicalDevice>,
    queue_family_index: u32,
    device_extensions: DeviceExtensions,
) -> (Arc<Device>, impl ExactSizeIterator<Item = Arc<Queue>>) {
    let (device, queues) = Device::new(
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
    physical_device
        .queue_family_properties()
        .iter()
        .position(|queue_family_properties| {
            queue_family_properties
                .queue_flags
                .contains(QueueFlags::GRAPHICS)
        })
        .expect("couldn't find a graphical queue family") as u32
}

fn create_physical_device(instance: Arc<Instance>) -> Arc<PhysicalDevice> {
    instance
        .enumerate_physical_devices()
        .expect("could not enumerate enumerate devices")
        .next()
        .expect("no devices available")
}

fn create_instance(
    library: Arc<VulkanLibrary>,
    required_extensions: vulkano::instance::InstanceExtensions,
) -> Arc<Instance> {
    Instance::new(
        library.clone(),
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            enabled_extensions: required_extensions,
            ..Default::default()
        },
    )
    .expect("failed to create instance")
}

fn generate_framebuffers(
    swapchain_images: Vec<Arc<Image>>,
    depth_image_view: Arc<ImageView>,
    render_pass: Arc<RenderPass>,
) -> Vec<Arc<Framebuffer>> {
    swapchain_images
        .iter()
        .map(|img| {
            let view = ImageView::new_default(img.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view.clone(), depth_image_view.clone()],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}
