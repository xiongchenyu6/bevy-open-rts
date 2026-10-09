# Linux runtime source materials

The current thin-LTO Linux candidate bundles libraries from glibc2.44-25,
Wayland1.26.0, libxkbcommon1.13.2, libffi3.8.0, GCC16.2.0 and ALSA1.2.16.1.
`BUILD_INFO.json` records the exact installed providers. The packager verifies
that these providers match `third-party/linux-runtime/manifest.json` and that
all13 verbatim notice texts match their recorded hashes before creating an
archive. Actual negative packaging checks reject changed providers and notices.

The local companion archive is `target/linux-source-materials-20261010.tar`.
It contains the six exact source inputs,18 ordered build patches, complete Nix
derivation metadata and the notices. Its manifest hashes45 files; each archived
file was independently checked against that manifest. Archive SHA256:
`5bc16e1d5ccf38261e9e3c34cc6aefccc8ec12798bd65c7f3345d70f98875540`.
The archive is139,581,440 bytes. The extraction and verification result is
`target/linux-source-materials-result-20261010.json`.

Keep the source companion with its matching binary and runtime notice manifest.
Before distribution, verify complete build inputs/instructions and a reproducible
library rebuild; these are not yet established. This companion is local and has
not been published. It covers the bundled dynamic-library providers only; Rust
dependencies and game assets have separate provenance requirements.
