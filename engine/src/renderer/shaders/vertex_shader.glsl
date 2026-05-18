#version 460

// The triangle vertex positions.
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 color;
// The per-instance data.
layout(location = 3) in mat4x4 model_mat;
layout(location = 0) out vec3 v_normal;
layout(location = 1) out vec3 v_color;

layout(set = 0, binding = 0) uniform Camera {
    vec4 view_position;
    mat4 view_proj;
} camera;

void main() {
    vec4 world_pos = model_mat * vec4(position, 1.0);
    gl_Position = camera.view_proj * world_pos;
    v_normal = mat3(model_mat) * normal;
    v_color = color;
}
