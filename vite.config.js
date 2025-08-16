import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  build: {
    rollupOptions: {
      input: {
        app: "static/index.html",
      },
    },
    // Relative to 'root'.
    outDir: "../dist",
  },
  // For getting out of index.html from dist/static directory.
  root: "static",
  worker: {
    format: "es",
  },
  resolve: {
    alias: {
      'pkg': '/home/akash/workspace/yawn/pkg'
    }
  },
  plugins: [
    // Makes us be able to use top level await for wasm.
    // Otherwise, we can restrict build.target to 'es2022', which allows top level await.
    wasm(),
  ],
  server: {
    port: 8080,
    headers: {
      'Cross-Origin-Embedder-Policy': 'require-corp',
      'Cross-Origin-Opener-Policy': 'same-origin',
    },
  },
  preview: {
    port: 8080,
    headers: {
      'Cross-Origin-Embedder-Policy': 'require-corp',
      'Cross-Origin-Opener-Policy': 'same-origin',
    },
  },
});
