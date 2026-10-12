# Asset Provenance

Current inventory update: 2026-10-10. The runtime tree contains 332 files after unused VFX retirement.
The current332-file runtime manifest includes326 asset-provenance rows plus six
font/logo notice documents. All326 asset hashes match the ledger; no content
asset is missing from it ([membership check](qa/current-runtime-provenance-parity.json)).
This proves inventory consistency, not distribution rights.
Historical import audits are distinguished below from current original assets;
unidentified remaining families are not licensed by their presence in the tree.

`assets/migration/asset_manifest.json` lists the current 332 runtime files,
including notice documents and migration metadata. Refresh it with
`python3 scripts/generate_registry.py --asset-manifest-only`; this does not need
the historical Godot checkout. Local full-font inputs and `.import` sidecars are
excluded. The previous imported-source listing is retained as historical audit
evidence, rather than used as the current runtime inventory.

## Inventory and evidence

The audit initially found **556 tracked files**. The restricted Lampe logo has
since been removed from the asset tree and migration inventory. The remaining
343-entry historical migration manifest names an unavailable local Godot source directory, but no
source revision, hashes or per-file licenses. It is an import inventory, not a
rights manifest. Runtime references are in `src/audio.rs`, the generated registry,
`assets/data/godot_model_map.model_map.ron`, and `src/menu.rs`.

| Group | Verified evidence | Remaining work |
| --- | --- | --- |
| 153 Space Kit GLBs | All 153 GLBs byte-match the directly downloaded official Kenney Space Kit archive; its embedded notice identifies version 2.0 and CC0/commercial use. Original Open RTS hash correspondence is retained. | Preserve pinned archive/member/license hashes and notices. The page update label says 1.0; archive notice says 2.0, so those source labels remain distinct. |
| Godot logo | [Local notice](../assets/logos/LOGO_LICENSES.md) identifies Andrea Calabró, 2017, CC-BY-4.0. | Include required attribution/license and modification information if distributed. |
| Lampe Games logo | The same local notice explicitly prohibits use in projects derived from Open RTS. | Removed from the asset tree and migration inventory; retain the restriction notice. Verify final packages contain no copy. |
| WenQuanYi subset font | Regenerated directly from SHA-verified official 0.2.0-beta TTC face 0 with fontTools 4.61.1. Two runs reproduce identical bytes; 1,219 codepoints cover Rust text, full ASCII, English/Chinese CSV catalogs and all prior glyphs with unchanged advance widths. [Font notices/source hashes](../assets/fonts/WQY-LICENSE-PROVENANCE.md) are retained. | Verify fresh gameplay/menu rendering and include notices in packages. The ignored local full TTF is untouched and no longer an input. |
| Current entity portraits and roster | All 70 entity portraits and RosterPreview now come from original native rendering of actual licensed Kenney/authored game geometry. Complete model-only references, independent render reproduction, alpha/framing, 128/72px readability and bounded normal menu/HUD/paid-model samples are recorded in `art/portraits/runtime-integration.json`. | Full-roster paid-gameplay/UI coverage and final package validation remain open. VFX, cursors and background are separate families. |
| SFX and music | Recovered oscillator/noise-only FFmpeg creation recipes for all 19 WAVs and the battle loop. Fourteen legacy SFX match resynthesis PCM; the other five SFX and music were regenerated with the checked-in deterministic script. Three independent FFmpeg 9.0.1 runs match their file bytes and decoded PCM exactly. | Preserve reproduction script, seed, tool/output hashes and validation records. Human listening on release hardware remains unverified. |

## Pinned source comparison

