# Steam Release Readiness

The goal is a complete, sellable RTS with a guided first session, a finishable
campaign, reliable saves, readable presentation and tested desktop delivery.
The game is **not ready for Steam submission**. All nine gates below remain open.
A bounded regression or an older package cannot establish final release readiness.

## Latest Verified Checkpoints

- Normal native M2 diagnostic17 completes Power, Factory and Radar, then
  stops before AA placement: the toast-overlapped current ghost has1274
  changed green pixels against the unchanged1500 threshold. The original
  45-second review expires; the review gate arrives85.16 seconds after the
  request and is never consumed. The adaptive Tank policy is not exercised.
  Raw `passed` remains false. Root rehashes all30 PNGs; normal close exits0,
  package/archive stay unchanged, no children remain and the GPU is released
  ([bounded failure](qa/sdk309-m2-grouping17-review.json)). Natural M2 victory
  and all nine release gates remain open.
- Current HUD source moves the objective tracker to the upper-right HUD
  column after W28 Patrol frames show Drone-body overlap. Owned-file format
  and all-target checks pass; fresh309 tests and four native builds pass.
  W29 clean/build and exact private HTTP integrity pass. Actual1280x720
  Patrol review verifies all17 motion frames keep the Drone body clear of
  the readable upper-right objective/progress panel; root rechecks25 PNG
  hashes, directly views motion/notice frames, and verifies normal exits0
  ([rendered fix](qa/web29-objective-runtime-review.json)). Populated support
  powers, other viewport/DPI/OS cases and public deployment remain open.
  W28 proofs below retain their pre-change source03c979d scope.
  OfficialSDK objective-HUD Linux packaging also passes: root independently
  rehashes144 sources,332 assets,four canonical binaries,all399 installed
  files and archive ([shipping proof](qa/native309-objective-hud-sdk-shipping-review.json)).
  Normal native M2 diagnostic16 on source `5fcf40196e3a4d12e16ee57c4a8a7b41720c9a87`
  proves six paid buildings, both outer producer Rally points and the exact
  initial eleven defenders. AA/HMG/Rocket cohorts complete Move, HQ-range
  arrival and Hold while all eight held anchors remain unchanged. The last
  three Tanks individually end Move, but never simultaneously satisfy
  idle/HQ-range across seventeen samples; one unheld Tank subsequently drifts
  beyond its 5 m range. The original 30-second cohort guard fails and raw
  `passed` remains false. The production cause is unverified; no movement fix
  follows from this diagnostic. Root independently rehashes all65 PNGs and
  directly reviews the failure image; normal close exits0, package/archive
  remain unchanged and the GPU is released
  ([bounded failure proof](qa/sdk309-m2-grouping16-review.json)). Later assault,
  natural M2 victory and six missions in one app remain unverified. All nine
  release gates remain open.
- Current worktree resolves physically blocked terrain formation endpoints. The
  genuine save-restored diagnostic changes4/5 arrivals to5/5 within the same
  30s; canonical tests now pass309/0/1ignored. All-target checking and four
  native builds pass;144 source files,332 assets and four binaries match the
  canonical pin ([bounded regression proof](qa/ground-formation-endpoint-regression-20261012.json)).
  Official-SDK309 build and packaging pass; all399 installed files are
  independently rehashed ([shipping proof](qa/native309-formation-sdk-shipping-review.json)).
  Normal-input tutorial attempt03 reaches7/7 and completes eight scout Moves,
  then stops when selected infantry acquire Attack orders during the ninth
  move; normal exit0 and unchanged package/archive are verified
  ([partial failure](qa/sdk309-bootcamp-partial03-review.json)). Graduation,
  fresh Replay and exact original blocked-endpoint GUI repair remain unverified.
  Attempt04 reaches7/7 but fails its first30s scout-arrival gate: all five
  units end idle, with two Infantry beyond the original0.3m tolerance.
  Normal exit0 and unchanged package/archive are verified
  ([preserved failure](qa/sdk309-bootcamp-partial04-review.json)); the cause
  requires trajectory evidence, so no path or arrival repair is claimed.
  Attempt05 saves27 unique trajectory snapshots: four units have qualifying
  arrivals, one Infantry is never observed within0.3m of its unchanged goal
  and ends idle0.914–1.045m away. Normal exit0 and all29 PNG/27 save hashes
  are independently verified ([preserved trajectory](qa/sdk309-bootcamp-partial05-review.json)).
  A frozen per-tick scheduled diagnostic proves genuine arrival at0.207m,
  followed by normal idle allied separation;1.1s sampling misses that window.
  This is CPU diagnosis, not GUI proof, and the stronger final-settling
  assertion is rejected as an original RED
  ([diagnostic](qa/sdk309-arrival-sampling-diagnostic-review.json)).
