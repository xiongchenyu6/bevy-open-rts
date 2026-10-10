# Steam Content Survey Draft

Unsubmitted development draft, checked against the [Steamworks Content Survey
documentation](https://partner.steamgames.com/doc/gettingstarted/contentsurvey)
on 2026-10-10. Steam distinguishes content produced during development from
content generated while the product runs; disclosure does not replace the
distribution rights and marketing review.

## Pre-generated AI content

Proposed description:

> AI tools were used during development for campaign briefing illustrations,
> prerecorded unit and announcer voices, and some UI artwork. Coding assistants
> also helped implement the game and author vector UI graphics. These outputs
> are reviewed and packaged as fixed game content.

The installed inventory includes six original campaign illustrations and 32
fixed English/Chinese voice files. Native entity portraits are renders of the
actual Kenney/authored game geometry, with separate model-source records.
The original menu background is currently a candidate; include it in the final
description only if its reviewed version is installed. Source records are in
`art/campaign/`, `art/voice/`, `art/portraits/` and `art/menu-background/`.

## Live-generated AI content

Proposed answer: none. The runtime uses fixed assets and ordinary game logic
for opponents. Generative image/speech models and the developer's ASR tools are
not shipped or invoked by the game.

## Final review still required

Inspect all six briefings, combat effects, voices, text and shipped files before
answering the general/mature-content fields. Combat includes explosions,
wreckage and unit destruction; regional ratings have not been established.
Reconcile this draft with the final archive inventory and close the outstanding
source/notice/provider-term gaps in `ASSET_PROVENANCE.md` and
`RUST_DEPENDENCY_MATERIALS.md`. No rights attestation or survey submission has
been made.
