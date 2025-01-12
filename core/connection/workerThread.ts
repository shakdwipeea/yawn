import { Attribute, draw, ProgramData } from "../gl";
import { createInstanceData, createModel, cubeVertices } from "../gl/vertices";
import { MessageType } from "./mainThread";

const handleConnection = (msg: MessageEvent<any>) => {
  const { data } = msg;

  if (!(data instanceof Array)) return;
  if (!data.length) return;

  switch (data[0]) {
    case MessageType.attachCanvas:
      const canvas = data[1];

      // Add canvas sizing
      // canvas.width = canvas.clientWidth;
      // canvas.height = canvas.clientHeight;

      const ctxWorker = canvas.getContext("webgl2");
      if (!ctxWorker) {
        console.error("Failed to get WebGL2 context");
        return;
      }

      var p: ProgramData = {
        vertexShaderSource: "/shaders/triangle/vertex.glsl",
        fragmentShaderSource: "/shaders/triangle/frag.glsl",
        attributes: [
          {
            name: "model",
            data: new Float32Array(createModel(0)),
            numInstances: 3,
            stride: 16,
          },
        ],
        model: "/models/cube.glb",
      };

      draw(ctxWorker, p); // Remove requestAnimationFrame for initial testing
      break;
  }

  console.log(...data);
};

export { handleConnection };