- HistoricalSDK309 BootCamp09 naturally earns completion: actual result
  shows Boot Camp Complete,one enemy building destroyed andzero own
  losses; original campaign profile persists training_complete:true.
  Four scout hops have all5 GUI arrival witnesses; hop04 records combat
  contact and is not counted as arrival. The harness fails when it tries
  another focus after victory; normal exit0 and all33 PNG hashes are verified
  ([actual partial victory](qa/sdk309-bootcamp-partial09-review.json)).
  Explicit visible-HQ attack input and a single uninterrupted whole-flow
  harness remain unproven. A separate fresh-process Replay on the latest
  objective-HUD SDK selects the genuine original09 user-data directory:
  current Replay Boot Camp opens lesson1/7 at64/32 with two Workers+Drone
  andzero Infantry. Root views both images, verifies unchanged campaign
  hash andnormal exit0/no forced cleanup/package unchanged
  ([actual replay proof](qa/sdk309-earned-replay-hud-review.json)).
- Previous source `4f8e588` prevents allied separation from displacing held units.
  Two genuine scheduled REDs show0.844m/0.817m drift; unchanged GREEN scenarios
  keep held translation exactly zero while friendly movement still works.
  That source’s tests pass305/0/1ignored, all-target checks and four native
  builds pass;143 sources,332 assets and four binary hashes are independently
  rechecked ([contract proof](qa/hold-separation-contract-20261012/review.json)).
  The earlier cached303 run is invalid and retained. Official SDK305 build and
  packaging now pass:399 installed files and the archive are independently
  rehashed ([shipping proof](qa/native305-hold-sdk-shipping-review.json)).
  Normal-input SDK305 held-unit traffic attempt05 passes: nine samples keep
  held translation exactly unchanged while the paid HMG moves nearby. The
  fresh baseline-to-final interval is5.195s; first-to-last samples cover4.636s.
  Actual game exit0, no forced cleanup and unchanged installation/archive are
  verified ([actual Hold proof](qa/sdk305-hold-traffic-review.json)). Attempts01–04
  remain failures. No Mission2 victory is claimed.
- Native309 optimized official-SDK audit builds with exit0 in352.444s.
  Root independently verifies its binary,144 sources,332 assets,1063 frozen
  snapshot inputs and unchanged399-file installation/archive
  ([audit build proof](qa/native309-optimized-skirmish-audit-review.json)).
  Current309 matrix coverage remains zero. A managed queue waits for the
  frozen305 matrix to finish and verify before starting current309 cases.
- The frozen native305 optimized official-SDK audit builds with exit0 in
  424.205s. Its binary,143 sources,332 assets and unchanged installed399-file
  package/archive are independently checked
  ([audit build proof](qa/native305-optimized-skirmish-audit-review.json)).
  No current305 matrix coverage or gameplay victory is established by this build.
- Previous native300 passes300 tests with0 failures and1 ignored; all three
  native checks and builds pass. Short and medium Patrol regressions reject the
  former endpoint oscillation. The official-SDK package contains399 files/332
  assets; its recorded evidence and complete installation inventory have been
  independently rehashed ([shipping proof](qa/native300-route-patrol-shipping-review.json)).
  Normal-input SDK300 short/medium starter Drone Patrol also passes:25 and26
  samples cover at least five simulation seconds each, short routes visit both
  endpoints and medium routes travel1.975m after the first turn. Nine screenshot
  hashes and trajectories are independently checked; actual game exit is0,
  with no forced cleanup and unchanged installation/archive. Raw evidence is
  [portable Patrol proof](qa/sdk300-short-medium-patrol-review.json). The bounded
  Wasm26 visual result below covers the native300 source; exact browser distances
  remain unverified. SDK296/W25 results retain their older artifact scope and all
  nine release gates remain open.
