# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview
Yawn is a WebGL-based graphics engine ("yet another webgl ngine") built with Rust/WASM and TypeScript. It uses a service worker architecture where rendering happens in a Web Worker with WebGPU, and the main thread handles UI events.

## Build Commands

### Development
- `npm run build` - Full development build (clean, WASM dev build, bundle dev)
- `npm run wasm-dev` - Build WASM module in development mode with atomics support
- `npm run bundle-dev` - Bundle frontend code for development
- `npm start` - Start preview server on port 8080

### Production  
- `npm run build-release` - Full production build (clean, WASM release, bundle release)
- `npm run wasm-release` - Build WASM module in release mode
- `npm run bundle-release` - Bundle frontend code for production

### Utility
- `npm run clean` - Remove dist and pkg directories
- `npm run clean-all` - Remove all generated files including node_modules and target

## Architecture

### Core Components
- **Main Thread** (`src/lib.rs`): Handles DOM events (resize, mouse), creates service worker, manages event channel
- **Service Worker** (`src/platform/web/worker/`): Contains WebGPU renderer, runs render loop
- **Renderer** (`src/renderer/mod.rs`): WebGPU-based rendering with uniform buffers, vertex/index buffers, GLTF loading
- **Message System** (`src/message.rs`): Event communication between main thread and worker via channels

### Platform Abstraction
- `src/platform/mod.rs` - Conditional compilation for WASM vs native targets
- `src/platform/web/` - Web-specific implementations (canvas handling, workers)
- `src/platform/native/` - Native platform code (currently minimal)

### Key Technologies
- **WASM**: Built with wasm-pack, uses nightly Rust with atomics/bulk-memory features
- **WebGPU**: Rendering via wgpu crate, requires Cross-Origin headers in dev server
- **Threading**: Uses Web Workers for rendering, std::sync::mpsc for communication
- **GLTF**: 3D model loading support via gltf crate

### Development Notes
- Requires Rust nightly toolchain (configured in rust-toolchain.toml)
- Web server configured with Cross-Origin headers for WebGPU SharedArrayBuffer support
- Canvas resizes automatically with device pixel ratio scaling
- Mouse events trigger asset reloading (current demo behavior)
- Static assets served from `static/` directory, built to `dist/`

### File Structure
- `src/lib.rs` - Main entry point and App struct
- `src/renderer/mod.rs` - WebGPU rendering logic  
- `src/platform/` - Platform-specific code
- `src/message.rs` - Inter-thread communication types
- `src/gltf.rs` - 3D model loading
- `static/` - Web assets (HTML, JS, models, textures)
- `pkg/` - Generated WASM bindings (auto-generated)