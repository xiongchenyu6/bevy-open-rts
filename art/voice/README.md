# Original voice candidates and integration plan

Candidate preparation only: Rust audio routes and runtime assets are unchanged.
Human listening/tone and actual loader/playback/volume acceptance remainfalse.

The32 portable mono24kHz PCM16 sources are under `sources/`. `manifest.json`
contains source hashes/text, clean output paths, expected decoded PCM and all17
UnitVoiceEvent mappings per language (16 unique files). Only SupportPowerFired
reuses Ack2. English Hello is the revised “Ready for orders.”; the rejected
short “Sir?” is not a portable candidate input. Original older proofs remain
historical artifacts. `provenance.json` pins model/voice records, generation
parameters/SDK versions and objective acoustic/ASR checks.

## Portable encoding

```sh
python3 art/voice/encode.py --check-inputs
python3 art/voice/encode.py --output /absolute/new/candidate-output
```

Inputs resolve relative to this directory; no absolute target/cache directory
is required. Use the pinned FFmpeg9.0.1. The script verifies32 source hashes and
complete mappings, refuses an existing output directory, then encodes Vorbis
quality5/mono24kHz/single-thread without volume processing. It records tool/hash,
parameters, source/output SHA256 and decoded PCM. Decoder hashes/frame counts
must match the retained references, with finite samples and no clipping.

Completed preparation already encoded64 files in two independent runs: all32
decoded PCM hashes and frame counts match, all32 are unclipped. OGG container
bytes differ32/32 because container identity/serial can vary; container hashes
are records, not deterministic acceptance. The existing encoded candidates and
independent32-file ffprobe/header review remain under
`target/voice-ogg-candidates-20261010`; do not repeat encoding to claim new proof.

## Core-seal integration changes to review

1. Replace all17 routes in each language in `src/audio.rs` directly from the
   new mapping. Dedicated support/enemy/superweapon warnings replace the former
   English fallback lines. Delete the legacy provider comments; no fallback
   routes or compatibility wrapper.
2. Remove all30 files in the old TTSMaker/Edge provider trees and
   `scripts/gen_zh_voice.sh`. Two old English battle_control files have no current
   runtime literal reference. `legacy-reference-audit.json` lists exact hashes
   and runtime/script/inventory references for the entire retired tree.
3. Add exactly32 selected OGG outputs beneath `assets/voice/{english,chinese}/`
   using `unit/` and `announcer/` paths. Verify output SHA256, decoded PCM,
   mapping count and absence of old provider directories/paths. With the current
   336-payload baseline, replacing30 with32 yields338 runtime payloads; recalculate
   actual inventory at integration rather than assuming no concurrent changes.
4. Replace active voice entries in the migration inventory/source-hash ledger;
   retain former hashes only as explicitly historical provenance. Include model
   notices/generation records in the reviewed package inventory. Ensure later
   imports cannot restore obsolete provider files. Pin final package/extracted
   exact-set/hash checks after all intended assets and notices are integrated.

## Acceptance still needed after integration

- Load all32 through the actual Bevy AssetServer/AudioSource decoder and verify
  every one of17 enums per language, including intended Ack2 reuse. A mapping
  assertion or FFmpeg decode alone is not game-loader proof.
- Exercise real audio-feedback playback and language choice with event identities
  recorded. An isolated all-events playback fixture must be labeled audio-only,
  not gameplay/paid-command acceptance.
- Verify voice volume, mixing/ducking against effects/music, output capture with
  a real audio device, rapid acknowledgments and warning audibility. Container
  “no audio device” runs cannot close audible-playback acceptance.
- Human listening/tone review remains open. ASR had short/script/homophone
  uncertainty; the revised English Hello matches two audio-only model decodes,
  but this is content evidence, not a listening approval. No runtime replacement
  or distributable-package acceptance is claimed by this candidate bundle.
