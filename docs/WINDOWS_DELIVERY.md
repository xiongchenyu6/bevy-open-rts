# Windows Desktop Delivery

## Native release build

Build on 64-bit Windows with Rust 1.98.0 and Visual Studio 2022 Build Tools
(Desktop development with C++, MSVC and Windows SDK). From the repository root:

```powershell
rustup target add x86_64-pc-windows-msvc
cargo build --locked --release --target x86_64-pc-windows-msvc --bin bevy-open-rts
./scripts/package_windows.ps1
```

The manual **Build Windows desktop package** GitHub workflow performs these
steps on a Windows runner and retains a downloadable artifact. It does not
publish a release or upload to Steam. Audio is included. The lockfile fixes
dependencies and the workflow pins Rust; byte-identical builds are not promised.
This native MSVC workflow has not yet been executed.

The package contains the AMD64 executable, runtime `assets/`, `launch.cmd`,
licenses, build metadata, and the asset inventory/provenance files
`ASSETS.md`, `ASSET_PROVENANCE.md` and `asset-source-hashes.json`. Local full-font sources (`*.full.ttf`) are excluded;
the runtime font and license notices are included. Packaging checks PE architecture
and refuses to replace existing artifacts. Unzip before running `launch.cmd`;
keep assets next to the executable. Installation paths with spaces are supported.
Launcher arguments pass through to the game.

The launcher sets `BEVY_ASSET_ROOT` to the installation and runs from
`%LOCALAPPDATA%\bevy-open-rts`. The game independently resolves saves, campaign
progress and settings in that user directory. An unavailable user directory is
an error, not a reason to write into the installation. Direct launch also works
with adjacent assets; the launcher supplies an explicit asset root and writable
working folder.

## Linux cross-build

```sh
./scripts/package_windows_cross.sh debug finalui-20261008
# Release candidate: ./scripts/package_windows_cross.sh release
```

This uses Docker `rust:1.96.1-bookworm`, Windows GNU standard libraries and
MinGW POSIX GCC. It builds with one job into `target/windows-cross`, separately
from Linux builds. Logs are `logs/windows-cross-debug.log` or `-release.log`.
The debug command also builds `capture.exe` for runtime verification. The optional
second argument adds a checkpoint tag to the directory, archive, and log names;
only letters, numbers, underscores, and hyphens are accepted. Existing packages
are never replaced.

GNU packages explicitly identify their target/profile. Debug sections are
stripped from the copied executable; the original remains intact. Required GNU
runtime DLLs are determined from PE imports, which are retained in
`PE_IMPORTS.txt`. Source/binary hashes and dirty-worktree status are recorded.
A debug package is investigation material, not release performance evidence.
Review compiler-runtime distribution obligations if a build imports GNU DLLs.

## Verified preliminary evidence — 2026-10-08

The Windows GNU dependency build completed in 39m19s. The final frozen-source
game and capture binaries then rebuilt successfully in 4m30s with Rust 1.96.1,
`--locked` and default audio. Logs: `logs/windows-cross-build.log` and
`logs/windows-cross-debug.log`. The resulting debug package is
`dist/bevy-open-rts-windows-gnu-x86_64-debug.zip`; use its `.sha256` sidecar.
Metadata retains the build-time commit plus dirty-worktree flag, not a claim
that a later commit was rebuilt cleanly.

Wine 11.0 used an isolated `target/windows-wine-prefix`. The game executable
selected the NVIDIA RTX 3070 Laptop GPU through Vulkan and created its window
(`logs/windows-wine-game.log`). DXGI initialization failed before Vulkan succeeded.
The final Windows capture executable produced the inspected English menu at
`screenshots/windows-wine/menu.png` (`logs/windows-wine-menu.log`, exit 0).

