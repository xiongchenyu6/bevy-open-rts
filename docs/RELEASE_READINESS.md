# Steam Release Readiness

The goal is a complete, sellable RTS with a guided first session, a finishable
campaign, reliable saves, readable presentation and tested desktop delivery.
The game is **not ready for Steam submission**. All nine gates below remain open.
A bounded regression or an older package cannot establish final release readiness.

## Acceptance Gates

| Area | Required evidence | Latest evidence and remaining work |
| --- | --- | --- |
| First session | Fresh final-package profile completes gathering, power, barracks, training, movement, attack and victory; skip and replay work. | The exact SDK253 archive completes all seven lessons through normal input, including natural victory within 212.718s of the original 300s attack bound, skip, persisted completion and fresh-process replay; both game processes and the outer service exit 0, and installation/archive bytes remain unchanged ([bounded proof](qa/current253-sdk-package-boot-camp-review.json)). Windows final-artifact first-session acceptance remains required; the native Linux proof does not establish browser or audible output. See [Boot Camp QA](BOOT_CAMP_QA.md). |
| Campaign | All six missions through normal controls in one application; correct objectives, restart/retry, Next, finale, six exact persisted prefixes and a fresh-process final campaign menu. | The sealed current252 capture now completes all six missions in one application through normal player orders, with six independently rehashed persisted prefixes and a fresh-process rendered final campaign menu; both child exits are 0 ([bounded chain proof](qa/current252-six-mission-chain-review.json)). Its initial Mission1 entry uses legitimate scenario setup, and it predates the flash cleanup. The exact SDK253 archive separately verifies normal initial menu/card entry, including a locked-card rejection and original Mission1 settings ([normal entry and save checks](qa/current253-sdk-normal-player-review.json)). Retry/restart and complete final SDK/Windows campaign validation remain required. Earlier genuine defeats are retained. See [campaign QA](CAMPAIGN_QA.md) and [mission 3 evidence](qa/current244-mission3-survivor-victory.json). |
| Save continuity | Fresh-process restoration of campaign triggers, orders, paid jobs, economy, fog, mines, status effects, weather and support; safe missing, corrupt and incompatible saves. | Schema 3 native orders, mines, F1/F5 and active construction/production/midfade-weather scenarios have bounded fresh-process proofs. The exact245 package also rejects missing/truncated saves without changing paused scene bytes ([negative-save proof](qa/current245-invalid-save-package-review.json)). The exact245 package also rejects an incompatible version without changing the scene ([version proof](qa/current245-incompatible-save-package-review.json)). Current250 official Runtime4 basic Save/Load and normal exits pass, but actual frames and source show saved unit Y is added to authored spawn height again; exact actor continuity is contradicted ([retained review](qa/current250-runtime4-basic-save-load-review.json)). The exact-world-translation correction is committed; meaningful RED assertions fail before it, and two focused tests plus the full 252-test suite pass afterward ([regression evidence](SAVE_WORLD_POSITION_QA.md)). The rebuilt SDK252 archive now passes three normal load/resave cycles and a fresh-process load/resave: own Drone and two Workers preserve exact f32 world Y/XZ, both processes normally exit 0, and archive/install bytes remain unchanged ([bounded runtime proof](qa/current252-sdk-repeat-load-height-review.json)). The exact SDK253 archive independently repeats own-unit height checks and verifies normal paid Power construction plus distinct Worker harvesting across same/fresh-process loads, with remapped targets, preserved cost/economy/progress and natural completion/ore increase ([current artifact checks](qa/current253-sdk-normal-player-review.json)). The exact SDK253 archive also preserves three normally paid Infantry jobs across same/fresh-process loads, including producer remapping, active progress and ordered tails; natural completion produces exactly three Infantry, leaves an empty queue and keeps the paid 23/14 bank unchanged. Both processes normally exit 0 and installation/archive bytes match ([production-queue proof](qa/current253-sdk-production-queue-review.json)). This covers one Barracks and one unit type. Normal ground rally also survives same/fresh-process loads and sends one paid Infantry to within 0.213m of the saved endpoint, with the Move order naturally consumed and unchanged 25/14 bank ([ground-rally proof](qa/current253-sdk-ground-rally-review.json)). Unit/resource rally targets remain a known save defect: the schema stores ground position/mode but omits the live entity reference needed for Follow/Harvest. Other producers and broader restoration remain open. Broader effects, queued targets, repair, garrison, rally and final-package restoration remain unproven. See [save QA](SAVE_CONTINUITY_QA.md). |
| Interface | English default; coherent icons/tooltips; no clipping at supported desktop/browser sizes; actual selection, targeting, menus and all overlays inspected. | Current253 Wasm10 normal browser input verifies compact Worker foot arcs, selection, movement and wheel zoom at 1280×720 and 900×720. Start/Back remain fixed and one reverse wheel immediately restores body content ([browser proof](qa/web10-current253-normal-input-review.json)). The corrected native menu also passes 696px windowed clients and 720px fullscreen, including normal Back ([native proof](qa/menu-footer-scroll-green-review.json)). These bounded checks do not cover all HUD overlays, supported DPI, OS pointer appearance or final Windows click alignment. |
| Art and audio | Complete menu, base, economy, combat, defeat/victory presentation; music, voices and SFX; volume controls, natural triggers and human mixing review. | Original portraits, surface texture, menu background, illustrations and cursors have bounded visual proofs. Voice decoding, gain tests and a natural selection acknowledgment pass. Allied support warning classification passes three meaningful regressions and the full250 suite ([correction](qa/allied-support-audio-review.json)). All natural audio triggers, audible final-package output, human mixing and complete presentation remain open. See [asset provenance](ASSET_PROVENANCE.md). |
| Skirmish and balance | All maps, factions, difficulties and starts playable; reproducible repeats and independent seeds; random-map behavior; no indefinite matches; broader balance checks. | Nine preserved historical timeout cases pass current244 rechecks with normal 32/16 resources and unchanged 600s simulation/180s wall limits. The isolated 3,726-case sweep is running; its independently checked snapshot has 1654 completed cases with zero failures or integrity errors ([independent snapshot](qa/current244-matrix-1654-review.json)). Neither partial sweeps nor deterministic repeats prove full coverage or independent seeds. See [current rechecks](qa/current244-historical-failure-rechecks.json) and [skirmish QA](SKIRMISH_QA.md). |
| Delivery | Windows/Linux optimized archives launch from arbitrary directories without Rust/Nix; read-only installation and writable user data; Steam Runtime verified. | The exact optimized SDK253 archive passes nonroot read-only Ubuntu Boot Camp with normal exits ([first-session proof](qa/current253-sdk-package-boot-camp-review.json)). Official Runtime4 separately passes normal same/fresh-process unit-position and paid construction/harvesting checks plus normal campaign entry; installation/archive bytes remain unchanged ([Runtime4 checks](qa/current253-sdk-normal-player-review.json)). Native Windows/MSVC installation/play and complete official Runtime4 player flow remain open. |
| Reliability | Full suite, exact-package smoke, long sessions, representative dense-combat hardware performance and advertised multiplayer behavior. | Latest library checkpoint: 253 passed, 0 failed, 1 ignored; current main/capture child builds pass ([flash regression and build proof](qa/match-end-flash-review.json)). Current main/capture builds pass with unchanged inputs; the prior244 checkpoint builds all four programs. An exact245 four-Normal-AI hardware probe records 597 periodic samples averaging 59.63 FPS with sampled P95 frame time 19.90ms; background sweeps were present. This does not establish stable 60 FPS or per-frame tails ([probe](qa/current245-four-ai-performance-probe.json)). Final-package performance/long sessions and final-core multiplayer, browser peers, forced TURN, eight-player control and disconnect/reconnect remain unproven. See [performance QA](RENDER_PERFORMANCE_QA.md) and [online QA](ONLINE_HUMAN_CONTROL_QA.md). |
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
- Wasm10 verifies compact Worker markers, normal movement, briefing dismissal
  and actual wheel zoom at two sizes ([browser proof](qa/web10-current253-normal-input-review.json)).
  Software WebGPU rendering provides no hardware FPS, complete campaign or
  final-browser multiplayer acceptance.

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
