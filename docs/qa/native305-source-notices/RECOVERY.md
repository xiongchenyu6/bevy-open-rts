# SDK305 recovery supplement, 2026-10-12

This supplement corrects and extends the initial AUDIT.md without altering its historical evidence or the sealed399 package. All 1,238 initial bundle-manifest hashes still pass. recovery-sha256.json hashes every new recovery file. Recovery is preparation only; complete distribution and commercial-rights review remains open.

## Completed materials

The official Creative Commons CC BY4.0 full legal code is copied to recovery/cc-by-4.0/legalcode.txt. The sealed LOGO_LICENSES creator statement and source link are retained byte-for-byte alongside it. Primary retrieval: https://creativecommons.org/licenses/by/4.0/legalcode.txt

Retrieved UTC: 2026-10-11T22:45:03.546145+00:00; full-text SHA256: `9ba9550ad48438d0836ddab3da480b3b69ffa0aac7b7878b5a0039e7ab429411`.

Exact SDK305 Debian source archives were already locally available, correcting the initial report's statement that exact source inputs were unestablished. All 19 .dsc, .orig, .debian/.diff and signature files for six providers are now copied into recovery/runtime-sources. Independently fetched official Sources.gz is byte-for-byte identical to the cached source index; every source-file checksum and size matches that fresh index, and DSC member checksums also pass. The initial Nix companion remains irrelevant to this SDK package.

Official source index: https://repo.steampowered.com/steamrt4/images/4.0.20260805.254769/sources/Sources.gz

The packaging image was queried in a disposable, read-only, network-disabled container. Its actual dpkg source versions match the sealed305 runtime manifest: glibc2.41-12+deb13u3, ALSA1.2.14-1, libffi3.4.8-2, GCC14.2.0-19, Wayland1.23.1-3, libxkbcommon1.7.0-2. Its BUILD_ID is4.0.20260805.254769. `sdk-provider-metadata.txt` records the result; `packaged-library-buildids.json` records hashes and ELF Build IDs of all ten bundled libraries.

706 regular Debian patch/build-input files were extracted without symlinks or archive path traversal to recovery/runtime-build-inputs. This includes package build rules, patches and available supporting files; Wayland's Debian diff is retained verbatim decompressed. Indexes record source archives, members and hashes. No compile/rebuild was attempted, no signature trust chain is claimed, and no corresponding-source publication mechanism has been established. The fetched official source directory lists no .buildinfo file; this is a bounded directory check, not proof that no buildinfo exists anywhere.

## Genuine unrecovered originals

For each missing crate, the exact locked .crate was rehashed; the complete pinned upstream commit tarball and untruncated Git tree were freshly downloaded. Every regular source member was scanned for dedicated notice filenames and embedded copyright/permission/license statements. Full original license or NOTICE files were not recovered. Manifest and README SPDX/license declarations remain evidence of declarations and are not substituted with generic text.

- constgebra 0.1.4: commit `eae8e094e5779f42f1db2a7adf9036aa33744bc8`; no original recovered.
- hexf-parse 0.2.1: commit `4225763d744183d720f575ae96d04161b4d08ea0`; no original recovered.
- simd_helpers 0.1.0: commit `ca1a2f84aa386d758e98f8a609d990263932fb85`; no original recovered.

`rust-recovery-review.json` records exact-member hashes and matching declaration lines; `addition-index.json` records archive/tree URLs, revisions, retrieval timestamps and hashes. Missing originals, applicable NOTICE requirements, license selection and component-use review remain concrete open items. Asset/model/font/provider/marketing-rights review remains separate.
