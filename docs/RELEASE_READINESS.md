# Steam Release Readiness

The goal is a complete, sellable RTS with a guided first session, a finishable
campaign, reliable saves, readable presentation and tested desktop delivery.
The game is **not ready for Steam submission**. All nine gates below remain open.
A bounded regression or an older package cannot establish final release readiness.

## Latest Verified Checkpoints

- The current SDK291 archive (399 files, 332 assets) launches through the
  official Steam Runtime 4 launcher, renders the reviewed main menu and closes
  normally with exit 0; installation and archive hashes remain unchanged
  ([current runtime startup](qa/native291-trickle-official-runtime4-menu-review.json)).
  A fresh normal-input tutorial verifies Skip and the first six lessons, including
  earned Ore, paid Power/Barracks completion, four trained infantry and movement.
  Reconnaissance is too slow to establish graduation; the unfinished attempt closes
  normally and remains incomplete
  ([partial tutorial](qa/native291-runtime4-bootcamp-partial-review.json)).
  A subsequent fresh run on the unchanged artifact naturally completes all seven
  lessons through normal input and persists training completion. A second process
  renders Replay Boot Camp and starts lesson one at64/32. Both actual game exits
  are0 and all399 installation files/archive hashes remain unchanged; the attack
  completion observation is within154.701s of the original300s bound
  ([current official Runtime4 tutorial](qa/native291-official-runtime4-bootcamp-review.json)).
  Audio and Windows first-session acceptance remain required.
- The frozen native285 official-SDK matrix completes **3,726/3,726** cases
  within the original 600s simulation and 180s per-case wall limits. Independent
  verification accepts every terminal result, binary identity and all 332 assets.
  Highland Bastion's authored plateau height corrects the verifier's flat-ground
  assumption; three deliberately wrong coordinates remain rejected, and the
  original failed report is preserved ([matrix proof](qa/native285-full-matrix-terrain-height-review.json)).
  This does not establish current291 equivalence, independent seeds, random maps
  or balance acceptance. Historical failures below remain historical evidence.
- Exact Wasm20 verifies guest and host signaling loss recovery with two normal-input
  browsers, a private native authority and real loopback STUN. Recovery takes
  1.915s and 1.684s respectively; owned entity bindings persist, post-recovery
  commands deduct their actual price, and independent guest ECS comparisons
  have zero differences ([local recovery proof](qa/web20-local-game-reconnect-review.json)).
  Public TURN, deployed authority, eight-player recovery and complete matches
  remain unverified. The retained nonfatal host-case browser error is unclassified.
- Current native lifecycle regressions pass 291 tests with zero failures and one
  ignored test. The resumed Welcome preserves an established match rather than
  re-entering scene setup ([lifecycle proof](qa/online-resumed-welcome-lifecycle-review.json)).
  A real full-ICE TURN guest reconnect subsequently panicked in Matchbox's
  missing-peer data-channel expectation. The transport now rejects premature
  sends; eight transport tests, a real local two-peer channel exchange, the
  291-pass game suite, check and three native builds pass with 124 sources and
  332 assets independently hashed ([send guard proof](qa/transport-send-before-channel-review.json)).
  W22 now passes two normal-input browser cases over real provider relay-to-relay
  routes with a private native authority: guest signaling loss resumes in1.613s,
  host loss in4.484s, within the original30s limit. Independent actual guest ECS
  comparisons match128/120 pre/post ticks for guest loss and133/148 for host loss;
  both roles execute new paid commands and final frames are directly reviewed
  ([bounded real TURN proof](qa/web22-real-turn-game-recovery-review.json)).
  The API config preserves both servers/eight URLs; the existing product adapter
  uses all six URLs of the authenticated TURN entry. Public deployed authority,
  eight-player recovery, complete matches and browser reload remain unverified.
- SDK291 fresh Mission3 verifies a visible Ore-body click, immediate owned
  Harvest, and exactly six returned Ore with six consumed from that node,
  no losses and a surviving dropoff ([early income proof](qa/native291-m3-early-earned-ore-review.json)).
  A fresh run retains the strict witness during normal paid defenses. Its later
  construction window fails amid actual combat losses; no Mission3 victory or
  complete campaign acceptance is established.
  These checkpoints leave all nine release gates open.

