# Build, Lint, and Test Commands
- `npm run dev`: Start Vite dev server with hot reload for WASM bundle
- `npm run build`: Build optimized WASM and JS in `dist/` for development
- `npm run build-release`: Build optimized WASM and JS for production
- `cargo check`: Validate Rust sources quickly before full builds
- `cargo run -p level-editor`: Build and run the native Wayland level-editor app
- `cargo fmt`: Format Rust code with rustfmt
- No unit tests currently exist; add them as `*_tests.rs` modules

# Native App Verification
- Use `cargo check` first to validate native compilation quickly.
- Use `cargo run -p level-editor` to launch the native app. The app should keep running until the window is closed; if run through an automation timeout, timeout termination is expected once startup succeeds.
- To capture a screenshot on Hyprland/Wayland, launch the app in the background, locate the window with `hyprctl clients -j`, then capture its geometry with `grim -g "x,y wxh" /tmp/opencode/level-editor.png`.
- The native window currently uses class `dev.yawn.renderer.sctk-window` and title `renderer sctk window`, which can be used to identify it in `hyprctl clients -j` output.
- Store temporary screenshots and logs under `/tmp/opencode/`.

# Code Style Guidelines
- **Rust 2021 idioms**: Use snake_case for modules, files, functions, and variables
- **Indentation**: 4 spaces (configured in rustfmt)
- **Imports**: Group std library, external crates, then local modules
- **Types**: Use descriptive struct fields and enum variants (e.g., `positions`, `normals`)
- **Error handling**: Use `thiserror` derive macro for custom error types
- **Naming**: Mirror GLTF semantics explicitly in struct fields
- **WGSL shaders**: Keep binding names aligned with Rust bind group layouts
- **JavaScript/TypeScript**: Format with prettier defaults
- **Comments**: Add documentation comments for public APIs using `///`. All non-trivial code should have inline comments explaining **why** the code exists, not just what it does. Explain design decisions, invariants, and non-obvious reasoning. See `renderer/src/gltf.rs` as a reference for the expected level of commentary.
- **Logging**: Use `log::info!`, `log::error!`, etc. from the log crate, not `println!`
