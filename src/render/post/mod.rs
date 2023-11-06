use std::{any::Any, ops::Deref, sync::Arc};

use ash::vk;
use pyrite::{
    prelude::{AppBuilder, Assets, Res, ResMut, Resource},
    render::render_manager::{self, RenderManager},
    vulkan::{
        CommandBuffer, ComputePipeline, ComputePipelineInfo, DescriptorSet, DescriptorSetLayout,
        Image, ImageDep, ImageInfo, InternalImage, Sampler, SamplerInfo, Shader, Vulkan,
        VulkanAllocator,
    },
};

use super::{
    render::RenderPipeline,
    shell::ShellRenderer,
    watched_shaders::{self, DependencySignal, WatchedShaders},
};

pub fn setup_post_processing(app_builder: &mut AppBuilder) {
    let post_processing = {
        let in_image = {
            let a = app_builder.get_resource::<ShellRenderer>();
            a.resolve_image().create_dep()
        };
        let in_depth_image = app_builder
            .get_resource::<RenderPipeline>()
            .backbuffer_depth_image()
            .create_dep();
        PostProcessing::new(
            &*app_builder.get_resource::<Vulkan>(),
            &mut *app_builder.get_resource_mut::<VulkanAllocator>(),
            &*app_builder.get_resource::<RenderManager>(),
            &*app_builder.get_resource::<RenderPipeline>(),
            &mut *app_builder.get_resource_mut::<Assets>(),
            &mut *app_builder.get_resource_mut::<WatchedShaders>(),
            in_image,
            in_depth_image,
        )
    };
    app_builder.add_resource(post_processing);

    app_builder.add_system(PostProcessing::update_system);
}

struct PushConstants {
    width: u32,
    height: u32,
}

/// The post processor is responsible for setting up the different pipeline effects.
#[derive(Resource)]
pub struct PostProcessing {
    pipeline: Option<ComputePipeline>,
    shader_dependency_signal: DependencySignal,
    in_image: ImageDep,
    in_depth_image: ImageDep,
    out_image: Image,
    descriptor_set_layout: DescriptorSetLayout,
    depth_sampler: Sampler,
    descriptor_set: DescriptorSet,
}

impl PostProcessing {
    pub fn new(
        vulkan: &Vulkan,
        vulkan_allocator: &mut VulkanAllocator,
        render_manager: &RenderManager,
        render_pipeline: &RenderPipeline,
        assets: &mut Assets,
        watched_shaders: &mut WatchedShaders,
        in_image: ImageDep,
        in_depth_image: ImageDep,
    ) -> Self {
        let out_image = Image::new(
            vulkan,
            vulkan_allocator,
            &ImageInfo::builder()
                .extent(render_pipeline.backbuffer_image().image_extent())
                .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC)
                .format(vk::Format::R8G8B8A8_UNORM)
                .view_subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .layer_count(1)
                        .level_count(1)
                        .build(),
                )
                .build(),
        );

        let shader_dependency_signal = watched_shaders.create_dependency_signal();
        watched_shaders.load_shader(
            assets,
            "shaders/post.comp",
            "post_comp",
            &shader_dependency_signal,
        );

        let descriptor_set_layout = DescriptorSetLayout::new(
            vulkan,
            &[
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(2)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .build(),
            ],
        );

        let depth_sampler = Sampler::new(vulkan, &SamplerInfo::builder().build());

        let descriptor_set = render_pipeline
            .descriptor_pool()
            .allocate_descriptor_sets(&descriptor_set_layout, 1)
            .pop()
            .unwrap();

        descriptor_set
            .write()
            .set_storage_image(0, in_image.clone())
            .set_storage_image(1, out_image.create_dep())
            .set_combined_image_sampler(
                2,
                vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                in_depth_image.clone(),
                &depth_sampler,
            )
            .submit_writes();

        Self {
            pipeline: None,
            shader_dependency_signal,
            in_image,
            in_depth_image,
            out_image,
            descriptor_set_layout,
            depth_sampler,
            descriptor_set,
        }
    }

    pub fn render(
        &self,
        command_buffer: &mut CommandBuffer,
        render_pipeline: &RenderPipeline,
    ) -> Vec<Arc<dyn Any + Send + Sync>> {
        if let Some(pipeline) = &self.pipeline {
            command_buffer.pipeline_barrier(
                vk::PipelineStageFlags::ALL_GRAPHICS,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[self.out_image.image_memory_barrier(
                    vk::ImageLayout::UNDEFINED,
                    vk::ImageLayout::GENERAL,
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::SHADER_WRITE,
                )],
            );

            command_buffer.bind_compute_pipeline(pipeline);

            command_buffer.bind_descriptor_sets(
                vk::PipelineBindPoint::COMPUTE,
                pipeline.pipeline_layout(),
                &[&self.descriptor_set],
            );

            command_buffer.write_push_constants_typed(
                pipeline.pipeline_layout(),
                vk::ShaderStageFlags::COMPUTE,
                0,
                &PushConstants {
                    width: render_pipeline.backbuffer_image().image_extent().width,
                    height: render_pipeline.backbuffer_image().image_extent().height,
                },
            );

            command_buffer.dispatch_compute(
                render_pipeline.backbuffer_image().image_extent().width / 16,
                render_pipeline.backbuffer_image().image_extent().height / 16,
                1,
            );
        }
        vec![]
    }

    pub fn is_ready(&self) -> bool {
        self.pipeline.is_some()
    }

    pub fn output_image(&self) -> &Image {
        &self.out_image
    }

    fn refresh_pipeline(
        &mut self,
        vulkan: &Vulkan,
        vulkan_allocator: &mut VulkanAllocator,
        render_pipeline: &RenderPipeline,
        watched_shaders: &WatchedShaders,
    ) {
        let pipeline = ComputePipeline::new(
            vulkan,
            ComputePipelineInfo::builder()
                .shader(Shader::new(
                    vulkan,
                    &watched_shaders.get_shader("post_comp").unwrap(),
                ))
                .descriptor_set_layouts(vec![&self.descriptor_set_layout])
                .push_constant_ranges(vec![vk::PushConstantRange::builder()
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .size(std::mem::size_of::<PushConstants>() as u32)
                    .build()])
                .build(),
        );
        self.pipeline = Some(pipeline);
    }

    pub fn update_system(
        vulkan: Res<Vulkan>,
        mut vulkan_allocator: ResMut<VulkanAllocator>,
        render_pipeline: Res<RenderPipeline>,
        mut post_processing: ResMut<PostProcessing>,
        watched_shaders: Res<WatchedShaders>,
    ) {
        if watched_shaders.is_dependency_signaled(&post_processing.shader_dependency_signal) {
            post_processing.refresh_pipeline(
                &*vulkan,
                &mut *vulkan_allocator,
                &*render_pipeline,
                &*watched_shaders,
            )
        }
    }
}
