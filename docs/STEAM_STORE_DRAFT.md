# Steam Store Copy Draft

Working title: **Open RTS**. This is reviewable copy, not a published listing.
Current release acceptance remains in [RELEASE_READINESS.md](RELEASE_READINESS.md).

## Short description

Build a base, control resources, and command infantry, armor, and aircraft in a stylized sci-fi RTS. Hold the line through a six-mission campaign or choose your faction and battlefield in skirmish.

## About this game

Build your foothold, protect your power supply, and choose where to fight.
Open RTS combines base construction and resource gathering with infantry,
armored vehicles, aircraft, and support powers.

The six-mission campaign moves from holding a defensive line to breaking a
siege and striking enemy command centers. Each mission has its own objective;
protecting your headquarters and replacing losses matter as much as pushing
forward.

In skirmish, choose Alliance, Demon, or Chaos, select one of eight battlefields,
and configure the players, teams, starting resources, and AI difficulty.
Scout the terrain, counter enemy forces, and defend the economy that keeps
your army moving.

Boot Camp teaches gathering, construction, production, movement, and combat
through guided play. Tactical pause lets you issue orders while the battle
clock is stopped. Offline matches support a local quicksave, and campaign
completion is stored separately from the current match.

## Internal claim review — do not copy into Steam

| Claim | Source and required acceptance |
| --- | --- |
| Six missions | `src/campaign.rs`; old pinned ordered sequence/persistence passes; new same-App/retry/AI audit pending. |
| Three skirmish factions/eight maps | Registry and `src/maps.rs`; old72-case canonical matrix resolves; balance and current AI revalidation pending. |
| Four AI difficulty choices | Source profiles exist; old difficulty sweep has genuine timeouts. Do not claim balanced difficulty yet. |
| Boot Camp/pause/local saves | Existing runtime proofs; broadened unit-order restoration and current source checks pending. |
| Online multiplayer | Implemented; current artifact peer/host-human protection and forced-relay match verification required before adding store feature claims. |
| Windows/Linux support | Intended targets; Wine/development and Linux checkpoint evidence do not yet establish final release/platform acceptance. |
| System requirements | Leave CPU/GPU/RAM/storage numbers unset until measured on final packages. |

Controller, Steam Deck, Steam Cloud, achievements, pricing, and a release date
have no approved listing claims here. This preserves the verification scope;
it does not remove any required game feature from the release plan.

The copy uses plain product language and omits promotional links, dated claims,
and simulated Steam interface graphics, following the
[Steamworks description guidance](https://partner.steamgames.com/doc/store/page/description?l=english).
Keep review notes and this reference outside the player-facing description.
