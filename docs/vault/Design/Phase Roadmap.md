# Phase Roadmap

Clean summary of what's done and what's ahead.

---

## Completed ✅

### Phase 0: SpacetimeDB Migration
Moved from hecs ECS to SpacetimeDB tables + reducers. Server module compiles to WASM, clients connect via WebSocket.

### Phase A: Core Simulation
All major systems: needs, activities, conversations, relationships, duty scheduling, atmosphere, ship systems, maintenance, events.

### Phase B: Top-Down 3D Game
Bevy client with WASD movement, room rendering, people as capsules, HUD with needs/status/room info.

### Phase B2: Layout & Collision Overhaul
BFS pathfinding, room bounds collision, wall-slide, corridor spine.

### Phase C: Polish & Gameplay
Room colors, door frames, activity indicators, conversation bubbles, context actions (eat/sleep/repair), ship overview panel.

### Phase D: Graph-Driven Layout
Grid-stamped layout, infrastructure-first corridors, explicit Door table, elevator/ladder shafts.

### Phase E (partial): Realistic Scale
400m×65m hull, 21 decks, treemap room packing, facility manifest (80+ room types), mathematical door verification pipeline (0 errors).

### Layout Bug Fix Rounds (E.5)
1. Absolute door coordinates
2. NORTH/SOUTH convention fix
3. Data-driven verification + systematic fixes
4. Distance-based door detection, movement throttling, smooth interpolation
5. Player entity preservation, camera fix, room-to-room door filtering

---

## Current Focus 🔧

### Room Size & Count (E.2 continuation)
The biggest visual problem: rooms are too large, too few. Treemap inflates them. Need a strategy for hundreds of small cabins per deck.

---

## Up Next 📋

### Client Polish (E.2.10–12)
- [ ] Camera adjustments for 400m×65m decks
- [ ] Room type colors for all 50+ types
- [ ] Room name labels
- [ ] Minimap / deck overview

### Population Scale-Up
- [ ] Generate 5,000+ people
- [ ] LOD system for off-deck NPCs
- [ ] Performance profiling at scale

### Interstitial Service Decks (E.1.8)
- [ ] Generate service decks between habitation floors
- [ ] HVAC/power/water routing data

---

## Future Phases 🔮

### Phase F: Enhanced 3D
- Lighting per room type
- Dialogue system with branching choices
- Quest/mission system
- Player skill progression
- First-person camera option

### Phase G: Multiplayer
- Multiple players in same world
- Synced movement and actions
- Role-based gameplay (captain, engineer, doctor)

---

## See Also
- [[Architecture Overview]]
- [[Open Problems]]
- [[Simulation Systems]]
