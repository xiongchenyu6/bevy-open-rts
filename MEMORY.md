# Open Bevy Online Notes

## Steam quality verification in progress

### Current verified state and next work

Latest construction checkpoint:
- `logs/steam-contact-full-tests.log`:164 passed,0 failed (58.63s).
- Worker contact selection checks actual static/unit clearance and a worker-sized
  routing grid, tries alternate sides, and preserves still-valid routes. Exact
  captured Power/idle-worker obstruction is covered by four real-system tests
  in `src/production/construction_tests.rs`; no teleport/cost/timeout changes.
- `logs/steam-contact-runtime.log` now completes the same factory(-28,-16),
  produces8 paid tanks and wins/continues mission index1. Evidence screenshots
  in `screenshots/steam-quality/contact-fixed`.
- `logs/steam-training-flow.log` verifies normal-input skip→Campaign→Back,
  seven lessons, graduation→Replay Boot Camp and a fresh first lesson with no
  surviving combat army. Screenshots in `screenshots/steam-quality/training-flow`.
- Current proof bundle is `screenshots/result/12`,450 frames,15s,30fps.
- Windows GNU build is genuinely running in Docker `bevy-windows-cross`,
  owned by art_audit, isolated `target/windows-cross` and Wine prefix
  `target/windows-wine-prefix`. Logs/windows-cross-build.log; no Windows success
  until executable actually runs. Do not restart a live build merely for silence.
- Release audit is writing docs/CAMPAIGN_QA.md for indices2/4 failures; current
  all-army attack may abandon HQ. Do not weaken missions to satisfy that bot.

- `logs/steam-final-quality-tests.log`: 160 passed, 0 failed (65.27s).
- Compact icon command cards (352px) pass worker selection, disabled tooltip
  and real production queue at 1280x720 and 900x720; screenshots under
  `screenshots/steam-quality/hud-1280`, `hud-900` and `hud-font-final`.
- Boot Camp uses the actual command-card attack key (E), not obsolete T. Its
  result identifies graduation and labels destination buttons accurately.
- Native profile sandbox `target/steam-profile-proof-aZvc19hY` contains actual
  `training_complete:true` and `completed:["hold_the_line"]` from normal input.
  `OPEN_RTS_CAPTURE_PLAYER_PROFILE=1` opts capture into production persistence;
  always give it an isolated absolute XDG_DATA_HOME. Defaults stay in memory.
- Current `campaign-play` captures pass indices 0, 3 and 5. Index 1's tank
  factory stayed unfinished, indices 2/4 lose. Prior index1 infantry passed.
  Construction failure diagnostics now print worker/target/path state; build
  the latest capture and rerun index1 to diagnose, not bypass, this failure.
- Survival missions use authored waves rather than additional skirmish AI.
  Protect rear power, front turrets, and add armour before the final wave.
  Index3 passed five minutes with normal paid tank production after supply.
- Twin Front now places enemy main force diagonally opposite and timed waves
  on both flanks. Campaign annihilation counts all living units/structures.
- Profiles retain verified corruption backups, block unsafe overwrites on
  backup failure, and retry once per new victory without per-frame writes.
  Native I/O tests live in `src/campaign_progress/tests.rs`.
- Official-source font now includes en/zh_CN CSV catalogs, not Rust-only glyphs.
  Current SHA: e4be13e4f6c1f226175906922777d5aeede8408601dbd7cdd92480ba5773c0af.
  Rebuild with `scripts/subset_font.sh`; the ignored full font is untouched.
- Packaging and asset audits remain finite open gates in RELEASE_READINESS.
  Windows scripts/workflow are authored but not executed; no Windows claim.
- `screenshots/result/11` is the current compact-HUD 450-frame/15s/30fps proof.
  Final-assets Linux archive is `/tmp/rts-linux-final-assets-20261008/`
  `bevy-open-rts-linux-x86_64.tar.gz`, hash
  5592a4575619cfbbf79a4dccc6c4fab1c3e510b4cc744d228ac814e1b2baf121.
  Exact read-only/no-Nix startup evidence is `readonly-result.txt` there.
