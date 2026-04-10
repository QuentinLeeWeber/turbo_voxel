use crate::engine::camera::{Camera, Projection};
use crate::game_object::GameObjectID;
use cgmath::{Deg, Point3, Rad};
use std::{collections::HashMap, ops::RangeInclusive, sync::Arc};
use vulkano::device::DeviceFeatures;
use vulkano::{
    Validated, VulkanError, VulkanLibrary,
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, DrawIndexedIndirectCommand, RenderPassBeginInfo,
        allocator::StandardCommandBufferAllocator,
    },
    descriptor_set::{
        DescriptorSet, WriteDescriptorSet, allocator::StandardDescriptorSetAllocator,
    },
    device::{
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
        physical::PhysicalDevice,
    },
    format::Format,
    image::{Image, ImageCreateInfo, ImageUsage, view::ImageView},
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
        graphics::GraphicsPipelineCreateInfo,
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            depth_stencil::{DepthState, DepthStencilState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            subpass::PipelineSubpassType,
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Viewport, ViewportState},
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    single_pass_renderpass,
    swapchain::{
        Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo, acquire_next_image,
    },
    sync,
    sync::GpuFuture,
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
pub mod prelude;
use prelude::*;

pub struct RenderData {
    pub window: Arc<Window>,
    surface: Arc<Surface>,
    swapchain: Arc<Swapchain>,
    swapchain_images: Vec<Arc<Image>>,
    msaa_image: Arc<Image>,
    viewport: Viewport,
    framebuffers: Vec<Arc<Framebuffer>>,

    render_pass: Arc<RenderPass>,

    pipeline: Arc<GraphicsPipeline>,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,

    pub camera_uniform_descriptor_set: Arc<DescriptorSet>,

    depth_image: Arc<Image>,
    depth_view: Arc<ImageView>,
    msaa_view: Arc<ImageView>,
}
struct MeshBufferInfo {
    first_vertex: u32,
    index_count: u32,
    index_start: u32,
}

pub struct Renderer {
    library: Arc<VulkanLibrary>,
    object_data: HashMap<ObjectDataID, ObjectData>,
    mesh_data: HashMap<u32, MeshData>,
    instance: Arc<Instance>,
    instances: HashMap<ObjectDataID, Vec<GPUInstance>>,

    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,

    instance_buffer: Subbuffer<[InstanceData]>,
    vertex_buffer: Subbuffer<[VertexData]>,
    index_buffer: Subbuffer<[u32]>,
    indirect_buffer: Subbuffer<[DrawIndexedIndirectCommand]>,

    physical_device: Arc<PhysicalDevice>,
    device: Arc<Device>,

    queue_family_index: u32,
    queue: Arc<Queue>,

    camera_buffer: Subbuffer<vs::Camera>,

    pub render_data: Option<RenderData>,

    last_mesh_id: u32,
    last_object_id: u32,

    last_index_index: u32,
    last_vertex_index: u32,

    window: Option<Arc<Window>>,
    cursor_grabbed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectDataID(pub u32);

impl Renderer {
    fn next_mesh_id(&mut self) -> u32 {
        self.last_mesh_id += 1;
        self.last_mesh_id
    }
    fn next_object_id(&mut self) -> u32 {
        self.last_object_id += 1;
        self.last_object_id
    }

    pub fn remove_game_object(&mut self, instance_id: GameObjectID) {
        //entferne instanz

        self.instances
            .iter_mut()
            .for_each(|(_, v)| v.retain(|i| i.instance_id != instance_id));

        self.recreate_buffers();
    }

    /*
     * recreates all buffers from mesh_object and meshes and instances
     */
    fn recreate_buffers(&mut self) {
        //für jedes Mesh in welchem Objekt
        let mut mesh_object: HashMap<u32, Vec<ObjectDataID>> = HashMap::new();
        for (id, object) in &self.object_data {
            for &mesh in &object.meshes {
                mesh_object.entry(mesh).or_default().push(*id);
            }
        }
        let meshes: Vec<u32> = mesh_object.keys().cloned().collect();

        //entferne ungenutzte Meshes aus MeshData
        self.mesh_data.retain(|id, _| mesh_object.contains_key(id));

        let mut instance_data: Vec<InstanceData> = Vec::new();
        let mut vertex_bufer: Vec<VertexData> = Vec::new();
        let mut index_buffer: Vec<u32> = Vec::new();
        let mut commands: Vec<DrawIndexedIndirectCommand> = Vec::new();

        for mesh in meshes {
            //für jedes Mesh in Objekt Reihenfolge die InstanceData
            let mut mesh_instances: Vec<InstanceData> = mesh_object
                .get(&mesh)
                .unwrap()
                .iter()
                .flat_map(|o| {
                    self.instances
                        .get(o)
                        .unwrap()
                        .iter()
                        .map(|i| i.instance)
                        .clone()
                })
                .collect();

            //erstelle meshBufferMapping
            let data = self.mesh_data.get(&mesh).unwrap();

            //für jedes Mesh IndirectDrawCommand
            let command = DrawIndexedIndirectCommand {
                index_count: data.indices.len() as u32,
                instance_count: mesh_instances.len() as u32,
                first_instance: instance_data.len() as u32,
                first_index: index_buffer.len() as u32,
                vertex_offset: 0,
            };

            //erstelle Vertex und IndexBuffer
            vertex_bufer.append(&mut data.vertices.clone());
            index_buffer.append(&mut data.indices.clone());
            commands.push(command);
            instance_data.append(&mut mesh_instances);
        }

        self.recreate_index_buffer(&index_buffer);
        self.recreate_vertex_buffer(&vertex_bufer);
        self.recreate_instance_buffer(&instance_data);
        self.recreate_command_buffer(&commands);
    }

