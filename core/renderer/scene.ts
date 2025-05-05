import { mat4, vec3 } from "gl-matrix";
import { ECS } from "../ecs/ecs";
import { createProgram, setMat4, setupVAO, updateVAO } from "./gl";
import defaultVertexSrc from "../shaders/triangle/vertex.glsl";
import defaultFragSrc from "../shaders/triangle/frag.glsl";

class Scene {
  canvas: HTMLCanvasElement;
  gl: WebGL2RenderingContext;
  ecs: ECS<Record<string | number, any>>;
  activeCamera: number;

  constructor(canvas: HTMLCanvasElement, ctx: WebGL2RenderingContext) {
    this.canvas = canvas;
    this.gl = ctx;
    this.activeCamera = -1;

    /** make ecs */
    this.ecs = new ECS();
    this.initECS();
  }

  private initECS() {
    const projection = mat4.create();
    mat4.perspective(
      projection,
      45,
      this.canvas.width / this.canvas.height,
      0.1,
      100.0,
    );

    this.ecs.addComponent("isMesh");
    this.ecs.addComponent("isCamera");
    this.ecs.addComponent("isMaterial");
    this.ecs.addComponent("program");
    this.ecs.addComponent("vao");
    this.ecs.addComponent("name", "deafult", "default");
    this.ecs.addComponent("attr_pos", [], []);
    this.ecs.addComponent("attr_normals", [], []);
    this.ecs.addComponent("u_model_matrix", mat4.create(), mat4.create());
    this.ecs.addComponent("u_projection_matrix", projection, projection);

    const defaultMaterial = this.addMaterial(
      "default-material",
      defaultVertexSrc,
      defaultFragSrc,
    );
    this.ecs.addComponent("applied_material", defaultMaterial, defaultMaterial);

    this.ecs.addSystem((data) => {
      this.render(data);

      const model = mat4.create();
      const angle = Date.now() * 0.001;
      mat4.translate(model, model, vec3.fromValues(0, 0, -5));
      mat4.rotate(model, model, angle, [0.5, 1, 0]);

      if (data["isMesh"]) {
        return {
          u_model_matrix: model,
        };
      }

      return null;
    });
  }

  runSystems() {
    this.ecs.runSystems();
  }

  private render(data: any) {
    if (this.activeCamera === -1) return;
    const camera = this.ecs.getById(this.activeCamera);

    const view = mat4.create();
    mat4.invert(view, camera["u_model_matrix"] as mat4);

    const projection = camera["u_projection_matrix"] as mat4;

    const gl = this.gl;
    gl.enable(gl.DEPTH_TEST);

    gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);

    gl.clearColor(0.0, 0.0, 0.0, 1.0);
    // gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

    if (data["isMesh"]) {
      const { program, vao } = this.ecs.getById(data["applied_material"]);

      const attrCollection = [
        {
          name: "attr_pos",
          data: data["attr_pos"],
          size: 3,
        },
        {
          name: "attr_normals",
          data: data["attr_normals"],
          size: 3,
        },
      ];

      updateVAO(gl, program as any, vao as any, attrCollection);
      gl.useProgram(program!);

      const model = data["u_model_matrix"];
      setMat4(gl, program as any, "model", model);
      setMat4(gl, program as any, "view", view);
      setMat4(gl, program as any, "projection", projection);

      gl.bindVertexArray(vao as any);
      const vertexCount = data["attr_pos"].length / 3;
      gl.drawArrays(gl.TRIANGLES, 0, vertexCount);
      console.log(vertexCount);
    }

    gl.bindVertexArray(null);
    gl.useProgram(null);
  }

  addMaterial(name: string, vs: string, fs: string) {
    const program = createProgram(this.gl, vs, fs);
    const vao = setupVAO(this.gl, program, []);
    return this.ecs.addEntity({
      name,
      isMaterial: true,
      program,
      vao,
    });
  }

  addCamera(name: string, setActive = true) {
    const uid = this.ecs.addEntity({
      name,
    });
    if (setActive) this.activeCamera = uid;
  }

  addMesh(name: string, positions: number[], normals?: number[]) {
    const numVerts = Math.floor(positions.length / 3);
    const numTris = Math.floor(numVerts / 3);
    positions.length = numTris * 9;

    if (!normals) {
      normals = [];
      const a = vec3.create();
      const b = vec3.create();
      const c = vec3.create();
      const e1 = vec3.create();
      const e2 = vec3.create();

      for (let i = 0; i < numTris; i++) {
        const base = i * 9;

        vec3.set(
          a,
          positions[base + 0],
          positions[base + 1],
          positions[base + 2],
        );
        vec3.set(
          b,
          positions[base + 3],
          positions[base + 4],
          positions[base + 5],
        );
        vec3.set(
          c,
          positions[base + 6],
          positions[base + 7],
          positions[base + 8],
        );

        vec3.subtract(e1, b, a);
        vec3.subtract(e2, c, a);
        vec3.cross(e1, e2, e1);
        vec3.normalize(e1, e1);

        normals.push(e1[0], e1[1], e1[2]);
        normals.push(e1[0], e1[1], e1[2]);
        normals.push(e1[0], e1[1], e1[2]);
      }
    }

    this.ecs.addEntity({
      name,
      attr_pos: new Float32Array(positions),
      attr_normals: new Float32Array(normals),
      isMesh: true,
    });
  }
}

export { Scene };
