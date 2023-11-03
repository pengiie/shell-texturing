use std::sync::Arc;

use ash::vk;
use pyrite::vulkan::{BufferInfo, UntypedBuffer, Vulkan, VulkanAllocator, VulkanStager};

// Align to 16 bytes for GLSL compatibility.
//
// From the OpenGL spec:
// "If the member is a three-component vector with components consuming N basic machine units, the base alignment is 4N."
// See https://registry.khronos.org/OpenGL/specs/gl/glspec45.core.pdf#page=159 for more info.
#[repr(align(16))]
struct GlslVec3f {
    x: f32,
    y: f32,
    z: f32,
}

pub struct Vertex {
    position: GlslVec3f,
}

pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    vertex_buffer: Arc<UntypedBuffer>,
    index_buffer: Arc<UntypedBuffer>,
}

impl Mesh {
    pub fn new(
        vulkan: &Vulkan,
        vulkan_allocator: &mut VulkanAllocator,
        vulkan_stager: &mut VulkanStager,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
    ) -> Self {
        // Create buffers.
        let vertex_buffer = Arc::new(UntypedBuffer::new(
            vulkan,
            vulkan_allocator,
            &BufferInfo::builder()
                .size((vertices.len() * std::mem::size_of::<Vertex>()) as u64)
                .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                .build(),
        ));

        let index_buffer = Arc::new(UntypedBuffer::new(
            vulkan,
            vulkan_allocator,
            &BufferInfo::builder()
                .size((indices.len() * std::mem::size_of::<u32>()) as u64)
                .usage(vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                .build(),
        ));

        // Stage them to the GPU.
        let data_ptr = vertices.as_slice().as_ptr() as *const u8;
        let data_size = vertices.len() * std::mem::size_of::<Vertex>();

        // Safety: data_ptr is a valid pointer to data_size bytes.
        unsafe {
            vulkan_stager.schedule_stage_buffer(
                vulkan,
                vulkan_allocator,
                data_ptr,
                data_size as u64,
                &vertex_buffer,
                pyrite::vulkan::StageType::Immediate,
            )
        };

        let data_ptr = indices.as_slice().as_ptr() as *const u8;
        let data_size = indices.len() * std::mem::size_of::<u32>();

        unsafe {
            vulkan_stager.schedule_stage_buffer(
                vulkan,
                vulkan_allocator,
                data_ptr,
                data_size as u64,
                &index_buffer,
                pyrite::vulkan::StageType::Immediate,
            )
        };

        Self {
            vertices,
            indices,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn vk_vertex_input_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn vk_vertex_input_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 1] {
        [vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build()]
    }

    pub fn vk_vertex_input_assembly_info() -> vk::PipelineInputAssemblyStateCreateInfo {
        vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
            .build()
    }

    pub fn vertex_buffer(&self) -> &Arc<UntypedBuffer> {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &Arc<UntypedBuffer> {
        &self.index_buffer
    }

    pub fn vertex_count(&self) -> usize {
        self.indices.len()
    }
}

pub struct MeshFactory<'a, 'b, 'c> {
    vulkan: &'a Vulkan,
    vulkan_allocator: &'b mut VulkanAllocator,
    vulkan_stager: &'c mut VulkanStager,
}

impl<'a, 'b, 'c> MeshFactory<'a, 'b, 'c> {
    pub fn factory(
        vulkan: &'a Vulkan,
        vulkan_allocator: &'b mut VulkanAllocator,
        vulkan_stager: &'c mut VulkanStager,
    ) -> Self {
        Self {
            vulkan,
            vulkan_allocator,
            vulkan_stager,
        }
    }

    /// Creates a plane on the XZ plane.
    pub fn create_plane(&mut self) -> Mesh {
        let vertices = into_vertices(vec![
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (1.0, 0.0, 1.0),
            (0.0, 0.0, 1.0),
        ]);

        let indices = vec![0, 1, 2, 2, 3, 0];

        Mesh::new(
            self.vulkan,
            self.vulkan_allocator,
            self.vulkan_stager,
            vertices,
            indices,
        )
    }

    /// Create a sphere with the given subdvisions.
    pub fn create_sphere(&mut self, subdivisions: u32) -> Mesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let mut index = 0;
        for i in 0..subdivisions {
            let i = i as f32;
            for j in 0..subdivisions {
                let j = j as f32;

                let x = (i / subdivisions as f32) * std::f32::consts::PI * 2.0;
                let y = (j / subdivisions as f32) * std::f32::consts::PI;

                let x = x.sin() * y.sin();
                let y = y.cos();
                let z = x.cos() * y.sin();

                vertices.push((x, y, z));
                indices.push(index);
                index += 1;
            }
        }

        Mesh::new(
            self.vulkan,
            self.vulkan_allocator,
            self.vulkan_stager,
            into_vertices(vertices),
            indices,
        )
    }
}

fn into_vertices(vertices: Vec<(f32, f32, f32)>) -> Vec<Vertex> {
    vertices
        .into_iter()
        .map(|(x, y, z)| Vertex {
            position: GlslVec3f { x, y, z },
        })
        .collect()
}