    fn recreate_index_buffer(&mut self, indices: &Vec<u32>) {
        let new_buffer = Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            indices.iter().cloned(),
        )
        .unwrap();
        self.index_buffer = new_buffer;
    }
    fn recreate_vertex_buffer(&mut self, vertices: &Vec<VertexData>) {
        self.vertex_buffer = Buffer::from_iter(
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
            vertices.iter().cloned(),
        )
        .unwrap();
    }
    fn recreate_instance_buffer(&mut self, instances: &Vec<InstanceData>) {
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
            instances.iter().cloned(),
        )
        .unwrap();
    }
    fn recreate_command_buffer(&mut self, commands: &Vec<DrawIndexedIndirectCommand>) {
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
            commands.iter().cloned(),
        )
        .unwrap();
    }

    fn add_mesh(&mut self, mesh: MeshData) -> u32 {
        let id = self.next_mesh_id();
        self.mesh_data.insert(id, mesh);
        id
    }

    /*
     * inserts an ObjectData into the internal datastructures and returns its ids
     */
    pub fn create_object_data(&mut self, meshes: Vec<MeshData>) -> ObjectDataID {
        let ids = meshes.iter().cloned().map(|m| self.add_mesh(m)).collect();
        let oid = ObjectDataID(self.next_object_id());
        let data = ObjectData {
            id: oid,
            meshes: ids,
        };
        self.object_data.insert(oid, data);
        oid
        //TODO: check if fitting object is present
    }

    /*
     * Instantiates an Object where ObjectData is allready uploaded
     */
    pub fn add_object_instance(
        &mut self,
        instance: InstanceData,
        instance_id: GameObjectID,
        object_id: ObjectDataID,
    ) {
        self.instances
            .entry(object_id)
            .or_default()
            .push(GPUInstance {
                instance,
                instance_id,
            });
        self.recreate_buffers();
    }
    /*
     * creates a new object instance from an object that was never uploaded
     * returns the new id of ObjectData
     * TODO: add deduplication here
     */
    pub fn instantiate_object(
        &mut self,
        meshes: Vec<MeshData>,
        instance: InstanceData,
        id: GameObjectID,
    ) -> ObjectDataID {
        let object_id = self.create_object_data(meshes);

        self.add_object_instance(instance, id, object_id);

        object_id
    }

    /*
     * update a allready present instance and change their transforms
     * use this to set new positions from simulation
     * TODO: Batch operation
     */
    pub fn update_object_instance(
        &mut self,
        objekt_id: ObjectDataID,
        instanz_id: GameObjectID,
        instanz: InstanceData,
    ) {
        let obj = self
            .instances
            .get_mut(&objekt_id)
            .expect("Objekt nicht gefunden");
        let ind = obj
            .iter()
            .position(|i| i.instance_id == instanz_id)
            .expect("Instanz ID nicht gefunden");
        obj[ind].instance = instanz;

        self.recreate_buffers();
    }

    pub fn new(event_loop: &impl HasDisplayHandle) -> Renderer {
        let library = VulkanLibrary::new().expect("no local Vulkan library");
        let required_extensions = Surface::required_extensions(&event_loop).unwrap();
        let instance = create_instance(library.clone(), required_extensions);
        let physical_device = create_physical_device(instance.clone());

        let queue_family_index = create_queue_family_index(physical_device.clone());

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let features = DeviceFeatures {
            multi_draw_indirect: true,
            ..Default::default()
        };

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                enabled_features: features, // <-- ADD THIS LINE
                ..Default::default()
            },
        )
        .expect("failed to create device");

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

        let vertex_buffer = create_vertex_buffer(&memory_allocator);
        let index_buffer = create_index_buffer(&memory_allocator);
        let indirect_buffer = create_indirect_buffer(&memory_allocator);
        let instance_buffer = create_instance_buffer(&memory_allocator);

        let mut camera = Camera::new(
            Point3::new(0.0, 0.0, 0.0),
            Rad::from(Deg(90.0)),
            Rad::from(Deg(0.0)),
            Projection::new(10, 10, Rad::from(Deg(90.0)), 1.0, 1000.0),
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
            instances: HashMap::new(),
            object_data: HashMap::new(),
            index_buffer,
            camera_buffer,
            descriptor_set_allocator,
            last_mesh_id: 1024,
            last_object_id: 1024,
            last_vertex_index: 0,
            last_index_index: 0,
            mesh_data: HashMap::new(),
            window: None,
            cursor_grabbed: false,
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
                    multisample_state: Some(MultisampleState {
                        rasterization_samples: vulkano::image::SampleCount::Sample4,
                        ..Default::default()
                    }),
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

        let msaa_image = self.create_msaa_image(&new_swapchain);
        let msaa_view = ImageView::new_default(msaa_image.clone()).unwrap();

        let data = self.render_data.as_mut().unwrap();
        data.swapchain = new_swapchain;
        data.framebuffers = generate_framebuffers(
            msaa_view.clone(),
            new_images,
            depth_view,
            data.render_pass.clone(),
        );
        data.msaa_image = msaa_image;
        data.msaa_view = msaa_view;
        data.viewport.extent = viewport_extent;
        data.recreate_swapchain = false;
    }

    pub fn render(&mut self) {
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
                    clear_values: vec![Some([0.0, 0.0, 0.0, 1.0].into()), None, Some(1.0.into())],
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
        let command_count = self.indirect_buffer.len();

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

    pub fn update_camera_uniform(&mut self, camera: &mut Camera) {
        let proj = camera.calc_matrix().into();
        let camera_uniform = vs::Camera {
            view_position: [camera.position.x, camera.position.y, camera.position.z, 0.0],
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

    pub fn set_cursor_grab(&mut self, grabbed: bool) {
        if let Some(window) = &self.window {
            self.cursor_grabbed = grabbed;

            let grab_mode = if self.cursor_grabbed {
                winit::window::CursorGrabMode::Locked
            } else {
                winit::window::CursorGrabMode::None
            };

            if let Err(e) = window.set_cursor_grab(grab_mode) {
                eprintln!("Error grabbing the cursor {:?}", e);
            }

            window.set_cursor_visible(!self.cursor_grabbed);
        }
    }

    pub fn resize(&mut self, window: Arc<Window>, camera: &mut Camera) {
        let surface = Surface::from_window(self.instance.clone(), window.clone()).unwrap();

        self.window = Some(window.clone());

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

            camera
                .projection
                .resize(dimensions.width, dimensions.height);

            Swapchain::new(
                self.device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: caps.min_image_count,
                    image_format,
                    image_extent: dimensions.into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST,
                    composite_alpha,
                    ..Default::default()
                },
            )
            .unwrap()
        };

        let render_pass = single_pass_renderpass!(
            self.device.clone(),
            attachments: {
                msaa_color: {
                    format: swapchain.image_format(),
                    samples: 4,
                    load_op: Clear,
                    store_op: DontCare,
                },
                resolve_color: {
                    format: swapchain.image_format(),
                    samples: 1,
                    load_op: DontCare,
                    store_op: Store,
                },
                depth: {
                    format: Format::D32_SFLOAT,
                    samples: 4, // Muss mit msaa_color übereinstimmen
                    load_op: Clear,
                    store_op: DontCare,
                }
            },
            pass: {
                color: [msaa_color],
                color_resolve: [resolve_color], // Muss vor depth_stencil stehen!
                depth_stencil: {depth}
            }
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

        let msaa_image = self.create_msaa_image(&swapchain);
        let msaa_view = ImageView::new_default(msaa_image.clone()).unwrap();

        self.render_data = Some(RenderData {
            window,
            surface,
            swapchain,
            swapchain_images: images.clone(),
            render_pass: render_pass.clone(),
            framebuffers: generate_framebuffers(
                msaa_view.clone(),
                images,
                depth_view.clone(),
                render_pass,
            ),
            pipeline,
            recreate_swapchain: false,
            previous_frame_end,
            viewport,
            camera_uniform_descriptor_set,
            depth_image,
            depth_view,
            msaa_image,
            msaa_view,
        })
    }

    fn create_msaa_image(&mut self, swapchain: &Arc<Swapchain>) -> Arc<Image> {
        let extend = swapchain.image_extent();
        let mut e = [1; 3];
        e[0] = extend[0];
        e[1] = extend[1];
        
        Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                usage: ImageUsage::TRANSIENT_ATTACHMENT | ImageUsage::COLOR_ATTACHMENT,
                format: swapchain.image_format(),
                samples: vulkano::image::SampleCount::Sample4,
                extent: e,
                ..Default::default()
            },
            AllocationCreateInfo {
                ..Default::default()
            },
        )
        .unwrap()
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
                samples: vulkano::image::SampleCount::Sample4,
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
) -> Subbuffer<[u32]> {
    Buffer::new_slice(
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
        1024,
    )
    .expect("failed to create index buffer")
}

fn create_vertex_buffer(
    memory_allocator: &Arc<
        vulkano::memory::allocator::GenericMemoryAllocator<
            vulkano::memory::allocator::FreeListAllocator,
        >,
    >,
) -> Subbuffer<[VertexData]> {
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
    msaa_image_view: Arc<ImageView>,
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
                    attachments: vec![
                        msaa_image_view.clone(),  // Index 0: Match 'msaa_color' (4 samples)
                        view.clone(),             // Index 1: Match 'resolve_color' (1 sample)
                        depth_image_view.clone(), // Index 2: Match 'depth' (4 samples)
                    ],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}
