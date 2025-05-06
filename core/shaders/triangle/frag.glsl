#version 300 es

precision highp float;

in vec3 vNormal;
out vec4 fragColor;

void main() {
    vec3 normal = normalize(vNormal);
    vec3 lightDir = normalize(vec3(0.0, 0.0, 1.0));
    float diff = max(dot(normal, lightDir), 0.0);

    vec3 diffuseColor = vec3(1.0, 0.5, 0.3) * diff;

    fragColor = vec4(diffuseColor, 1.0);
}
