import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { chromium } from "playwright-core";

const gameUrl = process.env.OPEN_BEVY_GAME_URL
  ?? "https://xiongchenyu6.github.io/bevy-open-rts/";
const signalingUrl = process.env.OPEN_BEVY_SIGNALING_URL
  ?? "https://signal.101.78.126.6.sslip.io";
const outputDir = resolve(
  process.env.GITHUB_WORKSPACE ?? process.cwd(),
  process.env.OPEN_BEVY_BROWSER_OUTPUT
    ?? "/tmp/open-bevy-multiplayer-smoke",
);
const softwareWebGpu = process.env.OPEN_BEVY_SOFTWARE_WEBGPU === "1";
const runId = process.env.OPEN_BEVY_ONLINE_VERIFY_RUN
  ?? `browser-${Date.now()}-${Math.random().toString(16).slice(2, 10)}`;
const timeoutMs = Number(process.env.OPEN_BEVY_MULTIPLAYER_TIMEOUT_MS ?? 180_000);
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

function verificationUrl(role) {
  const url = new URL(gameUrl);
  url.searchParams.set("online_verify", role);
  url.searchParams.set("online_run", runId);
  url.searchParams.set("online_service", signalingUrl);
  return url.toString();
}

function browserArgs() {
  const schedulingArgs = [
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-renderer-backgrounding",
    "--disable-ipc-flooding-protection",
    "--disable-features=CalculateNativeWinOcclusion",
  ];
  if (softwareWebGpu) {
    return [
      ...schedulingArgs,
      "--enable-unsafe-webgpu",
      "--enable-unsafe-swiftshader",
      "--ignore-gpu-blocklist",
      "--enable-features=Vulkan,UseSkiaRenderer,WebGPU",
      "--use-angle=swiftshader",
      "--use-vulkan=swiftshader",
      "--disable-vulkan-surface",
      "--disable-gpu-sandbox",
    ];
  }
  return [
    ...schedulingArgs,
    "--enable-unsafe-webgpu",
    "--ignore-gpu-blocklist",
    "--enable-features=Vulkan,UseSkiaRenderer,WebGPU",
    "--use-angle=vulkan",
  ];
}

function collectDiagnostics(page, role) {
  const diagnostics = [];
  page.on("console", (message) => {
    if (["warning", "error"].includes(message.type())) {
      diagnostics.push(`${role} ${message.type()}: ${message.text()}`);
    }
  });
  page.on("pageerror", (error) => {
    diagnostics.push(`${role} pageerror: ${error.message}`);
  });
  page.on("requestfailed", (request) => {
    diagnostics.push(
      `${role} requestfailed: ${request.url()} ${request.failure()?.errorText ?? ""}`,
    );
  });
  return diagnostics;
}

async function bootVerificationClient(page, role) {
  await page.goto(verificationUrl(role), {
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
    { timeout: 90_000 },
  );
  const startup = await page.evaluate(() => ({
    loadingError: document.querySelector("#loading")?.classList.contains("error") ?? false,
    unsupportedHidden: document.querySelector("#unsupported")?.hidden ?? false,
    canvasWidth: document.querySelector("canvas")?.width ?? 0,
    canvasHeight: document.querySelector("canvas")?.height ?? 0,
  }));
  if (startup.loadingError || !startup.unsupportedHidden) {
    throw new Error(`${role} WebGPU startup failed: ${JSON.stringify(startup)}`);
  }
  return startup;
}

async function waitForTerminalReport(page, role) {
  await page.waitForFunction(
    () => {
      const text = document.querySelector("#open-bevy-online-verification")?.textContent;
      if (!text) return false;
      try {
        return JSON.parse(text).terminal === true;
      } catch {
        return false;
      }
    },
    undefined,
    { timeout: timeoutMs },
  );
  const report = await page.evaluate(() => JSON.parse(
    document.querySelector("#open-bevy-online-verification").textContent,
  ));
  if (report.role !== role) {
    throw new Error(`${role} reported unexpected role ${report.role}`);
  }
  return report;
}

async function capturePage(context, page, path) {
  const session = await context.newCDPSession(page);
  try {
    const { data } = await session.send("Page.captureScreenshot", {
      format: "png",
      fromSurface: true,
      captureBeyondViewport: false,
    });
    writeFileSync(path, Buffer.from(data, "base64"));
    return path;
  } finally {
    await session.detach();
  }
}

mkdirSync(outputDir, { recursive: true });

const browserOptions = {
  executablePath,
  headless: true,
  args: browserArgs(),
};
const [hostBrowser, playerBrowser] = await Promise.all([
  chromium.launch(browserOptions),
  chromium.launch(browserOptions),
]);

try {
  // Two separate browser processes prevent headless Chrome from treating either
  // real-time simulation as a background tab and reducing requestAnimationFrame
  // to a handful of ticks per minute.
  const hostContext = await hostBrowser.newContext({ viewport: { width: 640, height: 360 } });
  const playerContext = await playerBrowser.newContext({ viewport: { width: 640, height: 360 } });
  const hostPage = await hostContext.newPage();
  const playerPage = await playerContext.newPage();
  const hostDiagnostics = collectDiagnostics(hostPage, "host");
  const playerDiagnostics = collectDiagnostics(playerPage, "player");

  const [hostStartup, playerStartup] = await Promise.all([
    bootVerificationClient(hostPage, "host"),
    bootVerificationClient(playerPage, "player"),
  ]);
  const [host, player] = await Promise.all([
    waitForTerminalReport(hostPage, "host"),
    waitForTerminalReport(playerPage, "player"),
  ]);

  const diagnostics = [...hostDiagnostics, ...playerDiagnostics];
  const fatalDiagnostics = diagnostics.filter((message) =>
    /pageerror|requestfailed|boot failed|panicked at|RuntimeError/i.test(message),
  );
  const passed = host.passed === true
    && player.passed === true
    && host.run_id === runId
    && player.run_id === runId
    && host.room_code
    && host.room_code === player.room_code
    && host.connected_humans === 2
    && player.connected_humans === 2
    && host.snapshot_tick > 0
    && player.snapshot_tick > 0
    && player.command_sent === true
    && host.command_observed === true
    && player.command_observed === true
    && host.result === "victory"
    && player.result === "defeat"
    && fatalDiagnostics.length === 0;

  // Publish the functional result before optional visual evidence so a slow
  // software renderer cannot hide the authoritative client reports.
  console.log(JSON.stringify({ phase: "terminal-reports", passed, host, player }, null, 2));

  const screenshotResults = await Promise.allSettled([
    capturePage(hostContext, hostPage, `${outputDir}/host-final.png`),
    capturePage(playerContext, playerPage, `${outputDir}/player-final.png`),
  ]);
  const screenshotErrors = screenshotResults
    .filter((result) => result.status === "rejected")
    .map((result) => String(result.reason));

  console.log(JSON.stringify({
    passed,
    runId,
    gameUrl,
    signalingUrl,
    executablePath,
    softwareWebGpu,
    hostStartup,
    playerStartup,
    host,
    player,
    diagnostics,
    screenshotErrors,
    screenshots: [
      `${outputDir}/host-final.png`,
      `${outputDir}/player-final.png`,
    ],
  }, null, 2));

  if (!passed) {
    process.exitCode = 1;
  }
} finally {
  await Promise.allSettled([hostBrowser.close(), playerBrowser.close()]);
}
