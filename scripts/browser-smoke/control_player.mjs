// Local fresh-bundle player for the ordinary human+AI native/browser proof.
import { existsSync, writeFileSync, renameSync } from "node:fs";
import { chromium } from "playwright-core";
const gameUrl = new URL(process.env.OPEN_BEVY_GAME_URL);
gameUrl.searchParams.set("online_verify", "player");
gameUrl.searchParams.set("online_mode", "human-ai");
gameUrl.searchParams.set("online_run", process.env.OPEN_BEVY_ONLINE_VERIFY_RUN);
gameUrl.searchParams.set("online_service", process.env.OPEN_BEVY_SIGNALING_URL);
const output = process.env.OPEN_BEVY_ONLINE_VERIFY_STATUS;
const candidates = [process.env.CHROME_BIN, "/etc/profiles/per-user/freeman.xiong/bin/google-chrome-stable", "/usr/bin/google-chrome-stable", "/usr/bin/chromium"].filter(Boolean);
const executablePath = candidates.find(existsSync);
if (!executablePath) throw new Error("Chrome missing");
const browser = await chromium.launch({ executablePath, headless: true, args: [
  "--enable-gpu", "--enable-unsafe-webgpu", "--ignore-gpu-blocklist", ...(process.env.OPEN_BEVY_SOFTWARE_WEBGPU === "1"
    ? ["--enable-unsafe-swiftshader", "--use-angle=swiftshader", "--use-vulkan=swiftshader", "--enable-features=Vulkan", "--disable-gpu-sandbox"]
    : ["--enable-features=Vulkan,UseSkiaRenderer,WebGPU", "--use-angle=vulkan"]),
  "--disable-background-timer-throttling", "--disable-backgrounding-occluded-windows", "--disable-renderer-backgrounding",
] });
const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
let running = true;
let firstReadyCaptured = false;
process.on("SIGINT", () => { running = false; });
process.on("SIGTERM", () => { running = false; });
try {
  await page.goto(gameUrl.toString(), { waitUntil: "domcontentloaded", timeout: 60000 });
  while (running) {
    if (!firstReadyCaptured) {
      const ready = await page.evaluate(() => {
        const canvas = document.getElementById("bevy-canvas");
        return canvas?.getAttribute("data-game-assets-ready") === "true"
          && Number(canvas.getAttribute("data-assets-total")) > 0;
      });
      if (ready) {
        await page.screenshot({ path: output.replace(/\.json$/, "-first-ready.png") });
        firstReadyCaptured = true;
      }
    }
    const rawReport = await page.evaluate(() => document.getElementById("open-bevy-online-verification")?.textContent);
    if (rawReport) {
      // Preserve the original JSON text: JS numbers cannot retain map IDs above 2^53.
      const report = JSON.parse(rawReport);
      writeFileSync(output + ".pending", rawReport);
      renameSync(output + ".pending", output);
      if (report.terminal) {
        await page.screenshot({ path: output.replace(/\.json$/, "-terminal.png") });
        while (running) await new Promise(resolve => setTimeout(resolve, 100));
        break;
      }
    }
    await new Promise(resolve => setTimeout(resolve, 100));
  }
} finally { await browser.close(); }
