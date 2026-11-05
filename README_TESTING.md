# The Latest Keeper — Testing Walkthrough

Companion to `README.md` (systems) and `README_PACING.md` (pacing). This guide walks a tester through the game in a sensible order so each feature gets exercised and you know exactly how to reach it.

Each step is written as **Do** → **Expect**. If what you see differs from "Expect", that's worth reporting.

---

## 0. Before you start

### Run it

```bash
cargo run            # or: cargo run --release
```

Run from the project root so the game can find `data/`, `assets/`, and `save.ron`.

### Use the test thresholds

`data/progression.ron` ships with **testing values** (discovery / milestone rep per pair slot):

| Slot | Discovery | Sacrifice milestone |
| --- | --- | --- |
| 1st pair | 8 | 15 |
| 2nd pair | 16 | 25 |
| 3rd pair | 24 | 35 |
| 4th pair | 32 | 45 |

With these, a full first life takes roughly 4-6 in-game days instead of 24. For a real-pacing playthrough, switch to the intended table in `README.md` (40/100, 90/170, 150/260, 220/350).

### Controls cheat sheet

| Key | Action |
| --- | --- |
| `W` `A` `S` `D` / arrows | Move (you also *face* that direction) |
| `E` | Talk to NPC / enter a discovered village / pick up reagents from the zone in front of you |
| `F` | Corpse work (hold to autopsy or dig, tap once to light the incinerator) |
| `1`-`4` | Select tool slot (`2` = Shovel) |
| `I` | Inventory window |
| `Q` | Quest log |
| `C` | Cellar records (only inside the Cellar) |
| `R` | Sacrifice ritual (only inside the Cellar, once unlocked) |
| `T` | Tend the place you're standing in (once per day) |
| `Space` | Perform the rite and dispose of the corpse |
| `Enter` | Confirm sacrifice / accept or turn in a quest |
| `M` | Map window (placeholder) |
| `Esc` | Close a window, or pause menu |
| `F5` / `F9` | Save / load |
| `F2` / `F6` | Map edit mode / save tiles |

> **Facing matters.** `E` checks a point ~40 px in front of you, not under you. Face the thing you want to interact with. `F` actions check where you are *standing*, so stand inside the zone box.

### Finding places on the hub

Zones are drawn as colored boxes with a name label:

| Place | Look for |
| --- | --- |
| Morgue | purple box |
| Workyard | brown/gold box |
| Cellar | dark green box |
| Burial Site | lighter green box |
| Incinerator | orange box |
| Keeper's Shack | grey box |
| Village (Village faction) | blue box with grave markers and a temple |
| Village (Cult faction) | red box with grave markers and a temple |
| Undiscovered village | dim grey box with a thick border and `???` |

The player starts near the middle of the hub. The eight villages ring the graveyard.

---

## 1. New game and the theme picker

**Do:** Main menu → **New Game**.

**Expect:**
- A tutorial banner (`game_start`) and a log line.
- Your character says "Another day among the dead..." in a speech bubble.
- The **theme picker** ("Where does the next road lead?") opens right away. It can't be dismissed with `Esc` (known rough edge).

**Do:** Pick a theme (Grief, Love, Guilt, or Hope).

**Expect:** Log line "You turn towards X." Your first life targets that theme's **Village-side** settlement:

| Theme | Village side | Cult side |
| --- | --- | --- |
| Grief | Mournhollow | Ashmourn |
| Love | Heartwarden | Heartforge |
| Guilt | Penncross | Guiltpyre |
| Hope | Hopewell | Hopeforsworn |

