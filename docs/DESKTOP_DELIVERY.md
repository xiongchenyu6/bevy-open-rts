# Desktop delivery

## Build and package Linux

Build the native game with `cargo build --release --bin bevy-open-rts` inside the repository's development environment. Then run:

```sh
./scripts/package_linux.sh target/release/bevy-open-rts dist/linux-release
```

The packager requires Python 3.11+, `patchelf`, `strip`, `ldd`, and `tar`. The build environment's `LD_LIBRARY_PATH` must contain Wayland and xkbcommon libraries. It packages the existing executable, assets, ELF loader, and direct/transitive runtime libraries. It removes Nix ELF search paths. It refuses to replace an existing package directory or archive. No compilation or dependency-lock updates occur.

Extract the archive anywhere and run `bevy-open-rts-linux-x86_64/launch.sh`. The launcher resolves assets from its own installation directory, regardless of the calling directory. Saves and progress use `${XDG_DATA_HOME}/bevy-open-rts`, falling back to `${HOME}/.local/share/bevy-open-rts`. The installation can remain read-only.

Vulkan/OpenGL drivers and a Wayland desktop are host prerequisites. Graphics drivers are deliberately supplied by the host/Steam Runtime, so bundling libc is insufficient proof of GPU compatibility. Audio requires a working host ALSA configuration/device; missing sound hardware must not prevent startup. Desktop hardware, driver, and audio combinations need testing before release.

## Validation and release gates

`BUILD_INFO.json` records source and packaged executable hashes, build profile, asset/notice inventories, and bundled-library provenance. Packaging runs `scripts/verify_package.py` against the completed archive before reporting success. The checker rejects missing inventories, changed bytes, duplicate members and unsafe paths. The accompanying `.sha256` file verifies archive integrity. These checks do not prove runtime compatibility. Debug archives are validation artifacts, not release candidates.

Test extraction and launch from `/tmp` in a Steam Runtime container with **no `/nix` mount**, using a Wayland compositor and software Vulkan. Check asset loading, main menu, campaign/tutorial entry, gameplay, quit, persistence, and restart. Repeat on physical AMD, Intel, and NVIDIA GPUs with audio and read-only installation. Preserve logs/screenshots and identify the exact archive hash. A startup-only test does not prove gameplay or commercial readiness.

Before distributing bundled libraries, complete their license notices and corresponding-source obligations, plus the game asset license inventory. Library provenance alone does not satisfy those requirements.

## Windows gate

A Windows package remains unverified. Build a native Windows release, include assets beside the executable, resolve assets from the installation directory, and store saves under `%LOCALAPPDATA%/bevy-open-rts`. Test a clean Windows machine without Rust/Nix, a non-repository calling directory, read-only installation, fresh profile, saving/loading, audio, and supported GPUs. Linux success does not prove Windows or Steam Deck compatibility.

## Historical development evidence (2026-10-08)

The development executable was packaged and extracted under `/tmp`, then launched from `/tmp` in `rogue-commander-linux-smoke:ubuntu24` with no `/nix` directory or mount. The bundled loader resolved every direct dependency from the extracted package. Mesa 25.2.8 / llvmpipe LLVM 20 initialized Vulkan, and the native game created its window under headless Weston. An intentional timeout stopped the smoke run. This proves native startup outside the build machine's Nix store, not full gameplay, audio, GPU hardware coverage, or release readiness.

The existing sniper smoke image contains Mesa 20.3 / LLVM 11; its software Vulkan driver crashed during adapter enumeration. Steam Runtime compatibility remains unverified and needs a supported recent driver environment or an actual Steam Runtime build. Do not distribute this development archive as a commercial build.

The current native development binary was repackaged after the compact HUD, sandbox Boot Camp, and final English/Chinese CSV font coverage changes. Source executable SHA256: `5b9fa368067fa5bc555931b1d0ca53449c9ed3f32e7136a4486cf5a46b90f8b9`. Archive SHA256: `3131ca76b1665e3d1868ded36fe2f2ab383a66af66fcb6ac05d4b887488bb245`. The artifact is `/tmp/rts-linux-clean-checkpoint-20261008/bevy-open-rts-linux-x86_64.tar.gz`; its `BUILD_INFO.json` labels the profile `development`.

