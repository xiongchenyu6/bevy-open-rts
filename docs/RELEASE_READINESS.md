# Steam Release Readiness

The goal is a complete, sellable RTS with a guided first session, a finishable
campaign, reliable saves, readable presentation and tested desktop delivery.
The game is **not ready for Steam submission**. All nine gates below remain open.
A bounded regression or an older package cannot establish final release readiness.

## Acceptance Gates

| Area | Required evidence | Latest evidence and remaining work |
| --- | --- | --- |
| First session | Fresh final-package profile completes gathering, power, barracks, training, movement, attack and victory; skip and replay work. | The exact SDK267 archive completes all seven lessons through normal input in a nonroot read-only Linux input container: natural victory within 160.395s of the original 300s attack bound, Skip, persisted completion and fresh-process Replay. Both game processes and the container exit0; installation/archive bytes remain unchanged ([bounded current proof](qa/current267-sdk-boot-camp-review.json)). Full official Runtime4 Boot Camp remains separate. Windows final-artifact first-session acceptance remains required; the native Linux proof does not establish browser or audible output. See [Boot Camp QA](BOOT_CAMP_QA.md). |
| Campaign | All six missions through normal controls in one application; correct objectives, restart/retry, Next, finale, six exact persisted prefixes and a fresh-process final campaign menu. | The sealed current252 capture now completes all six missions in one application through normal player orders, with six independently rehashed persisted prefixes and a fresh-process rendered final campaign menu; both child exits are 0 ([bounded chain proof](qa/current252-six-mission-chain-review.json)). Its initial Mission1 entry uses legitimate scenario setup, and it predates the flash cleanup. The exact SDK253 archive separately verifies normal initial menu/card entry, including a locked-card rejection and original Mission1 settings ([normal entry and save checks](qa/current253-sdk-normal-player-review.json)). Retry/restart and complete final SDK/Windows campaign validation remain required. Earlier genuine defeats are retained. See [campaign QA](CAMPAIGN_QA.md) and [mission 3 evidence](qa/current244-mission3-survivor-victory.json). |
| Save continuity | Fresh-process restoration of campaign triggers, orders, paid jobs, economy, fog, mines, status effects, weather and support; safe missing, corrupt and incompatible saves. | Format 4 preserves and remaps rally entity references; three focused regressions, the 258-pass suite and main/capture builds pass ([native proof](qa/native-schema4-rally-entity-review.json)). The exact optimized SDK258 package verifies normal same/fresh-process own-Worker rally restoration, paid production and real Follow behavior, with unchanged 30/16 bank and two normal exits ([player proof](qa/current258-sdk-own-worker-rally-review.json)). It also rejects missing, truncated, invalid-reference and genuine old-format saves without changing any serialized paused-scene bytes; the game exits normally ([negative proof](qa/current258-sdk-negative-save-review.json)). Normal native Ore-body clicks, paid Worker production and same/fresh-load reference remapping now pass with bank 30/16 and two normal exits ([bounded native proof](qa/native-visible-ore-rally-review.json)). The exact optimized SDK267 package now separately passes visible-Ore rally and same/fresh-load paid Worker Harvest orders ([SDK proof](qa/current267-sdk-visible-ore-rally-review.json)); A further exact SDK267 official Runtime4 check restores paid partial Power construction and a distinct gatherer across same/fresh-process loads, then naturally completes Power and raises ore29→41; both game processes exit0 and package bytes remain unchanged ([construction and income proof](qa/current267-sdk-construction-harvest-review.json)). Broader state coverage remains open. Delayed Harvest in the earlier fixed-pixel attempt was HQ dropoff followed by automatic retargeting, not a direct mineral click. Broader campaign state, producer/types, effects, queued targets, fog, mines, weather, support, repair and garrison on the new final artifact remain unproven. Earlier schema3 construction, queues, ground rally and position checks apply only to their frozen artifacts; see [save QA](SAVE_CONTINUITY_QA.md). |
| Interface | English default; coherent icons/tooltips; no clipping at supported desktop/browser sizes; actual selection, targeting, menus and all overlays inspected. | Current253 Wasm10 normal browser input verifies compact Worker foot arcs, selection, movement and wheel zoom at 1280×720 and 900×720. Start/Back remain fixed and one reverse wheel immediately restores body content ([browser proof](qa/web10-current253-normal-input-review.json)). The corrected native menu also passes 696px windowed clients and 720px fullscreen, including normal Back ([native proof](qa/menu-footer-scroll-green-review.json)). These bounded checks do not cover all HUD overlays, supported DPI, OS pointer appearance or final Windows click alignment. |
| Art and audio | Complete menu, base, economy, combat, defeat/victory presentation; music, voices and SFX; volume controls, natural triggers and human mixing review. | Original portraits, surface texture, menu background, illustrations and cursors have bounded visual proofs. Voice decoding, gain tests and a natural selection acknowledgment pass. Allied support warning classification passes three meaningful regressions and the full250 suite ([correction](qa/allied-support-audio-review.json)). The exact SDK267 package now produces actual game-only PCM in an isolated output device: normal Master0% clicks silence match/cue samples, restoring100% returns nonzero PCM, with no signed16 clipping and unchanged package bytes ([output and controls proof](qa/sdk267-isolated-game-audio-pcm-review.json)). Individual natural triggers, browser/Windows audio, human listening/mixing and complete presentation remain open. See [asset provenance](ASSET_PROVENANCE.md). |
| Skirmish and balance | All maps, factions, difficulties and starts playable; reproducible repeats and independent seeds; random-map behavior; no indefinite matches; broader balance checks. | Nine preserved historical timeout cases pass current244 rechecks with normal 32/16 resources and unchanged 600s simulation/180s wall limits. The isolated 3,726-case sweep is running; its latest independently checked snapshot has 2,284 terminal cases: 2,282 completed, one 600-second simulation timeout in Big Arena (Chaos Easy versus Chaos Beginner, starts 2/1), and one 180-second wall timeout (Chaos Normal versus Alliance Normal, starts 4/0), with no integrity errors ([independent snapshot](qa/current244-matrix-2284-review.json)). That exact case now resolves on current267 at 415.653 simulation seconds within the unchanged 600/180 limits ([fix and recheck](qa/case1885-no-path-pursuit-review.json)); the ongoing old244 sweep still retains its historical timeout. Case2220 also completes on current267 at 232.187 simulation seconds in 46.779 wall seconds under unchanged limits; its original244 repeat still reaches the wall bound, without establishing an infinite match ([exact repeats](qa/case2220-immutable-repeat-review.json)). Neither partial sweeps nor deterministic repeats prove full coverage or independent seeds. See [current rechecks](qa/current244-historical-failure-rechecks.json) and [skirmish QA](SKIRMISH_QA.md). |
| Delivery | Windows/Linux optimized archives launch from arbitrary directories without Rust/Nix; read-only installation and writable user data; Steam Runtime verified. | The exact optimized SDK253 archive passes nonroot read-only Ubuntu Boot Camp with normal exits ([first-session proof](qa/current253-sdk-package-boot-camp-review.json)). Official Runtime4 separately passes normal same/fresh-process unit-position and paid construction/harvesting checks plus normal campaign entry; installation/archive bytes remain unchanged ([Runtime4 checks](qa/current253-sdk-normal-player-review.json)). Current SDK267 builds and passes archive integrity with exact source/assets and corrected SDK notices ([build proof](qa/linux-current267-sdk-package-build-review.json)). Normal official Runtime4 checks also pass paid Ore rally, own-Worker Follow and same/fresh-process loads, plus four failed loads that preserve the entire paused save ([Ore](qa/current267-sdk-visible-ore-rally-review.json), [Follow](qa/current267-sdk-own-worker-review.json), [failed loads](qa/current267-sdk-negative-review.json)); broader player acceptance remains open. Native Windows/MSVC installation/play and complete official Runtime4 player flow remain open. |
| Reliability | Full suite, exact-package smoke, long sessions, representative dense-combat hardware performance and advertised multiplayer behavior. | Latest library checkpoint: 267 passed, 0 failed, 1 ignored; current main, capture and skirmish audit builds pass with unchanged source/assets after the blocked-pursuit correction ([regression, build and exact-case proof](qa/case1885-no-path-pursuit-review.json)). The prior244 checkpoint separately builds all four programs. An exact245 four-Normal-AI hardware probe records 597 periodic samples averaging 59.63 FPS with sampled P95 frame time 19.90ms; background sweeps were present. This does not establish stable 60 FPS or per-frame tails ([probe](qa/current245-four-ai-performance-probe.json)). Two real browser Human peers now pass normal Host/Join/Ready/Start and guest-paid Power construction through completion on pinned Wasm13: exact 3/1 charge, 29/15 final bank, host-sent and guest-received tick44 agree, and the final rendered building is inspected ([bounded browser proof](qa/web13-normal-two-human-paid-power-review.json)). This predates the current visible-resource correction and does not independently prove the guest world. Current Wasm14 forced-relay raw observations separately show 40 exact shared snapshots and completed paid Power at 12/12 HP with bank 29/15; the overall run still exits124 without its final rendered screenshot ([bounded raw proof](qa/web14-normal-relay-paid-raw-review.json)). Final-package performance/long sessions, complete final-core multiplayer/TURN acceptance, eight-player control and disconnect/reconnect remain unproven. See [performance QA](RENDER_PERFORMANCE_QA.md) and [online QA](ONLINE_HUMAN_CONTROL_QA.md). |
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
