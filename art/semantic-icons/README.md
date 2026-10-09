# Original semantic icon candidates

Twenty repository-authored vector pictograms use original geometric paths and
the game's teal, gold and pale-metal palette. There are no imported images or
path definitions. The individual icons contain no text; the licensed repository
font appears only in the review sheet.

`candidates-v3/` is the selected candidate set. Each PNG is128×128 RGBA8 with a
transparent background. Two independent resvg0.48.1 runs produce identical
bytes; source/output hashes and decoded-alpha checks are retained alongside
the72px review sheet. Earlier candidates/reproduction copies remain local and
are excluded from Git.

Reproduce into a new directory:

```sh
python3 art/semantic-icons/generate.py candidates-review
```

The generator refuses an existing output directory. Original SVG source and
generated PNGs follow the repository's dual MIT/Apache license. Current game
assets are unchanged: actual command-card/support-panel rendering, tooltip
identity and supported-window review must pass before integration.