The exact final-assets archive was extracted and mounted read-only for a 20-second no-Nix Ubuntu 24.04 startup smoke, from `/tmp`, with an independent XDG profile. Vulkan initialized and the native game created its window. Writing to the installation failed with “Read-only file system”; the restricted logo was absent. The font inside the tested archive is 215,372 bytes, SHA256 `4b9451969be0c79fe523fe7132813ab1ad5bda36efab7929e5b596570e65ef7f`. Current asset provenance, hash records, font notices, and `ASSETS.md` are included.

Evidence files beside the archive are `readonly-launch.log`, `readonly-result.txt`, and `readonly-mounts.json`. Intentional timeout exit code 124 is expected. The container has no sound device, so audio hardware remains unverified. These checks do not remove the release-profile, Steam Runtime, hardware, gameplay, licensing, or Windows gates above.

The packager excludes `*.full.ttf`: these are ignored source-font files, not runtime assets. The original user file remains unchanged. An earlier checkpoint accidentally included it and is retained locally as `/tmp/rts-linux-integration-20261008/bevy-open-rts-linux-x86_64.before-font-exclusion.tar.gz` for audit; that archive is not the current delivery artifact. The corrected script-generated archive was checked for its absence.

A separate development capture harness also passed ordinary save/load input in a clean no-Nix Ubuntu container (`SAVE_LOAD_EXIT=0`). It built a reactor and barracks, paid for one queued infantry unit, saved with Ctrl+S, paid for a second unit, and restored with Ctrl+L. The restored state recovered 25 ore, 14 crystal, three units, three structures, and the original single paid job; the clock differed by one simulation tick. Missing and corrupt saves preserved the paused match and produced visible feedback. Five screenshots, `observations.json`, and the sandbox `quicksave.ron` are under `container-save-load/` beside the corrected archive.

This harness is not included in the player archive. Its source executable SHA256 is `4b4eb52b9ff32e35305b14a805fdefe732be584799cde585ef5aeb005fc68386`; the stripped/patched validation executable is `/tmp/rts-linux-integration-harness-20261008/capture`, SHA256 `86bfaa2078e2343c81df15793ff832530bb41a53733cdca4237cc50e3772a9a1`. It uses the same completed source build as the native game and a read-only runtime asset snapshot matching every one of the corrected archive's 560 file hashes (`capture-asset-snapshot-equality.json`). This proves that input flow under the recorded software-rendering configuration, not release-profile, Steam Runtime, Windows, physical GPU/audio, or broad gameplay readiness.

## Optimized Linux checkpoints (2026-10-08)

Two unchanged standard release builds terminated in kernel global OOM; neither produced a release executable. Evidence is retained in `/tmp/rts-standard-release-failed-20261008/` and `/tmp/rts-standard-release-retry1-failed-20261008/`. These failures are not successful standard-profile builds.

A separately labeled native candidate preserves the existing dependency profile and overrides only final-executable compilation:

```sh
RUSTC=/nix/store/yhmi70ln28n1j6wn82h61b8r8q4g562i-rustc-1.95.0/bin/rustc \
  cargo rustc --locked --release --jobs 1 --bin bevy-open-rts -- \
  -C lto=thin -C codegen-units=8 -C llvm-args=-threads=1
```

The online-default candidate completed in 11m22s with exit 0 and 2.8 GiB peak memory. Build log: `logs/steam-native-thin-online-candidate-20261008.log`. Its compile-start context is `/tmp/rts-native-thin-online-candidate-build-context-20261008.json`. The context records commit `6d8a0672dca7ed908552f114dbdecddd59129756`, dirty helper state, and later source differences. This checkpoint includes the verified online-default fix but predates the subsequent setup-summary UI fix; it must not be called the latest source build.

Source executable SHA256: `cd0574925412bd8a0184ee0de484d5f30ce0aaacfcfaa42a1f0e5f6d30c33129`. Archive: `/tmp/rts-linux-native-thin-online-checkpoint-20261008/bevy-open-rts-linux-x86_64.tar.gz`, SHA256 `bebd49e3cbe409b0e02ede5f8e966912b853f0c7766e7b8f6776740587eb5596`. `BUILD_INFO.json` labels it `native-thin-online-default-pre-summary-fix-checkpoint`. The packaged README matches `docs/PLAYER_GUIDE.md` byte-for-byte; the ignored full font and restricted logo are absent, and the original user font hash guard passes.