- Construction diagnostics build was terminated with143; terminal confirmed,
  retry started with a PTY. Do not claim the diagnostic reproduction complete
  until the new binary runs. Current native game/UI binaries and160 tests are
  already verified; the extra diagnostic helper changes no gameplay behavior.
- PTY diagnostic rebuild succeeded and `logs/steam-construction-repro.log`
  reproduces index1. Factory target(-28,-16), selectable radius1.5, remaining1.
  Worker529 remains at(-23.94,-17.40), still ConstructOrder(target1896), requests
  contact(-26.04,-17.39), path endpoint(-26.25,-17.25). Nearby power at(-26,-19.5)
  and idle worker526(-25.11,-17.64) obstruct that approach. Investigate reachable
  alternate contact approaches in update_construct_orders/orders/NavGrid, not
  simply raising the capture timeout. Repro image is campaign/1/failure.png.

- New `capture campaign-play <zero-based-index> <dir>` harness exercises
  economy/build/train/combat, checks campaign completion, then clicks the real
  Next Mission/finale button. It does not change resources, AI, clocks or armies.
  Mission 2 baseline passed (`logs/steam-campaign-1.log`); Mission 1 baseline
  lost at 01:08 because the army was not brought back to defend headquarters.
  The survival harness now sends its first six troops back before finishing
  production. Difficulty has not been changed based on this failed strategy.
- Campaign annihilation previously counted only headquarters/workers. The
  evaluator now requires every enemy unit/structure to fall; survival victories
  require the authored timer and run after PostCombat defeat evaluation.
  New tests cover remaining army, remaining structures, timer and defeat.
- Failure screenshot showed production cards covering result buttons. The
  command stack is now hidden once the match ends; runtime recheck pending.

- Offscreen capture UI focus needs a window render target during
  `UiSystems::Focus`; capture restores its image target before rendering.
  This now lets the actual menu Boot Camp button enter the match.
- `screenshots/steam-quality/boot-camp/02_gathered.png` proves delivered
  harvesting advances the tutorial. Full graduation is still unverified.
- Moving harvesters invalidate a cached click position during camera-settle
  ticks. The build harness now queries their current position before clicking.
- Hide the briefing reopen button while training is active; it previously
  overlapped the tutorial title. Fresh screenshot verification is pending.
- The current full library run is logged in
  `logs/steam-quality-tests-current.log`: 146 passed, 0 failed (67.75s).
- Boot Camp initially referenced nonexistent `PowerSupply`; registry and
  command card use `PowerReactor`. Both tutorial and harness now use the real ID.
  A capture has built the reactor; allow the following UI update to observe
  removal of `UnderConstruction` before asserting the next stage.
- Full real-input Boot Camp capture now passes all seven lessons. Evidence:
  `logs/steam-boot-camp-current.log` and `06_graduated.png` in
  `screenshots/steam-quality/boot-camp`. Reactor/barracks, four infantry and
  enemy HQ destruction were issued through normal command-card/input paths.
- `screenshots/result/9` contains the current 450-frame / 15-second / 30-fps
  proof bundle. Opening and combat inspected; movement 4.24m, siege 113 damage.
  Remaining visible polish: wide unit health bars can overlap the objective
  panel; battle notices still resemble debug overlays; HUD action text remains
  crowded. These are open release-quality issues, not accepted final visuals.

- Bevy Open RTS uses Bevy 0.19 and runs its simulation in normal `Update`
  systems with floating-point transforms and timers. It is not deterministic
  enough for peer lockstep without a much larger simulation rewrite.
- Existing orders contain local Bevy `Entity` handles. Online commands need
  stable network IDs and host-side resolution/validation.
- `matchbox_socket` 0.14 supports native and wasm WebRTC with multiple reliable
  and unreliable channels. `bevy_matchbox` currently targets an older Bevy, so
  the project integrates `matchbox_socket` directly.
