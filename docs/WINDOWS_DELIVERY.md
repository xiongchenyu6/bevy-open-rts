# Windows Desktop Delivery

Build on 64-bit Windows with Rust 1.98.0 and Visual Studio 2022 Build Tools
(Desktop development with C++, MSVC and Windows SDK). From the repository root:

```powershell
rustup target add x86_64-pc-windows-msvc
cargo build --locked --release --target x86_64-pc-windows-msvc --bin bevy-open-rts
./scripts/package_windows.ps1
```

The manual **Build Windows desktop package** GitHub workflow performs these
steps on a Windows runner and retains a downloadable artifact. It does not
publish a GitHub Release or upload to Steam. The default build includes audio.
The lockfile fixes dependency versions; the workflow pins the Rust version.
Byte-identical builds are not promised.

The package contains the AMD64 executable, the complete `assets/` directory,
`launch.cmd`, licenses, and build metadata. Packaging checks the PE architecture
and refuses to replace existing artifacts. Output is
`dist/bevy-open-rts-windows-x86_64.zip` with a SHA-256 sidecar. Unzip it before
running `launch.cmd`; keep the assets next to the executable. Installation paths
with spaces are supported. Launcher arguments pass through to the game.

The launcher sets `BEVY_ASSET_ROOT` to the installation and runs from
`%LOCALAPPDATA%\bevy-open-rts`. The game independently resolves saves, campaign
progress and settings in that user directory; the installation may be read-only.
An absent or unwritable LOCALAPPDATA directory is an error, not a reason to write
into the installation. The executable also supports direct launch with adjacent
assets, but the launcher supplies an explicit asset root and writable working
folder.

## Runtime release gate

Before distribution, test the extracted package on a clean Windows machine
without Rust or Visual Studio installed. Verify startup, asset loading, audio,
menu navigation, a moving/combat match, save/reload and settings persistence.
Repeat under an installation directory without write access. Check GPU support
and record OS/GPU/driver, logs, screenshots, and package checksum. MSVC runtime
imports must be inspected on the produced executable; redistribute any required
runtime through its supported installer rather than assuming the SDK is present.

Wine can provide an additional executable launch smoke test:

```sh
wine cmd /c 'Z:\absolute\path\to\package\launch.cmd'
```

Use a dedicated Wine prefix. Confirm the actual window and logs, then save and
reload; a process starting alone is insufficient. Wine success does not establish
clean Windows hardware compatibility.

## Current evidence and limits

On 2026-10-08 this Linux workspace had Wine 11.0, a Linux-only Rust 1.98.1
sysroot, and a cached Rust 1.96.1 Docker image with only the Linux target and no
MinGW compiler. No existing Windows game executable was found in `target/`.
Consequently no Windows executable build, package execution, or Wine launch has
yet been verified. The new PowerShell packager and CI workflow require execution
on Windows; their presence is not a passed runtime gate.
