import wbg_init, { start } from "../level-editor/pkg/level_editor.js";

const init = async () => {
  await wbg_init();
  const app = start();

  // Expose globally for console access
  window.app = app;
};

// Wait for DOM to be ready before starting
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", init);
} else {
  init();
}
