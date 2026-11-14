import assert from "node:assert/strict";
import { test } from "node:test";
import { frameStats } from "./visible_frame.mjs";

test("ready-but-black and flat fallback canvases fail the visual gate", () => {
  assert.equal(frameStats(Array(4096).fill([0, 0, 0, 255]).flat()).visible, false);
  assert.equal(frameStats(Array(4096).fill([200, 100, 50, 255]).flat()).visible, false);
});

test("a small cursor on an otherwise black canvas cannot pass", () => {
  const pixels = Array(4096).fill([0, 0, 0, 255]);
  for (let i = 0; i < 32; i += 1) pixels[i] = [i * 8, i * 4, 255, 255];
  assert.equal(frameStats(pixels.flat()).visible, false);
});

test("a varied rendered frame passes", () => {
  const pixels = Array.from({ length: 4096 }, (_, i) => [i % 256, (i * 7) % 256, 80, 255]).flat();
  assert.equal(frameStats(pixels).visible, true);
});
