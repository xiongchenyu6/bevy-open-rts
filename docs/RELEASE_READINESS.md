# Steam Release Readiness

The goal is a complete, sellable RTS with a guided first session, a finishable
campaign, reliable saves, readable presentation and tested desktop delivery.
The game is **not ready for Steam submission**. All nine gates below remain open.
A bounded regression or an older package cannot establish final release readiness.

## Acceptance Gates

| Area | Required evidence | Latest evidence and remaining work |
| --- | --- | --- |
| First session | Fresh final-package profile completes gathering, power, barracks, training, movement, attack and victory; skip and replay work. | The exact247 archive completes all seven lessons and natural victory within the original bounds. Its first outer service times out during optional menu review; a separate same-profile process normally exits after verifying persistence and replay reset ([bounded proof](qa/current247-package-boot-camp-review.json)). Current250 SDK artifacts still require full revalidation. See [Boot Camp QA](BOOT_CAMP_QA.md). |
| Campaign | All six missions through normal controls in one application; correct objectives, restart/retry, Next, finale, six exact persisted prefixes and a fresh-process final campaign menu. | The latest247 chain verifies four prefixes, then its helper aborts during still-running Mission5 after HQ capture ([retained failure](qa/current247-chain-selection-hq-loss.json)). Mission5 separated rallies win standalone. Current250 repeats the full chain with the exact helper error admitted to existing guarded survivor continuation; completion is unproven. See [campaign QA](CAMPAIGN_QA.md) and [mission 3 evidence](qa/current244-mission3-survivor-victory.json). |
| Save continuity | Fresh-process restoration of campaign triggers, orders, paid jobs, economy, fog, mines, status effects, weather and support; safe missing, corrupt and incompatible saves. | Schema 3 native orders, mines, F1/F5 and active construction/production/midfade-weather scenarios have bounded fresh-process proofs. The exact245 package also rejects missing/truncated saves without changing paused scene bytes ([negative-save proof](qa/current245-invalid-save-package-review.json)). The exact245 package also rejects an incompatible version without changing the scene ([version proof](qa/current245-incompatible-save-package-review.json)). Current250 official Runtime4 basic Save/Load and normal exits pass, but actual frames and source show saved unit Y is added to authored spawn height again; exact actor continuity is contradicted ([retained review](qa/current250-runtime4-basic-save-load-review.json)). The exact-world-translation correction is committed; meaningful RED assertions fail before it, and two focused tests plus the full 252-test suite pass afterward ([regression evidence](SAVE_WORLD_POSITION_QA.md)). Rebuilt SDK252 normal-player repeated-load and fresh-process verification is still pending. Broader effects, queued targets, repair, garrison, rally and final-package restoration remain unproven. See [save QA](SAVE_CONTINUITY_QA.md). |
| Interface | English default; coherent icons/tooltips; no clipping at supported desktop/browser sizes; actual selection, targeting, menus and all overlays inspected. | Compact unit markers, native cards/tooltips and corrected mission objectives are inspected. Current247 Wasm09 Worker markers and four-team/alliance menus are visually verified at 1280×720; the alliance menu also fits 900×720 ([browser review](qa/web09-four-team-menu-visual-review.json)). The exact247 native fullscreen four-AI/alliance menus are also inspected ([native review](qa/current247-native-four-team-menu-review.json)); its 696px windowed client clips Back and is outside the reviewed 720px client scope. Four cursor CSS frames pass on an older build. Final browser/native artifacts, OS pointer appearance, click alignment and remaining overlays require review. |
| Art and audio | Complete menu, base, economy, combat, defeat/victory presentation; music, voices and SFX; volume controls, natural triggers and human mixing review. | Original portraits, surface texture, menu background, illustrations and cursors have bounded visual proofs. Voice decoding, gain tests and a natural selection acknowledgment pass. Allied support warning classification passes three meaningful regressions and the full250 suite ([correction](qa/allied-support-audio-review.json)). All natural audio triggers, audible final-package output, human mixing and complete presentation remain open. See [asset provenance](ASSET_PROVENANCE.md). |
| Skirmish and balance | All maps, factions, difficulties and starts playable; reproducible repeats and independent seeds; random-map behavior; no indefinite matches; broader balance checks. | Nine preserved historical timeout cases pass current244 rechecks with normal 32/16 resources and unchanged 600s simulation/180s wall limits. The isolated 3,726-case sweep is running; its independently checked snapshot has 1198 completed cases with zero failures or integrity errors ([independent snapshot](qa/current244-matrix-1198-review.json)). Neither partial sweeps nor deterministic repeats prove full coverage or independent seeds. See [current rechecks](qa/current244-historical-failure-rechecks.json) and [skirmish QA](SKIRMISH_QA.md). |
| Delivery | Windows/Linux optimized archives launch from arbitrary directories without Rust/Nix; read-only installation and writable user data; Steam Runtime verified. | The optimized245 archive includes result-HUD cleanup and passes nonroot read-only Ubuntu 24.04 AMD Vulkan keyboard/mouse Save/Load without Nix mounts ([package proof](qa/linux-current245-package-smoke.json)). It predates the newer lobby labels; full gameplay acceptance remains open. The SDK-built247 archive also renders its menu on AMD Vulkan inside official Runtime4 with read-only installation ([bounded render](qa/steamrt4-sdk247-package-render-review.json)). Native Windows/MSVC, complete Runtime4 player flow and the newer250 artifacts remain open. |
| Reliability | Full suite, exact-package smoke, long sessions, representative dense-combat hardware performance and advertised multiplayer behavior. | Latest library checkpoint: 250 passed, 0 failed, 1 ignored. Current main/capture builds pass with unchanged inputs; the prior244 checkpoint builds all four programs. An exact245 four-Normal-AI hardware probe records 597 periodic samples averaging 59.63 FPS with sampled P95 frame time 19.90ms; background sweeps were present. This does not establish stable 60 FPS or per-frame tails ([probe](qa/current245-four-ai-performance-probe.json)). Final-package performance/long sessions and final-core multiplayer, browser peers, forced TURN, eight-player control and disconnect/reconnect remain unproven. See [performance QA](RENDER_PERFORMANCE_QA.md) and [online QA](ONLINE_HUMAN_CONTROL_QA.md). |
| Distribution | Accurate unpublished store copy, complete provenance/license obligations/generated inventory, content survey and final Steam media. | Drafts and provenance inventories exist; runtime has 332 files and the active payload ledger has 326 records, with different scopes. These counts do not prove commercial rights. Remaining library notices, generated-asset terms, final rights/rating review, capsules and gameplay media remain open. See [store draft](STEAM_STORE_DRAFT.md), [content survey draft](STEAM_CONTENT_SURVEY_DRAFT.md) and [media requirements](STEAM_MEDIA.md). |

