use ash::vk;
use pyrite::{
    prelude::{AppBuilder, Assets, Res, ResMut, Resource},
    render::render_manager::{self, RenderManager},
    vulkan::{
        AttachmentInfo, CommandBuffer, GraphicsPipeline, GraphicsPipelineInfo, Image, RenderPass,
        Shader, Subpass, Vulkan, VulkanAllocator, VulkanStager,
    },
};

use self::mesh::{Mesh, MeshFactory};

use super::{
    render::RenderPipeline,
    watched_shaders::{self, WatchedShaders},
};

mod mesh;

pub fn setup_shell_renderer(app_builder: &mut AppBuilder) {
    let shell_renderer = ShellRenderer::new(
        &mut *app_builder.get_resource_mut::<Assets>(),
        &mut *app_builder.get_resource_mut::<WatchedShaders>(),
        &*app_builder.get_resource::<Vulkan>(),
        &mut *app_builder.get_resource_mut::<VulkanAllocator>(),
        &mut *app_builder.get_resource_mut::<VulkanStager>(),
    );
    app_builder.add_resource(shell_renderer);
    app_builder.add_system(ShellRenderer::update_system);
}

const VERTEX_FILE_PATH: &str = "shaders/shell.vert";
const FRAGMENT_FILE_PATH: &str = "shaders/shell.frag";
const VERTEX_NAME: &str = "shell_vert";
const FRAGMENT_NAME: &str = "shell_frag";

#[derive(Resource)]
pub struct ShellRenderer {
    shader_dependency_signal: watched_shaders::DependencySignal,
    pipeline: Option<ShellPipeline>,
    plane_mesh: Mesh,
}

struct ShellPipeline {
    graphics_pipeline: GraphicsPipeline,
}

impl ShellRenderer {
    fn new(
        assets: &mut Assets,
        watched_shaders: &mut WatchedShaders,
        vulkan: &Vulkan,
        vulkan_allocator: &mut VulkanAllocator,
        vulkan_stager: &mut VulkanStager,
    ) -> Self {
        // Load shaders and create dependency signal to them.
        let shader_dependency_signal = watched_shaders.create_dependency_signal();
        watched_shaders.load_shader(
            assets,
            VERTEX_FILE_PATH,
            VERTEX_NAME,
            &shader_dependency_signal,
        );
        watched_shaders.load_shader(
            assets,
            FRAGMENT_FILE_PATH,
            FRAGMENT_NAME,
            &shader_dependency_signal,
        );

        let plane_mesh =
            MeshFactory::factory(vulkan, vulkan_allocator, vulkan_stager).create_plane();

        Self {
            shader_dependency_signal,
            pipeline: None,
            plane_mesh,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.pipeline.is_some()
    }

    pub fn render(&self, render_manager: &mut RenderManager, render_pipeline: &RenderPipeline) {
        if let Some(pipeline) = &self.pipeline {
            let backbuffer_image = render_manager.backbuffer_image();

            let render_area = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: backbuffer_image.image_extent().width,
                    height: backbuffer_image.image_extent().height,
                },
            };

            render_manager
                .frame()
                .command_buffer()
                .dynamic_state_viewport(
                    vk::Viewport::builder()
                        .width(backbuffer_image.image_extent().width as f32)
                        .height(backbuffer_image.image_extent().height as f32)
                        .build(),
                );
            render_manager
                .frame()
                .command_buffer()
                .dynamic_state_scissor(render_area);
            render_manager
                .frame_mut()
                .command_buffer_mut()
                .bind_graphics_pipeline(&pipeline.graphics_pipeline);

            let descriptor_sets = [render_pipeline.frame(render_manager).descriptor_set()];
            render_manager
                .frame_mut()
                .command_buffer_mut()
                .bind_descriptor_sets(
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.graphics_pipeline.pipeline_layout(),
                    &descriptor_sets,
                );

            let clear_values = &[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }];

            render_manager.frame().command_buffer().begin_render_pass(
                pipeline.graphics_pipeline.render_pass(),
                render_area,
                clear_values,
            );

