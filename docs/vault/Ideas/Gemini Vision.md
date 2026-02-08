# Gemini Vision Document

Original design brainstorm from a conversation with Google Gemini. Preserved here as reference and inspiration. Some ideas have been adopted, others are future possibilities.

---

## Phase 1: Structural Procedural Generation (The Shell)

### 1.1 Global Architecture: "Vertical Spine"
Ship as a skyscraper-in-space silhouette:
- **Bottom Tier (The Drive)**: Heavily armored, main engine bell + reactor core
- **Mid Tier (The Habitats)**: 90+ repeating decks subdivided by purpose (Residential, Hydroponic, Industrial)
- **Top Tier (The Prow)**: Massive Whipple Shield (ice/metal) + Command Bridge

> **Status**: We use a horizontal hull layout currently, not vertical. The vertical spine concept is interesting for a cylindrical ship with spin gravity. Could be a future "ship class" option.

### 1.2 Interior Partitioning: "Radial Slice" Algorithm
For each deck:
1. Establish a CentralShaft (6m radius) for elevators/life-support conduits
2. Create a RingCorridor at 30m radius
3. Radial Slicing to divide area between shaft and outer hull into "pie slices"
4. BSP subdivision within slices for individual apartments/offices

> **Status**: Not implemented. We use rectangular grid + infrastructure-first layout instead. Radial slicing would require circular geometry support throughout the codebase. Worth revisiting if we switch to cylindrical ship shape.

### 1.3 Mesh Synthesis (Bevy-side)
- Input: Room table polygon vertices
- Output: Procedural `bevy::render::mesh::Mesh`
- Use `lyon` crate for 2D polygon triangulation → 3D extrusion
- Triplanar mapping for wall textures (avoids UV stretching)

> **Status**: Currently using simple box meshes. Lyon-based polygon meshing is a good Phase F idea for non-rectangular rooms.

---

## Phase 2: Deep Simulation (The Life Support)

### 2.1 Environmental Grid
Every room tracks its own atmospheric data:
- O2, CO2, Temperature, PowerDraw, WasteLevel
- `update_environment` reducer on 1-second tick
- O2 flows between adjacent rooms based on pressure differentials
- CO2 increases based on number of agents in room

> **Status**: We have per-deck atmosphere. Per-room is a planned upgrade. See [[Simulation Systems#Environmental Grid]].

### 2.2 The Social & Biological Agent ("Centurion" AI)
Every agent is a persistent DB row. Utility AI scoring:
- **Metabolic**: Hunger, Fatigue, Oxygen Saturation
- **Psychological**: Stress (noise/overcrowding), Morale (food quality/safety), Social (neighbor affinity)
- **Occupation**: Duty cycles with shift-table workstations

Relationship Graph:
- `Connection` table: (AgentA, AgentB, Affinity, HistoryTags)
- Social interactions when same room + off-duty

> **Status**: Mostly implemented! We have needs, activities, conversations, relationships, duty scheduling. Missing: overcrowding stress, food quality impact, noise modeling, oxygen saturation as metabolic need.

---

## Phase 3: Software Architecture & Data Flow

### 3.1 SpaceTimeDB Backend
Reducers:
- `ship_init(seed)` — generate entire ship ✅
- `tick_agent_ai()` — process utility AI for all agents ✅ (tiered)
- `process_economy()` — resource distribution 🔴 (basic, not full loop)
- `trigger_maneuver(type)` — flip state / zero-G toggle 🔴 (not started)

### 3.2 Bevy Frontend
Systems:
- `sync_db_to_world` — listen for row updates ✅
- `spatial_culling` — only instantiate entities for current deck ✅
- `flip_physics` — rotate gravity vector on flip signal 🔴

> **Status**: Architecture is exactly as described. SpacetimeDB server + thin Bevy client. Spatial culling is deck-based (only render current deck).

---

## Phase 4: Implementation Specs

### Pathfinding
> Use `petgraph` crate to build navigation mesh where each Room is a node and Doors are edges.

**Status**: Can't use `petgraph` in SpacetimeDB WASM (no external crates). We use manual BFS through Door table instead.

### Technical Constraints Noted
- Scale: 10,000 agents — use spatial queries for proximity
- Performance: AI tick must be tiered (500 agents/frame) for stability
- Precision: f64 for orbital/travel, f32 for local Bevy rendering

> See [[Technical Constraints]] for full list.

---

## Key Takeaways
1. The **vertical spine / radial slice** concept is compelling for a rotating habitat ship
2. **Per-room atmosphere** is the right target for environmental simulation
3. **Economy loop** (production vs consumption) needs implementation
4. **Flip maneuver / zero-G** is a unique gameplay mechanic worth pursuing
5. Most simulation ideas are already partially implemented — need deepening, not rewriting