- Matchbox's standard wire format is `JsonPeerRequest`/`JsonPeerEvent` from
  `matchbox_protocol`. The custom Open Bevy server retains this format so the
  stock socket client remains compatible.
- The signaling container was built and run locally on 2026-07-24. Docker
  reported it healthy; `/healthz`, `/metrics`, and `/v1/config` were exercised.
- Browser WebSocket clients cannot attach an Authorization header. Connection
  tickets therefore use query parameters; production proxies must use WSS and
  must not persist request URIs.
- `open-bevy-net` uses `ehttp` for one HTTP API on native/wasm without requiring
  a Tokio runtime in Bevy. Its Matchbox message-loop future is executor-neutral
  and can run on Bevy's task pool.
- The universal signaling API and game packet compatibility are separate
  version domains. `session_protocol` must match the service contract;
  `game_protocol` is any non-zero game-owned value used in room discovery and
  paths. `open-bevy-net` therefore must not hardcode an RTS game ID.
- A real native WebRTC integration test starts the custom server, creates a room
  through `RoomServiceClient`, connects two stock Matchbox sockets with host ICE
  candidates, and verifies both reliable and unreliable payload delivery.
- The signaling service keeps the host ticket valid and preserves connected
  players for a configurable 30-second host reconnect window. A resumed host
  receives a new Matchbox peer ID, so game-level identity must use its own
  stable session key rather than treating `PeerId` as player identity.
- `open-bevy-signaling` now has protocol-compatible native and Cloudflare
  deployment targets. The Worker uses `RoomDirectory` plus isolated `GameRoom`
  Durable Objects with hibernatable WebSockets. Both targets request short-lived
  Cloudflare Realtime TURN credentials without exposing the long-term key or
  API token.
- `src/online.rs` now provides the first real game integration slice: public
  room discovery/create, code/token join, reconnect-aware player identity,
  exact map-derived slot rows, host-validated lobby options, readiness, and a
  synchronized transition into the existing match setup.
- Running online matches now have stable `NetworkEntityId` components and a
  first host-authoritative replication path. The host alone advances economy,
  build, AI, combat, death, and victory systems, then broadcasts 10 Hz world
  snapshots over the unreliable channel. Clients reconcile replicated entities,
  economies, and match state by stable ID and interpolate transform corrections.
- Snapshot receivers only accept the negotiated host, current RTS protocol, and
  monotonically newer ticks. A postcard roundtrip test keeps a representative
  eight-player/512-entity snapshot under the 64 KiB channel limit.
- Unit orders and structure rally points now use bounded, sequenced reliable
  commands containing stable network IDs. Host-local input uses the same inbox
  as remote input, and the host validates protocol, replay sequence, ownership,
  capabilities, target relations, liveness, and map bounds before mutating the
  authoritative simulation. A maximum 256-unit order batch fits the 256 KiB
  reliable-channel budget.
- Training, production cancellation, and structure placement now use bounded,
  sequenced reliable commands with stable producer and Worker IDs. The host
  validates ownership, faction production relationships, technology, queue
  capacity, resources, placement legality, and construction capability. Exact
  charged costs drive cancellation refunds, and authoritative build queues are
  mirrored in snapshots for client HUD progress.
- Remote human teams must never be inferred as AI merely because they are not
  the locally controlled team. Automatic construction now checks the configured
  slot controller explicitly, so only real AI slots progress foundations
  without an assigned Worker.
- RTS wire protocol v3 is command-complete for current player-facing match
  controls. Stop/hold/guard/scatter/deploy, sell/repair/cancel-construction, and
  all support powers use the reliable host inbox. The host validates stable-ID
  ownership and capabilities, prevents same-tick duplicate structure refunds or
  repair charges, validates support doctrine/tech/power/cooldown/bounds, and
  mirrors support cooldowns in snapshots. Maximum action packets and a
  representative cooldown-bearing snapshot remain below channel limits.
