import { mat4, vec3 } from "gl-matrix";
import { createInstanceData, createModel, cubeVertices } from "./vertices";

type RawDataTypes = Float32Array | Int32Array;
type AttributeDataTypes = number[] | RawDataTypes;

export interface Attribute {
  name: string;
  data: Float32Array;
  numInstances: number;
  stride: number;
}

export interface Dimensions {
  width: number;
  height: number;
}

export interface ProgramData {
  vertexShaderSource: string;
  fragmentShaderSource: string;
  attributes: Attribute[];
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

async function setupProgram(
  gl: WebGL2RenderingContext,
  programData: ProgramData
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

function processAttribute(
  gl: WebGL2RenderingContext,
  program: WebGLProgram,
  attribute: Attribute
) {
  let rawData = attribute.data;
  const stride = attribute.stride;

  const isMatrix = attribute.stride >= 16;

  if (attribute.numInstances > 1) {
    rawData = createInstanceData(attribute.numInstances);
  }

  const baseLocation = gl.getAttribLocation(program, attribute.name);
  if (baseLocation === -1) {
    console.error(`failed to get attribute location for ${attribute.name}`);
    return;
  }
  const buffer = gl.createBuffer();
  if (!buffer) {
    console.error("failed to create buffer");
    return;
  }

  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(gl.ARRAY_BUFFER, rawData, gl.STATIC_DRAW);

  if (isMatrix) {
    // if its more than 4, its a matrix
    // for which we need to access succesive attribute location

    for (let i = 0; i < 4; i++) {
      const location = baseLocation + i;
      gl.enableVertexAttribArray(location);

      if (location === -1) {
        console.error(
          `failed to get attribute location for ${attribute.name} at index ${i}`
        );
        return;
      }

      gl.enableVertexAttribArray(location);
      gl.vertexAttribPointer(
        location,
        4,
        gl.FLOAT,
        false,
        stride * 4,
        i * stride
      );

      if (attribute.numInstances > 0) {
        gl.vertexAttribDivisor(location, 1);
      }
    }
    return;
  }

  gl.enableVertexAttribArray(baseLocation);
  gl.vertexAttribPointer(baseLocation, 3, gl.FLOAT, false, stride * 4, 0);

  if (attribute.numInstances > 1) {
    gl.vertexAttribDivisor(baseLocation, 1);
  }
}

export async function draw(
  gl: WebGL2RenderingContext,
  programData: ProgramData
) {
  gl.enable(gl.DEPTH_TEST);

  const program = await setupProgram(gl, programData);
  if (!program) {
    console.error("failed to setup program");
    return;
  }

  gl.useProgram(program);

  const vao = gl.createVertexArray();
  if (!vao) {
    console.error("failed to create vertex array object");
    return;
  }

  gl.bindVertexArray(vao);

  for (const attribute of programData.attributes) {
    processAttribute(gl, program, attribute);
  }

  let view = mat4.create();
  view = mat4.translate(view, view, [0, 0, -5]);

  const projection = mat4.create();
  mat4.perspective(
    projection,
    45,
    gl.canvas.width / gl.canvas.height,
    0.1,
    100.0
  );

  const viewLoc = gl.getUniformLocation(program, "view");
  const projectionLoc = gl.getUniformLocation(program, "projection");

  gl.uniformMatrix4fv(viewLoc, false, view);
  gl.uniformMatrix4fv(projectionLoc, false, projection);

  gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);

  gl.clearColor(0.0, 0.0, 0.0, 1.0);
  gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

  gl.drawArraysInstanced(gl.TRIANGLES, 0, 36, 2);

  requestAnimationFrame(() => draw(gl, programData));
}
