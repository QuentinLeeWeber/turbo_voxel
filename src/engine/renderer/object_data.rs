use std::alloc::alloc;
use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CopyBufferToImageInfo, PrimaryAutoCommandBuffer,
};
use vulkano::descriptor_set::WriteDescriptorSet;

use vulkano::{
    descriptor_set::{
        DescriptorSet, allocator::StandardDescriptorSetAllocator, layout::DescriptorSetLayout,
    },
    device::Device,
    format::Format,
    image::{
        Image, ImageCreateInfo, ImageUsage,
        sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
        view::ImageView,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
};

use crate::engine::renderer::prelude::{ImageData, LoadedTexture, ObjectData, Texture};

impl Texture for ImageData {
    fn load_textures(
        &self,
        allocator: std::sync::Arc<vulkano::memory::allocator::StandardMemoryAllocator>,
        command_buffer_allocator: Arc<
            vulkano::command_buffer::allocator::StandardCommandBufferAllocator,
        >,
        queue: std::sync::Arc<vulkano::device::Queue>,
        layout: Arc<DescriptorSetLayout>,
        device: Arc<Device>,
        descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) -> Vec<LoadedTexture> {
        let img = Image::new(
            allocator.clone(),
            ImageCreateInfo {
                image_type: vulkano::image::ImageType::Dim2d,
                format: Format::R8G8B8A8_SRGB,
                extent: [self.width, self.height, 1],
                usage: ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap();

        let view = ImageView::new_default(img.clone()).unwrap();
        let sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            },
        )
        .unwrap();
        let set = DescriptorSet::new(
            descriptor_set_allocator.clone(),
            layout.clone(),
            [
                WriteDescriptorSet::image_view(0, view.clone()),
                WriteDescriptorSet::image_view_sampler(1, view.clone(), sampler.clone()),
            ],
            [],
        )
        .unwrap();
        let buf = Buffer::from_iter(
            allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_RANDOM_ACCESS,
                ..Default::default()
            },
            self.data.clone(),
        )
        .expect("failed to create buffer");

        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(buf, img.clone()))
            .unwrap();

        vec![LoadedTexture {
            image: img,
            view: view,
            sampler: sampler,
            descriptor_set: set,
        }]
    }
}

impl Texture for ObjectData {
    fn load_textures(
        &self,
        allocator: std::sync::Arc<vulkano::memory::allocator::StandardMemoryAllocator>,
        command_buffer_allocator: Arc<
            vulkano::command_buffer::allocator::StandardCommandBufferAllocator,
        >,
        queue: std::sync::Arc<vulkano::device::Queue>,
        layout: Arc<DescriptorSetLayout>,
        device: Arc<Device>,
        descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) -> Vec<LoadedTexture> {
        self.materials
            .iter()
            .flat_map(|mat| {
                mat.diffuse_texture.load_textures(
                    allocator.clone(),
                    command_buffer_allocator.clone(),
                    queue.clone(),
                    layout.clone(),
                    device.clone(),
                    descriptor_set_allocator.clone(),
                    builder,
                )
            })
            .collect()
    }
}