- A reconnect Welcome no longer sends a client back to the lobby during a live
  match; the client keeps `InMatch` state and catches up from the next accepted
  authoritative snapshot. Snapshots carry active anchor teams and a finished
  flag. The host waits until no hostile sides remain, while each client derives
  its own victory or defeat from alliance perspective, so eliminating the host
  does not prematurely stop remaining opponents.
- Remote players receive a 30-second game-level reconnect grace after their data
  channel disconnects. Rejoining with the stable session key cancels the timer;
  expiry permanently forfeits that player for the current match, removes its
  match-scoped entities and pending paradrops, and clears its production queue.
  Unknown late joiners and already-forfeited identities are rejected while the
  match is running.
- Returning to the online lobby is host-authoritative. The host can abort to the
  lobby, while client requests are accepted only after the global result. The
  host broadcasts a reset lobby snapshot, retains connected identities, resets
  readiness, and reclaims disconnected/forfeited player rows. Online restart and
  client-side speed changes are disabled to prevent divergent local state.
- RTS protocol v4 replaces raw full-state broadcasting with a one-second
  compressed keyframe plus independently recoverable 10 Hz deltas. Every delta
  references the latest full keyframe rather than the previous unreliable
  packet, carries changed/new entities and removals, and is ignored until the
  exact keyframe exists. The reusable `open-bevy-net` snapshot envelope applies
  LZ4 only when smaller and validates the declared decoded size before
  allocation. A 2,048-entity test exceeds 64 KiB before compression and fits the
  data channel after encoding.
- Shot pulses, impact bursts, support warnings, structure destruction, and
  veterancy promotions now receive host event IDs, remain present in snapshots
  while alive, and are deduplicated by clients. This gives dropped unreliable
  packets another chance to carry the visual event without replaying it when
  multiple packets arrive.
- The game now has an opt-in two-client product verification path rather than a
  signaling-only smoke test. A run-tagged host and player use the real public
  room discovery, lobby readiness, match transition, reliable unit-order
  validation, authoritative snapshots, and victory flow. Native clients write
  atomic JSON reports; browser clients expose the same report in a hidden DOM
  element for Playwright. A 2026-07-25 native run completed room `B7116009` at
  snapshot tick 14 with host victory and player defeat.
- Client admission cannot rely on a one-shot `Hello`/`Welcome` exchange. A real
  two-process run showed the host admitting the player while the first Welcome
  was not observed by the client. Clients now retain the candidate host peer
  and retry the same stable-session Hello once per second until admission;
  host admission is idempotent through the existing resume-key path.
- Directly launching `target/debug/bevy-open-rts` does not inherit Cargo's
  `CARGO_MANIFEST_DIR`, so Bevy otherwise looks under `target/debug/assets`.
  Native verification sets `BEVY_ASSET_ROOT` explicitly; normal `cargo run`
  behavior is unchanged.
- Product verification now uses the same role-driven browser harness for both
  the two-browser CI gate and native/browser compatibility. A deployed-Pages
  browser player and a locally built native host completed run
  `cross-1784925664-1332` in room `366BE938`: both reported two connected humans,
  snapshot ticks 10/9, the browser's reliable move command was observed on both
  sides, and the authoritative results were browser defeat/native victory.
  Browser startup is retried only before entering a room because the live Pages
  path can transiently expose the unsupported/loading fallback; match failures
  are never retried. Terminal reports are frozen on first publication so peer
  shutdown cannot rewrite the player count after successful completion.
- TURN is verified at the selected-route level, not inferred from configuration.
  Playwright can force `iceTransportPolicy: relay`, retain the actual browser
  peer connections, and inspect their selected candidate pairs after the match.
  Run `browser-1784925977685-56195672` completed room `937CFB2A` with host/player
  snapshot ticks 10/11 and complementary results. Both selected pairs were
  `relay` to `relay` over UDP; each side reported the same 6,652/2,873 bytes in
  opposite sent/received directions. The Pages workflow now requires this
  forced-relay full-match gate on every release.
