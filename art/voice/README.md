# Original runtime voices

The runtime now uses 32 original Kokoro-generated OGG clips: 16 unique clips per
language and all 17 UnitVoiceEvent mappings. SupportPowerFired intentionally
reuses Ack2. English Hello says “Ready for orders.” The former 30 provider clips
and generation script are removed; no fallback routes remain.

Portable WAV sources, model/SDK licenses and pinned generation records are in
`sources/`, `manifest.json` and `provenance.json`. Source/output hashes and decoded
PCM references are retained. Encode with the pinned FFmpeg 9.0.1:

```sh
python3 art/voice/encode.py --check-inputs
python3 art/voice/encode.py --output /absolute/new/output
```

The script refuses existing output and checks source hashes, complete mappings,
finite decoded samples, clipping and reference PCM. Two independent preparation
runs reproduced all 32 decoded PCM hashes; OGG serials may differ. No new encoding
run was needed for integration.

Actual native QA on capture SHA
`cc05c6043f5cb1ae639b04f67c8de02bafcdea42678348dd4f0d9549b6a712c6`
loaded and decoded all 32 through Bevy AssetServer/AudioSource, then played all
34 event/language mappings through the shared feedback system. Every real sink
reported Linear(0.92) and completed naturally: zero timeout or forced cleanup.
The private virtual-output monitor recorded 69.8 seconds, stereo 48 kHz,
3,350,400 frames, peak PCM16 19,553 and no clipped samples. Default output was
unchanged and the owned virtual output was unloaded. Before/after asset hashes
matched for all 338 payloads. Evidence: `target/voice-runtime-audio-qa-20261010`.

Existing Master/Voice/SFX options now multiply the corresponding real playback
settings; defaults preserve the original baseline gains. Actual spawn tests
cover master mute, independent channel mutes and fractional gains. Their result
must be retained with the shared compiler report.

This is audio-only fixture evidence. Natural game-event triggering, overlapping
music/effects mix, rapid acknowledgments, physical speaker/headphone listening,
tone approval and final-core/package revalidation remain open. ASR recovered the
revised Hello with two audio-only models; that does not approve human listening.
