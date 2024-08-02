# yawn
yet another webgl ngine

## accepted plans
- most of the logic in service workers
- ts / webgl based
- as declarative as possible
- no backwards compatibility until v1.0
- as much geometric algebra as reasonable

## planned milestone

- connect events from main thread to service worker
- share canvas b/w sw
- render triangle

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

## are we gon use ogl or start from scratch?

## we need to decide architecture
- ECS?
- babylon-like
- threejs-like

- message passing?
- observables?
- events?
- runnable on a server?
- worker/main thread adaptable? or just worker?