The exact extracted archive ran through its shipped launcher from `/tmp`, mounted read-only, in Ubuntu 24.04 without `/nix`. An isolated Sway compositor used physical AMD Radeon Graphics (RADV RENOIR), Mesa 25.2.8, Vulkan, at 1280×720. Persistent Wayland virtual devices sent ordinary physical Enter/F10 keys and mouse clicks: skirmish started, tactical pause appeared, and a selected worker moved after a right-click order and resume. No private game state was modified. The launcher now defaults `XLOCALEDIR` to host X11 locale data while preserving an explicit user override; this test supplied no override and emitted no Compose/XKB errors.

Evidence: `/tmp/rts-native-thin-validation-20261008/perf/online-checkpoint/runtime.log`, `01-menu.png`, `02-physical-f10.png`, `03-worker-moved.png`, and `readonly-error.txt`. The installation write probe failed with “Read-only file system”. Audio hardware was absent.

Earlier offline checkpoint source SHA `fadde65718e2dc0f9aac3207e62e57ab1efe517aa55f0a7db52226e3d636e01d` and archive SHA `370755ccb228ac8103f244253c6dfe60dc940e2eb327385a2c64b96cebdb2007` remain preserved under `/tmp/rts-linux-native-thin-offline-b416-20261008/`. That executable retained the old online default, and its input test required an explicit locale override.

MangoHUD 0.8.4 recorded raw frame-time CSVs for the offline checkpoint under `/tmp/rts-native-thin-validation-20261008/perf/`. The idle-menu sample averaged 62.4 FPS; separate samples cover setup and a small opening match with worker movement. They do not establish representative combat performance. Present mode/vsync were not overridden. Combat workload, broader GPU coverage, audible output, Steam Runtime, Windows, and multiplayer-session validation remain separate gates. Optimized compilation and these runtime checks do not establish commercial release readiness.

## Current Steam Runtime target review (2026-10-10)

[Valve's current runtime guidance](https://github.com/ValveSoftware/steam-runtime/blob/master/README.md) recommends Steam Linux Runtime 4 for new native games. Sniper 3 remains an explicitly selectable older target. The current Linux archive has not yet passed either runtime's complete player-flow acceptance; Ubuntu container success is separate evidence.

The locally installed Sniper depot is `0.20240916.101795`, with pressure-vessel `0.20240916.0`. A CPU-only command preflight failed before reaching the requested command: `bwrap: execvp true: No such file or directory`. Retrying with the known compiler-tool PATH produced the same error. Both logs remain in `target/steam-runtime-installed-preflight-20261010` and `target/steam-runtime-installed-preflight-retry02-20261010`. This diagnoses the local runtime launcher environment; it does not demonstrate a game crash or GPU incompatibility. The installed Steam files were not changed, and runtime scratch data used private repository directories.

The cached official Sniper platform image is build `3.0.20260805.254768`, digest `sha256:8cd1bdfc418afd8b019a69d6f5a9305865a5f3bb5554102006fdb8f6853d5f67`. Its identity and OS release were inspected without launching the game. Select and pin the release runtime, build with its SDK, then verify the exact archive with supported host graphics, read-only installation, writable user data and actual normal-input gameplay. No Steam submission is implied.

Follow-up preflight identifies the launcher PATH reset. Its supported `SYSTEM_PATH=/run/current-system/sw/bin:/usr/bin:/bin` reaches graphics-library import, which then fails on the host's root-only `/var/cache/ldconfig/ld.so.cache`. Disabling graphics import with `--graphics-provider=` successfully executes `/bin/sh` and reads the Sniper OS release (exit 0). This establishes only the historical runtime's CPU launcher, not game/GPU compatibility. Four preserved attempts and exact launcher/log identities are in [the CPU preflight record](qa/steam-runtime-installed-cpu-preflight-20261010.json). Do not change system cache permissions to make the test pass; supply a supported graphics-provider environment for subsequent rendering acceptance.

The exact245 packaged executable also passes bundled-loader `--list` inside this Sniper CPU environment: all nine resolved library entries point into the extracted archive, with no missing entries or resolved Nix paths. [Exact library hashes](qa/steam-runtime-current245-link-review-20261010.json) are retained. Dynamic-link resolution does not exercise Vulkan/ALSA loading, window creation, gameplay or persistence.