- Native296 passes296 tests with0 failures and1 ignored; all three native checks
  and builds pass. Its official-SDK package contains399 files/332 assets
  ([current shipping proof](qa/native296-toast-shipping-review.json)).
  A fresh normal-input SDK296 run verifies one Saved notice and only the latest
  Pause state; after7.5 wall seconds while paused, notices disappear and the
  entire saved game remains byte-identical. Normal exit is0, with installation
  and archive unchanged ([actual notice proof](qa/sdk296-toast-notices-review.json)).
  This establishes the specific UI fix, not full campaign or platform acceptance.
- SDK309 fresh Mission2 attempt12 verifies paid opening20/9, exact11
  initial defenders, outer Factory Rally, and both AA units’ normal
  Move-to-HQ-range then Hold. HMG formation fails because all20 macro
  candidates are blocked by OWN actors. Game exit0,77 PNG hashes and
  unchanged installation/archive are verified
  ([actual partial failure](qa/sdk309-m2-grouping12-review.json)). Later
  assault, Mission2 victory and six-mission completion remain unproven.
- SDK294 same-App campaign attempt01 naturally completes Mission1 and persists
  exactly one completed mission, then naturally loses Mission2 at04:11. It closes
  normally with one game process and unchanged package/archive. Mission3 and
  later missions are not reached ([chain outcome](qa/sdk294-same-app-chain01-review.json)).
  Fresh Mission2 income/targeting diagnostics and the six-mission acceptance remain open.
- Wasm28 builds from current native309 formation source: clean/build exit0
  in629.357s,144 sources/332 assets, exact Wasm/gzip and645 frozen inputs
  independently verified. Private HTTP serves the same27,092,940 bytes
  ([build proof](qa/web28-formation-shipping-build-review.json)). Browser
  runtime and public deployment remain pending; Wasm27 visual proofs below
  retain their older305 scope.
  Actual W28 Worker default/closer selection and17-frame recurring Patrol
  visual checks pass; notices expire after7.5s with clock exactly10.245503.
  Root verifies all29 PNG hashes and actual browser/node/driver exits0
  ([bounded browser review](qa/web28-runtime-review.json)). The objective
  banner obscures some Drone frames; no exact route distance or newHUD
  layout acceptance is claimed.
- Wasm27 builds from the previous native305 Hold source: actual clean and
  build exit0 in655.457s. Independent checks verify143 sources,332 assets,
  four frozen compile paths,642 non-web inputs, exact Wasm bytes and gzip
  equality. Protected originals remain unchanged; only a private QA alias is
  created ([shipping proof](qa/web27-hold-shipping-build-review.json)). Actual
  Worker default/normal closer-zoom selection passes actual browser review:
  same Worker, compact foot arcs, no huge ring, empty browser errors and
  supervisor/node/driver exit0 in36.132s
  ([actual selection proof](qa/web27-worker-selection-review.json)). Broader
  Long-Patrol visual review also passes all17 motion frames; transient notices
  disappear after7.5 wall seconds with paused clock unchanged, errors empty
  and all browser exits0 in80.97s
  ([bounded Patrol/notice proof](qa/web27-notices-patrol-review.json)). Some
  airborne body frames overlap the objective banner; broader UI readability,
  exact browser distances, other player flows and public deployment remain open.
- Wasm26 builds successfully from the native300 source pin. Its actual Wasm
  bytes, input manifest and641 non-web frozen inputs are independently rehashed
  ([browser build proof](qa/web26-route-patrol-shipping-build-review.json)).
  Browser attempts01/02 stop at screenshot-review deadlines before Patrol.
  Attempt02 verifies menu approval within17.31s; this proves neither gameplay
  nor browser Patrol. Both browsers close normally. Attempt03 then passes five
  timely current-image reviews, real Drone/Patrol input and17 visibly recurring
  motion frames; notices expire with paused clock unchanged, and Chrome closes
  normally in129.779s ([bounded visual proof](qa/web26-notices-patrol-review.json)).
  Browser exact-distance/ECS checks, broader player flows and release acceptance
  remain open.
- All326 active asset-ledger records independently match both the current
  worktree and exact SDK300 installation; the packaged ledger and archive
  hashes also match ([current asset identity](qa/native300-active-asset-ledger-review.json)).
  This verifies payload identity only. Generated terms, commercial rights and
  library obligations remain open.
