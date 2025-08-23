# AGENTS.md - Working with the Yawn WebGPU Renderer

This document provides guidance for AI agents working with this WebGPU/WASM rendering codebase.

## Project Overview

This is a Rust WebGPU renderer that compiles to WebAssembly and runs in the browser. It renders GLTF models using a worker-based architecture.

## Key Commands

- **Build**: `npm run build` (builds WASM + bundles with Vite)
- **Development**: `npm run dev` (live reload for development)
- **Check Rust**: `cargo check` (verify Rust code compiles)
- **Test**: Serve `dist/` folder on HTTP server and open in browser

## Project Structure

```
src/
├── lib.rs              # Main entry point, worker setup
├── renderer/
│   └── mod.rs         # Main renderer with WebGPU pipeline
├── gltf.rs            # GLTF model loading and parsing
├── gltf.wgsl          # WGSL shader for GLTF rendering
├── example.wgsl       # Simple triangle shader
└── message.rs         # Worker communication types

static/               # Static assets (GLTF models, textures)
dist/                # Built output (WASM + JS bundles)
```

## Common Issues & Solutions

### 1. GLTF Rendering Problems
- **Symptoms**: Black screen, wrong vertex counts, empty vertex buffers
- **Common causes**:
  - Index format mismatch (U16 vs U32)
  - Wrong attribute semantic mapping
  - Single buffer slicing instead of separate buffers
- **Debug approach**: Check browser console for logs, inspect WebGPU frame capture

### 2. WebGPU Pipeline Issues  
- **Symptoms**: Compilation errors, render pipeline failures
- **Check**: Shader syntax, vertex buffer layouts, bind group layouts
- **Key files**: `*.wgsl` shaders, pipeline creation in `gltf.rs`

### 3. WASM Build Failures
- **Common**: Missing atomics features, linker errors
- **Solution**: Ensure proper RUSTFLAGS in build scripts
- **Command**: `npm run build` handles all flags correctly

## Development Workflow

1. **Make changes** to Rust code
2. **Build**: `npm run build`  
3. **Test**: Serve `dist/` and check browser
4. **Debug**: Browser DevTools → Console for logs, GPU tab for WebGPU inspection

## Key Technical Details

### WebGPU Architecture
- **Worker-based**: Main thread sends events to renderer worker
- **Surface**: OffscreenCanvas rendering
- **Buffers**: Separate vertex buffers per attribute (positions, normals, UVs)

### GLTF Loading Pipeline
1. Fetch GLB from `http://localhost:8080/cube.glb`
2. Parse with `gltf` crate
3. Extract vertex attributes by semantic (not iteration order!)
4. Create separate wgpu::Buffer for each attribute
5. Detect index format (U16/U32) and calculate count correctly

### Critical Implementation Details
- **Attribute mapping**: Must map `gltf::Semantic::Positions` → shader location 0
- **Index format**: Auto-detect from GLTF accessor data type
- **Buffer creation**: One buffer per attribute, not sliced from single buffer
- **Render loop**: Bind each vertex buffer to correct slot

## Debugging Tips

### WebGPU Inspection
- Chrome DevTools → GPU tab
- Check vertex buffer contents, index counts
- Verify pipeline state and draw calls

### Console Logging
- Use `info!()` macro for debugging (appears in browser console)
- Common debug points: attribute parsing, buffer creation, render stats

### Build Issues
- Check for unstable Rust features warnings
- Ensure proper target features: `atomics`, `bulk-memory`, `mutable-globals`
- Use `cargo check` before full build

## Asset Requirements

- **GLTF Models**: Place in `static/` folder, serve via HTTP
- **Expected location**: `http://localhost:8080/cube.glb` 
- **Format**: GLB (binary GLTF) preferred
- **Attributes**: Must have POSITION, NORMAL, TEXCOORD_0

## Performance Notes

- **Worker threading**: Rendering happens off main thread
- **Memory**: Each vertex attribute gets separate buffer
- **Draw calls**: Single indexed draw per model
- **Shaders**: Keep fragment operations simple for better performance

## Future Improvements

When extending this codebase:
1. **Add proper matrix transformations** instead of hardcoded scaling
2. **Implement texture loading** for more realistic rendering  
3. **Add multiple model support** with instancing
4. **Optimize vertex buffer usage** with interleaved layouts
5. **Add error handling** for missing GLTF attributes
