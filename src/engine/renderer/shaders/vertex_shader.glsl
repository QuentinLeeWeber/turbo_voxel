#version 460

// The triangle vertex positions.
layout(location = 0) in vec2 position;

// The per-instance data.
layout(location = 1) in mat4x4 model_mat;

layout(set = 0, binding = 0) uniform Camera {
    vec4 view_position;
    mat4 view_proj;
} camera;

void main() {
    vec4 local_pos = model_mat * vec4(position, 0.0, 1.0);
    gl_Position = camera.view_proj * local_pos;
}
