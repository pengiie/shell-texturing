use pyrite::{
    prelude::*,
    render::render_manager::{setup_render_manager, RenderManagerConfig},
};

use self::{
    post::setup_post_processing, render::setup_render_pipeline,
    watched_shaders::setup_watched_shaders,
};

pub mod camera;
pub mod post;
pub mod render;
pub mod shell;
pub mod watched_shaders;

pub fn setup_render_preset(app_builder: &mut AppBuilder) {
    setup_render_manager(
        app_builder,
        &RenderManagerConfig::builder().frames_in_flight(2).build(),
    );

    setup_watched_shaders(app_builder);
    setup_render_pipeline(app_builder);
    setup_post_processing(app_builder);
}