The Windows save/load flow passed with the package directory read-only and a
separate user-data sandbox. Actual Ctrl+S/Ctrl+L restored resources, entity
counts, paid production queue, simulation time and explored cells. Missing and
corrupt saves preserved the match and displayed feedback. Evidence:
`logs/windows-wine-save-load.log` (exit 0),
`screenshots/windows-wine/save-load/observations.json` and five screenshots.
The real save was written under `target/windows-save-sandbox/bevy-open-rts`.
Wine filters host XDG variables and supplies its own LOCALAPPDATA, so the proof
runner set matching Windows-absolute XDG_DATA_HOME and LOCALAPPDATA **inside a
Windows batch file**, before launching the save/load harness.

## Refreshed debug checkpoint — 2026-10-08

The one-job cached Rust 1.96.1 GNU refresh completed in 3m00s (exit 0), with
unchanged default features/profile. Durable unit:
`rts-windows-finalui-20261008.service`; log:
`logs/windows-cross-debug-finalui-20261008.log`. Compile-start source hashes,
dirty state, and four later capture-only file differences are recorded in
`logs/windows-refresh-finalui-20261008-context.json`. The game source includes
the setup-summary and finale duplicate-Map fixes. Windows finale victory behavior
has not been exercised; source inclusion alone does not prove that UI gate.

Archive: `dist/bevy-open-rts-windows-gnu-x86_64-debug-finalui-20261008.zip`,
SHA256 `90f340fd7287e3de9c1a6b1f91452d4ccefd265d1a84a46af602ffad66844624`.
Source executable SHA256:
`20f1c8f4b397ad8db9e64c4afbd3e157ec46b15ad0ba3ce163e721cc741afe60`;
packaged executable SHA256:
`b89bc3551cf14836f6c5cccc87b9f14f9203b6f20b1884cd11e4bd125c00aa83`.
The earlier untagged archive remains untouched (SHA256
`23819149bf202a08eaa29ba7e43dd76804c129b3d8773a80ce090997d8d62bcb`).
The refreshed README equals the player guide byte-for-byte; ignored full-font
sources and the restricted logo are absent.

The exact packaged player executable also launched through its shipped `launch.cmd`,
selected NVIDIA RTX 3070 Laptop GPU / driver 615.71.09 / Vulkan, and created
its window (`logs/windows-finalui-player.log`; intentional timeout exit 124).
DXGI initialization failed before Vulkan fallback. Xvfb root-window readback
was black (`player-menu.png`); it is not visual player-menu proof.

Wine 11.0 ran serially on isolated Xvfb `:95`, without host desktop input.
The separate capture helper (not shipped) rendered the full English command menu
and passed ordinary mouse Save/Load, including paid queue/resource/time/fog
continuity and missing/corrupt-save feedback. A second process loaded the same
save using the mouse menu, verified continuity, and preserved save bytes.
The exact archive was extracted under `/tmp` and made non-writable;
the write probe failed with Permission denied. User data used a separate sandbox.

Inspected evidence: `screenshots/windows-finalui-20261008/menu.png`,
`save-menu/save_menu.png`, `save-menu/observations.json`, and
`resume/00_resumed.png` / `resume/observations.json` below the same directory.
Logs: `logs/windows-finalui-menu.log`, `windows-finalui-save-menu.log`,
and `windows-finalui-resume.log` (all exit 0).
Helper SHA256: `1b08cdabd07ad6d2087cca44c1559570a3c42d20d29327ad7a1aa1accb25a8da`.
These are debug/Wine functional checks; they do not establish native Windows,
MSVC, release performance, audible output, or multiplayer interoperability.

## Remaining release gate

Wine success does not establish clean Windows hardware compatibility. Test the
extracted native release package on Windows without Rust or Visual Studio.
Verify startup, asset loading, audio, menus, movement/combat, save/reload and
settings persistence; repeat with installation write access disabled. Record
OS/GPU/driver, logs, screenshots and package checksum. Inspect MSVC runtime
imports and use the supported redistributable installer when required. Windows
release performance, audio, online interoperability and clean hardware execution
remain unverified.
