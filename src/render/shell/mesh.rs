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

#[repr(align(8))]
struct GlslVec2f {
    x: f32,
    y: f32,
}

#[repr(C)]
pub struct Vertex {
    position: GlslVec3f,
    uv: GlslVec2f,
    normal: GlslVec3f,
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

    pub fn vk_vertex_input_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        [
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(std::mem::align_of::<GlslVec3f>() as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(
                    (std::mem::align_of::<GlslVec3f>() + std::mem::align_of::<GlslVec2f>()) as u32,
                )
                .build(),
        ]
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
            ((0.0, 0.0, 0.0), (0.0, 0.0), (0.0, 1.0, 0.0)),
            ((1.0, 0.0, 0.0), (1.0, 0.0), (0.0, 1.0, 0.0)),
            ((1.0, 0.0, 1.0), (1.0, 1.0), (0.0, 1.0, 0.0)),
            ((0.0, 0.0, 1.0), (0.0, 1.0), (0.0, 1.0, 0.0)),
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
    pub fn create_sphere_uv(&mut self, slices: u32, stacks: u32) -> Mesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Add top point.
        vertices.push(((0.0, 1.0, 0.0), (0.5, 0.0), (0.0, 1.0, 0.0)));
        for i in 0..stacks {
            let phi = std::f32::consts::PI * (i as f32 + 1.0) / (stacks as f32 + 1.0);
            for j in 0..slices {
                let theta = 2.0 * std::f32::consts::PI * (j as f32) / (slices as f32);
                let x = phi.sin() * theta.cos();
                let y = phi.cos();
                let z = phi.sin() * theta.sin();
                vertices.push(((x, y, z), ((j as f32) / (slices as f32), phi), (x, y, z)));
            }
        }

        // Add bottom point.
        vertices.push(((0.0, -1.0, 0.0), (0.5, 1.0), (0.0, -1.0, 0.0)));

        // Add top triangle fan.
        for i in 0..slices {
            indices.push(0);
            indices.push(i + 1);
            indices.push((i + 1) % slices + 1);
        }

        // Add middle triangle strips.
        for i in 0..stacks - 1 {
            for j in 0..slices {
                let a = i * slices + j + 1;
                let b = i * slices + (j + 1) % slices + 1;
                let c = (i + 1) * slices + (j + 1) % slices + 1;
                let d = (i + 1) * slices + j + 1;
                indices.push(a);
                indices.push(b);
                indices.push(c);
                indices.push(c);
                indices.push(d);
                indices.push(a);
            }
        }

        // Add bottom triangle fan.
        for i in 0..slices {
            indices.push(vertices.len() as u32 - 1);
            indices.push(vertices.len() as u32 - 2 - i);
            indices.push(vertices.len() as u32 - 2 - (i + 1) % slices);
        }

        Mesh::new(
            self.vulkan,
            self.vulkan_allocator,
            self.vulkan_stager,
            into_vertices(vertices),
            indices,
        )
    }