[asset-source-hashes.json](asset-source-hashes.json) records exact paths, local and
upstream SHA-256 values, differences and source notice hashes. The official source
archive is pinned to [revision a628ad3bb6a9a903587a181a7ee3300dbf735e81](https://github.com/lampe-games/godot-open-rts/tree/a628ad3bb6a9a903587a181a7ee3300dbf735e81)
(2025-01-07); archive SHA-256:
`97a43db32a3f1d15da5da4714dbb1d671770f538d287f7cc19659b15448e9494`.
The historical comparison recorded 379 byte matches, including 14 former English
voices and 210 import-era UI files. Current original voices and model portraits
have separate source/render provenance; these old matches do not describe them.
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

- Current armor albedo: `assets/textures/armor-matte-original-v2.png` has
  built-in imagegen prompts and hashes in `art/armor-surface/candidates-v2/`.
  [ASSETS.md](../ASSETS.md) records current use and bounded verification; the
  replaced painted-steel texture is historical.
- Current campaign art: all six `assets/campaign/mission_0N-v2.jpg` files
  derive from the original built-in imagegen PNGs recorded under
  `art/campaign/candidates/`. Encoding commands and source/runtime hashes are in
  `art/campaign/runtime-derivatives-20261010.json`. All six current code references,
  active ledger hashes and SDK294 packaged files match those records. Retired
  ComfyUI/FLUX JPEGs are absent from the runtime tree and SDK294 package; their
  hashes remain historical evidence under `retired_campaign_art` in the ledger.
  Generated-asset terms and final commercial-rights review remain open.
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
  Remaining VFX/cursor/background provenance and human listening are unaffected.
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
images; the command-center T button is AttackMove, not Repair. The remaining ten command/power glyphs were inspected at 1280/900 in the clearly
labeled shared-component visual fixture; other normal HUD views cover the
remaining glyphs. This is bounded UI evidence, not all-power gameplay proof.
VFX, cursor and background provenance remain separate open work.

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
unclipped. Listening quality and pronunciation remain unverified;
no runtime audio is replaced. This candidate set supplies distinct English
support/superweapon warning texts that existing runtime fallback audio lacks.


Local audio-only transcription is retained in
`target/voice-asr-evaluation-20261010/`: official pinned Whisper base decodes all32
candidates, and small independently checks six priority lines, both exiting0.
Neither decoder receives expected text or phonemes. Five Mandarin priority
lines are recovered by the larger model, allowing script differences; both
models decode the English “Sir?” as “Sorry.” These results identify a review
concern, without establishing human listening quality or a pronunciation defect.

A separate “Ready for orders.” Hello candidate is retained in
`target/voice-hello-revision-20261010/`. Its generator exits0 and an independent
PCM check verifies mono24kHz16-bit audio, 1.575seconds, no clipping and SHA-256
`a8da92e15192baa257417f995cd727516ae880c5f620850fc77db30bc5718ea4`.
The original32 candidates and ASR outputs remain unchanged. Independent
audio-only base/small transcription in `target/voice-hello-asr-review-20261010/`
exits0: both recover “Ready for orders”, differing only in punctuation, with no
reference text supplied. Listening/tone and in-game mixing remain unverified;
no runtime substitution is accepted.


`target/voice-ogg-candidates-20261010/` stages32 clean-path Vorbis candidates
(16 unique files and17 enum mappings per language; SupportPowerFired reuses
Ack2). Both FFmpeg runs finish successfully, and all32 decoded PCM hashes match
without clipping. OGG container bytes differ because their stream serials vary;
byte reproducibility is not claimed. An independent ffprobe/header and SHA check
confirms all32 first-run files are Vorbis, mono24kHz. The manifest retains
source WAV hashes, complete event mapping and encoder commands. Actual Bevy
loading, event playback, volume controls, listening and mixing still require
verification; no runtime paths or assets have been replaced.

The current voice pack contains 32 original Kokoro-generated OGG files (16 per language), mapped to all 17 voice events with SupportPowerFired reusing Ack2. Source/model revisions, licenses, prompts, source and encoded hashes, and deterministic decoded-PCM checks are recorded in `art/voice/manifest.json` and `art/voice/provenance.json`. English Hello uses “Ready for orders.” Both official local ASR models recovered that line; ASR is not human listening or tone approval. Actual native audio-only QA loaded all 32 clips and naturally completed all 34 event/language mappings with real sinks at Linear(0.92); private-output recording contained no clipped samples and preserved the default sink. Evidence and scope are in `art/voice/README.md`. Natural gameplay triggering, overlapping mix, physical-output listening and human tone approval remain pending. The former provider pack and its generation script have been removed.

## Current original native model images

Exactly 71 existing runtime PNG paths now contain the approved original native
outputs: 70 entity portraits plus the six-model mixed-faction RosterPreview.
`art/portraits/runtime-integration.json` records every selected PNG/decoded-RGBA
hash, renderer/source/model/layout references, CC0 Space Kit and authored-code
source terms, contextual-marker exclusion and bounded review records. It does
not claim whole-scene parity or all-70 paid gameplay. Normal gameplay retains
its contextual faction indicators; only model portraits omit them.

The old 71 image bytes and source records remain historical outside the runtime
tree at `target/portrait-legacy-before-integration-20261010`. The exact 338-file
set is retained; the other 267 runtime hashes are unchanged. The active 332-entry
source ledger has zero hash mismatches; six font/logo notice documents remain
outside that payload-comparison ledger and are retained in the runtime/package
inventory. Three stale derived registry/import metadata hashes were refreshed,
with their prior records explicitly historical and no upstream-equality claim.
Post-integration native UI and the final packaged exact-set/hash checks remain
separate verification steps.

## Original menu backdrop integration (2026-10-10)

`assets/ui/background-original-v1.png` now supplies the main menu and skirmish
setup backdrop. It is an original built-in image generation output; the prompt,
source hash and no-reference generation record are in
`art/menu-background/candidates-v1/`. The previous background with unresolved
source history was retired outside the packaged asset tree. Native private
staging verified readable menus at 900×720 and 1920×1080 with proportional
cover cropping; `art/menu-background/runtime-integration.json` separately tracks
the passed three-image post-integration run and still-pending final package
and distribution review.
The runtime inventory remains 338 files and the active payload ledger 332 entries.

## Unused VFX texture retirement (2026-10-10)

Six unreferenced legacy PNGs (`exhaust_puff`, `fireball`, `impact_smoke`,
`impact_spark`, `muzzle_flash`, `shockwave_ring`) were removed from runtime assets.
Their hashes and historical source status remain in `retired_unused_vfx` in the
source ledger; archived bytes are outside the packaged tree. Runtime code and
data contained no texture references. A private asset copy without these files
passed startup and normal paid building/production HUD checks; this is not a
full combat or final package test. Procedural effects in `src/combat_vfx.rs`
remain active. Current inventory is 332 files, with 326 payload ledger entries.

## Original cursor atlas integration (2026-10-10)

The runtime four-frame cursor atlas now uses repository-authored SVG geometry
with reproducible resvg rasterization. Source SVGs, 32px dark/light visual checks,
frame layout and hotspots are recorded in `art/cursors/candidates-v1/`. Normal
native controls verified loaded RGBA pixels and all four cursor indices before
integration; `art/cursors/runtime-integration.json` distinguishes this from the
pending post-integration run. Native offscreen screenshots omit the OS pointer.
Final browser four-state appearance, click alignment and packaged behavior
remain unverified. Legacy atlas bytes are archived outside runtime assets.

## Original armor surface integration (2026-10-10)

`assets/textures/armor-matte-original-v2.png` replaces the unresolved legacy
painted-steel texture. Built-in image generation, the original candidate and
its targeted smoother revision have prompts and hashes in `art/armor-surface/`.
The first candidate was rejected after actual models showed excessive grain.
The second passed a private900×720 headquarters/Worker/Barracks sample; its
post-integration run, broader vehicle review and final package remain pending.
Original portrait render evidence predates this texture change and is preserved
as historical evidence, rather than claimed as exact new-material parity.
Provider-term and distribution review remain open. Runtime inventory stays332.