Check the HUD while you're here:
- Top bar: day, weekday, clock, stipend.
- Top-left panel: a corpse (it's already been delivered on day 1) with its quality and "Requests: <tag>".
- Reputation panel: one line per faction.
- Bottom-left: event log.
- Bottom-center: four tool slots (Hoe, Shovel, Spell A, Spell B).

---

## 2. Gathering reagents

**Do:** Walk to the **Morgue**. Face into the box and tap `E` four times. Do the same at the **Workyard**.

**Expect:**
- Each `E` takes one of *each* item stocked in that zone (Morgue: salt + prayer candle; Workyard: grave dirt + iron nail).
- Tutorial `first_pickup` fires on the first pickup.
- The Morgue panel under the corpse box shows remaining stock and what you're carrying.
- Reagent slots appear above the tool bar once you carry something.

Base daily stock: Morgue 5 salt / 4 candle, Workyard 5 grave dirt / 4 iron nail. It restocks at the start of each day.

---

## 3. Choosing reagents for a rite

**Do (inventory window):** Press `I`. Left-click an item tile to add one to the next rite. Right-click removes one.

**Do (bottom bar):** Click the small reagent slots above the tool bar (left-click adds, right-click removes, hover for the name).

**Expect:**
- Selected tiles turn green; a gold number on the bar slot and `sel N` on the inventory tile show how many of that reagent are selected.
- Cap of **4 reagents** per rite, repeats allowed up to the number you carry. Going past either limit logs "Rite already has 4 reagents." or "You don't have more of that."
- Tutorial `first_reagent_toggle` fires on the first successful add.
- `I` closes the window; while it's open you can't walk.

---

## 4. Burial (Village-flavored disposal)

The full chain is **autopsy → dig → bury**.

1. **Autopsy.** Stand inside the **Morgue**. **Hold `F`** until the progress bar fills (15 s). Letting go or walking away cancels ("You stop.").
   *Expect:* "corpse #N examined." and the tutorial `first_disposal_choice` banner. The HUD hint changes to "Dig [2]+[F] or light fire [F]".
2. **Equip the Shovel.** Press `2`. The Shovel slot highlights green. Pressing `2` again unequips it.
3. **Dig.** Stand inside the **Burial Site**. **Hold `F`** (20 s).
   *Expect:* "The grave is ready." Without the Shovel you get "You need the Shovel [2] to dig."
4. **Bury.** With reagents selected, still inside the Burial Site, press `Space`.
   *Expect:* "Buried corpse #N. Paid X stipend." plus indented lines like `village: +4.5 reputation`. Selected reagents are consumed. A new corpse arrives only the *next* day.

Edge cases worth trying:
- Press `F` in the Burial Site **before** the autopsy → nothing starts. In the Morgue before autopsy, you'll see "Examined the corpse in the Morgue first." if you try in the wrong order.
- Press `Space` at the grave with **no reagents selected** → "No reagents available." and nothing is consumed (known rough edge; empty-rite handling is on the roadmap).

---

## 5. Burning (Cult-flavored disposal)

1. Autopsy as above.
2. Stand inside the **Incinerator** and **tap `F` once**.
   *Expect:* "You light the fire. It needs time." and nothing else happens. You can walk away.
3. Wait ~20 s of real time. The log says "The fire is ready."
4. Return to the Incinerator and press `Space` with reagents selected.
   *Expect:* "Burned corpse #N..." with the disposal adding a hidden `profane` weight.

Tip: use the wait time to run an errand (tend, talk to an NPC, enter a village). The corpse keeps decaying while the fire burns.

---

## 6. Reputation math

Fastest way to move reputation on purpose (using current faction multipliers):

| Rite | Disposal | Approx. result |
| --- | --- | --- |
| 4× Prayer Candle (`holy` ×4) | Burial | Village +4.5, Cult -4.5 |
| 4× Grave Dirt (`profane` ×4) | Burning | Cult +4.5, Village -4.5 |
| 1 Candle + 1 Grave Dirt | Burial | Holy/profane cancel; only the disposal's 0.5 moves rep (Village +0.5, Cult -0.5) |
| Salt / Iron Nail only | Either | Only the disposal's 0.5 weight (`preserved` and `bound` are neutral for rep) |

**Expect:**
- The reputation panel updates immediately after `Space`.
- `first_reputation_conflict` fires the first time a rite raises one faction and lowers another.
- The log shows each faction's delta.

### Corpse requests (+5 stipend)

**Do:** Read the "Requests: <tag>" line in the corpse panel. Select reagents that carry that tag, then dispose of the corpse.

**Expect:** "Request fulfilled (tag): +5.0 bonus stipend". The hidden holy/profane weight from the disposal method does **not** count toward fulfilling a request; the reagents themselves must carry the tag.

---

## 7. A corpse that rots

**Do:** Ignore a delivered corpse and do other things (or wait in the village).

**Expect:** Quality in the corpse panel drops. At zero the log reads "Corpse #N rotted away untended." and `missed_days_this_tenure` increases. That count later appears in the Cellar record's epitaph as "They did not always return in time...".

Also check that a new corpse is only delivered when none is on hand *and* a new day has started.

---

## 8. Discovering a village

**Do:**
1. Build Village-side reputation to the discovery value for your first slot (8 with test values) by burying with holy reagents. Two or three burials with 4 candles each is plenty.
2. Before then, walk within ~150 px of your chosen village marker.

**Expect:**
- Before discovery: a hint line appears in your speech bubble when you approach (each village has its own text, e.g. Mournhollow: "Something quiet waits past the mist out of the north."). Only villages whose **theme has been picked** give hints.
- Reaching the threshold logs the discovery text, and the box changes from `???` to a colored village zone with grave markers and a temple.
- From then on, village reagent stock restocks daily (1 unit per day) and the log shows "Supplies restocked." at the start of each day.

---

## 9. Entering a village

**Do:** Face the discovered village box and press `E`.

**Expect:**
- "You step into <Village Name>". `first_village_travel` fires.
- The map switches to a generated village (its own tilemap, zones and NPCs). The layout is seeded, so it looks identical on every visit and after loading.
- Your hub position is remembered. A **"Path home"** zone is drawn in green.

**Do:** Walk into "Path home".

**Expect:** "You return to the graveyard." and you reappear where you left the hub.

---

## 10. Tending

**Do:** Inside a village, walk around until the top-left panel shows **"[T] Tend this place"**, then press `T`.

**Expect:**
- Flavor text in the log, +2 reputation to the village's faction (only if that village is in your *current* pair), and the village reagent restocked by 1.
- Pressing `T` again the same day → "Already tended today." (the HUD hint changes to match).
- Next day the prompt returns.

Pick up the village reagent with `E` facing the zone it was deposited in. These are the rare reagents (e.g. Mourning Bell Flower) that carry theme tags like `grief`.

---

## 11. NPCs and quests

**Do:** Walk up to an NPC (in the hub or a village) and press `E`.

**Expect:**
- A speech bubble with a line that depends on your current reputation, and the same line in the log.
- NPCs only appear during their scheduled hours, so check back at a different time of day if one is missing. NPCs in undiscovered villages are hidden.
- Gold `!` over their head = quest available. Grey `?` = in progress. Green bobbing `?` = ready to turn in.
- If the NPC has a quest, a dialogue window opens. Click **Accept**/**Turn in** or press `Enter`. `E` or `Esc` closes it.

Quest types to test:

| Kind | How to complete |
| --- | --- |
| **Fetch** | Gather N of an item, return, **Turn in**. The items are removed from your inventory. |
| **Tag** | Perform a rite whose reagents carry the tag (the hidden disposal weight doesn't count). The quest becomes ready on disposal. |
| **Timing** | Be inside the target zone on or before the due day. It flips to ready automatically. |

**Do:** Press `Q` for the quest log.

**Expect:** Active quests with objective text (e.g. "Bring 2 Salt (1/2)"), "Ready to turn in" in green when you have what's needed, and an "Available again soon" list for quests on cooldown.

**Do:** Talk to the NPC again after turning in.

**Expect:** "Nothing for now. Come back in N day(s)." for repeatable quests, or "Nothing more to say." for one-time ones. Rewards (stipend, rep, items) show on the dialogue window before you accept.

---

## 12. Cellar records and the sacrifice

### Unlock it

1. Raise the faction's reputation to the **milestone** (15 with test values) **and** make sure the village is discovered.
2. Watch the log for the milestone announcement.

**Expect:** The log shows the announce text. Stand inside the **Cellar**: the HUD now shows a red "[R] The ritual calls to you...".

### Read the Cellar

**Do:** Stand inside the Cellar, press `C`.

**Expect:** Seed records (and, later, your own), plus a "What you remember" list of memory fragments. First visit fires `first_cellar_visit`. Outside the Cellar, `C` logs "Step into the cellar to read the markers."

### Perform the sacrifice

**Do:** Inside the Cellar, press `R`. Left-click item rows to add (up to **5 total**), right-click to remove. Press `Enter` to confirm (or `R` again to cancel).

**Expect after confirming:**
- A new Cellar record ("Keeper #1") with the day range, an epitaph from the memory fragment, and who you sacrificed to.
- The chosen items appear in the **Keeper's Shack** (walk there and `E` to pick them up).
- Inventory, selected reagents, corpse and burn timer are cleared; `last_tended` resets; you become **Keeper #2**.
- Stipend is kept. Reputation resets, but the *other* faction (the one you pursue next) starts with 20% of the milestone as carry-over (3.0 with the test values). The reputation panel shows this immediately.

---

## 13. Life 2: the Cult mirror

**Do:** Repeat the loop, but with profane rites: 4× Grave Dirt, burn at the Incinerator.

**Expect:**
- Discovery threshold still comes from the same slot (8 for the first pair), so the Cult-side village (e.g. Ashmourn) is discovered after a couple of burnings.
- The Cult village's box is red; it has its own NPCs and its own tending task.
- The milestone fires when Cult reputation reaches 15.

Completing the second sacrifice should:
- Bump the pair counter and **reopen the theme picker** with only the remaining three themes.
- Set the next target to that theme's Village side, now using the 2nd slot's thresholds (16 / 25).

You can keep going through pair 4. When all four pairs are done the log says "The cycle is complete." (there's no win screen yet).

---

## 14. Save and load

| Test | Steps | Expect |
| --- | --- | --- |
| Save from hub | `F5` (or Pause → Save Game) | "Game saved." and a `save.ron` file next to the executable's working directory |
| Load | `F9` (or Pause → Load Game, or main menu → Continue) | "Game loaded." Day, time, rep, inventory, discoveries and quests restored |
| Save inside a village | Enter a village, `F5`, quit to menu, Continue | You reappear inside that same village with the same layout and stock |
| Mid-corpse save | Autopsy a corpse, `F5`, quit, Continue | The current code saves the corpse and incinerator timer (the README still says it doesn't; confirm which is true for you) |
| Tutorial persistence | Save after the first burial, reload | Tutorial banners already seen don't repeat |

To see a **fresh** tutorial again, choose **New Game** (it ignores the save file) or delete `save.ron`.

---

## 15. Pause menu and main menu

- `Esc` with no windows open → pause screen: **Resume / Save Game / Load Game / Quit to Menu / Quit Game**.
- `Esc` with a window open closes that window. It won't close the Dialogue or Theme Picker windows via `Esc` the same way (dialogue closes with `E` or `Esc`; the picker can't be dismissed).
- Main menu **Continue** is greyed out until a `save.ron` exists.

---

## 16. Map edit mode (dev tool)

**Do:** `F2` to toggle. `[` selects grass, `]` selects path. Left-click paints the selected tile, right-click paints grass. `F6` writes the current map to `data/tilemap.hub.ron` (hub) or `data/tilemap_<village>.ron` (village).

**Expect:** A yellow help line near the bottom of the screen. If the tilemap file exists on the next run, it replaces the procedurally generated one.

---

## 17. Fast-forward tricks for testers

- **Skip waiting on thresholds:** lower the `discovery` / `milestone` numbers in `data/progression.ron` even further (restart the game afterward).
- **See a specific day sooner:** there's no time-skip key. If you need one, shorten the day-length constant (450 s) wherever it lives in `clock.rs`.
- **Reach a late-game state quickly:** play one real pair, `F5` right before each sacrifice, and `F9` to retry the choice.
- **Check restock logic:** the Morgue and Workyard refill at the start of every in-game day regardless of which map you're on, so you can stay in a village and still verify "Supplies restocked.".

---

## 18. Quick feature checklist

- [ ] Theme picker appears on New Game
- [ ] Reagents picked up with `E` in Morgue and Workyard
- [ ] Reagent selection: inventory window and bottom bar, 4-item cap, repeats
- [ ] Autopsy cancel on release / walk-away
- [ ] Dig requires the Shovel
- [ ] Burial pays stipend and changes reputation
- [ ] Incinerator timer ("The fire is ready.")
- [ ] Hidden holy/profane weight visible in the numbers
- [ ] Requested-tag bonus (+5)
- [ ] Corpse rots untended; shows up in epitaph later
- [ ] Proximity hints, then discovery at the threshold
- [ ] Enter and exit a village; layout is the same each visit
- [ ] Tend once per day; "Already tended today."
- [ ] Quest accept → in progress → turn in (Fetch, Tag, Timing)
- [ ] Quest log (`Q`) and cooldown list
- [ ] Milestone announce and `R` prompt in the Cellar
- [ ] Sacrifice: 5-item cap, Shack deposit, Keeper #2, carry-over rep
- [ ] Life 2 (Cult mirror) completes and reopens the theme picker
- [ ] Save/load from hub and from inside a village
- [ ] Edit mode paints and saves tiles

---

## Known rough edges (not bugs to re-report)

- Theme picker can't be dismissed.
- Movement and hotkeys aren't fully blocked while some windows are open.
- Map window (`M`) is a placeholder; Hoe, Spell A and Spell B are placeholders.
- Disposing with no reagents selected just logs a message.
- `README.md` may be out of date on what the save file stores (corpse and burn timer).
- Full discovery of all 8 villages only logs a line; there's no ending screen yet.