# yawn

yet another webgl ngine

### Building

Run

```
npm run dev
```

- Open http://localhost:8080
- Write rust and see it in the browser

## accepted plans

- most of the logic in workers and wasm
- rust / wgpu based
- as declarative as possible
- no backwards compatibility until v1.0
- as much geometric algebra as reasonable

## planned milestone

- [done] connect events from main thread to worker
- [done] share canvas b/w worker
- [done] render triangle

## Goal: All the good algorithms

What algorithms are we planning to have,

- gpu picking?
- HZB occlusion + frustum/portal culling?
- deferred + forward lighting?
- hair/fur support?
- splines?
- good auto lod (how?)
- auto billboarding?
- selective raytracing? (maybe to bake static lighting on init?)
- Global Illumination? (how?)
- postprocesses?
- physics?
- simulations? (fluid, cloth, rigid body, wind)
- g-splats?
- volumetrics? (openvdb?)
- edge detection?
- SDFs?
- animation/bones?
- spatial audio?
- particles?
- instancing?
- procedural gen? (providing noise textures, perlin, voronoi)
- mesh edits/CSG?
- glass/refraction?
- nanites?
- caching?
- streaming mesh data? (for progressive loading/nanites/volumetrics/sectors/occlusion-optimised loading)
- server components? (websockets for mesh streaming)
- collision sounds?
- HDR/LDR rendering?
- tonemapping?
- lighting bsdf? materials?
- trails?
- vfx?
- vr/ar?
- dynamic textures?
- shadow casting?
