# Current local browser QA

The current WebGPU release bundle built successfully on October 10, 2026. Offline real-input screenshots verify the English menu, skirmish setup, match startup, briefing dismissal, unit selection, and visible movement at 1280×720 and 900×720. Browser/native online control remains pending.

## Build provenance

- Rust 1.95.0, locked dependencies, one Cargo job, unchanged release profile.
- `scripts/build_web.sh`, canonical compiler environment, unit `rts-current-wasm-build-20261010-02`.
- Source recorded at `501fdea0a8036d036b58aeecf2049c421e8c1e18`, including uncommitted browser bridge and campaign helper changes; all recorded source hashes remained unchanged during compilation, except the intentional HTML build stamp.
- Source digest: `2b350e8c38c8e350c4015100b1f09b251049955e42e5f8383f18b8c9a598da30`.
- Build wall time: 14m53.745s; Rust release compile/link: 14m25s. One unused `CampaignProgress.path` warning, no build errors.
- Optimized Wasm: 26,884,725 bytes, SHA256 `e576b058bf1e82151d96964a29caeef3d40ed6df9fee8cfb45d97351481d8475`.
- HTML build ID: `e2bad858d163079f`. Artifact hashes, source hashes, and asset hashes are in `target/web-current-20261009/build-provenance-02.json`.
- Local preview only: `http://127.0.0.1:8765/`. Nothing was hosted or published externally.

Build01 was deliberately stopped for the subsequently verified EMP application correction. Its log and source record are retained separately; completed dependency cache was reused by build02.

## Actual offline evidence

Normal menu input starts Plain & Simple, Human Alliance slot 0 against AI Easy Demon slot 1, with normal 32 ore/16 crystal. The driver sends real pointer and keyboard events. It does not edit game state or grant resources.

| Viewport | Actual selection and movement | Evidence |
| --- | --- | --- |
| 1280×720 | Worker selected; moved from the lower left toward the clicked ground | `target/web-current-20261009/offline-full-02/1280x720/03-unit-selected.png`, `04-unit-moved.png` |
| 900×720 | Drone selected above the base; moved into clear ground to its left | `target/web-current-20261009/offline-plane-900-02/900x720/03-unit-selected.png`, `04-unit-moved.png` |

Menu, setup, and match screenshots accompany both runs. Both reported positive actual asset completion, 145/145, with loading overlay hidden, no unsupported-browser message, and no page errors, failed requests, or error-console diagnostics. Visual inspection, rather than stage filenames alone, establishes selection and displacement. The combined reviewed result and screenshot hashes are in `target/web-current-20261009/offline-visual-verification.json`.

The narrower display's Worker was obscured by its command panel in an earlier run; that frame was insufficient movement evidence. The final 900-pixel proof uses the visible Drone. Earlier fast clicks and immediate cursor recentering were also corrected before certification: clicks/keys last 200ms and the pointer stays at the actual clicked position.

## Browser capture configuration

Hardware-Vulkan headless captures were black, even after 30 seconds with all assets loaded. Another browser configuration reported `DeviceLost`. The unchanged bundle rendered correctly with the explicit software Vulkan compositor configuration: `--enable-gpu`, `--enable-unsafe-webgpu`, `--enable-unsafe-swiftshader`, `--use-angle=swiftshader`, `--use-vulkan=swiftshader`, `--enable-features=Vulkan`, and an available X display. This matches the upstream [WebGPU canvas capture report](https://github.com/visgl/luma.gl/issues/2874). The browser drivers select that configuration with `OPEN_BEVY_SOFTWARE_WEBGPU=1`.

Reproduce the offline input sequence against a freshly built local preview:

```sh
OPEN_BEVY_SOFTWARE_WEBGPU=1 \
OPEN_BEVY_READY_FRAME_WAIT_MS=10000 \
OPEN_BEVY_GAME_URL=http://127.0.0.1:8765/ \
OPEN_BEVY_OFFLINE_RECIPE=scripts/browser-smoke/offline_recipe.json \
node scripts/browser-smoke/offline_ui_smoke.mjs
```

The recipe contains the inspected real coordinates. The driver's stage-completion result still requires screenshot review. This smoke proves the tested offline interactions and rendered English UI; it does not establish browser FPS, all browsers, or online interoperability. The exact first uncovered render frame has not been video-verified.

The English loading overlay now uses real tracked asset counts and only hides after a validated AssetLoading exit with a positive completed total. A bare 0/0 value or elapsed timeout cannot dismiss it. Raw done/total counts remain DOM telemetry rather than player copy.

## Browser/native attempts: gate remains open

The first direct attempt from `127.0.0.1:8765` could not fetch the provider's room list because that browser origin is outside the published allowlist. Its supervised failure disconnected both clients and confirmed the QA room expired with HTTP 404. Read-only provider checks and the checked-in Worker configuration show that `localhost:8080` and `127.0.0.1:8080` are allowed. No proxy, browser security bypass, or public service change was needed.

The second attempt served the identical bundle on IPv6 loopback `localhost:8080`, leaving the unrelated service on IPv4 port 8080 untouched. Both humans connected through the actual published signaling service and entered a normal Human/Human/AI match. The browser sent three ordinary move commands, completed its route, and observed completed AI production. Its first readiness screenshot is a visible English online menu, rather than a black frame.

The authoritative host acceptance check failed: it never observed both queued moves simultaneously and later detected an order on an untouched Human unit at game time 64.801 seconds. The live cross gate has **not passed**. Read-only comparison found 16 common cached snapshot summaries, 13 after 30 seconds, with identical tracked Worker route positions, Human economy, and AI entity summaries. This does not meet the required 20 matching ticks. Both clients disconnected normally and the second QA room also expired with HTTP 404.

Evidence is in `target/online-human-ai-browser-native-20261010-01` and `-02`, with origin checks in `target/web-current-20261009/provider-local-origin-check.json`. The pair uses the sealed EMP application native build and the recorded Wasm, before subsequent mine damage work. A read-only diagnostic of packet timing, actual host queue lengths, and offending entity identity is prepared; no bootstrap or gameplay workaround has been applied.