- Fresh SDK300 Mission2 diagnostics earn36 Ore and complete six paid buildings
  for20/9, leaving80/23. The run stops before any Rally command or unit training:
  the fixed assembly proposals are offscreen, under HUD or occupied. It closes
  normally with unchanged package/archive
  ([actual bounded failure](qa/sdk300-fresh-m2-rally07/review.json)).
  Recorded OWN/public geometry identifies14 alternatives; they require fresh
  screenshot review before use. No defender, combat or mission pass is claimed.
- Original capsule/library and client-icon candidates export reproducibly, with
  directly inspected crops and small title samples
  ([capsules](../art/steam/capsules-v2/README.md), [icons](../art/steam/icons-v1/README.md)).
  Final client placement, commercial rights and actual final-build gameplay media
  remain open; promotional illustration does not count as a gameplay screenshot.
- Native294 includes the unarmed Drone Patrol fix:294 tests pass,0 fail and1
  is ignored; check and all three native builds pass. Its official-SDK package
  contains399 files/332 assets, and Wasm24 is sealed
  ([current shipping proof](qa/native294-patrol-shipping-review.json)).
  Normal-input SDK294 Patrol moves4.385m over5.409 simulation seconds and reverses
  toward its origin ([actual Patrol proof](qa/sdk294-solo-drone-patrol/review.json));
  broader campaign/tutorial acceptance on this artifact remains
  open. Previous SDK291/W22 checkpoints below retain their original artifact scope.
- The retained SDK292 Mission3 continuation ends in natural defeat at712.75s,
  with the earned campaign prefix still containing two missions. Normal exit and
  save/package integrity pass; this is a diagnostic failure to win, not fresh
  Mission3 or six-mission acceptance
  ([retained outcome](qa/sdk292-retained-m3-natural-defeat-review.json)).
- Historical SDK291 official Runtime4 passes six normal volume-control phases and
  final fresh-process reload in541.025s of the original600s limit. Twelve private
  game-only PCM sections verify channel isolation, master mute and restored output
  with no clipped samples; seven actual game exits are0 and399 installation files
  plus archive bytes remain unchanged
  ([current audio controls](qa/native291-official-runtime4-audio-controls-review.json)).
  This does not establish all17 natural cues, human mixing review or Windows/browser audio.
- The historical SDK291 archive (399 files, 332 assets) launches through the
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
- Frozen native300 matrix completes3,726/3,726 unique terminal cases with zero
  failures/integrity errors; driver and independent verifier exit0
  ([complete matrix](qa/native300-complete-matrix-review.json)). The earlier
  [2,668-case snapshot](qa/native300-matrix-partial2668-review.json) remains
  historical. Frozen305 has started after300; both precede the current formation
  endpoint fix and establish no current309 coverage or balance acceptance.
- The frozen native291 official-SDK authored-start matrix completes
  **3,726/3,726** unique terminal cases with no integrity errors or failures.
  Driver and independent verifier exit0; recorded evidence hashes are rechecked
  ([complete frozen matrix](qa/native291-complete-authored-start-matrix-review.json)).
  Subsequent movement, UI and Hold changes prevent treating that pin as current.
  This proves the frozen291 matrix only; current305 coverage, independent seeds, random maps, statistical
  balance and human gameplay remain open.
- The frozen native285 official-SDK matrix completes **3,726/3,726** cases
  within the original 600s simulation and 180s per-case wall limits. Independent
  verification accepts every terminal result, binary identity and all 332 assets.
  Highland Bastion's authored plateau height corrects the verifier's flat-ground
  assumption; three deliberately wrong coordinates remain rejected, and the
  original failed report is preserved ([matrix proof](qa/native285-full-matrix-terrain-height-review.json)).
  This does not establish current294 equivalence, independent seeds, random maps
  or balance acceptance. Historical failures below remain historical evidence.
- Exact Wasm20 verifies guest and host signaling loss recovery with two normal-input
  browsers, a private native authority and real loopback STUN. Recovery takes
  1.915s and 1.684s respectively; owned entity bindings persist, post-recovery
  commands deduct their actual price, and independent guest ECS comparisons
  have zero differences ([local recovery proof](qa/web20-local-game-reconnect-review.json)).
  Public TURN, deployed authority, eight-player recovery and complete matches
  remain unverified. The retained nonfatal host-case browser error is unclassified.
