import "./style.css";
import { attachCanvas, connectWorker } from "../core/connection/mainThread.ts";
import MainSceneWorker from "../examples/starterScene?worker";
import { ECS } from "../core/ecs/ecs";

const mainSceneWorker = new MainSceneWorker();
connectWorker(mainSceneWorker);
attachCanvas(mainSceneWorker, "rendering-canvas");

(window as any).ecs = new ECS();
