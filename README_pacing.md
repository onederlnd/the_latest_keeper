# The Latest Keeper — Pacing & Progression Guide

Secondary README. Systems live in `README.md`, narrative in `README_STORY.md`. This file covers **how long each life should take, in what order, and what the player does in each.**

> Status: design target, decisions locked, numbers to confirm by playtest.

---

## 1. Locked decisions

- **Full 8-life run, no early ending.** The run only ends after all 8 sacrifices — one sacrifice per faction is not a valid ending.
- **Partial reset with carry-over.** On sacrifice, reputation mostly resets, but the next life starts with **20% of the milestone just reached** (starting value — see section 4 for why, and playtest before locking it further). Only the faction the next life pursues receives the carry-over; the other starts at 0. Discovered villages, fired story beats, milestones, memory fragments, and Cellar records always persist regardless.
- **Village order is player-chosen, in Village/Cult pairs.** After each completed pair, the player picks the next theme (Grief, Love, Guilt, Hope) from whichever aren't done yet. A theme's Village life always comes before its Cult mirror.
- **Thresholds are keyed to pair order, not theme.** Whichever theme is picked 1st uses the 100-rep threshold for both its lives, 2nd uses 170, 3rd uses 260, 4th uses 350 — the same stagger as before, just decoupled from a fixed theme order. These live in `data/progression.ron`.
- **Idle time is filled by activity, not more bodies.** Corpses stay at 1/day (occasional 0 or 2 planned). The day fills with tending, quests, upkeep, and exploration instead.

## 2. Assumptions

- Day = 7.5 min real time, target pace ≈ 4.2 rep/day.
- Both lives in a pair share the same milestone number.
- Multi-step burial and disposal choice are now built (see `README.md`); their timing is part of "filling the day," not part of the rep pace.
- `progression.ron` is currently on test values (8/15, 16/25, 24/35, 32/45). The tables in sections 3-5 use the real values (100/170/260/350).

## 3. Full Run Overview (with carry-over)

Carry-over = 20% of the rep total reached at the end of the previous life. Days rounded up; hours = days × 7.5 / 60.

| Life | Pair | Threshold | Carry-in | Rep still needed | Days | Hours | Total hrs |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 1st (Village) | 100 | 0 | 100 | 24 | 3.0 | 3.0 |
| 2 | 1st (Cult mirror) | 100 | 20 | 80 | 19 | 2.4 | 5.4 |
| 3 | 2nd (Village) | 170 | 20 | 150 | 36 | 4.5 | 9.9 |
| 4 | 2nd (Cult mirror) | 170 | 34 | 136 | 33 | 4.1 | 14.0 |
| 5 | 3rd (Village) | 260 | 34 | 226 | 54 | 6.8 | 20.8 |
| 6 | 3rd (Cult mirror) | 260 | 52 | 208 | 50 | 6.3 | 27.1 |
| 7 | 4th (Village) | 350 | 52 | 298 | 71 | 8.9 | 36.0 |
| 8 | 4th (Cult mirror) | 350 | 70 | 280 | 67 | 8.4 | 44.4 |

**Full run: about 44 hours.** Carry-in for each life is 20% of the *threshold*, not the previous life's carry-in — a keeper sacrifices at exactly their milestone, so 20% of that milestone is what the next life starts with. Which theme fills which pair slot doesn't change these numbers, since thresholds follow pair order.

**Why this is better than a flat reset:**
- Every life after the first gets shorter relative to its threshold, so the grind doesn't feel identical each time.
- Lives 7-8 drop from a flat 10.5 hours each to about 8.4-8.9 hours — still the longest lives (as they should be, being the climax), but not dragging as much as a flat reset would.
- It reads as the keeper's continuity growing across lives, which fits the reincarnation story better than starting from nothing every time.

**To retune:** the whole table moves if the carry-over percentage changes, the per-burial rep average changes (via `factions.ron` multipliers), or new idle-time content changes how much of the day goes to rep-earning versus other activities. Re-run the table after any of those change.

## 4. Life 1 — First Village, Village Side (24 days, about 3 hours)

This table doesn't depend on which theme is picked first — it's always the shape of "life 1," just with the chosen village's name and NPC.