- Historical native lifecycle regressions pass 291 tests with zero failures and one
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
| First session | Fresh final-package profile completes gathering, power, barracks, training, movement, attack and victory; skip and replay work. | Historical SDK291 official Runtime4 completes all seven lessons through normal input, persists completion and starts fresh-process Replay at64/32; both game exits0 and399 files/archive unchanged ([proof](qa/native291-official-runtime4-bootcamp-review.json)). Skip is verified on the unchanged artifact ([proof](qa/native291-runtime4-bootcamp-partial-review.json)). Windows and audio remain open. |
| Campaign | All six missions through normal controls in one application; correct objectives, restart/retry, Next, finale, six exact persisted prefixes and a fresh-process final campaign menu. | Normal earlier-package Mission1/2 victories earned the retained two-mission prefix. Current real-system Mission3 paid double-AA policy wins two identical-counter repeats at213.025s ([comparison](qa/m3-real-system-strategy-root-review.json)); this is headless policy evidence, not normal-input acceptance. Current-package normal Mission3, natural-defeat Retry, all six missions in one application and final fresh-process menu remain required. See [campaign QA](CAMPAIGN_QA.md). |
| Save continuity | Fresh-process restoration of campaign triggers, orders, paid jobs, economy, fog, mines, status effects, weather and support; safe missing, corrupt and incompatible saves. | Format 4 preserves and remaps rally entity references; three focused regressions, the 258-pass suite and main/capture builds pass ([native proof](qa/native-schema4-rally-entity-review.json)). The exact optimized SDK258 package verifies normal same/fresh-process own-Worker rally restoration, paid production and real Follow behavior, with unchanged 30/16 bank and two normal exits ([player proof](qa/current258-sdk-own-worker-rally-review.json)). It also rejects missing, truncated, invalid-reference and genuine old-format saves without changing any serialized paused-scene bytes; the game exits normally ([negative proof](qa/current258-sdk-negative-save-review.json)). Normal native Ore-body clicks, paid Worker production and same/fresh-load reference remapping now pass with bank 30/16 and two normal exits ([bounded native proof](qa/native-visible-ore-rally-review.json)). The exact optimized SDK267 package now separately passes visible-Ore rally and same/fresh-load paid Worker Harvest orders ([SDK proof](qa/current267-sdk-visible-ore-rally-review.json)); A further exact SDK267 official Runtime4 check restores paid partial Power construction and a distinct gatherer across same/fresh-process loads, then naturally completes Power and raises ore29→41; both game processes exit0 and package bytes remain unchanged ([construction and income proof](qa/current267-sdk-construction-harvest-review.json)). The exact SDK267 archive also restores an active paid three-Infantry queue and both pending entries across same/fresh-process loads, then naturally produces exactly three units with bank23/14 and empty jobs; both processes exit0 ([production queue proof](qa/current267-sdk-production-queue-review.json)). A separate current267 run preserves a genuine ground rally (`target_entity: None`) and paid Infantry job through same/fresh-process loads; the naturally produced unit consumes its Move order within0.214m of the restored target, with bank25/14 and two normal exits ([ground rally proof](qa/current267-sdk-ground-rally-review.json)). Broader state coverage remains open. Delayed Harvest in the earlier fixed-pixel attempt was HQ dropoff followed by automatic retargeting, not a direct mineral click. Broader campaign state, producer/types, effects, queued targets, fog, mines, weather, support, repair and garrison on the new final artifact remain unproven. Earlier schema3 construction, queues, ground rally and position checks apply only to their frozen artifacts; see [save QA](SAVE_CONTINUITY_QA.md). |
| Interface | English default; coherent icons/tooltips; no clipping at supported desktop/browser sizes; actual selection, targeting, menus and all overlays inspected. | Current253 Wasm10 normal browser input verifies compact Worker foot arcs, selection, movement and wheel zoom at 1280×720 and 900×720. Start/Back remain fixed and one reverse wheel immediately restores body content ([browser proof](qa/web10-current253-normal-input-review.json)). The corrected native menu also passes 696px windowed clients and 720px fullscreen, including normal Back ([native proof](qa/menu-footer-scroll-green-review.json)). Current Web14 normal Host/Create/Map input separately exposes a release-blocking Big Arena eight-slot layout defect: Start/Leave are clipped and ordinary down/up wheel input does not reveal them ([actual render and input failure](qa/web14-eight-slot-lobby-footer-red-review.json)). The scrolling-body/fixed-footer correction passes two focused tests and the full275 native suite ([native correction](qa/online-lobby-footer-native-review.json)); The exact Wasm16 build now passes normal Host/Create/Map and down/reverse-wheel input at1280×720: all three footer buttons stay visible, scrolling reveals slots7/8, and one reverse event responds immediately. Root independently verified all nine recorded evidence hashes and directly inspected the rendered reverse-scroll frame; the browser closes normally within the original90s bound ([current rendered correction](qa/web16-eight-slot-lobby-footer-review.json)). This is host-only layout acceptance, not eight real peers or other viewport coverage. Actual SDK267 victory additionally exposes a stale Train Tank tooltip beside the result overlay. The existing result-update system now clears it for victory and defeat, with a real-App focused regression passing; full-suite/new-render/final-package acceptance remains pending ([focused correction](qa/result-tooltip-cleanup-focused-review.json)). These bounded checks do not cover all HUD overlays, supported DPI, OS pointer appearance or final Windows click alignment. |
| Art and audio | Complete menu, base, economy, combat, defeat/victory presentation; music, voices and SFX; volume controls, natural triggers and human mixing review. | Original portraits, surface texture, menu background, illustrations and cursors have bounded visual proofs. Voice decoding, gain tests and a natural selection acknowledgment pass. Allied support warning classification passes three meaningful regressions and the full250 suite ([correction](qa/allied-support-audio-review.json)). The exact SDK267 package now produces actual game-only PCM in an isolated output device: normal Master0% clicks silence match/cue samples, restoring100% returns nonzero PCM, with no signed16 clipping and unchanged package bytes ([output and controls proof](qa/sdk267-isolated-game-audio-pcm-review.json)). A subsequent normal-channel run exposes fresh-process volume reset: Music/SFX/Voice0 becomes SFX/Voice100 after restart, and a recorder boundary defect invalidates full cue-phase files. The semantic-stop run is preserved, with a separately closed5.045s all-zero idle sample ([actual failure](qa/sdk267-audio-channels-reset-failure-review.json)). Player preferences now have separate native/browser storage and passing overwrite/reload and invalid-file preservation regressions in the full279 suite ([focused correction](qa/player-preferences-persistence-focused-review.json)). The exact optimized native279 archive additionally passes normal channel controls across five normally closed fresh game processes in a nonroot privateUbuntu audio container: all-zero PCM is exactly silent, music/voice/SFX isolate as expected, restoration returns output, no signed16 clipping occurs, and saved volumes survive actual restarts. Root directly inspects all five Options states and independently measures closed recordings; package bytes remain unchanged ([current output and persistence](qa/native279-isolated-audio-channels-persistence-review.json)). Full official Runtime4 audio remains separate. Individual natural triggers, browser/Windows audio, human listening/mixing and complete presentation remain open. See [asset provenance](ASSET_PROVENANCE.md). |
| Skirmish and balance | All maps, factions, difficulties and starts playable; reproducible repeats and independent seeds; random-map behavior; no indefinite matches; broader balance checks. | Nine preserved historical timeout cases pass current244 rechecks with normal 32/16 resources and unchanged 600s simulation/180s wall limits. The historical native244 3,726-case sweep had an independently checked snapshot has 3,304 terminal cases: 3,299 completed, two simulation timeouts and three wall timeouts, with no integrity errors. The checker now uses each map’s exact authored path, verified against the frozen244 source; the original false path-mismatch report is retained ([checker correction](qa/current244-matrix-map-path-verifier-review.json)). Newly exposed Crossfire case3276 completes on the sealed native279 officialSDK optimized audit with unchanged opening and limits: 179.34s simulation/5.93s wall; the original244 wall timeout remains preserved ([bounded replay](qa/case3276-native279-sdk-optimized-review.json)) ([independent snapshot](qa/current244-matrix-2823-review.json)). New Big Arena Chaos Easy versus Chaos Beginner failures remain unresolved: current267 starts6/2 reaches600.008s without completion ([actual simulation failure](qa/case2668-current267-simulation-failure-review.json)); starts6/1 reaches the original180s wall bound even in a repeat without CPU quota, with no result ([actual wall failure](qa/case2641-current267-unthrottled-repeat-review.json)). The shared ground-wave correction resolves exact2668 at250.024simulation/24.471wall seconds under unchanged600/180 limits ([bounded fix](qa/ai-ground-wave-focus-review.json)). Same-core optimized officialSDK audit resolves2641 at277.087simulation/37.304wall seconds with the original32/16 resources and limits; debug wall failures remain preserved ([optimized recheck](qa/case2641-native279-sdk-optimized-review.json)). This does not prove full matrix coverage or repair the separately observed construction-contact failures. That exact case now resolves on current267 at 415.653 simulation seconds within the unchanged 600/180 limits ([fix and recheck](qa/case1885-no-path-pursuit-review.json)); the retained old244 sweep records its historical timeout. Case2220 also completes on current267 at 232.187 simulation seconds in 46.779 wall seconds under unchanged limits; its original244 repeat still reaches the wall bound, without establishing an infinite match ([exact repeats](qa/case2220-immutable-repeat-review.json)). Neither partial sweeps nor deterministic repeats prove full coverage or independent seeds. See [current rechecks](qa/current244-historical-failure-rechecks.json) and [skirmish QA](SKIRMISH_QA.md). |
| Delivery | Windows/Linux optimized archives launch from arbitrary directories without Rust/Nix; read-only installation and writable user data; Steam Runtime verified. | Current official-SDK archive has399 files/332 assets and includes local Matchbox fork notices ([package](qa/native291-trickle-sdk-package-review.json)). Exact archive launches and completes Boot Camp through official Runtime4 with read-only installation and isolated writable profile ([runtime](qa/native291-official-runtime4-bootcamp-review.json)). Windows/MSVC native installation/play, arbitrary-directory acceptance and broader final-package player flows remain open. |
| Reliability | Full suite, exact-package smoke, long sessions, representative dense-combat hardware performance and advertised multiplayer behavior. | Historical native lifecycle/transport checkpoint passes291 tests, check and three builds ([transport](qa/transport-send-before-channel-review.json), [fork](qa/matchbox-wasm-ice-trickle-source-review.json)). Exact W22 guest/host signaling recovery over real TURN passes with new paid commands and exact guest ECS ([recovery](qa/web22-real-turn-game-recovery-review.json)). W22 eight-player normal production/movement over real TURN passes; each guest matches341–378 actual ECS ticks,56 directed mapped links pass and normal closure occurs ([independent proof](qa/web22-eight-human-root-independent-review.json)). Complete combat matches, eight-player recovery, public deployed authority, broader actions, final dense-combat frame tails and long sessions remain required. Browser render interpolation is not instantaneous full-world equality. |
| Distribution | Accurate unpublished store copy, complete provenance/license obligations/generated inventory, content survey and final Steam media. | Drafts and provenance inventories exist; runtime has 332 files and the active payload ledger has 326 records, with different scopes. These counts do not prove commercial rights. Remaining library notices, generated-asset terms, final rights/rating review, capsules and gameplay media remain open. See [store draft](STEAM_STORE_DRAFT.md), [content survey draft](STEAM_CONTENT_SURVEY_DRAFT.md) and [media requirements](STEAM_MEDIA.md). |

