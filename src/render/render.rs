use ash::vk;
use pyrite::{
    desktop::RENDER_STAGE,
    prelude::*,
    render::render_manager::{FrameConfig, RenderManager},
    vulkan::{DescriptorSet, DescriptorSetLayout, DescriptorSetPool},
};

use super::{
    camera::Camera,
    shell::{setup_shell_renderer, ShellRenderer},
};

pub fn setup_render_pipeline(app_builder: &mut AppBuilder) {
    // Setup render pipeline resource.
    let render_pipeline = RenderPipeline::new(
        &*app_builder.get_resource::<Vulkan>(),
        &*app_builder.get_resource::<RenderManager>(),
    );
    app_builder.add_resource(render_pipeline);
    app_builder.add_system(RenderPipeline::update_system);
    app_builder.add_system_to_stage(RenderPipeline::render_system, RENDER_STAGE);

    // Setup shell renderer resource.
    setup_shell_renderer(app_builder);
}

#[derive(Resource)]
pub struct RenderPipeline {
    descriptor_set_pool: DescriptorSetPool,
    descriptor_set_layout: DescriptorSetLayout,
    frames: Vec<Frame>,
}

pub struct Frame {
    descriptor_set: DescriptorSet,
}

impl Frame {
    pub fn descriptor_set(&self) -> &DescriptorSet {
        &self.descriptor_set
    }
}

impl RenderPipeline {
    fn new(vulkan: &Vulkan, render_manager: &RenderManager) -> Self {
        let descriptor_set_layout = DescriptorSetLayout::new(
            vulkan,
            &[vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                p_immutable_samplers: std::ptr::null(),
            }],
        );

        let descriptor_set_pool = DescriptorSetPool::new(vulkan);
        let frames = descriptor_set_pool
            .allocate_descriptor_sets(&descriptor_set_layout, render_manager.frames_in_flight())
            .into_iter()
            .map(|descriptor_set| Frame { descriptor_set })
            .collect::<Vec<_>>();
        Self {
            descriptor_set_pool,
            descriptor_set_layout,
            frames,
        }
    }

    pub fn frame(&self, render_manager: &RenderManager) -> &Frame {
        &self.frames[render_manager.frame_index()]
    }

    pub fn frame_mut(&mut self, render_manager: &RenderManager) -> &mut Frame {
        &mut self.frames[render_manager.frame_index()]
    }

    pub fn descriptor_set_layout(&self) -> &DescriptorSetLayout {
        &self.descriptor_set_layout
    }

    fn update_system(mut render_pipeline: ResMut<RenderPipeline>, window: Res<Window>) {
        let render_pipeline = &mut *render_pipeline;
        let window = &*window;
    }

    fn render_system(
        mut render_pipeline: ResMut<RenderPipeline>,
        camera: Res<Camera>,
        mut render_manager: ResMut<RenderManager>,
        vulkan: Res<Vulkan>,
        shell_renderer: Res<ShellRenderer>,
    ) {
        let render_pipeline = &mut *render_pipeline;
        let render_manager = &mut *render_manager;

        let ready_to_render = shell_renderer.is_ready();

        // See if we are ready to render.
        if ready_to_render {
            let pipeline_frame = render_pipeline.frame_mut(render_manager);

            // Update descriptor sets
            let descriptor_set = &mut pipeline_frame.descriptor_set;
            descriptor_set
                .write()
                .set_uniform_buffer(0, &camera.camera_buffer())
                .submit_writes();

            shell_renderer.render(render_manager, render_pipeline);

            // Set the final layout of the backbuffer to the last layout.
            render_manager.set_frame_config(&FrameConfig {
                backbuffer_final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            });
        } else {
            // If not, do nothing.
            render_manager.set_frame_config(&FrameConfig {
                backbuffer_final_layout: vk::ImageLayout::UNDEFINED,
            });
        }
    }
}
