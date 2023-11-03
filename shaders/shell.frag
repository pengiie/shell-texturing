#version 450 core

layout (location = 0) out vec4 o_color;

layout (location = 0) in vec2 uv;
layout (location = 1) in vec3 pos;
layout (location = 2) flat in uint index;

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

const vec3 LIGHT_POS = vec3(4.3, 3.5, 5);
const float LIGHT_INTENSITY = 2.0;
const vec3 LIGHT_COLOR = vec3(1.0, 0.76, 0.09);
const vec3 UP_NORMAL = vec3(0.0, 1.0, 0.0);

// Density of grass blades per cm^2.
const float density = 20;

float hash(vec2 v) {
    v = (1./4320.) * v + vec2(0.25,0.);
    float state = fract( dot( v * v, vec2(3571)));
    return fract( state * state * (3571. * 2.));
}

void main() {
  vec3 grass_color = GRASS_COLOR;

  // Calculate the position of the grass blade.
  vec2 block_uv = floor(fract(uv + vec2(push_constants.time / 5, cos(push_constants.time/ 10))) * density) * 2 - 1;

  vec2 local_uv = fract(uv * density) * 2 - 1;
  vec2 seed = block_uv * 100.0 + vec2(12.9898, 78.233);
  float r = hash(seed);
  float di = 1 - (index / float(push_constants.resolution * push_constants.grass_height));
  float height = push_constants.grass_height * r;

  vec3 color_variance = (r * 2 - 1) * vec3(0.2, 0.2, 0.3);
  grass_color += color_variance;
  
  if (r > di && index != 0) {
    grass_color = vec3(1.0, 0.0, 0.0);  
    discard;
  }

  if (index == 0) {
    grass_color = vec3(0.2, 0.1, 0.07);
  }

  // Lighting calculations.
  vec3 grass_to_light = normalize(LIGHT_POS - pos);

  float theta = max(dot(UP_NORMAL, grass_to_light), 0.0);

  float s = length(LIGHT_POS - pos) / (6.0 * LIGHT_INTENSITY);
  float f = 10.0;
  float attenuation = LIGHT_INTENSITY * (pow(1-s*s, 2)/(1+f*s*s));
  vec3 bd = theta * attenuation * LIGHT_COLOR;
  o_color = vec4(grass_color * bd, 1.0);
}