## Current Artifact Boundaries

The current SDK300 route-aware Patrol archive has399 files and332 assets. Its archive
SHA256 is `24afda2babe97e7d3c37f5f97c2c311fd4a7355d340abae70ea5827002348685`;
the installed binary SHA256 is
`ec5aa728f8e79e3fb3e34cfb39b790bf278362042a87a424ab2237d94f614608`.
The bounded normal-input short/medium Patrol proof passes. Wasm26 is built with
SHA256 `f9df5efeb5a9323da51e80ff3e2bdeef22b2968773f53af2957039bc82994054`;
browser runtime and broader final-package player verification remain pending.
This archive does not
inherit earlier tutorial, save, campaign, audio or multiplayer acceptance.

The previous SDK296 notice-fix archive has399 files and332 assets. Its archive SHA256
is `19787ebddd97570e66d3b2576a1343a03cf66f6d875704ad84ab97ecd4a104cc`;
the installed binary SHA256 is
`165996cd616a91380a6a64785f134481ff466c8c744d34f6a35dc41f3821b1c0`.
Its normal notice proof is recorded above. Wider final-package flows remain open.

The previous SDK294 Patrol archive has399 files and332 assets. Its archive SHA256
is `76aaa6754e67d031a7ca8d41e9ad13539bc374ebef11af7ce166b47396312b6e`;
the installed binary SHA256 is
`85b1871dabf4fb7d235d499dad6d7f0fb42207491b1dff25fe0656cc9d30b8e8`.
Wasm24 SHA256 is
`bd5c48551312ecc4f77dc2a7ba8aa7db13a3f4c395acbe4eefab666a03a0f9f4`.
These new artifacts do not inherit older tutorial, audio or multiplayer acceptance.

