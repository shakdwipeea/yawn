// Generic worker that dynamically imports the app's WASM module
// This allows the same worker code to be reused across all apps
export function attachMain() {}

let isReady = false;

onmessage = async (event) => {
  console.log("worker received message", event);
  if (isReady) return;

  isReady = true;

  const wasmModule = event.data[0];  // WebAssembly.Module from wasm_bindgen::module()
  const workerId = event.data[1];    // worker ID
  const memory = event.data[2];      // shared memory
  const entryPtr = event.data[3];    // worker entrypoint function pointer

  console.log("worker: initializing with WASM module", wasmModule);

  // Calculate the URL to the WASM glue JS file based on this worker script's location
  // The worker JS is in pkg/snippets/.../mainWorker.js
  // The wasm-index.js is in pkg/wasm-index.js
  // We need to go up from snippets to pkg
  const workerUrl = new URL(import.meta.url);
  const baseUrl = new URL('../../../wasm-index.js', workerUrl);
  
  console.log("worker: importing glue from", baseUrl.href);

  // Dynamically import the app's WASM glue module
  const mod = await import(baseUrl.href);

  // Initialize WASM with the module and shared memory
  await mod.default({ module: wasmModule, memory });

  // Call the app-provided worker entrypoint
  mod.worker_entrypoint(entryPtr);
};