## Acceptance Gates

| Area | Required evidence | Latest evidence and remaining work |
| --- | --- | --- |
| First session | Fresh final-package profile completes gathering, power, barracks, training, movement, attack and victory; skip and replay work.  Current SDK291 official Runtime4 completes all seven lessons through normal input, persists completion and starts fresh-process Replay at64/32; both game exits0 and399 files/archive unchanged ([proof](qa/native291-official-runtime4-bootcamp-review.json)). Skip is verified on the unchanged artifact ([proof](qa/native291-runtime4-bootcamp-partial-review.json)). Windows and audio remain open. |
| Campaign | All six missions through normal controls in one application; correct objectives, restart/retry, Next, finale, six exact persisted prefixes and a fresh-process final campaign menu.  Normal earlier-package Mission1/2 victories earned the retained two-mission prefix. Current real-system Mission3 paid double-AA policy wins two identical-counter repeats at213.025s ([comparison](qa/m3-real-system-strategy-root-review.json)); this is headless policy evidence, not normal-input acceptance. Current-package normal Mission3, natural-defeat Retry, all six missions in one application and final fresh-process menu remain required. See [campaign QA](CAMPAIGN_QA.md). |
| Save continuity | Fresh-process restoration of campaign triggers, orders, paid jobs, economy, fog, mines, status effects, weather and support; safe missing, corrupt and incompatible saves. | Format 4 preserves and remaps rally entity references; three focused regressions, the 258-pass suite and main/capture builds pass ([native proof](qa/native-schema4-rally-entity-review.json)). The exact optimized SDK258 package verifies normal same/fresh-process own-Worker rally restoration, paid production and real Follow behavior, with unchanged 30/16 bank and two normal exits ([player proof](qa/current258-sdk-own-worker-rally-review.json)). It also rejects missing, truncated, invalid-reference and genuine old-format saves without changing any serialized paused-scene bytes; the game exits normally ([negative proof](qa/current258-sdk-negative-save-review.json)). Normal native Ore-body clicks, paid Worker production and same/fresh-load reference remapping now pass with bank 30/16 and two normal exits ([bounded native proof](qa/native-visible-ore-rally-review.json)). The exact optimized SDK267 package now separately passes visible-Ore rally and same/fresh-load paid Worker Harvest orders ([SDK proof](qa/current267-sdk-visible-ore-rally-review.json)); A further exact SDK267 official Runtime4 check restores paid partial Power construction and a distinct gatherer across same/fresh-process loads, then naturally completes Power and raises ore29→41; both game processes exit0 and package bytes remain unchanged ([construction and income proof](qa/current267-sdk-construction-harvest-review.json)). The exact SDK267 archive also restores an active paid three-Infantry queue and both pending entries across same/fresh-process loads, then naturally produces exactly three units with bank23/14 and empty jobs; both processes exit0 ([production queue proof](qa/current267-sdk-production-queue-review.json)). A separate current267 run preserves a genuine ground rally (`target_entity: None`) and paid Infantry job through same/fresh-process loads; the naturally produced unit consumes its Move order within0.214m of the restored target, with bank25/14 and two normal exits ([ground rally proof](qa/current267-sdk-ground-rally-review.json)). Broader state coverage remains open. Delayed Harvest in the earlier fixed-pixel attempt was HQ dropoff followed by automatic retargeting, not a direct mineral click. Broader campaign state, producer/types, effects, queued targets, fog, mines, weather, support, repair and garrison on the new final artifact remain unproven. Earlier schema3 construction, queues, ground rally and position checks apply only to their frozen artifacts; see [save QA](SAVE_CONTINUITY_QA.md). |
| Interface | English default; coherent icons/tooltips; no clipping at supported desktop/browser sizes; actual selection, targeting, menus and all overlays inspected. | Current253 Wasm10 normal browser input verifies compact Worker foot arcs, selection, movement and wheel zoom at 1280×720 and 900×720. Start/Back remain fixed and one reverse wheel immediately restores body content ([browser proof](qa/web10-current253-normal-input-review.json)). The corrected native menu also passes 696px windowed clients and 720px fullscreen, including normal Back ([native proof](qa/menu-footer-scroll-green-review.json)). Current Web14 normal Host/Create/Map input separately exposes a release-blocking Big Arena eight-slot layout defect: Start/Leave are clipped and ordinary down/up wheel input does not reveal them ([actual render and input failure](qa/web14-eight-slot-lobby-footer-red-review.json)). The scrolling-body/fixed-footer correction passes two focused tests and the full275 native suite ([native correction](qa/online-lobby-footer-native-review.json)); The exact Wasm16 build now passes normal Host/Create/Map and down/reverse-wheel input at1280×720: all three footer buttons stay visible, scrolling reveals slots7/8, and one reverse event responds immediately. Root independently verified all nine recorded evidence hashes and directly inspected the rendered reverse-scroll frame; the browser closes normally within the original90s bound ([current rendered correction](qa/web16-eight-slot-lobby-footer-review.json)). This is host-only layout acceptance, not eight real peers or other viewport coverage. Actual SDK267 victory additionally exposes a stale Train Tank tooltip beside the result overlay. The existing result-update system now clears it for victory and defeat, with a real-App focused regression passing; full-suite/new-render/final-package acceptance remains pending ([focused correction](qa/result-tooltip-cleanup-focused-review.json)). These bounded checks do not cover all HUD overlays, supported DPI, OS pointer appearance or final Windows click alignment. |
| Art and audio | Complete menu, base, economy, combat, defeat/victory presentation; music, voices and SFX; volume controls, natural triggers and human mixing review. | Original portraits, surface texture, menu background, illustrations and cursors have bounded visual proofs. Voice decoding, gain tests and a natural selection acknowledgment pass. Allied support warning classification passes three meaningful regressions and the full250 suite ([correction](qa/allied-support-audio-review.json)). The exact SDK267 package now produces actual game-only PCM in an isolated output device: normal Master0% clicks silence match/cue samples, restoring100% returns nonzero PCM, with no signed16 clipping and unchanged package bytes ([output and controls proof](qa/sdk267-isolated-game-audio-pcm-review.json)). A subsequent normal-channel run exposes fresh-process volume reset: Music/SFX/Voice0 becomes SFX/Voice100 after restart, and a recorder boundary defect invalidates full cue-phase files. The semantic-stop run is preserved, with a separately closed5.045s all-zero idle sample ([actual failure](qa/sdk267-audio-channels-reset-failure-review.json)). Player preferences now have separate native/browser storage and passing overwrite/reload and invalid-file preservation regressions in the full279 suite ([focused correction](qa/player-preferences-persistence-focused-review.json)). The exact optimized native279 archive additionally passes normal channel controls across five normally closed fresh game processes in a nonroot privateUbuntu audio container: all-zero PCM is exactly silent, music/voice/SFX isolate as expected, restoration returns output, no signed16 clipping occurs, and saved volumes survive actual restarts. Root directly inspects all five Options states and independently measures closed recordings; package bytes remain unchanged ([current output and persistence](qa/native279-isolated-audio-channels-persistence-review.json)). Full official Runtime4 audio remains separate. Individual natural triggers, browser/Windows audio, human listening/mixing and complete presentation remain open. See [asset provenance](ASSET_PROVENANCE.md). |
| Skirmish and balance | All maps, factions, difficulties and starts playable; reproducible repeats and independent seeds; random-map behavior; no indefinite matches; broader balance checks. | Nine preserved historical timeout cases pass current244 rechecks with normal 32/16 resources and unchanged 600s simulation/180s wall limits. The historical native244 3,726-case sweep had an independently checked snapshot has 3,304 terminal cases: 3,299 completed, two simulation timeouts and three wall timeouts, with no integrity errors. The checker now uses each map’s exact authored path, verified against the frozen244 source; the original false path-mismatch report is retained ([checker correction](qa/current244-matrix-map-path-verifier-review.json)). Newly exposed Crossfire case3276 completes on the sealed native279 officialSDK optimized audit with unchanged opening and limits: 179.34s simulation/5.93s wall; the original244 wall timeout remains preserved ([bounded replay](qa/case3276-native279-sdk-optimized-review.json)) ([independent snapshot](qa/current244-matrix-2823-review.json)). New Big Arena Chaos Easy versus Chaos Beginner failures remain unresolved: current267 starts6/2 reaches600.008s without completion ([actual simulation failure](qa/case2668-current267-simulation-failure-review.json)); starts6/1 reaches the original180s wall bound even in a repeat without CPU quota, with no result ([actual wall failure](qa/case2641-current267-unthrottled-repeat-review.json)). The shared ground-wave correction resolves exact2668 at250.024simulation/24.471wall seconds under unchanged600/180 limits ([bounded fix](qa/ai-ground-wave-focus-review.json)). Same-core optimized officialSDK audit resolves2641 at277.087simulation/37.304wall seconds with the original32/16 resources and limits; debug wall failures remain preserved ([optimized recheck](qa/case2641-native279-sdk-optimized-review.json)). This does not prove full matrix coverage or repair the separately observed construction-contact failures. That exact case now resolves on current267 at 415.653 simulation seconds within the unchanged 600/180 limits ([fix and recheck](qa/case1885-no-path-pursuit-review.json)); the retained old244 sweep records its historical timeout. Case2220 also completes on current267 at 232.187 simulation seconds in 46.779 wall seconds under unchanged limits; its original244 repeat still reaches the wall bound, without establishing an infinite match ([exact repeats](qa/case2220-immutable-repeat-review.json)). Neither partial sweeps nor deterministic repeats prove full coverage or independent seeds. See [current rechecks](qa/current244-historical-failure-rechecks.json) and [skirmish QA](SKIRMISH_QA.md). |
| Delivery | Windows/Linux optimized archives launch from arbitrary directories without Rust/Nix; read-only installation and writable user data; Steam Runtime verified.  Current official-SDK archive has399 files/332 assets and includes local Matchbox fork notices ([package](qa/native291-trickle-sdk-package-review.json)). Exact archive launches and completes Boot Camp through official Runtime4 with read-only installation and isolated writable profile ([runtime](qa/native291-official-runtime4-bootcamp-review.json)). Windows/MSVC native installation/play, arbitrary-directory acceptance and broader final-package player flows remain open. |
| Reliability | Full suite, exact-package smoke, long sessions, representative dense-combat hardware performance and advertised multiplayer behavior.  Current native lifecycle/transport checkpoint passes291 tests, check and three builds ([transport](qa/transport-send-before-channel-review.json), [fork](qa/matchbox-wasm-ice-trickle-source-review.json)). Exact W22 guest/host signaling recovery over real TURN passes with new paid commands and exact guest ECS ([recovery](qa/web22-real-turn-game-recovery-review.json)). W22 eight-player normal production/movement over real TURN passes; each guest matches341–378 actual ECS ticks,56 directed mapped links pass and normal closure occurs ([independent proof](qa/web22-eight-human-root-independent-review.json)). Complete combat matches, eight-player recovery, public deployed authority, broader actions, final dense-combat frame tails and long sessions remain required. Browser render interpolation is not instantaneous full-world equality. |
| Distribution | Accurate unpublished store copy, complete provenance/license obligations/generated inventory, content survey and final Steam media. | Drafts and provenance inventories exist; runtime has 332 files and the active payload ledger has 326 records, with different scopes. These counts do not prove commercial rights. Remaining library notices, generated-asset terms, final rights/rating review, capsules and gameplay media remain open. See [store draft](STEAM_STORE_DRAFT.md), [content survey draft](STEAM_CONTENT_SURVEY_DRAFT.md) and [media requirements](STEAM_MEDIA.md). |

## Current Artifact Boundaries

The current SDK291 trickle archive has399 files and332 assets. Its archive SHA256
is `2a6b917240812daa7c9d201db57cbe88ed49f08aded1ea99e695cc44c6e49f49`;
the installed binary SHA256 is
`f4fb85245a88307b651e22c6dabd54e53e0f38351c052c262e4f418d40b6acd7`.
Official Runtime4 tutorial evidence applies to this exact artifact.

W22 uses Wasm SHA256
`b617a80728b4586f03409b4bb68927dc736534463857f45a5fc75daebfb7449a`.
Its multiplayer proofs use a private authenticated native authority and real
provider TURN. They do not establish public deployment or Windows behavior.

Older matrix, campaign, audio and save evidence remains scoped to its recorded
artifact. Raw failures and historical proof files are retained; newer passing
attempts do not retroactively change their outcome. The latest Mission3 strategy
comparison changes no shipping stats and is not normal-input campaign acceptance.

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