The historical SDK291 trickle archive has399 files and332 assets. Its archive SHA256
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

SDK309 ordinary-input audio actual01 completed three voice-only recording workflows
in138.10s: owned Worker selection/two moves, paid Worker training/completion2ore,
and paid PowerReactor construction/completion3ore/1crystal. Actual game and three
private game-only monitor recorders exited0; all399 installation files and archive
remained unchanged. Independent comparison of32 shipped references against each
closed stereo48k recording yields four waveform candidates: Ack1, Ack2, Training
and ConstructionComplete. Hello and UnitReady remain below the candidate threshold;
ordinary workflow completion establishes neither their audible identity nor all17
coverage. All recordings have zero PCM16 clipping. The original silent-window
correlation result was rejected; the corrected verifier excludes low-energy windows
and passes independent-reference/silence numerical guards. Logical event identity,
full music/SFX/combat mix, human listening, Windows and browser audio remain open.
[Scoped actual01 report](qa/native309-normal-audio-review.json) pins the86-file
portable evidence archive under local `target/`; this is local evidence retention,
not a public source offer or published release artifact.

SDK309 audio successor02 repeats the same three ordinary voice-only workflows,
adds a genuine CommandCenter-to-Worker selection transition and preserves2.1s of
recording after actual paid Worker birth (shipped UnitReady duration1.6s). The
original30s completion and600s overall bounds remain. Actual controller and normal
game exit0 in143.97s; package399/archive unchanged, all three recordings unclipped.
Independent96-reference comparisons establish five waveform candidates including
UnitReady(.932865); Hello(.743153) remains below threshold and unaccepted. Multiple
normal selection voices can overlap; no production defect or logical cue identity
is inferred. Prior actual01 results remain unchanged. [Successor02 report](qa/native309-normal-audio-successor02-review.json)
pins a local portable archive including independent review. All17 natural event,
human listening/full mix and Windows/browser acceptance remain open.

