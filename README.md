# The Latest Keeper

A data-driven graveyard management sim built in Rust, inspired by the flow and timing systems of *Graveyard Keeper*. Run a graveyard, prepare the dead, manage reputation, and live through a chain of lives — fully offline, no external services required.

> **Status:** early playable prototype. Working core loop: corpse delivery and decay, reagent gathering, tag-based rites, multi-step burial or burning, two-faction reputation, daily restocking, village discovery and travel, tending, NPC quests (first pass), player-chosen theme order, milestone-gated sacrifice ritual with partial carry-over, Cellar records, main and pause menus, and manual save/load that works from the hub or inside a village.

**Currently building:** the weighted-tag reputation spec and the full 12-reagent roster, then the pacing pass in `README_PACING.md` (about 3 hours for life 1, about 44 hours for a full 8-life run) and the daily activities that fill the time.

Related docs: `README_STORY.md` (narrative), `README_PACING.md` (per-life pacing and design prompts).

---

## Concept

You maintain a graveyard. Bodies arrive, and how you prepare and dispose of them ripples out into a world of competing factions. There is no single "quality" score to maximize. Every rite carries **tags**, and each faction reacts to the reputation-linked tags differently. Please the Village and you anger the Cult. Time moves on a fixed clock whether you're productive or not, and unattended corpses rot, so the core tension is always **what do I do, for whom, with the time I have today.**

Reputation is also a gate. Reaching a high enough standing with a faction unlocks a **sacrifice** in the Cellar. That ends the current keeper's life, and you begin the next one as a new keeper. Each life pushes toward a different village.

## What makes this different from the genre template

- **Rites are emergent, not authored.** No fixed burial menu. You combine tagged reagents and the rite's tags come from what you combined.
- **Reputation is per-faction and in tension.** One rite can raise one faction and lower the other. No rite satisfies everyone, so every disposal is a decision.
- **Progress is a chain of lives.** Reputation resets on sacrifice, but discovered villages, the Cellar, and memory fragments carry over.

## Design pillars

- **One authoritative clock.** Every system reads a single shared clock.
- **Every day has more to do than time.** One corpse a day is the anchor, and errands, quests, village trips, and disposal steps fill the rest. *(Partly built; see Roadmap.)*
- **Tag-driven rites, not fixed recipes.**
- **Multi-faction reputation, not one score.** A faction's threshold is the gate to its sacrifice.
- **Zone-scoped storage.** Inventory is shared within a location, so placement and logistics matter.
- **Data-driven content.** Reagents, factions, zones, NPCs, quests, story beats, milestones, discoveries, progression tiers, tending, and tutorial steps live in `.ron` files.
- **Sleep-gated saves.** Saving at the bed. *(Not built; save is manual.)*
- **Built to expand, minimal dependencies.** Stable engine layer, growing content layer.
- **Multiplayer, eventually.** Deliberately deferred; single-player structs (`Economy`, `Zone`, `Corpse`) are kept clean to ease a later client/server split.

## Tech stack

