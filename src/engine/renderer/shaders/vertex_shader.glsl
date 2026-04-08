#version 460

// The triangle vertex positions.
layout(location = 0) in vec2 position;

// The per-instance data.
layout(location = 1) in vec2 position_offset;
layout(location = 2) in float scale;

layout(set = 0, binding = 0) uniform Camera {
    mat4 view_proj;
    vec4 view_position;
} camera;
void main() {
    vec4 local_pos = vec4(position * scale + position_offset, 0.0, 1.0);
    gl_Position = camera.view_proj * local_pos;
}
