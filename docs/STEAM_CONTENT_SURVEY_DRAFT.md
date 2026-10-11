# Steam Content Survey Draft

Unsubmitted development draft, checked against the [Steamworks Content Survey
documentation](https://partner.steamgames.com/doc/gettingstarted/contentsurvey)
on 2026-10-12. Steam distinguishes content produced during development from
content generated while the product runs; disclosure does not replace the
distribution rights and marketing review.

## Pre-generated AI content

Proposed description:

> AI tools were used during development to create campaign illustrations,
> the menu background, an armor texture, prerecorded unit and announcer voices,
> and UI artwork. These are packaged as fixed content; the game does not
> generate images, voices or other generative AI content during play.

The installed inventory includes six original campaign illustrations and 32
fixed English/Chinese voice files. Native entity portraits are renders of the
actual Kenney/authored game geometry, with separate model-source records.
The original generated desert menu background is now installed and passed
three native menu/setup screenshot checks at 900×720 and 1920×1080. Original
cursor graphics are authored SVG geometry rasterized into a fixed four-frame
atlas; native loaded pixels and state metadata were checked, while final
browser appearance and click alignment remain unverified. Source records are
in `art/campaign/`, `art/voice/`, `art/portraits/`, `art/menu-background/` and
`art/cursors/`. Six unused legacy VFX textures were retired from runtime assets.
These development checks do not establish final-package acceptance or complete
provider-term and distribution-rights review.

The current promotional illustration and capsule derivatives also use generated
art, recorded in `art/steam/keyart-v1/` and `art/steam/capsules-v2/`. They remain
unpublished and are illustrations, not gameplay captures. Valve focuses this
survey section on content players consume, rather than developer efficiency
tools. Keep coding-tool history in provenance records; reconcile any AI-assisted
player-facing narrative and localization before submitting the final description.

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
