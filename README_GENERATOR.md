# The Latest Keeper — World Generator

Secondary README. Covers how villages and wilds are generated, stored, and loaded. Systems live in `README.md`.

> Status: design target. **Current** = code today. **Target** = the plan.

---

## 1. Locked decisions

- **Everything is generated.** The player builds nothing. No placement UI, no runtime add/remove of structures.
- **The hub is static.** It comes from `zones.ron` and its tilemap, and is never regenerated.
- **Villages and wilds generate once, on unlock, then are locked in place.** After first generation the stored output is the source of truth.
- **Store the output, not the seed.** Generator code can keep changing without altering existing saves.
- **Each village and cult village has its own tileset.**
- **The cult village is a horizontal flip of its village's layout.** One layout per theme, stored once, flipped for the cult side.
- **Structures are dressing for now.** Homes, stores, and similar buildings have no behavior yet: a rect, a label, and an NPC anchor.
- The generator is deterministic for its seed (for tests and reproducible bugs), but saves never depend on that.

## 2. Current state

- `generate_village(zone_id, seed)` returns a `VillageInstance`: tilemap, npcs, zones, exit_zone, seed.
- Map is 50x60 tiles at 32px, centered on the origin.
- Only one zone is created: `<village>_grounds` at (-40, 60), size (80, 60), stocked with 3 of the village reagent.
- NPCs are filtered from `npcs.ron` by `map == zone_id`.
- Exit zone is fixed at (-30, -30, 60, 60). Player arrives at (0, 150).
- Tiles: grass and path only, with the legend hardcoded to `"grass"` / `"path"`.
- A village is created lazily, at first entry or the first daily restock after discovery.
- Saves store only the seed per village. Load regenerates, then restores stock by zone id.
- Randomness uses macroquad's global RNG, seeded at the start of generation and reseeded from the clock at the end.

### Known issues

- **No paths are carved in villages.** `targets` is built when only the grounds zone exists, then filters it out, so it is always empty.
- **Per-village tilemap lookup reads `data/tilemap_<id>.ron` from disk**, not the embedded `DATA`. Remove it.
- **Grounds name:** the Mournhollow special case yields "MourningGrounds" (no space), unlike the others.
- **Seed-only saves are fragile:** any generator change silently changes old saves.

## 3. Target pipeline

1. **Parameters:** look up per-village params (structure kinds and counts, size ranges, NPC count, `tileset` id) from a table keyed by theme and side.
2. **Layout (once per theme):** generate the village-side layout. The cult side never rolls its own.
3. **Place zones:** rejection sampling with the seeded RNG, largest first (store, shrine), then homes, then small props. Reject on overlap, out-of-bounds, or less than the minimum gap. Cap the attempts and fail loudly if the cap is hit.
4. **Assign kinds:** the generator sets `ZoneKind` in code. It is not read from RON.
5. **Carve paths:** from the entry to each structure's door edge (the edge facing the entry). Run this *after* all zones exist.
6. **Place NPCs:** anchor each NPC to a home or store, and position them inside it.
7. **Stock:** the grounds zone gets the village reagent.
8. **Cult side:** apply `flip_x` to the stored layout and apply the cult tileset.
9. **Freeze:** serialize both outputs into the save. The seed is not used again.

## 4. Structures

- **Kinds:** `Grounds`, `Home`, `Store`, plus more as needed (inn, well, shrine, smithy, graveyard plot). `draw_temple` and `draw_grave_markers` already cover shrine and graves.
- **Per-theme variety** comes from the params table, e.g. a smithy in Heartforge, a courthouse in Penncross.
- **Behavior:** none yet. Pickup works only on grounds zones. Store and home zones are inert.
- **Ids:** `<village>_home_0`, `<village>_store_0`, `<village>_grounds`. Ids are unique across *all* villages and must not encode left/right, so they survive the flip unchanged.
- **Open:** counts per village (see section 9).

## 5. Mirror flip

Applied once, when generating the cult side from the stored theme layout. The map is centered on x = 0.

| Thing | Flip |
| --- | --- |
| Zone rects | `x' = -(x + w)`, y unchanged |
| Tilemap | Reverse each row string; origin stays centered |
| NPC positions | `x' = -x` |
| Exit zone | Flip like a rect |
| Player spawn | (0, 150) is on the axis, so it is unchanged |
| Sprites and tile art | Not flipped, so text and asymmetric art stay readable |
| Door edges | Left and right swap |

## 6. Persistence (Target)

- `SaveData` holds each generated area by id: zone rects and stock, tilemap def, npc positions, exit zone, `ZoneKind`s.
- On load, deserialize. Never regenerate.
- **Derives needed:** `ZoneRegistry` has no `Serialize`. Check `TileMapDef` and `Npc` too.
- **Swap gotcha:** while the player is inside a village, `zones` and `npcs` are swapped with the cache entry, so the cache entry holds the *hub's* data. Saving must go through the existing `village_zones` / `hub_zones` helpers, not read the cache directly.
- **Shared type:** one `GeneratedArea` with an `AreaKind` (Village, Wild) can serve both villages and wilds.
- Keep a `save_version` so format changes migrate or reject cleanly.

## 7. Wilds

- Same pipeline as villages: generate on first unlock, store the output, never regenerate.
- Add `MapId::Wild(id)` (or generalize to an area type) reusing the zones/npcs swap and exit zone.
- Different generator: scatter pickup and lore zones (rare reagents) instead of a single grounds zone.
- Entry markers are static hub zones in `zones.ron`. The north side of the hub is currently empty.
- Gate entry with `Zone.unlocks_from` (declared, not yet read anywhere).

## 8. Assets

- **Tilesets:** `tiles/<village>/grass.png`, `tiles/<village>/path.png`, one folder per village and cult village (8 total).
- `generate_tilemap` takes a `tileset` id and builds legend ids like `mournhollow_grass`.
- `load_assets` is currently a fixed list. Load all 8 tilesets at startup first (simplest), or lazily on first entry later.
- Saved tilemaps store legend ids, so saves stay valid as long as asset ids are not renamed.
- `load_assets` panics on a missing file. Add a test that every `VILLAGE_STOCK` village has its tile files.

## 9. Constraints

- Zone ids are globally unique. Load restores stock by searching every instance for the zone id.
- Never iterate a `HashMap` while calling the RNG. Sort first.
- Generation must not read the filesystem. Use embedded data only.
- The generator never touches hub zones.

## 10. Roadmap

- [ ] Fix village path carving (build `targets` after all zones exist)
- [ ] Remove the per-village disk tilemap lookup
- [ ] Add `ZoneKind`, set by the generator
- [ ] Add `rects_overlap` and rejection-sampling placement with a min gap
- [ ] Add `flip_x` for layout, tilemap, NPCs, exit
- [ ] Add the `tileset` param to `generate_tilemap` and the 8 tileset assets
- [ ] Per-village params table
- [ ] `Serialize`/`Deserialize` on the instance types; store output in `SaveData`
- [ ] `save_version` and load handling
- [ ] Tests: no overlaps, all zones reachable by path, unique ids, stock present, flip is its own inverse
- [ ] Wilds: `MapId::Wild`, scatter generator, hub markers, `unlocks_from` gating

## 11. Open questions

- Structure counts and size ranges per village.
- Do wild pickups respawn daily like village grounds, or are they one-time finds?
- Do structures ever gain behavior (stores buying and selling, homes with interiors)? Parked for now.