SDK309 ordinary insufficient-resources audio actual01 completes sixteen paid
Worker jobs through enabled normal training cards, naturally spending32ore to0.
With actual empty queue, a further enabled Worker request leaves bank0ore/16crystal
and jobs unchanged. Root directly reviews “Not enough resources,” the2ore/0crystal
Worker tooltip and Queue0/5 in the actual screenshot. Controller and normal game
exit0 in244.95s; package399/archive unchanged, private game-only output removed.
All32 shipped references were compared against the closed7.583s stereo48k recording;
only English NotEnoughResources matches the waveform candidate threshold(.875402),
with no PCM16 clipping. [Scoped rejection report](qa/native309-insufficient-audio-review.json)
pins portable local evidence and visible rejection review. This adds one genuine
ordinary trigger with correlated output; exact logical audio receipts, remaining
rare natural events, Hello, human mix and Windows/browser audio remain open.

Frozen native305 deterministic headless matrix completed all3,726 unique cases under the original600-second simulation/180-second wall limits and32/16 starting resources. Driver and two independent verifier runs exited0; root rehashed143 frozen sources,332 assets and the exact audit binary. This historical result does not prove currentHUD equivalence, human play, rendering or hardware performance. The frozen309 formation matrix started only after305 terminal verification; its coverage is still pending. See [full305 evidence](qa/native305-complete-matrix-review.json). All release gates remain open.