            render_manager
                .frame_mut()
                .command_buffer_mut()
                .bind_vertex_buffer(0, self.plane_mesh.vertex_buffer());
            render_manager
                .frame_mut()
                .command_buffer_mut()
                .bind_index_buffer(self.plane_mesh.index_buffer(), vk::IndexType::UINT32);
            render_manager.frame().command_buffer().draw_indexed(
                self.plane_mesh.vertex_count() as u32,
                8,
                0,
                0,
                0,
            );

            render_manager.frame().command_buffer().end_render_pass();
        }
    }

    fn refresh_pipeline(
        &mut self,
        vulkan: &Vulkan,
        watched_shaders: &WatchedShaders,
        render_manager: &RenderManager,
        render_pipeline: &RenderPipeline,
    ) {
        let mut subpass = Subpass::new();
        subpass.color_attachment(
            &render_manager.backbuffer_image().as_color_attachment(
                AttachmentInfo::default().load_op(vk::AttachmentLoadOp::CLEAR),
            ),
        );

        let render_pass = RenderPass::new(vulkan, &[subpass]);

        let vertex_shader = Shader::new(vulkan, &watched_shaders.get_shader(VERTEX_NAME).unwrap());
        let fragment_shader =
            Shader::new(vulkan, &watched_shaders.get_shader(FRAGMENT_NAME).unwrap());

        let vertex_input_binding_descriptions = [Mesh::vk_vertex_input_binding_description()];
        let vertex_input_attribute_descriptions = Mesh::vk_vertex_input_attribute_descriptions();
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

        let graphics_pipeline = GraphicsPipeline::new(
            vulkan,
            GraphicsPipelineInfo::builder()
                .vertex_shader(vertex_shader)
                .fragment_shader(fragment_shader)
                .vertex_input_state(
                    vk::PipelineVertexInputStateCreateInfo::builder()
                        .vertex_binding_descriptions(&vertex_input_binding_descriptions)
                        .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
                        .build(),
                )
                .input_assembly_state(Mesh::vk_vertex_input_assembly_info())
                .rasterization_state(
                    vk::PipelineRasterizationStateCreateInfo::builder()
                        .polygon_mode(vk::PolygonMode::FILL)
                        .cull_mode(vk::CullModeFlags::NONE)
                        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                        .line_width(1.0)
                        .build(),
                )
                .viewport_state(
                    vk::PipelineViewportStateCreateInfo::builder()
                        .viewports(&[])
                        .viewport_count(1)
                        .scissors(&[])
                        .scissor_count(1)
                        .build(),
                )
                .color_blend_state(
                    vk::PipelineColorBlendStateCreateInfo::builder()
                        .logic_op(vk::LogicOp::CLEAR)
                        .attachments(&[vk::PipelineColorBlendAttachmentState::builder()
                            .blend_enable(false)
                            .color_write_mask(vk::ColorComponentFlags::RGBA)
                            .build()])
                        .build(),
                )
                .dynamic_state(
                    vk::PipelineDynamicStateCreateInfo::builder()
                        .dynamic_states(&dynamic_states)
                        .build(),
                )
                .descriptor_set_layout(render_pipeline.descriptor_set_layout())
                .render_pass(render_pass)
                .build(),
        );

        self.pipeline = Some(ShellPipeline { graphics_pipeline });
    }

    fn update_system(
        mut shell_renderer: ResMut<ShellRenderer>,
        vulkan: Res<Vulkan>,
        watched_shaders: Res<WatchedShaders>,
        render_manager: Res<RenderManager>,
        render_pipeline: Res<RenderPipeline>,
    ) {
        let shell_renderer = &mut *shell_renderer;

        if watched_shaders.is_dependency_signaled(&shell_renderer.shader_dependency_signal) {
            shell_renderer.refresh_pipeline(
                &*vulkan,
                &*watched_shaders,
                &*render_manager,
                &*render_pipeline,
            );
        }
    }
}
