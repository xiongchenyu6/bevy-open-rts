# Web delivery and touch play

The launcher is an installable PWA. English is the default; its language buttons work before the engine starts and carry the selection into the game. The install guide follows the browser's native installation affordance. It never claims that online rooms work without a network.

## Resource lifetime

- Start prepares only the engine and common interface assets. Battlefield models, textures and audio are in a shared combat pack. An asset-reader gate blocks these requests until the player starts a skirmish, campaign mission or opens online play.
- The preparation dialog reports missing bytes, permits Back without starting a match, retains verified partial downloads, and waits for Bevy's model dependencies after transfer. Entering a second match reuses this pack.
- Files have SHA-256 identities and immutable URLs. Updates download only missing changed content from installed groups. An update is offered on startup and explicitly accepted; a running match is never reloaded.
- The Settings download manager can save all content or remove extras. It preserves progress, the launcher and resources leased by an open match. Returning to the main menu releases the lease.
- The build stages an explicit runtime whitelist in `target/web-dist/`. Local `web/qa-*` snapshots and logs are never shipped. Runtime-only updates reuse verified published engine bytes when compiled sources, embedded files, Cargo inputs and the signaling environment are identical. The current release and two previous immutable releases remain available to open clients.

## Touch controls

Tap selects or activates the existing command card. Hold on the battlefield issues an order. Drag pans the camera; pinch zooms. The touch interpreter waits for the end of a gesture, so a drag or cancellation does not also issue a command. HUD drags cannot pan or produce world orders. Edge pan is disabled on touch devices. No duplicate mobile command toolbar is added.

Short landscape viewports use a two-column front command menu with 48 px buttons. Settings and campaign content scroll inside bounded panels. Landscape rotation guidance appears over the running canvas on portrait phones.

## Validation record

Local mobile/offline, real two-peer online and published mobile/cold-offline acceptance passed.

Validated locally before the live-render pass:

- `python3 -m unittest discover -s tools -p 'test_*.py'`: 14 passed (manifest identity, immutable retention, engine receipt integrity and the actual build-wrapper content-only reuse path).
- `tools/test_pwa_cache.cjs`: 10 real Chromium/SW cases passed.
- `tools/test_pwa_demand.cjs`: 11 cases passed, including browser-process restart offline and data-only updates without fetching unchanged engine/models.
- `tools/test_pwa_update_ui.cjs`: 17 cases passed, including pre-load language, 844×390/portrait dialogs, cancellation, explicit version choice, old-tab protection and activation during the initial worker STATUS response.
- `tools/test_touch.cjs`: 8 real CDP-touch cases passed (tap, hold, pan, cancellation, HUD isolation, safe-area offsets, pinch and menu scrolling).

The startup UI test stubs the heavyweight engine and GPU capability and does not count as a live game/rendering test. `bevy/web_asset_cache` in Bevy 0.19.1 is native-only; removing that unused web feature is cleanup, not the reason demand loading works. The reader gate establishes that guarantee.


- Native `cargo test --lib`: 338 passed, 1 ignored, 0 failed.
- WASM `cargo check --target wasm32-unknown-unknown --features webgpu --bin bevy-open-rts`: passed; final viewport-corrected release package built successfully (17m 29s Rust build, followed by bindgen/optimization).
- Actual desktop WebGPU: main menu, skirmish setup and rendered battle captured in `target/web-live/desktop-{menu,setup,match}.png`, no browser errors. First battle requested 5.3 MiB separately from the launcher.
- Actual 844×390 touch pass found and corrected a rendering/picking mismatch: CSS width 844.4 previously produced backing width 884. Final live assertion measures bitmap 844×391 against CSS 844.42×390.69; exact visible-center taps open Skirmish and Start without coordinate compensation.
- `tools/test_live_delivery.cjs` exercises visible-center touch activation, preparation cancellation, rendered battle and a fresh Chromium process with networking disabled before navigation. Its JSON report and screenshots are the live acceptance evidence.

- Final `tools/test_live_delivery.cjs`: passed with no browser errors. Real Bevy menu/setup, Back→Start→download→battle, then a new persistent Chromium process offline before navigation reached a rendered battle again. Combat demand was 5,580,955 bytes before confirmation and 0 afterward.
- Local network smoke must use allowed origin `http://localhost:8080` on IPv6 loopback. Port 8783 is not in the published signaling CORS allowlist; the unrelated IPv4 port-8080 service is preserved. No browser security flags or provider policy changes are used.

- Final startup signaling smoke passed with no diagnostics. Two actual browser peers entered the same match, observed commands, and reached HumanVictory with both reports passing (`network-match.json`).

## Hosted acceptance and CI

Published `ca742e7` passed exact revision/engine SHA verification and real visible-center mobile menu → setup → cancel → download → rendered battle → complete Chromium restart offline → rendered battle, with zero browser errors. Evidence is under `docs/verification/pwa-mobile-2026-11-16/public/`. Screenshots were visually reviewed. The hosted engine and JavaScript plus one asset per group were independently hashed; the live worker checks every actually used asset.

- Release pipeline (including startup, update/offline/touch and forced-relay two-browser match): https://github.com/xiongchenyu6/bevy-open-rts/actions/runs/43882430479 — passed.
- Signaling protocol/WebSocket/WebRTC, clippy, WASM transport and production container: https://github.com/xiongchenyu6/bevy-open-rts/actions/runs/43882430161 — passed.
- The failed earlier CI revealed an immediate-cancel/retry UI race. Back now releases its busy state immediately and late completions cannot modify a newer request. A deterministic delayed-completion test covers this ownership rule; the full 17-scenario startup UI suite also passes. A completed last file during cancellation is correctly reused instead of requiring a redundant download prompt.
- Signaling's formatting gate checks the full workspace, so its path filter now covers all Rust sources instead of omitting game-only changes.

Physical Android/iOS hardware remains untested; these are real Chromium/WebGPU games under a simulated touch viewport, not device-certification claims.
