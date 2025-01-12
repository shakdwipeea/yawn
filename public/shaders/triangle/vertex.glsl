#version 300 es

// an attribute is an input (in) to a vertex shader.
// It will receive data from a buffer
in vec3 position;
in vec3 normal;
in vec2 texcoord_0;
in mat4 model;

// uniforms
uniform mat4 view;
uniform mat4 projection;

out vec3 v_normal;
out vec2 v_texcoord_0;

// all shaders have a main function
void main() {
  v_normal = normal;
  v_texcoord_0 = texcoord_0;

  // gl_Position is a special variable a vertex shader
  // is responsible for setting
  gl_Position = projection * view * model * vec4(position, 1.0);
}
