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
