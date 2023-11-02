#version 450 core

layout(location = 0) in vec3 vertex;

layout(location = 0) out vec3 p_position;

layout(set = 0, binding = 0) uniform CameraUniform {
  mat4 proj;
  mat4 view;
} u_camera;

void main() {
  float y = vertex.y + gl_InstanceIndex * 3.0;
  gl_Position = u_camera.proj * u_camera.view * vec4(vertex * 10, 1.0);
  p_position = vertex;
}
