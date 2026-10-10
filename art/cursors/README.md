# Original RTS cursors

Four repository-authored SVG states use the game's existing atlas contract:
Default, Move, Attack and Build, in that order. Tiles are32×32; the atlas is128×32.
Default retains hotspot(2,1); all other states retain(16,16).

Generate a new directory with `python3 art/cursors/generate.py /new/directory`.
The checked-in candidates were rasterized by resvg0.48.1. Their manifest records
SVG, PNG and generator hashes. No external image or font is an input.

Two fresh generations reproduce all five PNGs byte-for-byte. Independent RGBA
decoding verifies transparent backgrounds, nonblank silhouettes and exact
dimensions. The32px and enlarged light/dark review sheet has been inspected.
Source geometry uses the repository's MIT OR Apache-2.0 terms.

These are candidates. Before replacing runtime assets, verify actual native and
browser custom-cursor loading, each state transition and hotspot alignment with
ordinary input. Retain the previous atlas outside runtime assets, update the
source inventory and revalidate the final package.


A private browser stage using immutable pre-Hold Wasm07 applies the default
candidate successfully (actual child exit0): the CSS custom image is32×32,
hotspot(2,1), and browser-decoded RGBA exactly matches the atlas's first tile.
No page errors occur. See `candidates-v1/browser-default-review.json`;
raw observations remain in `target/original-cursors-browser-01/`. This covers
only default image application, not the other three transitions, native cursor
behavior, pointer-to-order alignment or the final game build. Runtime assets
remain unchanged.


Two later ordinary-input recipes failed and remain recorded in
`candidates-v1/browser-transition-review.json`: a game-canvas screenshot times
out, and a subsequent patrol attempt still shows the default cursor. The
selected entity/command mode was not independently confirmed, so the latter
cannot be attributed to the atlas. Move/Attack/Build transitions remain open;
use an inspected selection/action recipe on the sealed final core before
runtime integration. These failures do not invalidate the bounded default
image application evidence or establish browser gameplay performance.
