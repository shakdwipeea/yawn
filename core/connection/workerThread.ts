import { Engine } from "../renderer/engine";
import { MessageType } from "./mainThread";

const cubeVertices = [
  1, -1, 1, -1, -1, 1, -1, 1, 1, 1, -1, 1, -1, 1, 1, 1, 1, 1, 1, 1, -1, -1, 1,
  -1, -1, -1, -1, 1, 1, -1, -1, -1, -1, 1, -1, -1, 1, 1, -1, 1, -1, -1, 1, -1,
  1, 1, 1, -1, 1, -1, 1, 1, 1, 1, -1, 1, 1, -1, -1, 1, -1, -1, -1, -1, 1, 1, -1,
  -1, -1, -1, 1, -1, -1, 1, 1, -1, 1, -1, 1, 1, -1, -1, 1, 1, 1, 1, -1, 1, 1, 1,
  1, -1, 1, 1, -1, -1, -1, -1, -1, 1, -1, 1, -1, -1, -1, -1, -1, 1,
];

const handleConnection = (msg: MessageEvent<any>) => {
  const { data } = msg;

  if (!(data instanceof Array)) return;
  if (!data.length) return;

  switch (data[0]) {
    case MessageType.attachCanvas:
      const canvas = data[1];

      const engine = new Engine(canvas);
      const scene = engine.createScene();
      scene.addMesh("box", cubeVertices);
      scene.addCamera("cam", true);

      const raf = () => {
        requestAnimationFrame(() => {
          scene.runSystems();
          raf();
        });
      };

      raf();

      break;
  }

  console.log(...data);
};

export { handleConnection };
