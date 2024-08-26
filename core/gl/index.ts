import { mat4 } from "gl-matrix";
import { cubeVertices } from "./vertices";

type AttributeDataTypes = Float32Array | Int32Array;

export interface Attribute<T> {
  name: string;
  data: T;
}

export interface Dimensions {
  width: number;
  height: number;
}

export interface ProgramData<T extends Attribute<AttributeDataTypes>> {
  vertexShaderSource: string;
  fragmentShaderSource: string;
  attributes: T[];
}

async function fetchShader(path: string) {
  const response = await fetch(path);
  return response.text();
}

async function createShader(
  gl: WebGL2RenderingContext,
  type: number,
  sourceFile: string
) {
  const source = await fetchShader(sourceFile);

  var shader = gl.createShader(type);
  if (!shader) return;

  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  var success = gl.getShaderParameter(shader, gl.COMPILE_STATUS);
  if (!success) {
    console.error(gl.getShaderInfoLog(shader)); // eslint-disable-line
    gl.deleteShader(shader);
  }

  return shader;
}

async function setupProgram<T extends Attribute<AttributeDataTypes>>(
  gl: WebGL2RenderingContext,
  programData: ProgramData<T>
) {
  var program = gl.createProgram();
  if (!program) return;

  var vertexShader = await createShader(
    gl,
    gl.VERTEX_SHADER,
    programData.vertexShaderSource
  );
  if (!vertexShader) return;

  var fragmentShader = await createShader(
    gl,
    gl.FRAGMENT_SHADER,
    programData.fragmentShaderSource
  );
  if (!fragmentShader) return;

  gl.attachShader(program, vertexShader);
  gl.attachShader(program, fragmentShader);
  gl.linkProgram(program);

  var success = gl.getProgramParameter(program, gl.LINK_STATUS);
  if (!success) {
    console.error(gl.getProgramInfoLog(program));
    gl.deleteProgram(program);
  }

  return program;
}

function setupAttribute<T extends Attribute<AttributeDataTypes>>(
  gl: WebGL2RenderingContext,
  program: WebGLProgram,
  attribute: T
) {
  const location = gl.getAttribLocation(program, attribute.name);
  if (location === -1) return;

  // setup a buffer for the attribute data
  const buffer = gl.createBuffer();
  if (!buffer) return;

  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(gl.ARRAY_BUFFER, attribute.data, gl.STATIC_DRAW);

  const vao = gl.createVertexArray();
  if (!vao) return;

  gl.bindVertexArray(vao);

  gl.enableVertexAttribArray(location);
  gl.vertexAttribPointer(location, 3, gl.FLOAT, false, 5 * 4, 0);
}

export async function draw<T extends Attribute<AttributeDataTypes>>(
  gl: WebGL2RenderingContext,
  programData: ProgramData<T>
) {
  gl.enable(gl.DEPTH_TEST);

  const program = await setupProgram(gl, programData);
  if (!program) {
    console.error("failed to setup program");
    return;
  }

  gl.useProgram(program);

  for (const attribute of programData.attributes) {
    setupAttribute(gl, program, attribute);
  }

  let view = mat4.create();
  view = mat4.translate(view, view, [0, 0, -5]);

  let model = mat4.create();
  const angle = Date.now() * 0.001;
  model = mat4.rotate(model, model, angle, [0.5, 1, 0]);

  const projection = mat4.create();
  mat4.perspective(
    projection,
    45,
    gl.canvas.width / gl.canvas.height,
    0.1,
    100.0
  );

  const viewLoc = gl.getUniformLocation(program, "view");
  const modelLoc = gl.getUniformLocation(program, "model");
  const projectionLoc = gl.getUniformLocation(program, "projection");

  gl.uniformMatrix4fv(viewLoc, false, view);
  gl.uniformMatrix4fv(modelLoc, false, model);
  gl.uniformMatrix4fv(projectionLoc, false, projection);

  gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);

  gl.clearColor(0.0, 0.0, 0.0, 1.0);
  gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

  gl.drawArrays(gl.TRIANGLES, 0, 36);

  requestAnimationFrame(() => draw(gl, programData));
}
