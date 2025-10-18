# Build, Lint, and Test Commands
- `npm run dev`: Start Vite dev server with hot reload for WASM bundle
- `npm run build`: Build optimized WASM and JS in `dist/` for development
- `npm run build-release`: Build optimized WASM and JS for production
- `cargo check`: Validate Rust sources quickly before full builds
- `cargo fmt`: Format Rust code with rustfmt
- No unit tests currently exist; add them as `*_tests.rs` modules

# Code Style Guidelines
- **Rust 2021 idioms**: Use snake_case for modules, files, functions, and variables
- **Indentation**: 4 spaces (configured in rustfmt)
- **Imports**: Group std library, external crates, then local modules
- **Types**: Use descriptive struct fields and enum variants (e.g., `positions`, `normals`)
- **Error handling**: Use `thiserror` derive macro for custom error types
- **Naming**: Mirror GLTF semantics explicitly in struct fields
- **WGSL shaders**: Keep binding names aligned with Rust bind group layouts
- **JavaScript/TypeScript**: Format with prettier defaults
- **Comments**: Add documentation comments for public APIs using `///`
- **Logging**: Use `log::info!`, `log::error!`, etc. from the log crate, not `println!`
