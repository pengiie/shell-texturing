#version 450 core

layout (location = 0) out vec4 o_color;

layout (location = 0) in vec3 p_position;

void main() {
    o_color = vec4(sin(p_position.x * 3.14* 500) * 2 - 1, cos(p_position.z * 3.14 * 1000) * 2 - 1, 0.5, 1.0);
}
