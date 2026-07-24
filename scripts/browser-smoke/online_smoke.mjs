import { existsSync, mkdirSync } from "node:fs";
import { chromium } from "playwright-core";

const gameUrl = process.env.OPEN_BEVY_GAME_URL
  ?? "https://xiongchenyu6.github.io/bevy-open-rts/";
const signalingUrl = process.env.OPEN_BEVY_SIGNALING_URL
  ?? "https://signal.101.78.126.6.sslip.io";
const outputDir = process.env.OPEN_BEVY_BROWSER_OUTPUT
  ?? "/tmp/open-bevy-browser-smoke";
const softwareWebGpu = process.env.OPEN_BEVY_SOFTWARE_WEBGPU === "1";
const chromeCandidates = [
  process.env.CHROME_BIN,
  "/etc/profiles/per-user/freeman.xiong/bin/google-chrome-stable",
  "/usr/bin/google-chrome-stable",
  "/usr/bin/google-chrome",
  "/usr/bin/chromium",
].filter(Boolean);
const executablePath = chromeCandidates.find(existsSync);

async function readGameState(page) {
  return page.evaluate(async () => {
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
}

async function bootGame(page) {
  let state;

  for (let attempt = 1; attempt <= 3; attempt += 1) {
    if (attempt === 1) {
      await page.goto(gameUrl, {
        waitUntil: "domcontentloaded",
        timeout: 60_000,
      });
    } else {
      await page.reload({
        waitUntil: "domcontentloaded",
        timeout: 60_000,
      });
    }

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
    state = await readGameState(page);

    // Chrome's software Vulkan backend can transiently return no adapter on
    // its first request. Retry only that unsupported-page state; a real boot
    // error remains visible and fails without being masked by a reload.
    if (state.unsupportedHidden || state.loadingError) {
      return { state, bootAttempts: attempt };
    }
    await page.waitForTimeout(750);
  }

  return { state, bootAttempts: 3 };
}

if (!executablePath) {
  throw new Error("Chrome not found; set CHROME_BIN to a Chromium executable");
}

mkdirSync(outputDir, { recursive: true });

const browser = await chromium.launch({
  executablePath,
  headless: true,
  args: softwareWebGpu
    ? [
        "--enable-unsafe-webgpu",
        "--enable-unsafe-swiftshader",
        "--ignore-gpu-blocklist",
        "--enable-features=Vulkan,UseSkiaRenderer,WebGPU",
        "--use-angle=swiftshader",
        "--use-vulkan=swiftshader",
        "--disable-vulkan-surface",
        "--disable-gpu-sandbox",
      ]
    : [
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

  const { state, bootAttempts } = await bootGame(page);

  const signaling = await page.evaluate(async ({ serviceUrl }) => {
    const configResponse = await fetch(`${serviceUrl}/v1/config`);
    if (!configResponse.ok) {
      throw new Error(`signaling config returned ${configResponse.status}`);
    }
    const config = await configResponse.json();
    const hasTurn = config.ice_servers.some((server) =>
      server.urls.some((url) => url.startsWith("turn:")),
    );
    if (!hasTurn) {
      throw new Error("signaling config did not issue a TURN credential");
    }

    const roomResponse = await fetch(`${serviceUrl}/v1/rooms`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        game_id: "bevy-open-rts",
        build_id: "browser-smoke",
        protocol_version: 1,
        max_peers: 2,
        visibility: "unlisted",
        metadata: { purpose: "browser-smoke" },
      }),
    });
    if (!roomResponse.ok) {
      throw new Error(`room creation returned ${roomResponse.status}`);
    }
    const room = await roomResponse.json();
    const websocketUrl = new URL(room.signaling_url);
    websocketUrl.searchParams.set("name", "Browser Smoke");
    websocketUrl.searchParams.set("role", "host");
    websocketUrl.searchParams.set("build_id", room.room.build_id);
    websocketUrl.searchParams.set("ticket", room.host_token);

    const firstEvent = await new Promise((resolve, reject) => {
      const socket = new WebSocket(websocketUrl);
      const timeout = setTimeout(() => {
        socket.close();
        reject(new Error("signaling WebSocket handshake timed out"));
      }, 15_000);
      socket.addEventListener("message", (event) => {
        clearTimeout(timeout);
        socket.close();
        resolve(JSON.parse(event.data));
      }, { once: true });
      socket.addEventListener("error", () => {
        clearTimeout(timeout);
        reject(new Error("signaling WebSocket failed"));
      }, { once: true });
    });

    const roomCode = room.room.room_code;
    return {
      service: config.service,
      hasTurn,
      roomCodeValid: /^[A-F0-9]{8}$/.test(roomCode),
      firstEventType: Object.keys(firstEvent)[0] ?? "unknown",
    };
  }, { serviceUrl: signalingUrl });

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
    && signaling.service === "open-bevy-signaling"
    && signaling.hasTurn
    && signaling.roomCodeValid
    && signaling.firstEventType === "IdAssigned"
    && fatalDiagnostics.length === 0;

  console.log(JSON.stringify({
    passed,
    gameUrl,
    signalingUrl,
    executablePath,
    softwareWebGpu,
    bootAttempts,
    state,
    signaling,
    diagnostics,
    screenshot: `${outputDir}/main-menu.png`,
  }, null, 2));

  if (!passed) {
    process.exitCode = 1;
  }
} finally {
  await browser.close();
}
