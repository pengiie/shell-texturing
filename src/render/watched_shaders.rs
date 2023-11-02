use std::collections::{HashMap, HashSet};

use pyrite::{
    asset::WatchedHandle,
    prelude::{AppBuilder, Assets, ResMut, Resource},
};
use uuid::Uuid;

pub fn setup_watched_shaders(app_builder: &mut AppBuilder) {
    app_builder.add_resource(WatchedShaders::new());
    app_builder.add_system(WatchedShaders::update_system);
}

#[derive(Resource)]
pub struct WatchedShaders {
    // The shaders with the key being the name, and the value being the handle to the shader.
    shaders: HashMap<String, WatchedHandle<Vec<u32>>>,
    shaders_loaded: HashSet<String>,

    // The key is the dependency signal, the value is the list of shaders that it depends on.
    dependency_signals: HashMap<DependencySignal, Vec<String>>,
    dirty_dependency_signals: HashSet<DependencySignal>,
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct DependencySignal(Uuid);

impl WatchedShaders {
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
            shaders_loaded: HashSet::new(),
            dependency_signals: HashMap::new(),
            dirty_dependency_signals: HashSet::new(),
        }
    }

    pub fn create_dependency_signal(&mut self) -> DependencySignal {
        let dependency_signal = DependencySignal(Uuid::new_v4());
        self.dependency_signals
            .insert(dependency_signal.clone(), Vec::new());
        dependency_signal
    }

    pub fn load_shader(
        &mut self,
        assets: &mut Assets,
        file_path: impl ToString,
        name: impl ToString,
        dependency_signal: &DependencySignal,
    ) {
        let watched_handle = assets.load::<Vec<u32>>(file_path).into_watched();
        self.shaders.insert(name.to_string(), watched_handle);
        self.dependency_signals
            .get_mut(dependency_signal)
            .unwrap()
            .push(name.to_string());
    }

    pub fn is_dependency_signaled(&self, dependency_signal: &DependencySignal) -> bool {
        self.dirty_dependency_signals.contains(dependency_signal)
    }

    pub fn get_shader(&self, name: impl ToString) -> Option<Vec<u32>> {
        self.shaders
            .get(&name.to_string())
            .map(|watched_handle| watched_handle.get().unwrap().clone())
    }

    pub fn update_system(mut watched_shaders: ResMut<WatchedShaders>, mut assets: ResMut<Assets>) {
        let watched_shaders = &mut *watched_shaders;
        watched_shaders.dirty_dependency_signals.clear();
        for (name, shader_handle) in &mut watched_shaders.shaders {
            let new_loaded =
                shader_handle.is_loaded() && !watched_shaders.shaders_loaded.contains(name);
            if new_loaded {
                watched_shaders.shaders_loaded.insert(name.clone());
            }

            // Signal if the shader has been updated (file was modified) or just loaded.
            if shader_handle.update(&mut *assets) || new_loaded {
                if !shader_handle.is_error() {
                    // Looks at what dependency signals this shader is a part of, and adds them to the
                    // dirty dependency signals list.
                    watched_shaders.dirty_dependency_signals.extend(
                        watched_shaders
                            .dependency_signals
                            .iter()
                            .filter(|(_, names)| names.contains(name))
                            .map(|(dependency_signal, _)| dependency_signal.clone()),
                    );
                } else {
                    println!(
                        "Shader {} failed to load. Error: {}",
                        name,
                        shader_handle.get_error().unwrap()
                    );
                }
            }
        }
    }
}
