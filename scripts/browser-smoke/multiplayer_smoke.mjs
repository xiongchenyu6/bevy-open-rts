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
const forceRelay = process.env.OPEN_BEVY_FORCE_RELAY === "1";
const roles = (process.env.OPEN_BEVY_BROWSER_ROLES ?? "host,player")
  .split(",")
  .map((role) => role.trim().toLowerCase())
  .filter(Boolean);
const expectedHumans = Number(process.env.OPEN_BEVY_EXPECTED_HUMANS ?? 2);
const reportPath = resolve(
  process.env.OPEN_BEVY_BROWSER_REPORT ?? `${outputDir}/result.json`,
);
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
if (
  roles.length === 0
  || roles.length > 2
  || new Set(roles).size !== roles.length
  || roles.some((role) => !["host", "player"].includes(role))
) {
  throw new Error(
    `OPEN_BEVY_BROWSER_ROLES must contain unique host/player roles: ${roles.join(",")}`,
  );
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

async function installRelayProbe(context) {
  await context.addInitScript(() => {
    const peerConnections = [];
    Object.defineProperty(window, "__openBevyPeerConnections", {
      value: peerConnections,
      configurable: false,
      enumerable: false,
      writable: false,
    });

    const NativePeerConnection = window.RTCPeerConnection;
    if (!NativePeerConnection) return;
    function RelayPeerConnection(configuration, constraints) {
      const relayConfiguration = {
        ...(configuration ?? {}),
        iceTransportPolicy: "relay",
      };
      const peerConnection = new NativePeerConnection(relayConfiguration, constraints);
      peerConnections.push(peerConnection);
      return peerConnection;
    }
    RelayPeerConnection.prototype = NativePeerConnection.prototype;
    Object.setPrototypeOf(RelayPeerConnection, NativePeerConnection);
    Object.defineProperty(window, "RTCPeerConnection", {
      value: RelayPeerConnection,
      configurable: true,
      writable: true,
    });
  });
}

async function collectIceRoutes(page) {
  return page.evaluate(async () => {
    const peerConnections = window.__openBevyPeerConnections ?? [];
    return Promise.all(peerConnections.map(async (peerConnection, index) => {
      try {
        const stats = await peerConnection.getStats();
        const reports = new Map();
        stats.forEach((report) => reports.set(report.id, report));
        const transport = [...reports.values()].find((report) =>
          report.type === "transport" && report.selectedCandidatePairId
        );
        const selectedPair = transport
          ? reports.get(transport.selectedCandidatePairId)
          : [...reports.values()].find((report) =>
            report.type === "candidate-pair"
              && report.state === "succeeded"
              && (report.nominated === true || report.selected === true)
          );
        const localCandidate = selectedPair
          ? reports.get(selectedPair.localCandidateId)
          : undefined;
        const remoteCandidate = selectedPair
          ? reports.get(selectedPair.remoteCandidateId)
          : undefined;
        return {
          index,
          connectionState: peerConnection.connectionState,
          iceConnectionState: peerConnection.iceConnectionState,
          selectedByTransport: Boolean(
            transport && selectedPair?.id === transport.selectedCandidatePairId
          ),
          pairId: selectedPair?.id ?? null,
          pairState: selectedPair?.state ?? null,
          nominated: selectedPair?.nominated ?? false,
          bytesSent: selectedPair?.bytesSent ?? 0,
          bytesReceived: selectedPair?.bytesReceived ?? 0,
          localCandidateType: localCandidate?.candidateType ?? null,
          localProtocol: localCandidate?.protocol ?? null,
          localRelayProtocol: localCandidate?.relayProtocol ?? null,
          remoteCandidateType: remoteCandidate?.candidateType ?? null,
          remoteProtocol: remoteCandidate?.protocol ?? null,
        };
      } catch (error) {
        return {
          index,
          connectionState: peerConnection.connectionState,
          iceConnectionState: peerConnection.iceConnectionState,
          error: String(error),
        };
      }
    }));
  });
}

async function bootVerificationClient(page, role) {
  let startup;
  for (let attempt = 1; attempt <= 3; attempt += 1) {
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
    startup = await page.evaluate((currentAttempt) => ({
      loadingError: document.querySelector("#loading")?.classList.contains("error") ?? false,
      unsupportedHidden: document.querySelector("#unsupported")?.hidden ?? false,
      canvasWidth: document.querySelector("canvas")?.width ?? 0,
      canvasHeight: document.querySelector("canvas")?.height ?? 0,
      attempt: currentAttempt,
    }), attempt);
    if (!startup.loadingError && startup.unsupportedHidden) {
      return startup;
    }
    if (attempt < 3) {
      await page.waitForTimeout(1_000);
    }
  }
  throw new Error(`${role} WebGPU startup failed: ${JSON.stringify(startup)}`);
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
const browsers = await Promise.all(roles.map(() => chromium.launch(browserOptions)));

try {
  // Separate processes prevent headless Chrome from treating either real-time
  // simulation as a background tab and reducing requestAnimationFrame to a
  // handful of ticks per minute. A single role is used by the native/browser
  // compatibility harness while the other peer runs as a desktop binary.
  const clients = await Promise.all(roles.map(async (role, index) => {
    const context = await browsers[index].newContext({ viewport: { width: 640, height: 360 } });
    if (forceRelay) {
      await installRelayProbe(context);
    }
    const page = await context.newPage();
    return {
      role,
      context,
      page,
      diagnostics: collectDiagnostics(page, role),
    };
  }));

  const startupEntries = await Promise.all(clients.map(async (client) => [
    client.role,
    await bootVerificationClient(client.page, client.role),
  ]));
  const reportEntries = await Promise.all(clients.map(async (client) => [
    client.role,
    await waitForTerminalReport(client.page, client.role),
  ]));
  const startups = Object.fromEntries(startupEntries);
  const reports = Object.fromEntries(reportEntries);
  const iceRouteEntries = await Promise.all(clients.map(async (client) => [
    client.role,
    await collectIceRoutes(client.page),
  ]));
  const iceRoutes = Object.fromEntries(iceRouteEntries);
  const diagnostics = clients.flatMap((client) => client.diagnostics);
  const fatalDiagnostics = diagnostics.filter((message) =>
    /pageerror|requestfailed|boot failed|panicked at|RuntimeError/i.test(message),
  );
  const browserReportsPassed = roles.every((role) => {
    const report = reports[role];
    return report.passed === true
      && report.run_id === runId
      && Boolean(report.room_code)
      && report.connected_humans === expectedHumans
      && report.snapshot_tick > 0
      && report.command_observed === true
      && (role !== "host" || report.result === "victory")
      && (role !== "player" || (report.command_sent === true && report.result === "defeat"));
  });
  const roomCodes = new Set(roles.map((role) => reports[role].room_code));
  const relayRoutesPassed = !forceRelay || roles.every((role) => {
    const routes = iceRoutes[role];
    return routes.length > 0 && routes.every((route) =>
      route.selectedByTransport === true
        && ["connected", "completed"].includes(route.connectionState)
        && ["connected", "completed"].includes(route.iceConnectionState)
        && route.nominated === true
        && route.bytesSent > 0
        && route.bytesReceived > 0
        && route.localCandidateType === "relay"
        && route.remoteCandidateType === "relay"
    );
  });
  const passed = browserReportsPassed
    && roomCodes.size === 1
    && relayRoutesPassed
    && fatalDiagnostics.length === 0;

  // Publish the functional result before optional visual evidence so a slow
  // software renderer cannot hide the authoritative client reports.
  console.log(JSON.stringify({ phase: "terminal-reports", passed, reports }, null, 2));

  const screenshotResults = await Promise.allSettled(clients.map((client) =>
    capturePage(client.context, client.page, `${outputDir}/${client.role}-final.png`)
  ));
  const screenshotErrors = screenshotResults
    .filter((result) => result.status === "rejected")
    .map((result) => String(result.reason));

  const result = {
    passed,
    runId,
    roles,
    gameUrl,
    signalingUrl,
    executablePath,
    softwareWebGpu,
    forceRelay,
    expectedHumans,
    startups,
    reports,
    iceRoutes,
    hostStartup: startups.host,
    playerStartup: startups.player,
    host: reports.host,
    player: reports.player,
    diagnostics,
    screenshotErrors,
    screenshots: roles.map((role) => `${outputDir}/${role}-final.png`),
  };
  writeFileSync(reportPath, `${JSON.stringify(result, null, 2)}\n`);
  console.log(JSON.stringify(result, null, 2));

  if (!passed) {
    process.exitCode = 1;
  }
} finally {
  await Promise.allSettled(browsers.map((browser) => browser.close()));
}
