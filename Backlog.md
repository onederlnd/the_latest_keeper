# The Latest Keeper — Backlog

## Pending verification

- [ ] Run `cargo test`
- [ ] Playtest one full loop at test values: tutorial, first burial, discovery, village trip, tend, milestone, sacrifice, life 2, theme picker
- [ ] Save and load mid-burn and mid-autopsy to confirm corpse and burn timer restore

## Decisions pending

- [ ] **Theme picker:** blocking and non-dismissible. Revisit after seeing gameplay.
- [ ] **Empty rites:** left as is ("No reagents available."). Revisit after playtest.
- [ ] **Carry-over:** 20% of the milestone, pursued faction only. Confirm by playtest.
- [ ] **Progression values:** test values active (8/15, 16/25, 24/35, 32/45). Swap to 40/100 … 220/350 before release.

## Code cleanup

- [ ] `base_quantity` only affects payment. Intended?
- [ ] Unused variables in `main.rs`: `last_seen_day`, `all_discovered`

## Post-demo

- [ ] Quest content and quest log
- [ ] Variable body count
- [ ] Graveyard upkeep chores
- [ ] Exploration
- [ ] `preserved` and `bound` gameplay effects
- [ ] Reagent bar rework
- [ ] Content for the remaining 7 villages
- [ ] Per-life tables for lives 2-8
- [ ] Functional Map window
- [ ] Win condition and ending
- [ ] Onboarding follow-up: re-check tutorial pacing after playtest

## Done

- [x] #2 Progression values (test values kept for dev length)
- [x] #3 Weighted-tag rep (tests pass)
- [x] #4 Village reagents in `items.ron` (tests pass)
- [x] #5 Input gate (`ui_open`); clock stops only on Esc pause
- [x] #6 Sacrifice cleanup (`burn_timer`, `last_tended`)
- [x] #7 Theme picker left as is
- [x] #8 Empty rites left as is
- [x] #9 Tutorial triggers (Inventory toggle, disposal choice at autopsy end)
- [x] #10 Save corpse and burn timer
- [x] `T` tend handler: `in_current_pair` gate, off-pair gives no rep
- [x] Stipend carries over on sacrifice
- [x] Removed `StokeIncinerator` from `ActionKind`; removed `last_tended.clear()` from the load handler
- [x] Removed unused `has_side` in `rite.rs`
- [x] `tutorial.ron`: new welcome text (WASD, `E`) and indent fix on `tut_reagent_toggle`