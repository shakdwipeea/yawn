import wbg_init, { main, App } from "../pkg/wasm-index.js";

const start = async () => {
  await wbg_init();

  main();
  const app = new App();
};

start();
