use std::sync::Arc;

use ash::vk;
use na::{Matrix4, Perspective3, Rotation3, Vector3};
use pyrite::{
    desktop::window::{CursorGrabMode, Window},
    prelude::{AppBuilder, Input, Key, Res, ResMut, Resource, Swapchain, Time},
    vulkan::{Buffer, BufferInfo, StageType, UntypedBuffer, Vulkan, VulkanAllocator, VulkanStager},
};

extern crate nalgebra as na;

const WALKING_SPEED: f32 = 1.42;
const RUNNING_SPEED: f32 = 3.0;

#[derive(Resource)]
pub struct Camera {
    position: Vector3<f32>,
    rx: f32,
    ry: f32,
    speed: f32,
    cursor_locked: bool,

    buffer: Arc<UntypedBuffer>,
    data: CameraBufferData,
}

struct CameraBufferData {
    projection: Matrix4<f32>,
    view: Matrix4<f32>,
}

impl Camera {
    pub fn new(
        vulkan: &Vulkan,
        vulkan_allocator: &mut VulkanAllocator,
        window: &mut Window,
    ) -> Self {
        let buffer = UntypedBuffer::new(
            vulkan,
            vulkan_allocator,
            &BufferInfo::builder()
                .size(std::mem::size_of::<CameraBufferData>() as u64)
                .usage(vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                .build(),
        );
        window.set_cursor_grab_mode(CursorGrabMode::None);
        window.set_cursor_visible(true);
        Self {
            position: Vector3::new(0.0, 5.0, 0.0),
            rx: 0.0,
            ry: 0.0,
            speed: WALKING_SPEED,
            cursor_locked: false,
            data: CameraBufferData {
                projection: Matrix4::identity(),
                view: Matrix4::identity(),
            },
            buffer: Arc::new(buffer),
        }
    }

    fn calculate_projection(&mut self, width: u32, height: u32, fov: f32, near: f32, far: f32) {
        self.data.projection = Perspective3::new((width as f32) / (height as f32), fov, near, far)
            .as_matrix()
            .to_owned();
        self.data.projection.m22 *= -1.0;
        self.data.projection.m33 *= -1.0;
        self.data.projection.m43 *= -1.0;
    }

    fn calculate_view(&mut self) {
        let inverted_position = -self.position;
        let inverted_rotation = Rotation3::from_euler_angles(-self.ry, 0.0, 0.0)
            * Rotation3::from_euler_angles(0.0, -self.rx, 0.0);
        self.data.view =
            inverted_rotation.to_homogeneous() * Matrix4::new_translation(&inverted_position);
    }

    pub fn update(
        input: Res<Input>,
        time: Res<Time>,
        vulkan: Res<Vulkan>,
        mut window: ResMut<Window>,
        mut vulkan_allocator: ResMut<VulkanAllocator>,
        mut camera: ResMut<Camera>,
        mut stager: ResMut<VulkanStager>,
    ) {
        // Calculate rotation if the cursor is locked.
        if camera.cursor_locked {
            let (mdx, mdy) = input.mouse_delta();
            camera.rx += mdx as f32 * 0.02;
            camera.ry += mdy as f32 * 0.02;
        }

        // Calculate translation
        let mut translation = Vector3::new(0.0, 0.0, 0.0);
        let mut speed = camera.speed;
        if input.is_key_down(Key::W) {
            translation.z = 1.0;
        }
        if input.is_key_down(Key::S) {
            translation.z = -1.0;
        }
        if input.is_key_down(Key::A) {
            translation.x = -1.0;
        }
        if input.is_key_down(Key::D) {
            translation.x = 1.0;
        }
        if input.is_key_down(Key::LShift) {
            translation.y = -1.0;
        }
        if input.is_key_down(Key::Space) {
            translation.y = 1.0;
        }
        if input.is_key_down(Key::LControl) {
            speed = RUNNING_SPEED;
        }
        let translation = translation.normalize() * (speed * time.delta().as_secs_f32());

        if translation.magnitude() > 0.0 {
            let translation = Rotation3::from_euler_angles(0.0, camera.rx, 0.0).to_homogeneous()
                * translation.to_homogeneous();
            camera.position += translation.xyz();
        }

        // Toggle cursor lock
        if input.is_key_pressed(Key::E) {
            camera.cursor_locked = !camera.cursor_locked;
            window.set_cursor_grab_mode(if camera.cursor_locked {
                CursorGrabMode::Confined
            } else {
                CursorGrabMode::None
            });
            window.set_cursor_visible(!camera.cursor_locked);
        }

        // Update camera matrix data and upload to GPU
        camera.calculate_projection(
            window.width(),
            window.height(),
            90.0f32.to_radians(),
            0.1,
            1000.0,
        );
        camera.calculate_view();

        let mut data = camera.data.projection.as_slice().to_owned();
        data.append(&mut camera.data.view.as_slice().to_owned());
        let data_slice = data.as_slice();

        unsafe {
            stager.schedule_stage_buffer(
                &*vulkan,
                &mut *vulkan_allocator,
                data_slice.as_ptr() as *const u8,
                (data.len() * std::mem::size_of::<f32>()) as u64,
                &camera.buffer,
                StageType::Immediate,
            );
        }
    }

    pub fn camera_buffer(&self) -> &Arc<UntypedBuffer> {
        &self.buffer
    }
}

pub fn setup_camera_preset(app_builder: &mut AppBuilder) {
    let camera = Camera::new(
        &*app_builder.get_resource::<Vulkan>(),
        &mut *app_builder.get_resource_mut::<VulkanAllocator>(),
        &mut *app_builder.get_resource_mut::<Window>(),
    );
    app_builder.add_resource(camera);
    app_builder.add_system(Camera::update);
}