- `OpenBevyGameClient` is the reusable SDK entry point for another Open Bevy
  title. It binds one service client to a validated `game_id`, local `build_id`,
  and non-zero game protocol, then scopes room create/list/get calls so game
  code cannot accidentally query another namespace. Bevy Open RTS uses this
  same adapter. Player/spectator transport now requires the caller's local
  build ID instead of copying the room host's build, restoring the server's
  pre-upgrade compatibility rejection. The native `universal_room` example and
  `docs/OPEN_BEVY_GAME_INTEGRATION.md` are the onboarding surface.
- The SDK onboarding executable was run against production as namespace
  `(open-bevy-sdk-proof, 1)` with build `0.1.0+sdk`; it resolved the deployed
  `open-bevy-signaling` service and public WSS endpoint. After migrating RTS to
  the scoped client, native/browser run `cross-1784926999-637` completed room
  `94423B34` with snapshot ticks 9/10 and complementary authoritative results.
- The `sg-office` Podman bridge advertised its embedded resolver at
  `10.89.0.1`, but DNS queries from the signaling container timed out while the
  host resolver remained healthy. The native deployment therefore makes
  container DNS explicit (Cloudflare DNS by default, operator-overridable) so
  short-lived Cloudflare TURN credential issuance does not depend on Podman's
  bridge DNS forwarder. Its explicitly named Compose network also uses
  `x-podman.disable_dns`, allowing Podman to honor those service-level resolvers.
- GitHub's two-browser SwiftShader gate can spend roughly 136 seconds entering
  both match scenes. The verification harness now allows 240 seconds internally
  (the workflow allows 300) and submits the player command as soon as an initial
  replicated unit exists, including on snapshot tick zero. This preserves the
  real full-match/relay assertion without failing before the first actionable
  match frame.

## Verified battlefield art pass — 2026-10-06

- Matte steel panels use assets/textures/armor-painted-steel.png (1254 square).
  Imported kit UV scale 0.08 avoids granular repetition; neutral armor roughness
  is 0.62, vehicle Paint 0.78, team paint 0.68. Team paint preserves hue with
  restrained brightness; its emission is limited to small unit accents.
- Soil uses sediment beds, compacted patches, mineral grain and a 2048 detail
  image. Cool ambient fill and slightly warm sun preserve readable dark armor;
  weather owns brightness (clear 1000/cloudy 850/rain 750/dust 850).
- structure_details.rs adds cached roof vents, conduits and foundation lips.
  Geometry audit: plain roof y1.0, conduit x1.75/2.25; window-center y0.85.
  Detailed/corner modules keep their authored ribs and gaps. Parents have Visibility.
- Material recoloring works in renderer-less fixtures; only image loading and
  decorative geometry need RenderDevice. The existing material test now checks
  matte team hues, unchanged colored parts and immutable shared source materials.
- Live and offscreen scenes share configure_scene_lighting, including 4096 shadow
  atlas. The isolated gallery does not run live faction material processing;
  use real match captures for surface verification.
- Verified: cargo fmt/check/build, all 137 library tests (0 failures), and a real
  Wayland native launch for 12s with no error output (intentional timeout 124).
  Logs: art-tests-fixed.log, art-build-checked.log, art-check-fixed.log and
  art-native-verified.log under logs/.
- All three factions train/build through real input successfully; captures under
  screenshots/art-revision/final-factions. Rain and sandstorm captures also inspected.
- Final proof: screenshots/result/7/video.mp4 + 450 raw frames, exactly 30 fps/15s.
  Bounded base movement 4.24m/150 moving frames, siege 113 damage/94 effect frames.
  Inspected opening, motion, battle and final frames. Earlier bundles are iterations.
