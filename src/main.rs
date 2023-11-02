use asset::setup_asset_loaders;
use pyrite::desktop::window::WindowState;
use pyrite::prelude::*;
use render::camera::setup_camera_preset;
use render::setup_render_preset;

mod asset;
mod render;

const APP_NAME: &str = "The Furry Game";

fn main() {
    let mut app_builder = AppBuilder::new();

    // Sets up the pyrite_desktop preset.
    setup_desktop_preset(
        &mut app_builder,
        DesktopConfig {
            application_name: APP_NAME.to_string(),
            window_config: WindowConfig {
                state: WindowState::Windowed(1280, 720),
                title: APP_NAME.to_string(),
            },
            ..Default::default()
        },
    );

    // Setup assets.
    setup_asset_loaders(&mut app_builder);

    // Setup rendering.
    setup_camera_preset(&mut app_builder);
    setup_render_preset(&mut app_builder);

    app_builder.run();
}
