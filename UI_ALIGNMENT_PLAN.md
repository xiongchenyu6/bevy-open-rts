# In-match HUD alignment to godot-open-rts

Target = godot's `source/match/Match.tscn` HUD (source of truth for every region).
Baseline = current bevy build (`screenshots/base/base.png`), NOT the older image #2
(image #2 is from a richer earlier build that is not on the current branch).

## Region-by-region comparison

| Screen region | godot (target) | bevy current | Verdict |
|---|---|---|---|
| **Top-left** | Economy/power bar; **Objectives button** + **Briefing panel** (toggle, hidden by default) below it | Economy/power bar ✓; `StatsText` transient command feedback | Economy OK. Missing Objectives button; briefing is misplaced (see left-center). |
| **Top-center** | `ObjectiveTracker` (title + objective + **progress bar** + breakdown); below it `BattleNotifications` (battle log) | nothing centered | **Move objectives + battle log here.** |
| **Top-right** | `SupportPowers` — horizontal row of **9 ability buttons** (64px icons + cooldowns) | `BattleLogRoot` (top:74 right:18) | **Add support-power row**; relocate battle log out. |
| **Left-center** | (nothing) | `战斗简报` (MatchBriefing) — large panel, **open by default** | **Wrong.** Make it a hidden top-left toggle. |
| **Bottom-left** | **Minimap / Radar** (~215×215) | `SelectionText` + production-queue slots + `SelectionPortrait` | **Wrong.** Minimap belongs here; evict the rest. |
| **Bottom-center** | `SelectionInfo`: portrait + name + count + **HP bar** + rank + stats (320×126), between minimap & command grid | selection is bottom-LEFT, split across two nodes | **Move + consolidate** into one bottom-center panel. |
| **Bottom-right** | `ProductionQueue` (row of 72px slots) **above** `UnitMenus` command grid (**6-column** grid of 112px buttons) | Minimap + a **full-width** command card strip | **Wrong.** Command card → bottom-right grid; queue above it; minimap leaves. |

### Key misplacements (the "piled up / not godot" feel)
1. **Minimap** bottom-right → should be **bottom-left**.
2. **Selection/unit info** bottom-left → **bottom-center**, one consolidated panel.
3. **Production queue** bottom-left → **bottom-right, above the command grid**.
4. **Command card** full-width strip → **bottom-right 6-column grid**.
5. **Objectives** top-right → **top-center** with a progress bar.
6. **Battle briefing** always-open left panel → **hidden top-left toggle** via an Objectives button.
7. **Battle log** top-right → **top-center** (under the tracker).
8. **Support powers** none → **top-right ability row** (cf. image #2's F-keys).

---

## Stage 1: Minimap → bottom-left
**Goal**: minimap occupies the bottom-left corner (~196–215px square).
**Changes**: re-anchor `MinimapRoot` to `left:5 bottom:5`; update `minimap_content_rect`/`minimap_contains_cursor`/`minimap_world_position` to the new rect (currently dead — re-wire click-to-move).
**Success**: minimap bottom-left; click recenters camera; nothing else in that corner.
**Verify**: `capture base`. **Status**: Not Started

## Stage 2: Selection/unit info → bottom-center
**Goal**: one consolidated unit panel (portrait + name + HP bar + rank + stats), bottom-center between minimap and command grid.
**Changes**: replace bottom-left `SelectionText`+`SelectionPortrait` with a centered panel (`left:225 right:~815 bottom:5`, min 320×126); structured rows + a real HP bar node.
**Success**: selecting shows it bottom-center; hidden when empty.
**Verify**: `capture base`. **Status**: Not Started

## Stage 3: Command card → bottom-right grid + production queue above it
**Goal**: command card is a bottom-right 6-column grid; production-queue slot row directly above it (right-aligned).
**Changes**: re-anchor command-card container bottom-right (`right:5 bottom:5`, content width, 6-col wrap); move production-queue container to `right:5 bottom:(card+gap)` as a right-aligned 72px row (declutter already done in WIP).
**Success**: grid bottom-right; queue above it; no full-width strip; no bottom-left queue.
**Verify**: `capture base` (+ a producing structure). **Status**: Not Started

## Stage 4: Top bar — objectives center, briefing toggle, battle log center
**Goal**: objectives tracker (with progress bar) top-center; battle log top-center beneath; briefing a hidden top-left panel toggled by an Objectives button under the economy bar.
**Changes**: re-anchor `ObjectiveTrackerText` centered (`left 50% ±220`, top:82) + add progress-bar node; move `BattleLogRoot` to centered top; `MatchBriefing` hidden by default + new `ObjectivesButton`.
**Success**: matches godot top row; briefing closed until opened.
**Verify**: `capture base`. **Status**: Not Started

## Stage 5 (optional): Support-power row top-right
**Goal**: top-right row of ability buttons (icons + cooldowns) for support powers (RadarSweep…Paradrop), like godot `SupportPowers` / image #2 F-keys.
**Changes**: `setup_support_powers` — right-anchored top row of 64px buttons bound to `SupportPowerKind`; per-ability icon + cooldown label; wire F1–F9.
**Success**: row top-right; reflects cooldown; click / F-key triggers power.
**Verify**: `capture base`. **Status**: Not Started

---

## Notes
- One stage = one commit; build warning-free + `capture base` before moving on.
- `minimap_world_position*` are currently dead (click-to-move not wired) — Stage 1 re-wires, not just moves.
- Current WIP (queue declutter: removed `ProductionQueueText`, moved `SelectionText` up) folds into Stages 2–3 — keep it.
- Order rationale: Stages 1→3 fix the bottom row (the most-divergent, highest-impact); Stage 4 fixes the top; Stage 5 is additive.