- The bundle script accepts CAPTURE_BIN to pin a prebuilt executable while other
  workspace builds run. A matching native binary is kept in target/art-verification.
  Use RUSTC=/nix/store/yhmi70ln28n1j6wn82h61b8r8q4g562i-rustc-1.95.0/bin/rustc
  to reuse this host's compiler cache; default shell compiler is newer.
- Historical screenshots/art-before.png uses an older capture binary and shadow
  settings, so it is an art reference rather than a controlled lighting comparison.
- No web publication was performed by this art pass.

## Compact selection feedback — 2026-10-07

- Worker display radius 0.34 replaces the old 0.78 full ring; four short arcs
  hug the footprint. Infantry also gets compact markers. Vehicles retain a
  suitable footprint; aircraft use a small ground cross rather than a duplicate ring.
- Small-unit health bars use display size; hit-testing, nav and combat radii are unchanged.
- Build/check/fmt and 2 selection tests pass. Real input captures verify move 3.87m,
  ore 32→44, training and completed construction. Worker close-up:
  screenshots/selection-compact/harvest/03_collecting_closeup.png.
- Final media: screenshots/result/8, 450 raw frames and 15s at 30fps.
  Logs: logs/selection-{build-all,check-final,tests,play,harvest,proof}.log.

## Save/input and desktop checkpoint — 2026-10-08

- Real Ctrl+S/Ctrl+L acceptance harness lives in src/bin/capture/save_flow.rs.
  Requires an explicit disposable absolute XDG_DATA_HOME and refuses existing
  quicksaves. Missing/corrupt load leaves paused observations unchanged and
  corrupt file bytes intact. Ordinary paid production/economy/fog restore;
  logs/steam-save-friendly-runtime.log and screenshots/steam-quality/save-friendly.
- Loading suppresses opening briefing. Save read errors now explain missing,
  damaged/incompatible files without OS/parser internals. README corrected actual
  shortcuts Ctrl+S/Ctrl+L (previous F5/F9 documentation was wrong).
- A real campaign control failure exposed Barracks S/Stop collision when mining
  removes idle-worker command. Grid reserves S/H/G/X/I/M/P and group digits;
  added regression. Campaign tab keys must be observed, not hardcoded T/Y.
  New defense/assault bot remains an acceptance experiment; no win claim yet.
- Integration fmt/check/bins build and 166 library tests pass. Native12s launch
  exits124 intentionally with empty log. Proof13:450 frames,30fps,15s, movement
  4.24m,113 siege damage,94 effect frames; opening/motion/battle/end inspected.
- Font regenerated twice identically:1219 codepoints,215372bytes,
  SHA4b9451969be0c79fe523fe7132813ab1ad5bda36efab7929e5b596570e65ef7f.
  All prior1213glyphs/advance widths retained;557 asset/catalog hashes verified.
  User ignored fullTTF and flake.lock remain untouched.
- Windows GNU game/capture actually compile and menu renders under Wine on Vulkan.
  Final package excludes ignored full-font source; clean hardware/MSVC/release
  performance still open. Linux exact archive save-flow acceptance in progress.
- Two campaign2 invocations omitted BEVY_ASSET_ROOT and searched target/debug/assets,
  hitting the30s startup limit before gameplay. Corrected invocation supplies the
  absolute repo asset root; no timeout padding or gameplay changes were needed.

### Verified desktop artifacts and next campaign action

- Linux corrected archive: /tmp/rts-linux-clean-checkpoint-20261008/bevy-open-rts-linux-x86_64.tar.gz,
  SHA3131ca76b1665e3d1868ded36fe2f2ab383a66af66fcb6ac05d4b887488bb245.
  Exact no-Nix/read-only /tmp startup and matching separate save-load harness
  passed; audio device absent in container, so audio acceptance remains open.
- Windows GNU DEBUG archive: dist/bevy-open-rts-windows-gnu-x86_64-debug.zip,
  SHA23819149bf202a08eaa29ba7e43dd76804c129b3d8773a80ce090997d8d62bcb.
  CRC/provenance inventory equality pass; Wine menu/read-only save recovery pass.
  Actual native Windows/MSVC/release performance remain unverified.
