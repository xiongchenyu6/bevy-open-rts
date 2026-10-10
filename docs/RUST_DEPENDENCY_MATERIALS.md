# Rust dependency source and notice materials

The locked/offline Cargo metadata audit for Linux x86_64 default features finds
635 reachable external packages through normal/build dependencies. Development
edges are excluded. This is a dependency graph, not an exact linker input list;
other platforms require their own graph review.

All635 local `.crate` source archives match their Cargo.lock checksums. The
published-source companion is retained locally at
`target/linux-rust-source-companion-20261010.tar` (78,766,080 bytes), SHA256
`987d7be1f870d69e1e5d87866a05fec94005106d4ad1cac72e65f0f433a8a207`.
It contains the exact source archives, declared license metadata and collected
notice files. It has not been published or included in a player archive yet.

The audit copies1,102 top-level notice files from608 packages and verifies each
copied hash. For23 additional packages, original notices were fetched from the
precise Git commit recorded in the published crate, including ancestor-directory
licenses where applicable. URLs, commits and hashes remain in
`target/linux-resolved-dependency-metadata-20261010/`.

A subsequent archive-level review finds crc-catalog2.5.0 original licenses
in its published `LICENSES/` subdirectory, which the top-level filename scan
missed. Exact MIT/Apache texts and archive/member hashes are preserved in
`third-party/rust/crc-catalog-2.5.0/`; its archive checksum matches Cargo.lock.
The older companion and audit counts above are unchanged historical artifacts.

Three packages still have no matching notice file found: constgebra0.1.4,
hexf-parse0.2.1 and simd_helpers0.1.0. Their published source
archives and license declarations are retained. Do not invent copyright notices
or describe generic license terms as retrieved package originals. Complete
license applicability, attribution and distribution review remains open.

A repeat audit of each published crate's exact upstream Git commit finds no
dedicated notice filename in any of the three complete, untruncated trees.
Commit IDs, Cargo.lock checksums, tree URLs, response hashes and filenames are
recorded in `rust-missing-notice-audit.json`. This confirms the remaining source
material gap; it does not replace the missing notices or establish clearance.

Keep these Rust materials separate from the six bundled dynamic-library source
providers documented in `LINUX_RUNTIME_SOURCE_MATERIALS.md`. Neither inventory
establishes rights for game art, models, fonts, effects or voices.

## Nested published-source review

A checksum-verified scan of all635 archives identifies seven legal-directory
files: the two crc-catalog licenses above and five texts in
`wayland-protocols-plasma0.3.12/plasma-wayland-protocols/LICENSES/`. The latter
are preserved under `third-party/rust/wayland-protocols-plasma-0.3.12/` with
exact archive-member hashes. They belong to embedded protocol sources; the
crate-level MIT declaration alone does not describe this source inventory.
Final component use and distribution applicability remain to be reviewed.
The original source companion already contains those complete crate archives;
these extracted texts supplement its historical top-level notice collection.

## Inline notice audit

A checksum-verified scan of the three exact published archives also checks Rust
source, Markdown and Cargo manifests for embedded copyright/license statements.
Only manifest license declarations are found; no full original notice is recovered.
Member hashes and matching lines are recorded in `rust-missing-notice-audit.json`.
The missing-material and final distribution review requirements remain open.
