// Real canvas input only. Coordinates come from an inspected fresh UI recipe.
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { chromium } from "playwright-core";
const gameUrl = process.env.OPEN_BEVY_GAME_URL ?? "http://127.0.0.1:8765/";
const output = resolve(process.env.OPEN_BEVY_BROWSER_OUTPUT ?? "target/web-current-20261009/offline");
const recipePath = process.env.OPEN_BEVY_OFFLINE_RECIPE;
const recipes = recipePath ? JSON.parse(readFileSync(recipePath, "utf8")) : {};
mkdirSync(output, { recursive: true });
const chrome = [process.env.CHROME_BIN, "/etc/profiles/per-user/freeman.xiong/bin/google-chrome-stable", "/usr/bin/google-chrome-stable", "/usr/bin/chromium"].filter(Boolean).find(existsSync);
if (!chrome) throw new Error("Chrome missing");
const browser = await chromium.launch({ executablePath: chrome, headless: true, args: [
  "--enable-gpu", "--enable-unsafe-webgpu", "--ignore-gpu-blocklist", ...(process.env.OPEN_BEVY_SOFTWARE_WEBGPU === "1"
    ? ["--enable-unsafe-swiftshader", "--use-angle=swiftshader", "--use-vulkan=swiftshader", "--enable-features=Vulkan", "--disable-gpu-sandbox"]
    : ["--enable-features=Vulkan,UseSkiaRenderer,WebGPU", "--use-angle=vulkan"]),
  "--disable-background-timer-throttling", "--disable-renderer-backgrounding", "--disable-backgrounding-occluded-windows",
] });
const results = [];
try {
  for (const width of [1280, 900]) {
    const directory = resolve(output, `${width}x720`); mkdirSync(directory, { recursive: true });
    const context = await browser.newContext({ viewport: { width, height: 720 }, deviceScaleFactor: 1 });
    const page = await context.newPage();
    const diagnostics = [];
    page.on("pageerror", error => diagnostics.push({ kind: "pageerror", message: error.message }));
    page.on("requestfailed", request => diagnostics.push({ kind: "requestfailed", url: request.url() }));
    page.on("console", message => { if (message.type() === "error") diagnostics.push({ kind: "console", message: message.text() }); });
    let error = null;
    let readiness = null;
    const completed = [];
    try {
      await page.goto(gameUrl, { waitUntil: "domcontentloaded", timeout: 60000 });
      await page.waitForFunction(() => {
        const canvas = document.getElementById("bevy-canvas");
        return canvas?.getAttribute("data-game-assets-ready") === "true"
          && Number(canvas.getAttribute("data-assets-total")) > 0
          && Number(canvas.getAttribute("data-assets-done")) >= Number(canvas.getAttribute("data-assets-total"));
      }, undefined, { timeout: 180000 });
      await page.waitForTimeout(Number(process.env.OPEN_BEVY_READY_FRAME_WAIT_MS ?? 1500));
      readiness = await page.evaluate(() => {
        const canvas = document.getElementById("bevy-canvas");
        return { language: document.documentElement.lang, width: canvas.width, height: canvas.height,
          ready: canvas.getAttribute("data-game-assets-ready"), done: canvas.getAttribute("data-assets-done"), total: canvas.getAttribute("data-assets-total"),
          loadingHidden: document.getElementById("loading").classList.contains("hidden"), unsupported: !document.getElementById("unsupported").hidden };
      });
      await page.screenshot({ path: `${directory}/00-menu.png` }); completed.push("menu");
      for (const step of recipes[String(width)] ?? []) {
        if (step.kind === "key") await page.keyboard.press(step.key, { delay: 200 });
        else if (step.kind === "click") {
          await page.mouse.move(step.x, step.y);
          await page.mouse.click(step.x, step.y, { button: step.button ?? "left", delay: 200 });
        } else throw new Error("unsupported real-input recipe action");
        await page.waitForTimeout(step.waitMs ?? 1200);
        await page.screenshot({ path: `${directory}/${step.name}.png` }); completed.push(step.name);
      }
    } catch (exc) { error = exc.message; await page.screenshot({ path: `${directory}/error.png` }).catch(() => {}); }
    const required = ["01-setup", "02-match", "03-unit-selected", "04-unit-moved"];
    const passed = !error && diagnostics.length === 0 && readiness?.language === "en" && readiness.loadingHidden && !readiness.unsupported && required.every(name => completed.includes(name));
    results.push({ viewport: [width, 720], passed, error, readiness, completed, diagnostics,
      scope: "Real mouse/keyboard canvas inputs; stage/English-HUD/movement screenshots still require visual inspection" });
    writeFileSync(`${output}/report.json`, JSON.stringify({ passed: results.every(x => x.passed) && results.length === 2, gameUrl, results }, null, 2));
    await context.close();
  }
} finally { await browser.close(); }
console.log(JSON.stringify({ passed: results.every(x => x.passed), gameUrl, results }, null, 2));
if (results.some(x => !x.passed)) process.exitCode = 1;