- Commits: b8113d6 source/save/hotkeys;1c03ecd Linux packager;
  c577845 cross-platform evidence;47bf770 Windows delivery.
- Campaign2 correctly rooted run reached real Barracks/turrets/factory completion,
  then bot failed selecting HMG2176v4 at63.5s. Tall AABB-center click selected a
  neighboring infantryman in crowded defense. Bot-only patch now clicks unit
  and producer ground projections. No product/AI/economy changes.
  Agent release_audit owns that uncommitted patch and live capture-only build
  session39831/logs/steam-campaign-foot-selection-build.log, then campaign2/4 runs.
  That build accidentally uses default rustc1.98.1; it is live and must not be
  restarted because observation times out. Future builds should reuse1.95 cache.
  Do not claim either remaining mission passed until actual result/continuation.

## Mouse save menu — 2026-10-08

- Desktop offline Esc menu now has Save/Load buttons showing Ctrl+S/Ctrl+L.
  Menu queues one request in MatchMenuState; existing save handlers consume it
  after button input and setup clears requests. Browser omits disk buttons;
  online disables them. Offline return label is Main Menu; online Return to Lobby.
- capture save-menu DIR WIDTH runs actual mouse save/restore/rejected loads in
  a disposable explicit XDG sandbox, then clicks Main Menu and verifies destination.
  Current final1280/final900 screenshot sets inspected; both complete successfully.
  Logs: steam-save-menu-final-{1280,900}.log. Production/economy/fog recover;
  corrupt/missing load leaves paused state/file unchanged. No compatibility path.
- Final fmt/check/bins build,166 library tests56.23s and native12s smoke pass.
  Proof14:450 rawframes,15s30fps, movement4.24m, siege113damage/94effectframes;
  opening/movement/combat/ending inspected.
- Product source frozen for standard release build. Initial nohup wrapper was
  authoritatively missing with no compiler, so durable user service replaced it:
  rts-linux-standard-release-20261008.service, confirmedMainPID2034332.
  Exact Rust1.95 cargo build --locked --release --jobs2 --bin bevy-open-rts.
  logs/steam-standard-linux-release-20261008.log advances through dependencies;
  service/log/exitfile must be checked, not presumed complete. hud_polish supervises.
- Campaign raw keys/build/grouping now work. Index2 actual t94.2 HQ loss remains
  combat/strategy investigation; index4 failed unchecked builder selection at39.2
  withample76/29resources/normalpower, HMG card selected. release_audit owns bot
  builder verification/setup telemetry/earlier Factory experiment. No game nerf.

## AA construction fallback verified — 2026-10-08

- Exact live Nav/geometry dump is campaign-contact-fixture-2/failure-contact.json:
  grid120x120/origin-30/version14; source open but mobile-unit inflation seals
  65cellcomponent. Resources204/240 are alive; Drone far away. Minimal6statics/
  3mobileallies reproduce realRust RED, with static-only route sanity passing.
- orders reachable_contact keeps mobile-aware routing preferred, retries static
  worker-inflated route if no candidate reachable. All endpoints still checked
  against occupied units; actual movement collision/yield is used, no teleport.
- 6constructiontests pass (steam-aa-contact-tests-mobile.log); full168 pass59.35s
  (steam-aa-contact-full-tests.log), check/build/fmt/native12s emptylog pass.
  Fixtures use real registry ally speeds, preserve Resource entities, and assert
  physical footprints rather than planner-only buffer after normal separation.
- Normalinput old AA(-22,-13) now completes49.3. Mission3 victory183.7/HQ20 with
  completion/nextbriefing inspected: campaign-contact-fixed-2. Ordinary autochase
  took tanks east (x+18); no explicit assault logged; resolver requires all living
  hostile units/structures gone. Mission5 victory257.8/HQ20/zero building losses,
  nextMission6 inspected: campaign-twin-guard-4. Bot commits a407376/60cb4a8;
  no resource/time/AI grants or level nerfs. All6 recipes have recorded winproof,
  complete current sequential/persistent campaign audit still needed.
