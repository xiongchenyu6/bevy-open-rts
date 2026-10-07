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

`BUILD_INFO.json` records source binary hash, build profile, and bundled-library provenance. The accompanying `.sha256` file verifies archive integrity. Debug archives are validation artifacts, not release candidates.

Test extraction and launch from `/tmp` in a Steam Runtime container with **no `/nix` mount**, using a Wayland compositor and software Vulkan. Check asset loading, main menu, campaign/tutorial entry, gameplay, quit, persistence, and restart. Repeat on physical AMD, Intel, and NVIDIA GPUs with audio and read-only installation. Preserve logs/screenshots and identify the exact archive hash. A startup-only test does not prove gameplay or commercial readiness.

Before distributing bundled libraries, complete their license notices and corresponding-source obligations, plus the game asset license inventory. Library provenance alone does not satisfy those requirements.

## Windows gate

A Windows package remains unverified. Build a native Windows release, include assets beside the executable, resolve assets from the installation directory, and store saves under `%LOCALAPPDATA%/bevy-open-rts`. Test a clean Windows machine without Rust/Nix, a non-repository calling directory, read-only installation, fresh profile, saving/loading, audio, and supported GPUs. Linux success does not prove Windows or Steam Deck compatibility.

## Current evidence (2026-10-08)

The development executable was packaged and extracted under `/tmp`, then launched from `/tmp` in `rogue-commander-linux-smoke:ubuntu24` with no `/nix` directory or mount. The bundled loader resolved every direct dependency from the extracted package. Mesa 25.2.8 / llvmpipe LLVM 20 initialized Vulkan, and the native game created its window under headless Weston. An intentional timeout stopped the smoke run. This proves native startup outside the build machine's Nix store, not full gameplay, audio, GPU hardware coverage, or release readiness.

The existing sniper smoke image contains Mesa 20.3 / LLVM 11; its software Vulkan driver crashed during adapter enumeration. Steam Runtime compatibility remains unverified and needs a supported recent driver environment or an actual Steam Runtime build. Do not distribute this development archive as a commercial build.

The current native development binary was repackaged after the compact HUD, sandbox Boot Camp, and final English/Chinese CSV font coverage changes. Source executable SHA256: `e511c402d46a89ffa533cc720509981540df98ffb8e36b7cbfdf46cc4ae31d44`. Archive SHA256: `5592a4575619cfbbf79a4dccc6c4fab1c3e510b4cc744d228ac814e1b2baf121`. The artifact is `/tmp/rts-linux-final-assets-20261008/bevy-open-rts-linux-x86_64.tar.gz`; its `BUILD_INFO.json` labels the profile `development`.

The exact final-assets archive was extracted and mounted read-only for a 20-second no-Nix Ubuntu 24.04 startup smoke, from `/tmp`, with an independent XDG profile. Vulkan initialized and the native game created its window. Writing to the installation failed with “Read-only file system”; the restricted logo was absent. The font inside the tested archive is 214,352 bytes, SHA256 `e4be13e4f6c1f226175906922777d5aeede8408601dbd7cdd92480ba5773c0af`. Current asset provenance, hash records, font notices, and `ASSETS.md` are included.

Evidence files beside the archive are `readonly-launch.log`, `readonly-result.txt`, and `readonly-mounts.json`. Intentional timeout exit code 124 is expected. The container has no sound device, so audio hardware remains unverified. These checks do not remove the release-profile, Steam Runtime, hardware, gameplay, licensing, or Windows gates above.
