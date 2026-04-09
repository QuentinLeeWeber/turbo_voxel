#version 460
layout(location = 0) in vec3 v_normal;
layout(location = 1) in vec3 v_color;

layout(location = 0) out vec4 f_color;

void main() {
    vec3 light_dir = normalize(vec3(-1.0, 0.0, 1.0));
    vec3 n = normalize(v_normal);
    float intensity = max(dot(n, light_dir), 0.0);

    vec3 ambient = vec3(0.1, 0.1, 0.1);

    f_color = vec4(ambient + v_color * intensity, 1.0);
}