- Proof15:450frames,15s30fps, movement4.24m,113damage/94effectframes; opening/
  movement/combat/ending inspected. Its baseline clip is not the AA-specific
  proof; constructor movement tests and normal-input runtime provide that scope.
- StandardLinuxrelease34m53s fatLTO was globallyOOM-killed; no executable.
  Evidence /tmp/rts-standard-release-failed-20261008. sg-office configuredSSH
  unreachable, serverregistryempty; no credentials/sourcecopy used. Warmcache
  unchangedflags retry awaits sourcefreeze and ~9GBstablememory, hud_polish
  supervising. Do not restart because observationtimeout or labeldebug asrelease.
- Sixaudio files regenerated/reproduced3runs+actualBevydecoder, ab9826a. Player
  guide+allpackagerREADME/BUILD_NOTES changes99353d3. Human listening/openlicense
  voices/art/performance/hardware gates remain; currentarchives are checkpoints.

### Release retry handoff

- Core verified/frozen commit b4160bd. Warm standard-profile retry launched on
  that source with ~8.77GiB available; no source delta, only user flake.lock dirty.
- Unit rts-linux-standard-release-retry1-20261008.service remains active,
  MainPID2258416. Final fat-LTO rustc2264620 was SIGSTOP'd by hud_polish to avoid
  growing allocations while another project compiles; root verified process
  state T and~3.2GiB RSS. Resume the SAME compiler via SIGCONT after headroom
  recovers. No kill/restart or goal pause; release executable still unproven.
- hud_polish owns supervision, final package/playerguide/hash verification and
  clean-container/AMD launch after actual artifact exists. All campaign captures
  and other agent tasks are terminal; all owned files committed. Six current
  mission recipes have wins at recorded revisions; final persistent/current
  complete sequence, balance/performance/audio/art/provider/browser gates remain.

## Campaign save and current profile sweep — 2026-10-08

- Capture-only save-mission added (no product/library edits). Actualmouse first-
  wave save/load passed exit0, restored screenshot inspected; observations.json
  retains hold_the_line/fired[true,true,false,false,false]/9livingunits/6222fog,
  41ore22crystal/one paid job vs changed40ore/twojobs.8normalUI/restartframes
  elapsed, no duplicated wave/briefing. Logs steam-save-mission-{check,build,runtime}.
- Currentprofile baseline0 failedactual198s (legacy no armor/refills);1 won110s
  and actualcampaign.ron strike_the_outpost;2 observedvictory+persistedcanyon_siege
  before wrapperSIGTERM143 (exitambiguity preserved);3 actuallysurvived300s and
  persistedhighland_bastion_stand but helperfalseerrored armyquota13/14 and skipped
  continuation. Remaining4/5 baseline supervised current-campaign-profile-baseline
  service; release_audit owns survival type-quota/terminal correction after sweep.
  Fresh sandbox target/current-campaign-profile-lium377c. Nextcoherent0→5mustpin
  executable andverifycomplete restart/menuunlocks; directentrybypass acknowledged.
- Standardfat retry1 also OOM-killed (whilepausedresidentIR); preserved at
  /tmp/rts-standard-release-retry1-failed-20261008, no executable. No thirdblind
  standardretry. hud_polish authorized final-bin-only thin candidate after actual
  Rust1.95flagprobe acceptedfat→thin/cgu1→8/LLVMthreads1. Currentunit
  rts-native-thin-candidate-20261008.service PID2300414/rustc2300470, actualargs
  captured. Cargoprofile/coresource unchanged; candidate is NOT standardfat proof.
  Samefeature/perf/hardware/packagegates stay open. sg-officeunreachable, no creds
  orsourcecopy. Userflake.lock remainsunstaged/unmodified.
