import { mat4, vec3 } from "gl-matrix";
import { createInstanceData, createModel, cubeVertices } from "./vertices";
import { load } from "@loaders.gl/core";
import { GLTFLoader } from "@loaders.gl/gltf";

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
  model: string;
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
  console.log("Processing instance attribute:", {
    name: attribute.name,
    data: attribute.data,
    stride: attribute.stride,
    location: gl.getAttribLocation(program, attribute.name),
  });

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
  gl.vertexAttribPointer(baseLocation, stride, gl.FLOAT, false, stride * 4, 0);

  if (attribute.numInstances > 1) {
    gl.vertexAttribDivisor(baseLocation, 1);
  }
}
const attributeNameMap: { [key: string]: string } = {
  POSITION: "position",
  NORMAL: "normal",
  TEXCOORD_0: "texcoord_0",
};
export async function loadModel(
  gl: WebGL2RenderingContext,
  program: WebGLProgram,
  path: string
) {
  // In loadModel, add this at the start:
  console.log(
    "Program info:",
    gl.getProgramParameter(program, gl.ACTIVE_ATTRIBUTES),
    "active attributes"
  );

  for (
    let i = 0;
    i < gl.getProgramParameter(program, gl.ACTIVE_ATTRIBUTES);
    i++
  ) {
    const info = gl.getActiveAttrib(program, i);
    console.log("Attribute:", info?.name, "type:", info?.type);
  }

  const glbData = await load(path, GLTFLoader);
  console.log("loaded model is", glbData);

  const gltfData = glbData.json;
  if (!gltfData.meshes || !gltfData.bufferViews || !gltfData.accessors) return;

  const bufferViews = gltfData.bufferViews;
  const accessors = gltfData.accessors;

  const arrBuffer = glbData.buffers[0];

  const dataBuffer = arrBuffer.arrayBuffer.slice(
    arrBuffer.byteOffset,
    arrBuffer.byteOffset + arrBuffer.byteLength
  );

  const attributes: Attribute[] = [];
  let indexCount = 0;

  gltfData.meshes.forEach((mesh) => {
    mesh.primitives.forEach((primitive) => {
      const attributeOrder = ["POSITION", "NORMAL", "TEXCOORD_0"];

      for (const attr of attributeOrder) {
        const accessorIndex = primitive.attributes[attr];
        console.log("buffer index is", accessorIndex);

        const accessor = accessors[accessorIndex];
        const bufferIndex = accessor.bufferView;

        if (bufferIndex === undefined) {
          console.warn(
            `no buffer index found for accessor ${accessorIndex} in 
            accessor`,
            accessor
          );
          continue;
        }

        const bufferView = bufferViews[bufferIndex];
        const offset = bufferView.byteOffset ?? 0;

        let componentsPerVertex = 0;
        switch (accessor.type) {
          case "SCALAR":
            componentsPerVertex = 1;
            break;
          case "VEC2":
            componentsPerVertex = 2;
            break;
          case "VEC3":
            componentsPerVertex = 3;
            break;
          case "VEC4":
            componentsPerVertex = 4;
            break;
          default:
            console.warn("unknown type", accessor.type);
            continue;
        }

        // const attributeData: Attribute = {
        //   name: attr.toLowerCase(),
        //   data: new Float32Array(
        //     glbData.buffers[bufferView.buffer].arrayBuffer.slice(
        //       offset,
        //       offset + bufferView.byteLength
        //     )
        //   ),
        //   numInstances: 0,
        //   stride,
        // };

        const buffer = gl.createBuffer();
        if (!buffer) {
          console.error("failed to create buffer");
          return;
        }
        const length = accessor.count * (attr === "TEXCOORD_0" ? 2 : 3);

        // Create a properly sized view into the buffer
        const data = new Float32Array(
          dataBuffer,
          offset,
          length // Explicitly set the length based on accessor count and components
        );

        console.log(`${attr} data:`, {
          first: Array.from(data.slice(0, 3)),
          count: accessor.count,
          components: attr === "TEXCOORD_0" ? 2 : 3,
        });

        gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
        gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);

        const attributeName = attributeNameMap[attr] || attr.toLowerCase();
        const location = gl.getAttribLocation(program, attributeName);
        if (location === -1) {
          console.error(`failed to get attribute location for ${attr}`);
          continue;
        }

        gl.enableVertexAttribArray(location);
        gl.vertexAttribPointer(
          location,
          componentsPerVertex,
          gl.FLOAT,
          false,
          0,
          0
        );
      }

      if (primitive.indices == undefined) return;

      const accessor = accessors[primitive.indices];
      const bufferIndex = accessor.bufferView;

      if (bufferIndex === undefined) {
        console.warn(`no buffer index found for indices`, accessor);
        return;
      }

      const offset = bufferViews[bufferIndex].byteOffset;
      if (offset === undefined) {
        console.warn(`no offset found for indices`, accessor);
        return;
      }

      const indexBuffer = gl.createBuffer();
      if (!indexBuffer) {
        console.error("failed to create index buffer");
        return;
      }
      // Create the index buffer directly from the arraybuffer
      const indexData = new Uint16Array(dataBuffer, offset, accessor.count);

      console.log("Index data:", {
        first: Array.from(indexData.slice(0, 6)),
        count: accessor.count,
      });

      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, indexBuffer);
      gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indexData, gl.STATIC_DRAW);
      console.log("Index data:", indexData.slice(0, 12)); // Show first few indices
      indexCount = accessor.count;
    });
  });

  return indexCount;
}

export async function render(
  gl: WebGL2RenderingContext,
  program: WebGLProgram,
  programData: ProgramData
) {
  for (const attribute of programData.attributes) {
    processAttribute(gl, program, attribute);
  }

  gl.clearColor(0, 0, 0, 1.0); // Dark greenish background
  gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

  gl.drawElementsInstanced(gl.TRIANGLES, 36, gl.UNSIGNED_SHORT, 0, 3);

  const err = gl.getError();
  console.log("error is", err);

  requestAnimationFrame(() => render(gl, program, programData));
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

  const count = await loadModel(gl, program, programData.model);
  if (!count) {
    console.error("failed to load model");
    return;
  }

  let view = mat4.create();
  view = mat4.translate(view, view, [0, 0, -15]);

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

  render(gl, program, programData);
}
