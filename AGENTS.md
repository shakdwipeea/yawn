# Build, Lint, and Test Commands
- `npm run dev`: Start Vite dev server with hot reload for WASM bundle
- `npm run build`: Build optimized WASM and JS in `dist/` for development
- `npm run build-release`: Build optimized WASM and JS for production
- `cargo check`: Validate Rust sources quickly before full builds
- `cargo run -p level-editor`: Build and run the native Wayland level-editor app
- `cargo fmt`: Format Rust code with rustfmt
- No unit tests currently exist; add them as `*_tests.rs` modules

# Visual Verification
- Keep the two runtime paths distinct: frontend screenshots exercise the WASM build in Chromium; native screenshots exercise the Linux ELF binary through Wayland and do not use a browser.
- Save screenshots intended for user review under `.amp/in/artifacts/`.

## Frontend / WASM in an Amp orb
- Run `.agents/capture-frontend-screenshot` to rebuild the frontend, ensure the supervised Vite preview service is running, launch headful Chromium inside Xvfb, wait for the WebGPU worker to initialize, and save `.amp/in/artifacts/frontend.png`.
- Pass a workspace-relative or absolute PNG path as the first argument to override the output path.
- A solid yellow-green image is only the canvas CSS fallback and means WebGPU did not present. The helper must observe the worker's adapter/surface logs and reject single-color captures.
- Use `npm test` for the existing Playwright behavioral and visual-regression suite; the capture helper is for a reviewable current-state artifact rather than a snapshot assertion.

## Native Linux app in an Amp orb
- Run `.agents/capture-native-screenshot` to build and launch the native `level-editor` binary under an ephemeral Weston headless Wayland compositor and save `.amp/in/artifacts/native.png`.
- Pass a workspace-relative or absolute PNG path as the first argument to override the output path.
- The orb has no physical GPU or desktop session. The helper uses Mesa software rendering, so it validates native startup, Wayland presentation, and visible output, but not hardware-GPU performance or driver-specific behavior.
- Do not use Xvfb for the native app: `WaylandAppRuntime` is Wayland-only. Weston must use its software GL renderer; its pixman headless renderer produces blank captures for this app.
- The helper enables Weston's privileged screenshot protocol only on its private, short-lived compositor and shuts the compositor down after capture.

## Native Linux app on a Hyprland workstation
- Use `cargo check` first to validate native compilation quickly.
- Use `cargo run -p level-editor` to launch the native app. It should keep running until its window is closed; timeout termination is expected when automation only verifies startup.
- Locate the window with `hyprctl clients -j`, then capture its geometry with `grim -g "x,y wxh" <output.png>`.
- The native window currently uses class `dev.yawn.renderer.sctk-window` and title `renderer sctk window`.

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
