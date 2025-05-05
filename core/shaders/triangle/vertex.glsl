#version 300 es

precision highp float;

in vec3 attr_pos;
in vec3 attr_normals;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

out vec3 vNormal;

void main() {
    mat3 normalMatrix = transpose(inverse(mat3(model)));

    vNormal = normalize(normalMatrix * attr_normals);

    gl_Position = projection * view * model * vec4(attr_pos, 1.0);
}