    pub fn create_sphere_icosahedron(&mut self, subdivisions: u32) -> Mesh {
        let (vertices, indices) = Self::icosahedron();

        let mut vertices = vertices;
        let mut indices = indices;

        for i in 0..subdivisions {
            let mut new_indices = Vec::new();
            for i in 0..indices.len() / 3 {
                let a = vertices[indices[i * 3] as usize];
                let b = vertices[indices[i * 3 + 1] as usize];
                let c = vertices[indices[i * 3 + 2] as usize];

                let ab = (
                    (a.0 .0 + b.0 .0) / 2.0,
                    (a.0 .1 + b.0 .1) / 2.0,
                    (a.0 .2 + b.0 .2) / 2.0,
                );
                let bc = (
                    (b.0 .0 + c.0 .0) / 2.0,
                    (b.0 .1 + c.0 .1) / 2.0,
                    (b.0 .2 + c.0 .2) / 2.0,
                );
                let ca = (
                    (c.0 .0 + a.0 .0) / 2.0,
                    (c.0 .1 + a.0 .1) / 2.0,
                    (c.0 .2 + a.0 .2) / 2.0,
                );

                let ab_uv = ((a.1 .0 + b.1 .0) / 2.0, (a.1 .1 + b.1 .1) / 2.0);
                let bc_uv = ((b.1 .0 + c.1 .0) / 2.0, (b.1 .1 + c.1 .1) / 2.0);
                let ca_uv = ((c.1 .0 + a.1 .0) / 2.0, (c.1 .1 + a.1 .1) / 2.0);

                // Project to unit sphere
                let length = (ab.0 * ab.0 + ab.1 * ab.1 + ab.2 * ab.2).sqrt();
                let ab = (ab.0 / length, ab.1 / length, ab.2 / length);
                let length = (bc.0 * bc.0 + bc.1 * bc.1 + bc.2 * bc.2).sqrt();
                let bc = (bc.0 / length, bc.1 / length, bc.2 / length);
                let length = (ca.0 * ca.0 + ca.1 * ca.1 + ca.2 * ca.2).sqrt();
                let ca = (ca.0 / length, ca.1 / length, ca.2 / length);

                vertices.push((ab, ab_uv, ab));
                vertices.push((bc, bc_uv, bc));
                vertices.push((ca, ca_uv, ca));

                let a = indices[i * 3];
                let b = indices[i * 3 + 1];
                let c = indices[i * 3 + 2];

                let ab = vertices.len() as u32 - 3;
                let bc = vertices.len() as u32 - 2;
                let ca = vertices.len() as u32 - 1;

                new_indices.push(a);
                new_indices.push(ab);
                new_indices.push(ca);

                new_indices.push(b);
                new_indices.push(bc);
                new_indices.push(ab);

                new_indices.push(c);
                new_indices.push(ca);
                new_indices.push(bc);

                new_indices.push(ab);
                new_indices.push(bc);
                new_indices.push(ca);
            }
            indices = new_indices;
            println!("Subdivision {} done, has {} indices", i, indices.len());
        }

        Mesh::new(
            self.vulkan,
            self.vulkan_allocator,
            self.vulkan_stager,
            into_vertices(vertices),
            indices,
        )
    }

    fn icosahedron() -> (
        Vec<((f32, f32, f32), (f32, f32), (f32, f32, f32))>,
        Vec<u32>,
    ) {
        fn ico(
            x: f32,
            y: f32,
            z: f32,
            u: f32,
            v: f32,
        ) -> ((f32, f32, f32), (f32, f32), (f32, f32, f32)) {
            let length = (x * x + y * y + z * z).sqrt();
            let normalized = (x / length, y / length, z / length);
            (normalized.clone(), (u, v), normalized.clone())
        }

        let t = (1.0 + 5.0f32.sqrt()) / 2.0;
        let vertices = vec![
            ico(0.0, t, -1.0, 1.0 / 11.0, 1.0),
            ico(-1.0, 0.0, -t, 0.0, 2.0 / 3.0),
            ico(1.0, 0.0, -t, 2.0 / 11.0, 2.0 / 3.0),
            ico(0.0, -t, -t, 1.0 / 11.0, 1.0 / 3.0),
            ico(t, -1.0, 0.0, 4.0 / 11.0, 1.0 / 3.0),
            ico(0.0, -t, 1.0, 2.0 / 11.0, 0.0),
            ico(t, 1.0, 0.0, 4.0 / 11.0, 2.0 / 3.0),
            ico(1.0, 0.0, t, 5.0 / 11.0, 1.0 / 3.0),
            ico(0.0, t, 1.0, 6.0 / 11.0, 2.0 / 3.0),
        ];

        // Triangles in order of the net going diagonally down.
        let indices = vec![
            0, 1, 2, 2, 1, 3, 2, 3, 4, 4, 3, 5, // First triangle strip.
            2, 6, 0, 2, 4, 6, 6, 4, 7, 7, 4, 5, // Second triangle strip.
            0, 6, 8,
        ];
        (vertices, indices)
    }
}

fn into_vertices(vertices: Vec<((f32, f32, f32), (f32, f32), (f32, f32, f32))>) -> Vec<Vertex> {
    vertices
        .into_iter()
        .map(|(position, uv, normal)| Vertex {
            position: GlslVec3f {
                x: position.0,
                y: position.1,
                z: position.2,
            },
            uv: GlslVec2f { x: uv.0, y: uv.1 },
            normal: GlslVec3f {
                x: normal.0,
                y: normal.1,
                z: normal.2,
            },
        })
        .collect()
}
