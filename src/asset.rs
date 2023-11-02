use pyrite::{asset::loaders::spirv::SpirVLoader, prelude::AppBuilder};

pub fn setup_asset_loaders(app_builder: &mut AppBuilder) {
    let mut assets = app_builder.get_resource_mut::<pyrite::asset::Assets>();
    assets.add_loader::<SpirVLoader>();
}
