import { mat4 } from "gl-matrix";

export const compileShader = (
  gl: WebGL2RenderingContext,
  type: number,
  source: string,
) => {
  const shader = gl.createShader(type);
  if (!shader) {
    throw new Error(`couldn't make the shader ${source}`);
  }

  gl.shaderSource(shader, source);
  gl.compileShader(shader);

  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(
      `Error in ${source}: ${gl.getShaderInfoLog(shader) ?? "no error returned"}`,
    );
  }

  return shader;
};

export const createProgram = (
  gl: WebGL2RenderingContext,
  vs: string,
  fs: string,
): WebGLProgram => {
  const program = gl.createProgram()!;

  const vShader = compileShader(gl, gl.VERTEX_SHADER, vs);
  const fShader = compileShader(gl, gl.FRAGMENT_SHADER, fs);

  gl.attachShader(program, vShader);
  gl.attachShader(program, fShader);

  gl.linkProgram(program);

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program) || "");
  }

  return program;
};

export type AttributeCollection = {
  name: string;
  data: Float32Array;
  size: number;
};

export const setupVAO = (
  gl: WebGL2RenderingContext,
  program: WebGLProgram,
  attributeCollections: AttributeCollection[],
) => {
  const vao = gl.createVertexArray()!;
  gl.bindVertexArray(vao);

  for (const attr of attributeCollections) {
    const loc = gl.getAttribLocation(program, attr.name);
    if (loc >= 0) {
      const buf = gl.createBuffer()!;
      gl.bindBuffer(gl.ARRAY_BUFFER, buf);
      gl.bufferData(gl.ARRAY_BUFFER, attr.data, gl.STATIC_DRAW);
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, attr.size, gl.FLOAT, false, 0, 0);
    }
  }

  gl.bindVertexArray(null);
  return vao;
};

export const updateVAO = (
  gl: WebGL2RenderingContext,
  program: WebGLProgram,
  vao: WebGLVertexArrayObject,
  attributeCollections: AttributeCollection[],
) => {
  gl.bindVertexArray(vao);

  for (const attr of attributeCollections) {
    const loc = gl.getAttribLocation(program, attr.name);
    if (loc >= 0) {
      const buf = gl.createBuffer()!;
      gl.bindBuffer(gl.ARRAY_BUFFER, buf);
      gl.bufferData(gl.ARRAY_BUFFER, attr.data, gl.STATIC_DRAW);
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, attr.size, gl.FLOAT, false, 0, 0);
    }
  }

  gl.bindVertexArray(null);
  return vao;
};

export const setMat4 = (
  gl: WebGL2RenderingContext,
  program: WebGLProgram,
  name: string,
  mat: mat4,
) => {
  const loc = gl.getUniformLocation(program, name);
  if (loc) gl.uniformMatrix4fv(loc, false, mat);
};
