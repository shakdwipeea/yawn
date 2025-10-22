import wbg_init, { main } from "../level-editor/pkg/wasm-index.js";

const start = async () => {
  await wbg_init();
  main();
};

// Wait for DOM to be ready before starting
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', start);
} else {
  start();
}
