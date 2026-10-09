# Asset Provenance

Audit date: 2026-10-08. This records evidence and unresolved release work; it does
not grant permission for unidentified assets.

## Inventory and evidence

The audit initially found **556 tracked files**. The restricted Lampe logo has
since been removed from the asset tree and migration inventory. The remaining
325-file migration manifest names an unavailable local Godot source directory, but no
source revision, hashes or per-file licenses. It is an import inventory, not a
rights manifest. Runtime references are in `src/audio.rs`, the generated registry,
`assets/data/godot_model_map.model_map.ron`, and `src/menu.rs`.

| Group | Verified evidence | Remaining work |
| --- | --- | --- |
| 153 Space Kit GLBs | All 153 GLBs byte-match the directly downloaded official Kenney Space Kit archive; its embedded notice identifies version 2.0 and CC0/commercial use. Original Open RTS hash correspondence is retained. | Preserve pinned archive/member/license hashes and notices. The page update label says 1.0; archive notice says 2.0, so those source labels remain distinct. |
| Godot logo | [Local notice](../assets/logos/LOGO_LICENSES.md) identifies Andrea Calabró, 2017, CC-BY-4.0. | Include required attribution/license and modification information if distributed. |
| Lampe Games logo | The same local notice explicitly prohibits use in projects derived from Open RTS. | Removed from the asset tree and migration inventory; retain the restriction notice. Verify final packages contain no copy. |
| WenQuanYi subset font | Regenerated directly from SHA-verified official 0.2.0-beta TTC face 0 with fontTools 4.61.1. Two runs reproduce identical bytes; 1,219 codepoints cover Rust text, full ASCII, English/Chinese CSV catalogs and all prior glyphs with unchanged advance widths. [Font notices/source hashes](../assets/fonts/WQY-LICENSE-PROVENANCE.md) are retained. | Verify fresh gameplay/menu rendering and include notices in packages. The ignored local full TTF is untouched and no longer an input. |
| English voices: 14 OGGs | All 14 are byte-identical to pinned official Open RTS. Voice history spans 2023–2025; project MIT notice retained. [Current TTSMaker terms](https://ttsmaker.com/terms-of-service) describe ownership of created audio. | No voice-specific grant or generation-time provider terms were found upstream. Source membership is verified; third-party output rights remain to document. |
| Chinese voices: 16 OGGs | `scripts/gen_zh_voice.sh` records edge-tts, Yunxi/Xiaoxiao and input lines. | Obtain applicable output-use/redistribution evidence for that actual service, or regenerate through a documented licensed source. The tool's availability is not permission. |
| Other art | Ten icons and `icosphere.obj` match official upstream. Another 105 icons, three emblems, two cursor files and six VFX images are absent; background and two icons differ. Explicit generated-sheet copy records were recovered for 27 current icons. | Recover missing generation/service records and transformed-image correspondence. VFX creation evidence remains missing. The official MIT notice does not establish rights in absent families. |
| SFX and music | Recovered oscillator/noise-only FFmpeg creation recipes for all 19 WAVs and the battle loop. Fourteen legacy SFX match resynthesis PCM; the other five SFX and music were regenerated with the checked-in deterministic script. Three independent FFmpeg 9.0.1 runs match their file bytes and decoded PCM exactly. | Preserve reproduction script, seed, tool/output hashes and validation records. Human listening on release hardware remains unverified. |

## Pinned source comparison

[asset-source-hashes.json](asset-source-hashes.json) records exact paths, local and
upstream SHA-256 values, differences and source notice hashes. The official source
archive is pinned to [revision a628ad3bb6a9a903587a181a7ee3300dbf735e81](https://github.com/lampe-games/godot-open-rts/tree/a628ad3bb6a9a903587a181a7ee3300dbf735e81)
(2025-01-07); archive SHA-256:
`97a43db32a3f1d15da5da4714dbb1d671770f538d287f7cc19659b15448e9494`.
This proves 379 byte matches, including 14 English voices and 210 UI files.
Source membership does not prove provider rights. The notice scan found project
MIT terms and explicit logo exceptions, with no audio-specific permission records.
The import-era customized checkout named by the migration manifest remains unavailable.

## Direct official Kenney archives

On 2026-10-08, the download links were read from the official asset pages; only
those linked Kenney-hosted ZIPs were fetched. Cached downloads remain under
ignored `target/kenney-source-audit/`. The original pinned Open RTS comparison
and all its hashes remain intact in the JSON inventory.

- [Space Kit archive](https://kenney.nl/media/pages/assets/space-kit/20874c75ac-1677698978/kenney_space-kit.zip):
  SHA-256 `d5d7cdf2635ed5a43a9187deaf409b6f47484e402321128341d3c3698e9ef4d9`.
  All 153 shipped GLBs exactly match members under `Models/GLTF format/`.
  `License.txt` identifies **Space Kit 2.0**, creation 2020-08-27, Kenney, CC0,
  and permission for commercial use. The current page's 1.0 update label is not
  used to override that embedded version. No semantic approximation was needed.

Per-file Space Kit archive member mappings, license hashes and retrieval
metadata are recorded under `official_kenney_archives` and
`official_kenney_correspondence` in `asset-source-hashes.json`. No Space Kit assets were replaced and no commercial permission was inferred for unrelated asset families.

## Generated content

- Armor albedo: [ASSETS.md](../ASSETS.md) records built-in imagegen generation,
  date and use for `textures/armor-painted-steel.png`.
- Six campaign JPEGs: commit `ea91fd9` records ComfyUI/FLUX.1-dev generation;
  `scripts/gen_cutscene_art.py` identifies the checkpoint and workflow. Preserve
  per-image prompts, seeds, checkpoint identity and applicable terms. The
  [current FLUX license](https://huggingface.co/black-forest-labs/FLUX.1-dev/blob/main/LICENSE.md)
  distinguishes model use from generated outputs and includes a commercial-output
  clause; a noncommercial model label alone does not establish that images are
  prohibited. Generation-time evidence and output review remain missing.
- Local Godot-era tool records dated June 14–16 recover all 19 SFX recipes,
  the oscillator-only music recipe, and generated-sheet copies for 27 icons,
  including RosterPreview. The JSON inventory preserves only relevant recipes,
  timestamps, command hashes and reproduction results. No service credentials or
  unrelated conversation are retained. Exact PCM correspondence is verified for
  14 legacy effects. The five remaining SFX and music were replaced with
  reproducible authored outputs; their former mismatches remain historical records.
- Current audio closure: `scripts/reproduce_authored_audio.py` pins FFmpeg 9.0.1,
  seed 20261008, scalar CPU flags and single-thread filters. Error tones use explicit half-gain
  mixing rather than timing-dependent dynamic normalization. It reuses only the
  recovered oscillator/noise/mix recipes. A 4 ms music endpoint taper and limiter
  latency compensation preserve the 24-second loop while reducing its join jump
  to 0.000029 in decoded float PCM. All six files remain mono 44.1 kHz, retain
  their original durations and have peaks below 0.088. Three independent runs
  produced identical bytes and decoded PCM; exact hashes/commands are in the JSON
  inventory. All six current files loaded through the actual Bevy AssetServer,
  AudioLoader and AudioSource decoder, without clipping or sample-count changes
  (`logs/authored-audio-assets-load.log`). A native match startup capture also
  passed (`logs/authored-audio-game-load.log`). Human listening remains unverified.
  Voice and generated-art provider gaps are unaffected.
- Generated-sheet source paths are evidence of production steps, not proof of
  applicable model/service terms. VFX creation records remain missing.

## Release closure

1. Replace the blanket asset copy with a reviewed release inventory; also exclude
   the gitignored full source font currently present locally.
2. Package the retained notices: [LICENSE-MIT](../LICENSE-MIT) preserves the
   exact upstream MIT text and `Copyright (c) 2023 Lampe Games` from revision
   `45717502e0e167432ef9759684b4730afc2f01b5`; [LICENSE-APACHE](../LICENSE-APACHE)
   is the canonical Apache text, matching Cargo's existing declaration.
   Imported material retains its source terms; these texts do not relicense assets.
   Font source notices and license alternatives are also retained. Verify all
   required notices reach release packages; unattributed imported assets remain open.
3. Pin source versions and record hashes, authors, licenses, modifications and
   generated-content records. Check the final packaged files against that inventory.

Verification: inspected tracked paths, migration manifest/report, embedded GLB
and font metadata, generation scripts, credits and the cited source pages.
The restricted logo was removed after inspection. No additional permissions
were inferred from source membership or file appearance.

## Unused crosshair removal

On2026-10-09, all200 unused crosshair PNGs were removed. No Rust runtime path
references this directory; selection and targeting marks are drawn in code.
The import manifest and its generator now exclude this family. The current
hash inventory no longer lists those files or their old archive mappings.
All359 remaining runtime payloads are byte-identical; only the migration
manifest changed. Evidence: `logs/unused-crosshairs-removal.json`.
Older checkpoint archives still contain the old assets and remain distinct;
new packages must be staged from the cleaned current tree. Other unresolved
icon, voice and generated-content evidence above remains open.
