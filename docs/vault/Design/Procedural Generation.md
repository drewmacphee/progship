# Procedural Generation

The ship is generated entirely at init time via `init_ship` reducer → `generation.rs`.

## Current Approach: Infrastructure-First Grid Stamp

**Principle**: Lay down ALL circulation infrastructure first, then fill remaining space with rooms.

### Pipeline
1. **Hull sizing** — Per-deck taper (bow/stern narrower, amidships widest)
2. **Grid allocation** — `grid[x][y]` where 1 cell = 1 meter
3. **Stamp corridors** — Main spine, service corridor, cross-corridors
4. **Stamp shafts** — Elevators and ladders at fixed (x,y) positions, same on every deck
5. **Zone identification** — Find empty rectangular blocks between infrastructure
6. **Treemap fill** — Squarified treemap packer assigns rooms to zones
7. **Door placement** — Scan for room↔corridor adjacencies, room↔room logical pairs
8. **Force-connect orphans** — Grid adjacency check for any unconnected rooms

### Hull Taper Per Deck
| Decks | Width | Length | Notes |
|-------|-------|--------|-------|
| 0–1 (Command) | 40m | 200m | Narrow prow |
| 2 to N-3 (Mid) | 65m | 400m | Full beam |
| Last 2 (Engineering) | 50m | 300m | Tapered stern |

### What Works Well ✅
- Grid eliminates overlap by construction (a cell is one thing only)
- Infrastructure-first guarantees corridors are always walkable
- Shafts align perfectly across all decks (fixed positions)
- Door verification pipeline: **0 errors, 0 warnings across 1,744 doors**
- Deterministic — same seed → same layout

### Known Problems ⚠️
- **Room size inflation** — Treemap packer expands rooms to fill zone space. A 14m² cabin target becomes 450m². The squarified treemap function ignores target sizes.
- **Not enough rooms** — Only ~76 rooms per deck instead of hundreds of cabins. Need way more rooms to fill zones properly.
- **Interstitial decks not implemented** — Service decks between habitation floors are planned but deferred.

### Research: Alternative Approaches Evaluated
| Approach | Verdict |
|----------|---------|
| Wave Function Collapse (ghx_proc_gen) | ❌ No global connectivity, wrong paradigm |
| dungoxide (BSP grid dungeon) | ⚠️ Right pattern, can't use directly (external deps) |
| Continuous BSP (floating-point) | ❌ Abandoned — precision errors cascade |
| Our grid-stamp | ✅ Best option for SpacetimeDB WASM |

---

## Gemini Vision: Radial Slice Architecture

An alternative concept for a cylindrical "skyscraper in space" ship:

### Vertical Spine
- Ship as a tall cylinder, not a horizontal hull
- **Bottom tier**: Drive section (engines, reactor, heavy armor)
- **Mid tier**: 90+ repeating habitat decks
- **Top tier**: Whipple shield + command bridge

### Radial Slice Algorithm (per deck)
1. Central shaft (6m radius) — elevators, life support conduits
2. Ring corridor at 30m radius
3. Radial slicing — divide annular area into "pie slices"
4. BSP subdivision within slices — creates individual rooms

### Why This Is Interesting
- More realistic for a rotating habitat (spin gravity)
- Natural central access point per deck
- Efficient space usage in circular cross-section
- Scales well to 100+ decks

### Why We Haven't Adopted It (Yet)
- Current codebase assumes rectangular grid layout
- Would require fundamental rewrite of grid system, door detection, movement
- Radial coordinates add complexity to pathfinding and rendering
- Current rectangular layout is working and verified

> **Future consideration**: If we redesign the ship shape, radial slicing is worth revisiting. Could be a "ship class" option — rectangular hull vs. cylindrical habitat.

## See Also
- [[Ship Overview]] — facility manifest, deck zoning
- [[Layout Algorithm Details]] — deep dive on current implementation
- [[Open Problems]] — room size inflation, etc.