## Current Artifact Boundaries

- [Current native build](qa/current244-native-build-review-20261010.json) identifies
  the library checkpoint and four native binary hashes. Later capture-only
  survivor changes are independently pinned in the mission 3 report.
- Mission 3's HQ still has 20 HP but belongs to the enemy in the retained failure
  snapshot. HQ ownership loss is not an automatic defeat in this annihilation
  mission. The runner continues existing units through normal select-all-army
  and attack-move input; original construction errors remain recorded.
- [Campaign objective regression](qa/objective-hud-regression-20261010.json)
  includes a failing test before correction, the full green suite and an actual
  native frame. End-of-match objective hiding uses existing cleanup logic.
- Current244 Linux packaging's first attempt returned success while reusing the
  prior240 binary. It is rejected as current-code evidence. Retry02 refreshes
  only frozen file timestamps and explicitly recompiles the game. Its archive
  passes integrity and basic clean keyboard/mouse Save/Load
  ([package evidence](qa/linux-current244-package-smoke.json)); complete gameplay
  and later result-HUD changes still require final-package verification.
- Clean mouse Save/Load must reopen the menu after Save, because Save closes it.
  The new smoke checks that mouse Save changes the simulation snapshot and Load
  preserves saved bytes. Preparation and service launch are not passing results.
- Wasm09 proves the compact Worker marker and current lobby labels through
  normal browser input. Software WebGPU rendering provides no hardware FPS or
  final-browser gameplay claim; the wheel input does not prove zoom changed.

## Verification Policy

Use the shared game scene and normal input for player-flow acceptance. Legitimate
scenario setup is distinct from testing the initial menu/mission-selection flow.
No QA resource/unit grants, hidden enemy targets, HP changes or timer extensions
may replace actual gameplay. Authored mission events remain part of normal play.

Inspect screenshots and recordings directly. Retain exact source, binary, archive,
asset and save identities, actual child exit codes, raw observations and failed
attempts. Do not restart a live job because an observation timed out. Compile,
formatting and meaningful regressions must pass; revalidate final artifacts after
behavior changes. Native evidence cannot prove browser/Windows behavior, menu FPS
cannot prove combat FPS, and scenario counts are not statistical win rates.

The audit currently exposes deterministic maps/starts rather than independent
random seeds. Do not relabel repeated configurations as independently seeded
matches. Keep seed coverage and random-map selection open until actually tested.

Final presentation evidence requires a fresh 450-frame sequence at 30 FPS with
opening, interactions, combat and ending inspected, plus at least five actual
1920×1080 gameplay screenshots. Complete capsule/library dimensions and export
requirements remain in [Steam media](STEAM_MEDIA.md).

All advertised features must work and the final build must launch. Store review,
content survey, rights and library distribution obligations remain required.
Steam submission needs the owner's account and explicit publication scope; the
quality goal does not authorize submission. Preserve user changes to
`flake.lock`, `web/index.html`, existing `AGENTS.md` and the ignored full font.
