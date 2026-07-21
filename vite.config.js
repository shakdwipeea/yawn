import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import { ViteRsw } from "vite-plugin-rsw";
import wasm from "vite-plugin-wasm";
import copy from "rollup-plugin-copy";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export default defineConfig({
  build: {
    rollupOptions: {
      input: {
        app: "static/index.html",
      },
    },
    // Relative to 'root'.
    outDir: "../dist",
    copyPublicDir: true,
  },
  // For getting out of index.html from dist/static directory.
  root: "static",
  publicDir: ".",
  worker: {
    format: "es",
  },
  resolve: {
    alias: {
      pkg: path.resolve(__dirname, "level-editor/pkg"),
    },
  },
  plugins: [
    copy({
      targets: [
        { src: "level-editor/pkg/*", dest: "static/level-editor/pkg" },
      ],
      hook: "buildStart",
    }),
    ViteRsw(),
    // Makes us be able to use top level await for wasm.
    // Otherwise, we can restrict build.target to 'es2022', which allows top level await.
    wasm(),
  ],
  server: {
    port: 8080,
    headers: {
      "Cross-Origin-Embedder-Policy": "require-corp",
      "Cross-Origin-Opener-Policy": "same-origin",
    },
    fs: {
      strict: false,
    },
  },
  preview: {
    port: 8080,
    headers: {
      "Cross-Origin-Embedder-Policy": "require-corp",
      "Cross-Origin-Opener-Policy": "same-origin",
    },
  },
});
