# Steam Release Readiness

The goal is a complete, sellable RTS with a guided first session, a finishable
campaign, reliable saves, readable presentation and tested desktop delivery.
The game is **not ready for Steam submission**. All nine gates below remain open.
A bounded regression or an older package cannot establish final release readiness.

## Acceptance Gates

| Area | Required evidence | Latest evidence and remaining work |
| --- | --- | --- |
| First session | Fresh final-package profile completes gathering, power, barracks, training, movement, attack and victory; skip and replay work. | Native Boot Camp completes its seven lessons, graduation, campaign transition and replay. Revalidate the actual final package. See [Boot Camp QA](BOOT_CAMP_QA.md). |
| Campaign | All six missions through normal controls in one application; correct objectives, restart/retry, Next, finale, six exact persisted prefixes and a fresh-process final campaign menu. | Current diagnostic clears missions 1–2; standalone mission 3 wins at 254.459s after the enemy captures the player's HQ. The corrected persistent six-mission chain is running. A previous chain omitted profile opt-in and cannot prove disk persistence. No current six-mission completion is accepted. See [campaign QA](CAMPAIGN_QA.md) and [mission 3 evidence](qa/current244-mission3-survivor-victory.json). |
| Save continuity | Fresh-process restoration of campaign triggers, orders, paid jobs, economy, fog, mines, status effects, weather and support; safe missing, corrupt and incompatible saves. | Schema 3 native orders, mines, F1/F5 and active construction/production/midfade-weather scenarios have bounded fresh-process proofs. Broader effects, queued targets, repair, garrison, rally and final-package restoration remain unproven. See [save QA](SAVE_CONTINUITY_QA.md). |
| Interface | English default; coherent icons/tooltips; no clipping at supported desktop/browser sizes; actual selection, targeting, menus and all overlays inspected. | Compact unit markers, native cards/tooltips and corrected mission objectives are inspected. Browser menu and four cursor CSS frames pass on an older Wasm build. Final browser/native artifacts, OS pointer appearance, click alignment and remaining overlays require review. |
| Art and audio | Complete menu, base, economy, combat, defeat/victory presentation; music, voices and SFX; volume controls, natural triggers and human mixing review. | Original portraits, surface texture, menu background, illustrations and cursors have bounded visual proofs. Voice decoding, gain tests and a natural selection acknowledgment pass. All natural audio triggers, audible final-package output, human mixing and complete presentation remain open. See [asset provenance](ASSET_PROVENANCE.md). |
| Skirmish and balance | All maps, factions, difficulties and starts playable; reproducible repeats and independent seeds; random-map behavior; no indefinite matches; broader balance checks. | Nine preserved historical timeout cases pass current244 rechecks with normal 32/16 resources and unchanged 600s simulation/180s wall limits. A new isolated 3,726-case sweep is running. Neither partial sweeps nor deterministic repeats prove full coverage or independent seeds. See [current rechecks](qa/current244-historical-failure-rechecks.json) and [skirmish QA](SKIRMISH_QA.md). |
| Delivery | Windows/Linux optimized archives launch from arbitrary directories without Rust/Nix; read-only installation and writable user data; Steam Runtime verified. | Prior Linux240 archive runs in nonroot read-only Ubuntu 24.04 with AMD Vulkan and keyboard Save/Load. Current244 optimized rebuild/package and subsequent clean input smoke are running or waiting. Native Windows/MSVC and Steam Runtime remain open. |
| Reliability | Full suite, exact-package smoke, long sessions, representative dense-combat hardware performance and advertised multiplayer behavior. | Latest library checkpoint: 244 passed, 0 failed, 1 ignored. Four native programs build with unchanged inputs. Earlier dense-combat sampling does not establish stable 60 FPS. Final-package performance/long sessions and final-core multiplayer, browser peers, forced TURN, eight-player control and disconnect/reconnect remain unproven. See [performance QA](RENDER_PERFORMANCE_QA.md) and [online QA](ONLINE_HUMAN_CONTROL_QA.md). |
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
  only frozen file timestamps and explicitly recompiles the game; its eventual
  package still requires independent integrity, runtime and gameplay acceptance.
- Clean mouse Save/Load must reopen the menu after Save, because Save closes it.
  The new smoke checks that mouse Save changes the simulation snapshot and Load
  preserves saved bytes. Preparation and service launch are not passing results.
- Wasm08 menu/cursor checks use earlier game code with current art. Software
  WebGPU rendering provides no hardware FPS or final-browser gameplay claim.

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
