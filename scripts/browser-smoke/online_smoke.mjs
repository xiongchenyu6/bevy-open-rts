import { existsSync, mkdirSync } from "node:fs";
import { chromium } from "playwright-core";

const gameUrl = process.env.OPEN_BEVY_GAME_URL
  ?? "https://xiongchenyu6.github.io/bevy-open-rts/";
const outputDir = process.env.OPEN_BEVY_BROWSER_OUTPUT
  ?? "/tmp/open-bevy-browser-smoke";
const chromeCandidates = [
  process.env.CHROME_BIN,
  "/etc/profiles/per-user/freeman.xiong/bin/google-chrome-stable",
  "/usr/bin/google-chrome-stable",
  "/usr/bin/google-chrome",
  "/usr/bin/chromium",
].filter(Boolean);
const executablePath = chromeCandidates.find(existsSync);

if (!executablePath) {
  throw new Error("Chrome not found; set CHROME_BIN to a Chromium executable");
}

mkdirSync(outputDir, { recursive: true });

const browser = await chromium.launch({
  executablePath,
  headless: true,
  args: [
    "--enable-unsafe-webgpu",
    "--ignore-gpu-blocklist",
    "--enable-features=Vulkan,UseSkiaRenderer,WebGPU",
    "--use-angle=vulkan",
  ],
});

try {
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
  });
  const page = await context.newPage();
  const diagnostics = [];

  page.on("console", (message) => {
    if (["warning", "error"].includes(message.type())) {
      diagnostics.push(`${message.type()}: ${message.text()}`);
    }
  });
  page.on("pageerror", (error) => {
    diagnostics.push(`pageerror: ${error.message}`);
  });
  page.on("requestfailed", (request) => {
    diagnostics.push(
      `requestfailed: ${request.url()} ${request.failure()?.errorText ?? ""}`,
    );
  });

  await page.goto(gameUrl, {
    waitUntil: "domcontentloaded",
    timeout: 60_000,
  });
  await page.waitForFunction(
    () => {
      const loading = document.querySelector("#loading");
      return loading?.classList.contains("hidden")
        || loading?.classList.contains("error");
    },
    undefined,
    { timeout: 60_000 },
  );
  await page.waitForTimeout(2_000);

  const state = await page.evaluate(async () => {
    const canvas = document.querySelector("canvas");
    const loading = document.querySelector("#loading");
    const unsupported = document.querySelector("#unsupported");
    const adapter = navigator.gpu
      ? await navigator.gpu.requestAdapter()
      : null;
    return {
      webgpu: Boolean(adapter),
      loadingHidden: loading?.classList.contains("hidden") ?? false,
      loadingError: loading?.classList.contains("error") ?? false,
      unsupportedHidden: unsupported?.hidden ?? false,
      canvasWidth: canvas?.width ?? 0,
      canvasHeight: canvas?.height ?? 0,
    };
  });

  await page.screenshot({ path: `${outputDir}/main-menu.png` });

  const fatalDiagnostics = diagnostics.filter((message) =>
    /pageerror|requestfailed|boot failed|panicked at|RuntimeError/i.test(message),
  );
  const validCanvas = state.canvasWidth >= 1280 && state.canvasHeight >= 720;
  const passed = state.webgpu
    && state.loadingHidden
    && !state.loadingError
    && state.unsupportedHidden
    && validCanvas
    && fatalDiagnostics.length === 0;

  console.log(JSON.stringify({
    passed,
    gameUrl,
    executablePath,
    state,
    diagnostics,
    screenshot: `${outputDir}/main-menu.png`,
  }, null, 2));

  if (!passed) {
    process.exitCode = 1;
  }
} finally {
  await browser.close();
}
