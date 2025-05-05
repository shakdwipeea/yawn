import { Scene } from "./scene";

class Engine {
  canvas: HTMLCanvasElement;
  ctx: WebGL2RenderingContext;
  scenes = [] as Scene[];

  constructor(canvas: string | HTMLCanvasElement) {
    /** get canvas by id */
    if (typeof canvas === "string") {
      canvas = document.getElementById(canvas)! as HTMLCanvasElement;
    }

    if (!canvas) {
      throw new Error("canvas not found");
    }
    this.canvas = canvas;
    this.ctx = canvas.getContext("webgl2")!;
    if (!this.ctx) {
      throw new Error("can't get webgl2 context");
    }
  }

  createScene() {
    const scene = new Scene(this.canvas, this.ctx);
    this.scenes.push(scene);
    return scene;
  }
}

export { Engine };
