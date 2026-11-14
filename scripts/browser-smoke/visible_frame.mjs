import { writeFileSync } from "node:fs";

// A ready canvas can still be black when Chromium cannot present WebGPU.
// Check the captured pixels, rather than accepting DOM readiness as rendering.
export function frameStats(rgba) {
  let lit = 0;
  let minimum = 255;
  let maximum = 0;
  const colors = new Set();
  for (let i = 0; i < rgba.length; i += 4) {
    const [r, g, b, a] = rgba.slice(i, i + 4);
    const brightness = (r + g + b) / 3;
    minimum = Math.min(minimum, brightness);
    maximum = Math.max(maximum, brightness);
    if (a > 0 && brightness > 24) lit += 1;
    colors.add(`${r >> 4},${g >> 4},${b >> 4}`);
  }
  const litFraction = lit / (rgba.length / 4);
  const brightnessRange = maximum - minimum;
  return {
    visible: litFraction >= 0.02 && brightnessRange >= 20 && colors.size >= 8,
    litFraction,
    brightnessRange,
    colorBins: colors.size,
  };
}

export async function captureVisibleFrame(page, path, timeoutMs = 60_000) {
  const session = await page.context().newCDPSession(page);
  const deadline = Date.now() + timeoutMs;
  let stats;
  try {
    do {
      const { data } = await session.send("Page.captureScreenshot", {
        format: "png", fromSurface: true, captureBeyondViewport: false,
      });
      writeFileSync(path, Buffer.from(data, "base64"));
      const samples = await page.evaluate(async (encoded) => {
        const bytes = Uint8Array.from(atob(encoded), (char) => char.charCodeAt(0));
        const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
        const canvas = new OffscreenCanvas(64, 64);
        const context = canvas.getContext("2d", { willReadFrequently: true });
        context.drawImage(bitmap, 0, 0, 64, 64);
        bitmap.close();
        return Array.from(context.getImageData(0, 0, 64, 64).data);
      }, data);
      stats = frameStats(samples);
      if (stats.visible) return stats;
      await page.waitForTimeout(1_000);
    } while (Date.now() < deadline);
    throw new Error(`Blank or flat WebGPU screenshot: ${path}: ${JSON.stringify(stats)}`);
  } finally {
    await session.detach();
  }
}
