// Imports wasm statically.
// Reasonable because worker can't do anything without wasm.
// And worker's loading can still be determined from external dynamically.
import wbg_init, { worker_entrypoint } from "../../../../wasm-index.js";

export function attachMain() {}

let isReady = false;

onmessage = async (event) => {
  console.log("event", event);
  if (isReady) return;

  isReady = true;
  await wbg_init({ module_or_path: event.data[0], memory: event.data[2] });
  worker_entrypoint(event.data[3]);
};
