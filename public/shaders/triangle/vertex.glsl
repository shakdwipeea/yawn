#version 300 es

// an attribute is an input (in) to a vertex shader.
// It will receive data from a buffer
in vec3 position;

// uniforms
uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

// all shaders have a main function
void main() {

  // gl_Position is a special variable a vertex shader
  // is responsible for setting
  gl_Position = projection * view * model * vec4(position, 1.0);
}