| Day | Time | Rep | What the player does | Unlocks / beats |
| --- | --- | --- | --- | --- |
| 1 | 0:08 | 4 | Tutorial, pick up reagents (`F`), first full burial cycle (autopsy → dig → bury) | `first_pickup`, `first_burial` |
| 2-3 | 0:22 | 12 | Toggle reagents, try candle vs grave dirt, feel the disposal-method weight | `first_reagent_toggle`, `first_reputation_conflict` |
| 4-6 | 0:45 | 24 | Learn requested-tag bonuses, start leaning Village (bury, not burn) | `village_trust` beat |
| 7-9 | 1:08 | 37 | Keep stacking holy rites, hear the proximity hint toward the village | `village_awakening` beat |
| 10 | 1:15 | 42 | Rep passes 40, village discovered | Discovery announce |
| 11-13 | 1:38 | 55 | First trip (`E` to enter), meet the NPC, first tend (`T`), pick up the village reagent | `first_village_travel` |
| 14-17 | 2:08 | 68 | Daily loop: burial, walk to the village, tend, restock | Village reagent in rites |
| 18-21 | 2:38 | 84 | Push up, village reagent + Candle for stronger holy swings | NPC dialogue shifts |
| 22-23 | 2:53 | 95 | Final push, first look at the Cellar | `first_cellar_visit` |
| 24 | 3:00 | 100 | Milestone fires, ritual unlocks | `R` prompt |

**Sacrifice:** Cellar → `R` → pick up to 5 items → `Enter`. Writes a Cellar record, stores a memory fragment, resets most rep (20 carries into life 2), resets inventory, becomes Keeper #2.

**After sacrifice — theme-pair prompt:** since the player already picked this theme before life 1, life 2 is automatic (the Cult mirror of the same village pair). The next real choice comes after life 2.

## 5. Life 2 — First Village, Cult Mirror (about 19 days, about 2.4 hours)

Starts at 20 rep (carry-over), needs 80 more to reach 100.

- Same beats as life 1, compressed — the player already knows the controls, so tutorial-style beats (`first_pickup`, `first_reagent_toggle`) don't refire.
- Focus: reversing habits (profane-leaning rites, burning instead of burial).
- First real shot at a `reconciled` rite if the paired village's reagent is available (e.g. this village's flower + its Cult-mirror's bloom, both sharing the pair's theme tag).
- Ends with a Cellar beat referencing Keeper #1 by name/number.

**After sacrifice — theme-pair prompt:** the player now picks the 2nd theme from the three remaining (Love, Guilt, or Hope, in whichever order Grief wasn't first). This is the first genuine open choice point, and where `README.md`'s open question on presentation matters (a Cellar vision, a dialogue prompt, etc.).

## 6. Lives 3-8 — Design Summaries

Each pair still needs its own day-by-day table once built (same shape as life 1/2, scaled to the pair's threshold). Summary of what's new per pair:

| Pair | Threshold | New thing introduced | Carries over |
| --- | --- | --- | --- |
| 2nd (Village + Cult) | 170 | Bound-tag rites (that theme's ring/locket pair). Longer grind, softened by carry-over | 1st pair's records, discovered village, memory fragments |
| 3rd (Village + Cult) | 260 | 4-reagent rites start to matter | All prior pairs' progress |
| 4th (Village + Cult) | 350 | Longest pair. Reconciled rites should pay off here, and this is where quests/upkeep/exploration matter most to keep it from dragging | All prior pairs' progress |

## 7. Filling the Day — Build Order

Per the "top down" build order: tending first (already built), then quests, then upkeep, then exploration.

| Priority | Activity | What it adds | Status |
| --- | --- | --- | --- |
| 1 | **Tending** | Rep nudge + stock refresh per discovered village, once per day each | Built |
| 2 | **NPC quests** | Talk to an NPC, get a small fetch/timing/tag task, turn it in for rep or reagents | First pass built (cooldown resets included); content and a quest log still to do |
| 3 | **Graveyard upkeep** | Small recurring hub chores (clear overgrowth, repair a marker) for minor stipend/rep | To design |
| 4 | **Exploration** | Small optional areas near villages with lore or rare reagents | To design |

None of these should move the rep pace — they exist to fill time and give the player agency over *how* they spend a day, not to speed up or slow down the milestone math in section 3.

## 8. Open Questions

- **Carry-over percentage:** 20% is the starting point. Confirm once quests/upkeep are in and a full pair can be played end to end. Also confirm whether the other faction should get any carry-over (currently 0).
- **Theme-pair selection presentation:** a theme picker exists; whether it becomes a Cellar vision, an NPC/dialogue prompt, or something else needs a design pass alongside the story beats in `README_STORY.md`.
- **Quest system scope:** how many quests per NPC, which are repeatable, and their `.ron` schema. Repeatable quests should keep `reward_rep` small.
- **Pair 3-4 length:** even with carry-over, lives 7-8 are still the longest (about 8.4-8.9 hrs). Confirm this feels like a climax and not a slog once quests/upkeep are in — content, not further threshold cuts, is the intended fix.
- **`preserved` / `bound` effects on pacing:** if `bound` raises carry-over, it shortens later lives and the section 3 table needs a re-run. Cap the bonus so lives 7-8 don't fall below the target length.