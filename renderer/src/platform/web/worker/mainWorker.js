// Generic worker that imports the app's WASM module relative to the generated pkg folder.
// Works for any application because the relative depth from this file to pkg is stable.
import initWasm, { worker_entrypoint } from "/level-editor/pkg/level_editor.js";

export function attachMain() {}

let isReady = false;

onmessage = async (event) => {
  console.log("worker received message", event);
  if (isReady) return;

  isReady = true;

  const wasmModule = event.data[0]; // WebAssembly.Module from wasm_bindgen::module()
  const workerId = event.data[1]; // worker ID
  const memory = event.data[2]; // shared memory
  const entryPtr = event.data[3]; // worker entrypoint function pointer

  console.log(
    "worker: initializing with WASM module",
    wasmModule,
    "id:",
    workerId,
  );

  // Initialize WASM with the shared module and memory forwarded from the main thread.
  await initWasm({ module_or_path: wasmModule, memory });

  // Call the app-provided worker entrypoint once initialization completes.
  worker_entrypoint(entryPtr);
};
