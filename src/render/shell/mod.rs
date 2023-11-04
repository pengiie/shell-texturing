use std::{any::Any, sync::Arc};

use ash::vk;
use pyrite::{
    prelude::{AppBuilder, Assets, Input, Key, Res, ResMut, Resource, Time},
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
    resolution: u32,
    shell_thickness: f32,
}

struct ShellPipeline {
    graphics_pipeline: GraphicsPipeline,
}

struct ShellPushConstants {
    // The current time in seconds since the start of the session.
    time: f32,
    // Planes per cm.
    resolution: u32,
    // The height of the grass in cm.
    grass_height: f32,
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

        let plane_mesh = MeshFactory::factory(vulkan, vulkan_allocator, vulkan_stager)
            .create_sphere_icosahedron(3);

        Self {
            shader_dependency_signal,
            pipeline: None,
            plane_mesh,
            resolution: 256,
            shell_thickness: 0.35,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.pipeline.is_some()
    }

    pub fn render(
        &self,
        render_manager: &mut RenderManager,
        render_pipeline: &RenderPipeline,
        current_time: f32,
    ) -> Vec<Arc<dyn Any + Send + Sync>> {
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
                        .min_depth(0.0)
                        .max_depth(1.0)
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

            let clear_values = &[
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.568, 0.8, 0.85, 1.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            render_manager.frame().command_buffer().begin_render_pass(
                pipeline.graphics_pipeline.render_pass(),
                render_area,
                clear_values,
            );

            render_manager
                .frame()
                .command_buffer()
                .write_push_constants_typed(
                    pipeline.graphics_pipeline.pipeline_layout(),
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    &ShellPushConstants {
                        time: current_time,
                        resolution: self.resolution,
                        grass_height: self.shell_thickness,
                    },
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
                // f32::floor(self.grass_height * self.resolution as f32) as u32,
                self.resolution,
                0,
                0,
                0,
            );

            render_manager.frame().command_buffer().end_render_pass();

            return vec![
                self.plane_mesh.vertex_buffer().clone(),
                self.plane_mesh.index_buffer().clone(),
            ];
        }

        vec![]
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
            &render_manager
                .backbuffer_image()
                .as_attachment(AttachmentInfo::default().load_op(vk::AttachmentLoadOp::CLEAR)),
        );
        subpass.depth_attachment(
            &render_pipeline.backbuffer_depth_image().as_attachment(
                AttachmentInfo::default()
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .is_depth(true),
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
                .depth_stencil_state(
                    vk::PipelineDepthStencilStateCreateInfo::builder()
                        .depth_test_enable(true)
                        .depth_write_enable(true)
                        .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
                        .build(),
                )
                .dynamic_state(
                    vk::PipelineDynamicStateCreateInfo::builder()
                        .dynamic_states(&dynamic_states)
                        .build(),
                )
                .descriptor_set_layout(render_pipeline.descriptor_set_layout())
                .push_constant_ranges(vec![vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    offset: 0,
                    size: std::mem::size_of::<ShellPushConstants>() as u32,
                }])
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
        input: Res<Input>,
        time: Res<Time>,
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

        // Edit resolution.
        let mut modified = false;
        if input.is_key_repeat(Key::H) || input.is_key_pressed(Key::H) {
            shell_renderer.resolution = (shell_renderer.resolution as i32 - 1).max(1) as u32;
            modified = true;
        }
        if input.is_key_repeat(Key::L) || input.is_key_pressed(Key::L) {
            shell_renderer.resolution += 1;
            modified = true;
        }
        if input.is_key_repeat(Key::J) || input.is_key_pressed(Key::J) {
            shell_renderer.shell_thickness = (shell_renderer.shell_thickness - 0.1).max(0.05);
            modified = true;
        }
        if input.is_key_repeat(Key::K) || input.is_key_pressed(Key::K) {
            shell_renderer.shell_thickness += 0.02;
            modified = true;
        }

        if modified {
            println!("Resolution: {}", shell_renderer.resolution);
            println!("Grass height: {}", shell_renderer.shell_thickness);
            println!(
                "Plane count: {}",
                f32::floor(shell_renderer.shell_thickness * shell_renderer.resolution as f32)
                    as u32
            );
        }
    }
}
