#version 450

layout(push_constant) uniform Constants {
    mat4 proj;
    mat4 view;
};

layout(binding = 0) uniform InstanceData {
    mat4 model;
} instance;

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;

layout(location = 0) out vec3 outNormal;

void main()
{
    gl_Position = proj * view * instance.model * vec4(position, 1.0);
    outNormal = normal;
}
