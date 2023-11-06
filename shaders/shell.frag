#version 450 core

layout (location = 0) out vec4 o_color;

layout (location = 0) in vec3 pos;
layout (location = 1) in vec2 uv;
layout (location = 2) in vec3 normal;
layout (location = 3) flat in uint index;
layout (location = 4) flat in uint v_index;

layout(push_constant) uniform PushConstants {
  // Seconds since start.
  float time;
  // Planes per cm.
  uint resolution;
  // Height in cm.
  float grass_height;
} push_constants;

const float TAU = 6.28318530718;
const vec3 GRASS_COLOR = vec3(0.0, 0.5, 0.0);

const vec3 LIGHT_POS = vec3(2.3, 1.5, -0.5);
const float LIGHT_INTENSITY = 1.6;
const vec3 LIGHT_COLOR = vec3(1.0, 1, 1.0);
const vec3 UP_NORMAL = vec3(0.0, 1.0, 0.0);

const float density = 126;
const float thickness = 3;

float hash(uint n) {
	// integer hash copied from Hugo Elias
	n = (n << 13U) ^ n;
	n = n * (n * n * 15731U + 0x789221U) + 0x13763129U;
	return float(n & uint(0x7fffffffU)) / float(0x7fffffff);
}

void main() {
  vec3 color = vec3(0.77, 0.97, 0.28);
  //
  // // Calculate the position of the grass blade.
  // vec2 block_uv = floor(fract(uv + vec2(push_constants.time / 5, cos(push_constants.time/ 10))) * density) * 2 - 1;
  //
  vec2 new_uv = vec2(uv * vec2(11, 3) * density);
  vec2 local_uv = fract(new_uv) * 2 - 1;
  uvec2 tid = uvec2(new_uv);
  uint seed = (tid.x + 100) * (tid.y + 50) * 10;
  float rand = hash(seed);
  float h = float(index) / float(push_constants.resolution);
  bool outsideThickness = length(local_uv) > (thickness * (rand-h));
  if (outsideThickness && index > 0) {
    discard;
  }
  // color = vec3(local_uv, 0.0);

  // float r = hash(seed);
  // float di = 1 - (index / float(push_constants.resolution * push_constants.grass_height));
  // float height = push_constants.grass_height * r;
  //
  // vec3 color_variance = (r * 2 - 1) * vec3(0.2, 0.2, 0.3);
  // grass_color += color_variance;
  // 
  // if (r > di && index != 0) {
  //   grass_color = vec3(1.0, 0.0, 0.0);  
  //   discard;
  // }
  //
  // if (index == 0) {
  //   grass_color = vec3(0.2, 0.1, 0.07);
  // }
  //
  // // Lighting calculations.
  vec3 grass_to_light = normalize(LIGHT_POS - pos);
  //
  // float theta = max(dot(normal, grass_to_light), 0);
  float theta = dot(normal, grass_to_light) * 0.5 + 0.5;
  //
  float s = length(LIGHT_POS - pos) / (6.0 * LIGHT_INTENSITY);
  float f = 10.0;
  float attenuation =LIGHT_INTENSITY * (pow(1-s*s, 2)/(1+f*s*s));
  float ao = pow(h, 2);
  // attenuation *= pow(h);
  vec3 bd = (ao * theta * attenuation) * LIGHT_COLOR;
  o_color = vec4(color * bd, 1.0);
}
