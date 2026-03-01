import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  timeout: 60_000,
  retries: 0,
  use: {
    baseURL: "http://localhost:4173",
    headless: false,
    launchOptions: {
      args: [
        "--enable-unsafe-webgpu",
        "--enable-features=Vulkan",
        "--ozone-platform=x11",
      ],
    },
  },
  projects: [
    {
      name: "chromium",
      use: { browserName: "chromium" },
    },
  ],
  webServer: {
    command: "npx vite preview --port 4173",
    port: 4173,
    reuseExistingServer: !process.env.CI,
  },
});