| Concern | Crate |
| --- | --- |
| Windowing / rendering / input | [`macroquad`](https://crates.io/crates/macroquad) |
| Serialization (saves, data files) | [`serde`](https://crates.io/crates/serde) + [`ron`](https://crates.io/crates/ron) |

No ECS crate. Game state is plain structs and enums. No networking, no telemetry.

## Project structure

```bash
├── Cargo.toml
├── src/
│   ├── main.rs             — game loop, state, input, HUD, UI windows, map switching, wiring between systems
│   ├── action.rs           — TimedAction: hold-to-complete actions (autopsy, dig) with progress and cancel
│   ├── clock.rs            — shared GameClock: day, time of day, weekday
│   ├── zone.rs             — zone-scoped inventory, positions, containment, ZoneDef + Zone::from_def, ZoneRegistry
│   ├── world.rs            — MapId (Hub / Village), tilemap + village generation (seeded), VillageInstance, VILLAGE_STOCK
│   ├── corpse.rs           — corpse state machine: Delivered → Autopsied → GraveDug / Fueled → Buried / Burned, with decay timer and requested tag
│   ├── rite.rs             — reagents in, rite out (moving to weighted-tag resolution)
│   ├── faction.rs          — per-faction reputation and per-tag multipliers
│   ├── economy.rs          — stipend + multi-faction reputation settlement
│   ├── discovery.rs        — Discovery: which village a discovery reveals, its faction, and announce text
│   ├── milestone.rs        — Milestone: which village's sacrifice it unlocks, plus text and memory fragment
│   ├── progression.rs      — Tier: discovery and milestone rep for each pair slot
│   ├── tending.rs          — TendTask: daily tend per village (rep nudge, stock refresh)
│   ├── quest.rs            — QuestDef, QuestKind (Fetch / Timing / Tag), QuestStatus
│   ├── npc.rs              — NPC: id, schedule, position, faction, zone, sprite, reputation-flavored line
│   ├── story.rs            — StoryBeat: one-time beats triggered by reputation thresholds
│   ├── tutorial.rs         — TutorialStep: event-triggered onboarding text
│   ├── cellar.rs           — CellarRecord: name, years, epitaph, faction sacrificed to, village
│   ├── save.rs             — SaveData + RON save/load
│   ├── render.rs           — camera, AssetStore, TileMap / TileMapDef
│   └── render_helpers.rs   — grave markers, temples, progress bar, text wrap, rect hit-testing
└── data/
    ├── items.ron           — reagent definitions (12 entries: 4 base + 8 village)
    ├── factions.ron        — faction definitions (per-tag multipliers, starting rep)
    ├── zones.ron           — hub zones (Morgue, Workyard, Cellar, Keeper's Shack, Burial Site, Incinerator) + 8 village markers
    ├── npcs.ron            — NPC definitions (each has a unique id used by quests)
    ├── quests.ron          — quest definitions (npc_id, kind, rewards, cooldown)
    ├── story_beats.ron     — one-time narrative beats
    ├── tutorial.ron        — onboarding steps and triggers
    ├── progression.ron     — one tier per pair slot: discovery rep and milestone rep
    ├── discoveries.ron     — per-village discovery announce text (threshold comes from progression.ron)
    ├── milestones.ron      — per-village sacrifice text and memory fragment (the threshold field is unused)
    ├── tending.ron         — per-village tend flavor, rep amount, stock refresh
    ├── cellar_records.ron  — seed Cellar entries
    ├── tilemap.hub.ron     — optional hand-edited hub tilemap (falls back to procedural)
    └── tilemap_<id>.ron    — optional hand-edited village tilemaps
```

## Timing system

- One in-game day is 450s real time (7m30s). The weekday cycles through a 6-day week.
- One corpse is delivered per day when none is on hand.
- Corpses decay after autopsy. A corpse that reaches zero quality rots away, and the miss is counted against the current keeper's tenure.
- **Autopsy and digging** are hold-to-complete actions (`F`). Releasing or walking away cancels them.
- **The incinerator** is a background timer. Press `F` once to light it, walk away, and return when the log says the fire is ready. The corpse keeps decaying while it burns.
- Base reagents (Morgue, Workyard) restock daily. Amounts are hardcoded for now, independent of `zones.ron`.
- Village reagent stock restocks daily once the village is discovered, and only after discovery.
- Restock runs on any map. It reaches the hub and cached villages regardless of where the player stands.

## World

- **Hub map.** The graveyard: Morgue (autopsy), Workyard, Burial Site, Incinerator, Cellar (records and ritual), and Keeper's Shack (holds items carried through a sacrifice). Eight village markers ring it in two mirrored hemispheres.
- **Village maps.** Each village is its own generated instance (own tilemap, zones, NPCs). Walk up to a discovered village marker and press `E` to enter, and walk into the "Path home" zone to return.
- **Seeded generation.** Each village's layout comes from a stored seed, so a village looks the same every visit and after loading a save.
- **Undiscovered villages** show as `???`. Once a village's theme has been picked, it also gives proximity hints as you walk near it.
- **Terrain** is a tilemap (grass and path). The hub loads `data/tilemap.hub.ron` if present and otherwise generates a random-walk path toward each village. An in-game edit mode (`F2`) paints tiles and `F6` saves them.
- **NPCs** appear on a daily schedule and speak lines that reflect your faction reputation. Villages carry their own NPCs.
- **Interactions are proximity-gated.** Autopsy only in the Morgue, digging and burial only at the Burial Site, lighting and burning only at the Incinerator, the Cellar window and ritual only in the Cellar, tending only inside a village. Zone checks use the player's position, so stand inside the zone box.

## Corpse flow

```
Delivered ──(F, Morgue)──▶ Autopsied ─┬─(Shovel [2] + F, Burial Site)──▶ GraveDug ──(Space, Burial Site)──▶ Buried
                                      └─(F once, Incinerator, wait)────▶ Fueled  ──(Space, Incinerator)──▶ Burned
```

- Space needs at least one reagent selected (`I` or the slots above the tool bar). With none selected it logs "No reagents available." and nothing happens.
- Each disposal adds a hidden 0.5 tag weight: `holy` for burial, `profane` for burning.

## Quests

- Quests are defined in `quests.ron` and matched to an NPC by `npc_id`, which must match an `id` in `npcs.ron`.
- Kinds: **Fetch** (bring N of an item), **Timing** (be in a zone by a given day), **Tag** (perform a rite whose reagents carry a tag; the hidden disposal weight doesn't count).
- Talk to the NPC with `E` to accept and to turn in. Rewards can include stipend, reputation, and items. A completed quest resets once `cooldown_days` have passed; `0` means it never resets.
- Quest progress is saved. Quest content and UI (a quest log) are still thin.

## Story

Full narrative lives in `README_STORY.md`. Mechanically:

- **The Cellar** records each past keeper: name, years, an epitaph from the memory fragment their sacrifice unlocked (plus a note if bodies rotted during their tenure), and which faction and village they sacrificed to.
- **NPCs speak with continuity, not alarm.** Mother Elsbet has spoken to keepers before you and reacts to your standing, not to what you personally did.
- **Story beats** fire once when a faction's reputation crosses a threshold.
- **Sacrifice** is unlocked by a higher, village-specific milestone. Confirming it ends the tenure, moves the chosen carry-forward items (up to 5) to the Keeper's Shack, resets reputation and inventory, and starts the next keeper.
- **`reconciled` rites** (planned): a rite combining a kept-side and burned-side reagent with the same theme tag (for example Mourning Bell Flower + Ember-Ash Bloom, both `grief`) is flagged `reconciled`. No faction reacts to it. It's reserved for story beats and epitaphs foreshadowing the reunification ending.
- **Full discovery** of all 8 villages logs a line but is not a win state yet. Since a village can only be discovered once its theme is picked, this can only fire after all four themes have been chosen.

### Future idea: attended rituals

NPCs attending the sacrifice (mourners and onlookers reacting in character, with dark humor). This would be the first real use of NPC movement and pathing, which current stationary NPCs don't support. Needs its own design pass.

## Rites and reputation

### Reagents and tags

- **Reputation-linked tags:** `holy`, `profane`, `preserved`, `bound`. Factions have opinions about these.
- **Theme tags:** `grief`, `love`, `guilt`, `hope`. Flavor and story only.
- **Rite cap:** at most **4 reagents** per rite, counting repeats. The same reagent can be added more than once, up to the number you carry. This keeps "use everything" from dominating and echoes the 5-item sacrifice carry limit.

| Reagent | Tags | Source | Daily stock |
| --- | --- | --- | --- |
| Salt | preserved | Morgue | 5 |
| Prayer Candle | holy, preserved | Morgue | 4 |
| Grave Dirt | profane, bound | Workyard | 5 |
| Iron Nail | bound | Workyard | 4 |
| Mourning Bell Flower | holy, grief | Mournhollow | 1-2 (tend-scaled) |
| Ember-Ash Bloom | profane, grief | Ashmourn | 1-2 (tend-scaled) |
| Widow's Ring | holy, love, bound | Heartwarden | 1-2 (tend-scaled) |
| Forge-Scarred Locket | profane, love, bound | Heartforge | 1-2 (tend-scaled) |
| Confessor's Stone | holy, guilt | Penncross | 1-2 (tend-scaled) |
| Blackened Ledger Page | profane, guilt | Guiltpyre | 1-2 (tend-scaled) |
| Dawnseed | holy, hope, preserved | Hopewell | 1-2 (tend-scaled) |
| Broken Compass | profane, hope, bound | Hopeforsworn | 1-2 (tend-scaled) |

Base reagents are reliable and plentiful. Village reagents are scarce by design, so visiting and tending villages regularly is what keeps them usable.

### Rite resolution: weighted tags

Each reputation-linked tag gets a **weight** equal to the number of selected reagents carrying it. Theme tags are unioned onto the rite for the `reconciled` check but carry no weight.

- **Stacking:** two `holy` reagents move Village reputation twice as far.
- **Cancellation:** one `holy` and one `profane` net to zero for a faction with equal and opposite multipliers. No special-case logic needed.

No bespoke tag-combination behavior is planned.

### Faction reputation formula

```
Δrep(faction) = BASE_UNIT × Σ over reputation-linked tags [ faction.multiplier[tag] × weight[tag] ]
```

There is no separate `BASE_UNIT` constant in code — the per-tag multiplier in `factions.ron` sets the scale directly. The numbers below are the current defaults, tuned as part of the pacing pass (see Pacing).

| Faction | holy | profane | bound | preserved |
| --- | --- | --- | --- | --- |
| Village | +1 | -1 | +0.5 | 0 |
| Cult | -1 | +1 | +0.5 | 0 |

Worked examples at these multipliers:

- Prayer Candle alone → Village +1, Cult -1 (before the disposal weight).
- Prayer Candle + Mourning Bell Flower (`holy` ×2) → Village +2, Cult -2.
- Prayer Candle + Grave Dirt (`holy` 1, `profane` 1, `bound` 1) → both factions +0.5. Holy and profane cancel, and the shared `bound` nudges both up.
- **Disposal method** adds a hidden 0.5 weight (`holy` for burial, `profane` for burning) on top of the chosen reagents, outside the 4-reagent cap. It follows the same zero-sum rule. Example: Candle + Salt, burned → holy 1, profane 0.5 → Village +0.5, Cult -0.5.

### Corpse requests

Each delivered corpse requests a tag. Disposing of it with a rite that includes that tag pays a bonus stipend (+5).

### Discovery and milestone thresholds

Thresholds belong to **pair slots**, not villages. `data/progression.ron` holds one tier per slot (discovery rep, milestone rep). The player's theme picks (`chosen_themes`) map each theme onto a slot, so a village's thresholds are the tier of the slot its theme was picked for.

- A village only starts giving proximity hints once its theme has been picked.
- A village is discovered when its faction's rep reaches its slot's discovery value.
- The sacrifice unlocks when that faction's rep reaches the slot's milestone value **and** the target village has been discovered.
- Both villages of a theme (Village and Cult) share a slot, so they share thresholds.

Intended values:

| Slot | Discovery | Milestone (sacrifice) |
| --- | --- | --- |
| 1st pair | 40 | 100 |
| 2nd pair | 90 | 170 |
| 3rd pair | 150 | 260 |
| 4th pair | 220 | 350 |

**Testing values are currently active** in `progression.ron`: 8/15, 16/25, 24/35, 32/45. Switch back to the table above for real playthroughs.

- **No reputation decay.** Standing only moves through disposal, quests, and tending. If late-game snowballing needs curbing, prefer soft diminishing returns over decay.
- **Tending** (`T`, once per discovered village per day) grants a flat +2 reputation to that village's faction and refreshes 1-2 units of its reagent stock.

## Pacing

Full detail in `README_PACING.md`. Summary:

| Item | Value |
| --- | --- |
| Corpses per day | 1 (occasional 0 or 2 planned) |
| Target rep pace | about 4.2 rep/day |
| Full run (8 lives) | about 44 hours, growing over time as content is added |
| Ending | Full run only — all 8 sacrifices, no early ending |

- **Partial reset on sacrifice.** Reputation resets, and only the faction the next life pursues starts with a **20% carry-over** of the milestone just reached (tuning target, to confirm by playtest). The other faction starts at 0. Discovered villages, fired story beats and milestones, memory fragments, and Cellar records always persist.
- **Village order is player-chosen, in Village/Cult pairs.** The player picks a theme (Grief, Love, Guilt, or Hope) to pursue. That theme's Village life comes first, its Cult mirror comes second, and then the player picks the next theme. This replaces a fixed Grief → Love → Guilt → Hope order.
- **Thresholds are keyed to pair order, not theme:** the 1st pair (whichever theme is chosen first) uses 100 rep for both its lives, the 2nd pair uses 170, the 3rd 260, the 4th 350 — same staggering as before, just decoupled from which theme is picked when.
- **Filling the day** (rather than adding bodies): multi-step disposal (built), the incinerator's wait time (built), daily errands, NPC quests (first pass built), graveyard upkeep chores, exploration, and variable body counts. None of these change the rep pace by design.

See `README_PACING.md` for the full per-life table and the reasoning behind the carry-over number.

## Save / Load

- Manual: `F5` saves to a single unversioned `save.ron`, `F9` loads it. Works from the hub or inside a village.
- **Persisted:** stipend, faction reputation, day and time, player position, hub return position, inventory, stock for the hub and every visited village, current map, village seeds, fired beats, milestones and tutorial steps, memory fragments, keeper number and tenure start, Cellar records, discovered zones, last-tended map, pending sacrifice state, next corpse id, missed days this tenure, quest status, pair index, cult-life flag, chosen themes, last delivery day, last restock day.
- **Not persisted:** current corpse, incinerator burn timer, event log. Loading mid-day leaves you with no corpse until the next day's delivery.
- Loading rebuilds each village from its saved seed, then restores its stock.
- Main menu (New Game / Continue / Quit) and pause menu (Esc: Resume / Save / Load / Quit to Menu / Quit Game). One save slot, no schema versioning yet.

## UI

- **Fixed HUD:** status bar (day, weekday, time, stipend), corpse panel with a next-step hint, Morgue stock and carried counts, faction reputation, scrolling event log, tutorial banner, and a progress bar during timed actions.
- **Inventory window** (`I`): carried reagents. Left-click adds one to the next rite, right-click removes one, up to 4 total. Each tile shows how many you carry and how many are selected.
- **Cellar window** (`C`, inside the Cellar): past keepers and recovered memory fragments.
- **Sacrifice window** (`R`, inside the Cellar once unlocked): left-click a row to add, right-click to remove, up to 5 items. `Enter` confirms.
- **Theme picker:** appears whenever a new pair slot needs a theme, including at the start of a new game.
- **Map window** (`M`): placeholder.
- **Bottom bar:** four tool slots and a row of reagent slots above them. Each reagent slot shows two initials, a carried count, and a gold number when selected; hover for the full name. Left-click adds, right-click removes. The Shovel (`2`) is functional and required for digging; Hoe, Spell A, and Spell B are placeholders.

## Controls

| Key | Action |
| --- | --- |
| `W`/`A`/`S`/`D` or arrows | Move |
| `E` | Talk to a nearby NPC (accept and turn in quests), enter a discovered village, or pick up reagents from the zone in front of you |
| `F` | Corpse work. **Hold** in the Morgue to autopsy; **hold** at the Burial Site with the Shovel selected to dig; **press once** at the Incinerator to light the fire |
| `1`-`4` | Select a tool slot (Shovel is `2`; the rest are placeholders) |
| Left / right click reagent slot or Inventory tile | Add / remove one of that reagent for the next rite (max 4, repeats allowed) |
| `Esc` | Close a window, or open and close the pause menu |
| `I` | Inventory window |
| `C` | Cellar window (inside the Cellar) |
| `R` | Sacrifice window (inside the Cellar, once unlocked) |
| `M` | Map window (placeholder) |
| `T` | Tend the village you're standing in (once per day) |
| `Space` | Perform the rite and dispose of the corpse (at the dug grave or the lit incinerator, with at least one reagent selected) |
| `Enter` | Confirm the sacrifice |
| `F5` / `F9` | Save / load |
| `F2` / `F6` | Toggle map edit mode / save the current map's tiles |

## Roadmap

### **Done**

- [x] Core loop: clock, zones, movement, corpse pipeline, economy, inventory
- [x] Data-driven content via `.ron`
- [x] Tilemap rendering, procedural hub map, map edit mode
- [x] Speech bubbles, NPCs with schedules and reputation-flavored lines
- [x] Story beats, milestone system, sacrifice ritual, Cellar records
- [x] Reagent taskbar (mouse), Inventory / Cellar / Sacrifice windows
- [x] 8-village layout, `faction_id` on zones, village reagents
- [x] Discovery system and village instances (enter and exit)
- [x] Tending system
- [x] Corpse requests and bonus stipend
- [x] Sacrifice tied to faction and village in Cellar records
- [x] Opening story arc through Mournhollow
- [x] Save/load from any map: village seeds, per-village stock, current map, hub return position
- [x] Keeper's Shack zone and safe sacrifice deposit
- [x] **Multi-step disposal:** autopsy → dig grave (Shovel) or light the incinerator (timed burn), then dispose
- [x] **Disposal choice:** Burial Site (Village-flavored) vs. Incinerator (Cult-flavored), chosen per corpse, each adding a hidden 0.5 tag weight
- [x] **Quest plumbing:** Fetch / Timing / Tag quests, NPC ids, quest status saved, cooldown resets
- [x] **Partial reset + carry-over:** next life starts at 20% of the milestone just reached, for the pursued faction only (tuning target; playtest and adjust)
- [x] **Theme-pair selection:** theme picker per pair slot; thresholds keyed to slot via `progression.ron`; discovery and sacrifice gated on the chosen theme
- [x] **Main menu and pause menu**
- [x] **Window title** renamed to *The Latest Keeper*
- [x] **Reagent stacking:** the same reagent can be added to a rite more than once
- [x] **Save fixes:** last delivery day and last restock day are saved

### **Next: reputation and pacing**

- [ ] **Weighted-tag reputation:** finish moving `rite.rs` / `faction.rs` fully to weighted-tag resolution; add `reconciled` flag; populate `items.ron` (8 village reagents) and confirm `factions.ron` multipliers against the pacing table; set `progression.ron` to the 40/100 → 220/350 table; add stock-refresh amounts to `tending.ron`
- [x] **Neutralize `bound` in reputation:** set `bound` to 0.0 for both factions in `factions.ron` so the Village/Cult mirror is exact and the pacing table holds. `preserved` is already 0. Keep both tags on reagents.
- [ ] **Give `preserved` and `bound` real effects (gameplay, not reputation):**
  - `preserved`: each unit of weight slows the corpse's decay after the rite is chosen, or extends how long a corpse stays fresh before it rots.
  - `bound`: each unit of weight raises the sacrifice carry-over, or adds one extra carry-forward slot to the 5-item limit.
  - Both effects must not change rep/day. Test them against the section 3 table in `README_PACING.md` before locking numbers.
- [ ] **Empty-rite handling:** allow a rite with no reagents (disposal weight only) or show a clearer message
- [ ] **Theme-pair story beat:** write the player's theme choice into a story beat
- [ ] **Save the current corpse and incinerator burn timer**
- [ ] **Cleanup pass:** clear `burn_timer` and `last_tended` on sacrifice, remove unused `ActionKind::StokeIncinerator`, block movement and hotkeys while windows are open, let the theme picker be dismissed
- [ ] **Variable body count:** mostly 1 per day, occasional 0 or 2
- [ ] **NPC quests, content pass:** more quests per NPC, a quest log, quest markers
- [ ] **Graveyard upkeep chores:** small recurring hub tasks (clear overgrowth, repair a marker) for minor stipend/rep
- [ ] **Exploration:** small optional areas near villages with lore or rare reagents
- [ ] **Onboarding pass:** update `tutorial.ron` for the `F` controls; verify `first_reagent_toggle`, `first_reputation_conflict`, `first_village_travel`, and `first_disposal_choice` fire correctly now that players can reach a village before their first burial

### **Content and UI**

- [ ] Reagent taskbar rework (more than 4 known reagents will not fit the current bar)
- [ ] Real-content pass for the remaining 7 villages
- [ ] Per-life day-by-day tables for lives 2-8, updated for carry-over and free theme order (see `README_PACING.md`)
- [ ] Grave markers as individually meaningful (burial site markers vs. incinerator scorch marks/urns)
- [ ] Sprites and art in place of remaining placeholder shapes
- [ ] Functional Map window
- [ ] Tie daily restock to zone data

### **Story and endgame**

- [ ] Reunification end-state and win condition — confirmed: full 8-life run only, no early ending
- [ ] Reflective closing story beat
- [ ] Attended rituals and NPC pathing
- [ ] Further content: more reagents, factions, burial variety
- [ ] Multiplayer (deferred)

## Open questions

- **Carry-over:** 20% of the milestone just reached, applied to the next life's faction only. Confirm the percentage by playtest, and whether the other faction should get anything.
- **Theme-pair selection UI:** the picker exists; how it should be presented (a Cellar prompt, a vision, a dialogue), and what story beat marks a completed pair, are still open.
- **Quest system scope:** how many quests per NPC and which ones repeat (`cooldown_days` now resets completed quests; content doesn't use it yet). Keep `reward_rep` small on repeatable quests so they don't move the rep pace.
- **`preserved` / `bound` effects:** reputation is now zero for both. Decide the exact effect sizes (decay slowdown, carry-over bonus), and whether either belongs in the rite UI as a visible preview.

## Building and running

Requires a local Rust toolchain ([rustup.rs](https://rustup.rs)).

```bash
git clone <this-repo>
cd mortmere
cargo run
```

Release build:

```bash
cargo run --release
```