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
| English voices: 14 OGGs | All 14 are byte-identical to pinned official Open RTS. Voice history spans 2023–2025; project MIT notice retained. [Current TTSMaker commercial terms](https://ttsmaker.com/copyright_and_commercial_license_terms) permit commercial use and distribution of service-generated audio, including games. Current terms alone do not establish the imported files’ generation-time grant. | No voice-specific grant or generation-time provider terms were found upstream. Source membership is verified; third-party output rights remain to document. |
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

## Current original campaign illustrations

Six original built-in imagegen illustrations are retained under
`art/campaign/candidates/`, with exact prompts, original output hashes, recorded
times and explicit unavailable model-version/seed fields. Each source has been
visually inspected. They are mission illustrations, not gameplay screenshots.
Mission2 uses its revised flagless v3 source; the other missions use v2 sources.

All six JPEG derivatives are wired in current campaign code. Per-image
provenance links the unchanged PNG to its runtime path/hash, and
`art/campaign/runtime-derivatives-20261010.json` retains exact encoding commands.
All six source/provenance/runtime hash triples were rechecked on2026-10-10.
The360-record source inventory and366-file runtime tree have different scopes.
Current native1280×720 briefings for missions1,2,3 and5 have been inspected.
Mission1/2 result panels are readable with no visible clipping; root capture
hashes/notes are in `current-campaign-chain-live-response/root-visual-review-01.json`
under `screenshots/steam-quality/`. Missions4/6, small-screen/browser layouts and
complete presentation remain unverified.

Six obsolete campaign JPEGs were removed on2026-10-10 after confirming all
six current code paths use the original v2 derivatives and verifying their
source/runtime hashes. Removed hashes remain in `retired_campaign_art` in the
source inventory; older sealed archives are unchanged. Current packaging no
longer needs legacy filename exclusions. Current runtime has336 files (plus
the ignored full-font source); the330-record source inventory has a different
scope. Independent336-asset package startup passed before this repository
cleanup because all six obsolete files were already excluded from that package.
Other icon, VFX and voice
rights gaps remain open; these generation records do not establish blanket
commercial clearance for unrelated assets.

## Packaged asset verification

Desktop packagers now record `runtime_asset_sha256` in BUILD_INFO.json for
all copied runtime files, including notices. Linux and GNU Windows dictionary
expressions were exercised against an actual staged360-file tree; every hash
matches both staged bytes and repository sources
(`target/package-asset-hashes-wmgre5rq/verification.json`). PowerShell uses the
same path/hash format but still requires execution on Windows. No final current
release archive or launch acceptance is implied by these staging checks.

After extracting a new desktop package, run:

```sh
python3 scripts/verify_runtime_assets.py /path/to/extracted-package
```

The command checks the exact file set and SHA256 of every recorded asset.
It exits1 for missing, extra or changed files, malformed inventory or invalid
paths. Actual360-file staging passes; deliberate corruption, removal, addition
and invalid paths fail (`target/runtime-assets-verifier-vrsv15rz/checks.json`).
This verifies correspondence to BUILD_INFO, not publisher authenticity, asset
rights or game launch; those remain separate gates.

Current TTSMaker terms were checked on2026-10-10. Retrieval scope and limitations
are recorded in [voice-provider-terms-20261010.json](voice-provider-terms-20261010.json).
The direct archive request returned403; no raw-page hash is claimed. Imported
English file provenance and historical terms still require evidence, and these
terms do not cover the separate edge-tts Chinese outputs.

## Original runtime semantic icons

Twenty action/support pictograms now use original repository-authored geometric
SVGs and resvg0.48.1 PNG rasterization. Paths are unchanged; old bytes are kept
outside runtime assets under `target/semantic-icon-legacy-20261010`. Every128px
RGBA8 icon reproduces byte-identically and decoded alpha is transparent/nonblank.
Source, recipe and original/current hashes are in
`art/semantic-icons/runtime-integration.json` and `candidates-v3/`.

Actual native b843 review covers RallyPoint, AttackMove, SellStructure,
RadarSweep, EmpPulse and ShieldOverdrive. Paid F1/F5 save/load/expiry and real
old-format rejection pass with unchanged save bytes. Root inspected the actual
images; the command-center T button is AttackMove, not Repair. Other14 icons,
all tooltips, smaller windows and browser rendering remain pending. This does
not close entity portraits, VFX, cursor, faction-emblem or voice rights gaps.

## Original faction and speech candidates

`art/faction-emblems/generate.py` authors three original SVG faction crests
without imported images or fonts. resvg0.48.1 renders128px RGBA8; two fresh
runs reproduce the PNGs byte-identically, and decoded alpha has0/255 pixels.
Root inspected16/24/32px sheets and actual1280px shared skirmish setup/dropdown
using isolated candidate assets; both children exit0 and all three read clearly.
Evidence: `screenshots/steam-quality/original-faction-emblems-01/visual-review.json`.
The three inspected original crests now replace the same runtime paths. Their
installed bytes match the isolated menu-review assets; exact old/new/source
hashes are recorded in `art/faction-emblems/runtime-integration.json`.
Browser setup/dropdown glyph presentation passes1280x720 and900x720
(actual child0, no page errors, inspected pictures) in an isolated oldWasm07/
new-glyph stage. Evidence: `original-faction-emblems-browser-02/visual-review.json`
under `screenshots/steam-quality/`. Final-core browser and wider layout review
remain open.

`target/voice-full-event-candidates-20261010/` contains32 CPU-only candidate
WAVs for16 complete event texts in each language. Generation uses the retained
commit-pinned official Kokoro82M model and stock voice files; exact model/voice
hashes and authored texts are in its manifest. Actual generation exits0, and
independent PCM checks verify all32 mono24kHz16-bit files are nonempty and
unclipped. Listening quality/pronunciation/content transcription remain unverified;
no runtime audio is replaced. This candidate set supplies distinct English
support/superweapon warning texts that existing runtime fallback audio lacks.
