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
