# Steam release quality plan

The goal is a complete, sellable RTS: a new player learns through play, finishes
a meaningful campaign, can save and resume, and installs a reliable desktop
build. Previous visual fixes and green unit tests do not establish that goal.

## Acceptance gates

| Area | Required evidence | Current state |
| --- | --- | --- |
| First session | Fresh profile → guided gathering, power, barracks, training, movement, attack and victory through normal controls; skip/replay work | Seven lessons and sandbox persistence pass; normal-input skip/return/graduation/fresh replay captured |
| Campaign | All six missions complete without cheats; correct objectives, retry/next/finale; completion survives restart | Missions1/2/4/6 win and continue; mission1 unlock survives restart; missions3/5 strategy/failure audit remains |
| Save continuity | Mid-mission save/restart/load preserves triggers, orders, paid production, fog, mines, effects and support; missing/corrupt files recover | Schema and restore behavior tests pass; player-facing runtime recovery pending |
| Interface | English default, coherent icons/tooltips, no clipping at supported desktop sizes; small browser viewport inspected | Compact cards, locked tooltip and queue captured at1280x720/900x720; browser, remaining screens/overlays pending |
| Art/audio | Fresh menu, base, economy, combat, defeat/victory captures; readable silhouettes; commands/music/volume controls verified | Prior material pass only; further review required |
| Skirmish/balance | All shipped maps/factions/difficulties playable; reproducible multi-seed difficulty and faction sweeps; no indefinite matches | Partial old evidence; full sweep required |
| Delivery | Windows and Linux archives launch from arbitrary working directories without Rust/Nix; writable user saves/config | Linux development archive starts in no-Nix Ubuntu container; release profile/Steam Runtime/gameplay/Windows pending |
| Reliability | Full tests, packaged-build smoke, long matches and measured performance on supported hardware | 164 library tests pass; no-Nix/read-only Linux startup passes; release/gameplay/performance gates outstanding |
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
