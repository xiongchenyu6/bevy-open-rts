# Generated Art Assets

## Battlefield material pass

- `assets/textures/armor-painted-steel.png`: generated neutral matte armor albedo,
  1254 × 1254 PNG. Fine scratches, dust and coating wear; no baked lighting
  or panel borders. Generated with the built-in imagegen tool on 2026-10-06.
- Used on neutral imported armor and procedural vehicle Paint materials.
  Repeating linear sampler supports kit UVs beyond 0–1; startup loading retains
  the handle before interactive matches begin.
- Verified in real match captures across all three factions, rain and sandstorm.
  Final proof media is recorded with the same shared scene and real unit orders.

## Authored audio lineage pass

- Reproduced `sfx/error.wav`, `explosion_small.wav`, `structure_lost.wav`,
  `support_power_fire.wav`, `weapon_hit.wav` and `music/rts_battle_loop.ogg`
  directly from oscillator/noise recipes using `scripts/reproduce_authored_audio.py`.
- FFmpeg 9.0.1, fixed seed 20261008; three independent runs match both file bytes
  and decoded PCM. All retain their original duration and mono 44.1 kHz format.
- Music retains its 24-second synthesis and level, with a 4 ms endpoint taper
  and compensated limiter delay for the loop seam. No imported samples or voices.
- Tool, script and final-output hashes are in `docs/asset-source-hashes.json`.
  All six pass the actual Bevy asset loader/decoder; native match startup passed.
  Human listening on release hardware remains unverified.

## Campaign illustration candidate (2026-10-09)

`art/campaign/candidates/mission01-canyon-v2.png` is an original built-in
imagegen candidate: gray/olive industrial outpost, utility robots and tracked
vehicles defending an ochre canyon. It was visually inspected for clear
silhouettes, no baked text/UI and a quiet lower briefing band. The1672×941
RGB8 output is copied unchanged; prompt/tool/time/hash details are in the
sibling provenance JSON. It is not referenced by the game or shipped assets,
and it is an illustration rather than a gameplay screenshot. Existing mission
art remains unchanged until a coherent set and actual UI presentation pass.

`art/campaign/candidates/mission02-crossfire-v2.png` adds an original1672×941
built-in imagegen illustration of two canyon crossfire positions and an enemy
command center. The lower band is quiet; robot/tank silhouettes and faction
colors were inspected. Fictional geometric banners require consistency review
before integration. Full prompt, tool limitations and SHA256 are in its sibling
provenance JSON. It remains outside shipped assets, without changing legacy art.

The original candidate set now covers all six missions. Missions3–6 depict
Ring Valley mining/siege, highland defense/airlift, four-corner pressure and
the final supply-starved assault. Each image was individually generated and
visually inspected; unchanged PNGs and complete per-image prompts/hashes are
under `art/campaign/candidates/`. Current game assets remain unchanged; actual
briefing integration and presentation verification are still required.

## Original campaign art integration (2026-10-10)

All six campaign entries now reference `campaign/mission_01-v2.jpg` through
`mission_06-v2.jpg`. These are JPEG encodings of the inspected original PNGs;
mission2 uses the banner-freev3 source. FFmpeg9.0.1 scalar/single-thread commands
and input/output hashes are in `art/campaign/runtime-derivatives-20261010.json`.
The six runtime files total3.28MB, decode successfully and resolve from every
campaign entry. Source PNGs remain unchanged. Mission5 native briefing was inspected at1280×720 in
`screenshots/steam-quality/current-campaign-twin-early-cover-2/00_briefing.png`: art
loads, title/copy are readable and no clipping appears. Remaining mission images,
smaller layouts and browser presentation still require verification. Legacy JPEGs are temporarily retained for sealed
older validation programs and must be removed before final distribution.
