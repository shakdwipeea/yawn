import { test, expect } from "@playwright/test";

/** Collect page errors and worker console messages. */
function setupListeners(page: import("@playwright/test").Page) {
  const errors: string[] = [];
  const workerLogs: string[] = [];

  page.on("pageerror", (err) => errors.push(err.message));
  page.on("console", (msg) => {
    if (msg.type() === "error") {
      // Ignore favicon and other non-critical resource 404s
      if (msg.text().includes("404") && msg.text().includes("Failed to load resource")) return;
      errors.push(msg.text());
    }
  });
  page.on("worker", (worker) => {
    worker.on("console", (msg) => {
      workerLogs.push(msg.text());
      if (msg.type() === "error") errors.push(`[worker] ${msg.text()}`);
    });
  });

  return { errors, workerLogs };
}

/** Wait for the WASM app handle to be available on window. */
async function waitForApp(page: import("@playwright/test").Page) {
  await page.waitForFunction(() => (window as any).app !== undefined, null, {
    timeout: 30_000,
  });
}

test.describe("WASM App", () => {
  test("loads without errors and initializes WebGPU", async ({ page }) => {
    const { errors, workerLogs } = setupListeners(page);

    await page.goto("/");
    await waitForApp(page);

    await expect
      .poll(async () =>
        page.evaluate(() => ({
          boxCount: (window as any).app.box_count(),
          meshCount: (window as any).app.mesh_count(),
        })),
      )
      .toEqual({ boxCount: 15, meshCount: 16 });

    // Canvas exists with non-zero dimensions
    const canvas = page.locator("#canvas0");
    await expect(canvas).toBeVisible();
    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThan(0);
    expect(box!.height).toBeGreaterThan(0);

    // Wait for the worker to finish GPU initialization
    await page.waitForTimeout(2000);

    // Verify the WebGPU pipeline was fully set up via worker logs
    const allLogs = workerLogs.join("\n");
    expect(allLogs).toContain("Adapter info:");
    expect(allLogs).toContain("suface size:");

    expect(errors).toEqual([]);
  });

  test("canvas renders visible content", async ({ page }) => {
    const { errors } = setupListeners(page);

    await page.goto("/");
    await waitForApp(page);

    // Wait for several frames to render and composite
    await page.waitForTimeout(2000);

    const canvas = page.locator("#canvas0");
    await expect(canvas).toHaveScreenshot("canvas-initial.png", {
      maxDiffPixelRatio: 0.05,
    });

    expect(errors).toEqual([]);
  });

  test("camera orbit does not crash", async ({ page }) => {
    const { errors } = setupListeners(page);

    await page.goto("/");
    await waitForApp(page);
    await page.waitForTimeout(1000);

    await page.evaluate(() => {
      const app = (window as any).app;
      app.orbit_camera(10.0, 5.0);
      app.orbit_camera(-20.0, 15.0);
      app.orbit_camera(0.0, 0.0);
    });

    await page.waitForTimeout(500);
    expect(errors).toEqual([]);
  });

  test("camera zoom does not crash", async ({ page }) => {
    const { errors } = setupListeners(page);

    await page.goto("/");
    await waitForApp(page);
    await page.waitForTimeout(1000);

    await page.evaluate(() => {
      const app = (window as any).app;
      app.zoom_camera(-5.0); // zoom in
      app.zoom_camera(10.0); // zoom out
      app.zoom_camera(0.0);
    });

    await page.waitForTimeout(500);
    expect(errors).toEqual([]);
  });
});
