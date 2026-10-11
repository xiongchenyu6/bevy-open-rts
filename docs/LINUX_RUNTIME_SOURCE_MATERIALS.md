# Linux runtime source materials

## Current official-SDK305 providers

The sealed SDK305 package contains Debian runtime providers: glibc2.41-12+deb13u3,
ALSA1.2.14-1, libffi3.4.8-2, GCC14.2.0-19, Wayland1.23.1-3 and
libxkbcommon1.7.0-2. Its recorded provider versions and ten library ELF Build IDs
were checked against the packaging image, BUILD_ID4.0.20260805.254769.

All19 exact source inputs match the independently retrieved
[official Steamrt source index](https://repo.steampowered.com/steamrt4/images/4.0.20260805.254769/sources/Sources.gz)
and DSC member checksums. The local archive is under
`target/steamrt4-library-source-companion-20261010/`; recovery includes706 regular
Debian patch/build files. Original and recovery hashes were independently
rechecked ([materials evidence](qa/native305-source-notices-review.json)).

These are local source materials. Signature trust, complete rebuild instructions,
reproducible library rebuild and a distribution/source-delivery mechanism remain
unverified. Rust dependencies, assets and generated-content rights are separate.
The Nix companion below covers its historical providers; it does not cover SDK305.

## Historical Nix companion

The historical Nix thin-LTO Linux candidate bundles libraries from glibc2.44-25,
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
