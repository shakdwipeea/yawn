import { Attribute, draw, ProgramData } from "../gl";
import { cubeVertices } from "../gl/vertices";
import { MessageType } from "./mainThread";

const handleConnection = (msg: MessageEvent<any>) => {
  const { data } = msg;

  if (!(data instanceof Array)) return;
  if (!data.length) return;

  switch (data[0]) {
    case MessageType.attachCanvas:
      const canvas = data[1];
      const dimensions = data[2];

      const ctxWorker = canvas.getContext("webgl2");
      var p: ProgramData<Attribute<Float32Array>> = {
        vertexShaderSource: "/shaders/triangle/vertex.glsl",
        fragmentShaderSource: "/shaders/triangle/frag.glsl",
        attributes: [
          {
            name: "position",
            data: new Float32Array(cubeVertices),
          },
        ],
      };

      requestAnimationFrame(() => draw(ctxWorker, p));

      break;
  }

  console.log(...data);
};

export { handleConnection };
