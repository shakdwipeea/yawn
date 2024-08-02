import './style.css'
import { attachCanvas, connectWorker } from "../core/connection/mainThread.ts";
import MainSceneWorker from "../examples/starterScene?worker";

document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
  <canvas id="rendering-canvas">
  </canvas>
`

const mainSceneWorker = new MainSceneWorker();
connectWorker(mainSceneWorker);
attachCanvas(mainSceneWorker, "rendering-canvas");
