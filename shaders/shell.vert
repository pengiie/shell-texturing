#version 450 core

layout(location = 0) in vec3 vertex;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec3 normal;

layout(location = 0) out vec3 p_position;
layout(location = 1) out vec2 p_uv;
layout(location = 2) out vec3 p_normal;
layout (location = 3) out uint p_index;
layout (location = 4) out uint p_v_index;

layout(set = 0, binding = 0) uniform CameraUniform {
  mat4 proj;
  mat4 view;
} camera;

layout(push_constant) uniform PushConstants {
  // Seconds since start.
  float time;
  // Planes per cm.
  uint resolution;
  // Height in cm.
  float grass_height;
} push_constants;

const float SHELL_LENGTH = 0.5;
const float CURVATURE = 0.88;

void main() {
  vec3 position = vertex;
  // position = ICOSAHEDRON_VERTICES[gl_VertexIndex];
  float h = float(gl_InstanceIndex) / push_constants.resolution;

  position += sqrt(h) * push_constants.grass_height * position;
  position.y -= (1 - CURVATURE) * pow(h, 1.5);

  gl_Position = camera.proj * camera.view * vec4(position, 1.0);

  p_position = position;
  // p_uv = ICOSAHEDRON_UVS[gl_VertexIndex];
  p_uv = uv;
  p_normal = normal;
  p_index = gl_InstanceIndex;
  p_v_index = gl_VertexIndex;
}
