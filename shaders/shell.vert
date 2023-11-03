#version 450 core

layout(location = 0) in vec3 vertex;

layout(location = 0) out vec2 p_uv;
layout(location = 1) out vec3 p_pos;
layout(location = 2) out uint p_index;

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

void main() {
  uint plane_count = uint(push_constants.resolution * push_constants.grass_height);
  float unit_offset = (push_constants.grass_height / 100.0) / plane_count;

  float y = vertex.y + gl_InstanceIndex * unit_offset;
  vec3 pos = vec3(vertex.x * 10, y, vertex.z * 10) ;

  gl_Position = camera.proj * camera.view * vec4(pos, 1.0);
  p_uv = pos.xz;
  p_pos = pos;
  p_index = gl_InstanceIndex;
}
