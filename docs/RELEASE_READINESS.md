# Steam release quality plan

The goal is a complete, sellable RTS: a new player learns through play, finishes
a meaningful campaign, can save and resume, and installs a reliable desktop
build. Previous visual fixes and green unit tests do not establish that goal.

## Acceptance gates

| Area | Required evidence | Current state |
| --- | --- | --- |
| First session | Fresh profile → guided gathering, power, barracks, training, movement, attack and victory through normal controls; skip/replay work | Seven lessons and sandbox persistence pass; normal-input skip/return/graduation/fresh replay captured |
| Campaign | All six missions complete without cheats; correct objectives, retry/next/finale; completion survives restart | Missions1/2/4/6 win and continue; mission1 unlock survives restart; missions3/5 strategy/failure audit remains |
| Save continuity | Mid-mission save/restart/load preserves triggers, orders, paid production, fog, mines, effects and support; missing/corrupt files recover | 14 save tests pass; real Ctrl+S/Ctrl+L restores paid queue/economy/fog and suppresses opening briefing; missing/corrupt loads preserve current match and show revised player-facing feedback; broader campaign runtime pending |
| Interface | English default, coherent icons/tooltips, no clipping at supported desktop sizes; small browser viewport inspected | Compact cards, locked tooltip and queue captured at1280x720/900x720; browser, remaining screens/overlays pending |
| Art/audio | Fresh menu, base, economy, combat, defeat/victory captures; readable silhouettes; commands/music/volume controls verified | Prior material pass only; further review required |
| Skirmish/balance | All shipped maps/factions/difficulties playable; reproducible multi-seed difficulty and faction sweeps; no indefinite matches | Partial old evidence; full sweep required |
| Delivery | Windows and Linux archives launch from arbitrary working directories without Rust/Nix; writable user saves/config | Corrected Linux development archive starts in no-Nix/read-only Ubuntu; matching separate harness passes save recovery; Windows GNU compiles and Wine menu/save recovery render correctly; native Windows/release profile/Steam Runtime/hardware pending |
| Reliability | Full tests, packaged-build smoke, long matches and measured performance on supported hardware | 166 library tests pass; no-Nix/read-only Linux startup passes; release/gameplay/performance gates outstanding |
| Distribution | Accurate feature/system-requirement copy, asset provenance/licenses and generated-content inventory; store-ready media | Restricted logo removed; upstream/font notices added; unresolved art/audio provenance in ASSET_PROVENANCE.md |

## Verification policy

Use the shared game scene and real input harnesses. Each visible fix needs a
fresh screenshot. Complete a current 450-frame, 30-fps proof sequence and inspect
its opening, interactions, combat and ending. Build/check/format and meaningful
behavior tests must pass. Claims about Windows, browser, performance or content
completion require evidence for those targets; native screenshots cannot prove
them. Preserve all gates until verified rather than reducing the release scope.

Steam reviews the store presence and build, including whether the build launches
and its advertised features work. Technical review is a release gate alongside
our player-experience criteria. See [Steam build review](https://partner.steamgames.com/doc/store/review_process)
and [content survey](https://partner.steamgames.com/doc/gettingstarted/contentsurvey).

The working title remains Bevy Open RTS. Desktop mouse/keyboard RTS play is the
primary product; controller/Deck claims need separate verification. Actual store
submission uses the owner's Steamworks account after reviewable artifacts exist.

## Current save acceptance evidence

`logs/steam-save-flow-final.log` and
`screenshots/steam-quality/save-load-final/observations.json` record normal-input
construction, paid infantry production, Ctrl+S, a second paid order, and Ctrl+L.
Ore returns from 24 to 25, two queued jobs return to the saved one, and all 5,771
explored cells remain explored. The production timer and match clock advance by
one normal frame during restoration. `02_restored.png` shows Game loaded with
the opening briefing dismissed. This is skirmish runtime evidence; campaign
triggers, entity remapping and delayed effects are covered by save tests and
still need broader campaign runtime acceptance. Save/load discovery in the
match menu remains a usability gap; currently these commands use shortcuts.

`logs/steam-save-recovery-runtime.log` adds ordinary Ctrl+L attempts with absent
and corrupt files in an explicit disposable XDG sandbox. Both attempts leave the
paused simulation observation byte-equivalent, retain the corrupt bytes and
display Load failed. The first captures exposed raw OS/parser errors; player
error copy now explains missing, damaged and incompatible saves without showing
serialization types. `logs/steam-save-friendly-tests.log` verifies that copy and
file preservation alongside the other 13 save regressions.

The revised feedback is verified in `logs/steam-save-friendly-runtime.log` and
`screenshots/steam-quality/save-friendly/{missing_save,corrupt_save}.png`.
The current integration passes 166 library tests, bins check/build and a
12-second native launch (intentional timeout, empty warning/error log).
`screenshots/result/13` contains 450 raw frames and a 15-second/30-fps video:
4.24m movement and 113 siege damage, with opening/movement/combat/ending inspected.

## Command shortcut regression

An actively gathering Worker can omit the idle-worker command and shift Barracks
onto grid slot 7. The old S binding stopped the worker instead of building;
other grid bindings collided with Guard, Hold, Scatter and numbered groups.
Grid keys now reserve all those shortcuts. The library regression rejects
duplicate/reserved bindings. Campaign macros observe actual tab keys rather
than assuming fixed T/Y positions; complete mission acceptance is still pending.

## Desktop checkpoint limits

The corrected Linux archive and exact-source executable hashes are recorded in
`docs/DESKTOP_DELIVERY.md`. Its read-only installation launches from `/tmp`
without Nix. A separate stripped/patched validation harness uses an asset
snapshot matching all 560 archive files and passes ordinary save/load and
missing/corrupt file recovery. `container-save-load-result.txt` beside that
archive records exit 0. Container ALSA has no audio device; this proves neither
audible sound nor physical GPU compatibility.

Windows GNU game and capture binaries compile. The menu and ordinary save/load
flow render under Wine with a read-only installation and separate user sandbox:
`screenshots/windows-wine/menu.png`, `save-load/02_restored.png` and
`save-load/observations.json`. Both images were inspected. This is useful Windows
executable evidence, but clean native Windows, MSVC distribution, release
performance and audible playback remain open. Both packagers exclude ignored
full-font source files and preserve the original user file.
