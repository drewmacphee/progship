# ProgShip Architecture

**Last Updated:** February 2026

This document provides a comprehensive overview of ProgShip's architecture for developers joining the project.

---

## 1. Overview

**ProgShip** is a real-time colony ship simulation featuring 5,000+ individually simulated crew and passengers aboard a multi-deck interstellar vessel. The project is built around a unique client-server architecture leveraging SpacetimeDB for multiplayer-native, server-authoritative simulation.

### Technology Stack

- **Server**: SpacetimeDB (Rust compiled to WebAssembly)
  - All game logic runs as "reducers" (atomic transactions)
  - Auto-persistence—no manual save system needed
  - Real-time client subscriptions via WebSocket
  - Server-authoritative—prevents cheating in multiplayer
- **Client**: Bevy 0.15 (Rust 3D game engine)
  - Thin client—renders state, sends inputs, performs no simulation
  - Top-down 3D camera viewing a single deck at a time
  - Direct SpacetimeDB SDK integration (not using bevy_spacetimedb plugin)
- **Client SDK**: Auto-generated from server tables via `spacetime generate`
  - Type-safe Rust bindings for all tables and reducers
  - Must be regenerated after any server schema changes

### Design Principles

1. **Multiplayer-First**: Multiple players can connect to the same ship, each controlling their own character while observing shared NPC behavior
2. **True Simulation**: Every person is individually simulated with needs, activities, and social interactions—not statistically sampled
3. **Server-Authoritative**: All simulation runs on the server; clients only render and send input
4. **Data-Oriented**: Tables store pure data; reducers contain all logic
5. **Tiered Updates**: Different simulation systems run at different frequencies to balance fidelity and performance

### Why SpacetimeDB?

SpacetimeDB was chosen early to support multiplayer as a core goal:
- **Shared world state**: All players see the same 5,000+ NPCs
- **Atomic transactions**: Reducers prevent duplication exploits
- **Rust-native**: Seamless integration with existing Rust codebase
- **Built-in persistence**: Survives crashes without custom save logic

**Trade-offs accepted:**
- WASM sandbox—no file I/O, no external crates (`rand`, `petgraph`)
- Single-player still requires local SpacetimeDB instance
- Non-traditional programming model (tables + reducers instead of ECS)

---

## 2. Data Flow

The data flow follows a unidirectional pattern from player input to server validation to client rendering:

```
┌────────────────────────────────────────────────────────────────┐
│                        Player Input                             │
│          (WASD for movement, E to enter doors, etc.)            │
└──────────────────────┬─────────────────────────────────────────┘
                       │
                       ▼
┌────────────────────────────────────────────────────────────────┐
│                      Bevy Client                                │
│  • Captures keyboard/mouse input                               │
│  • Throttles movement sends to 20Hz to reduce network traffic  │
│  • Calls SpacetimeDB reducer (e.g., player_move, elevator_up)  │
└──────────────────────┬─────────────────────────────────────────┘
                       │ WebSocket
                       ▼
┌────────────────────────────────────────────────────────────────┐
│                    SpacetimeDB Server                           │
│  • Validates input (e.g., is door reachable?)                  │
│  • Updates tables atomically (Position, Movement, etc.)        │
│  • Runs simulation reducers (tick_fast, tick_slow)             │
└──────────────────────┬─────────────────────────────────────────┘
                       │ Subscription push
                       ▼
┌────────────────────────────────────────────────────────────────┐
│                      All Clients                                │
│  • Receive table updates via WebSocket subscriptions           │
│  • Sync Bevy entities from table data (sync_rooms, sync_people)│
│  • Render updated world state                                  │
└────────────────────────────────────────────────────────────────┘
```

### Key Points

- **Input batching**: Client accumulates movement input and sends updates at 20Hz max
- **Validation**: Server checks all actions (Can player reach that door? Is elevator at this deck?)
- **Broadcast**: All connected clients receive the same table updates in real-time
- **Sync frequency**: NPCs are rebuilt at 5Hz on client; player entity is preserved for smooth control

---

## 3. Crate Structure

```
progship/
├── crates/
│   ├── progship-server/       # SpacetimeDB WASM module
│   │   └── src/
│   │       ├── lib.rs         # Module entry point
│   │       ├── tables.rs      # All table definitions (28 tables)
│   │       ├── reducers.rs    # Player actions (join, move, doors, elevators)
│   │       ├── generation.rs  # Procedural ship generation pipeline
│   │       └── simulation.rs  # Simulation systems (needs, activities, atmosphere)
│   │
│   ├── progship-client-sdk/   # Auto-generated SpacetimeDB bindings
│   │   └── src/lib.rs         # DO NOT MODIFY—regenerate via spacetime generate
│   │
│   ├── progship-client/       # Bevy 0.15 thin client
│   │   └── src/main.rs        # Rendering, input, camera, table sync
│   │
│   ├── progship-core/         # Legacy ECS core (archived, not used)
│   ├── progship-ffi/          # Legacy C FFI (archived, not used)
│   └── progship-viewer/       # Experimental viewer (WIP)
│
├── docs/
│   ├── vault/                 # Obsidian design notes
│   ├── ARCHITECTURE.md        # This file
│   └── CONTRIBUTING.md        # Setup and contribution guide
│
├── scripts/                   # PowerShell build/verify automation
├── archive/                   # Old Python/Godot prototype (read-only)
├── verify_doors.py            # Mathematical door verification script
└── categorize_errors.py       # Door error analysis tool
```

### Crate Responsibilities

| Crate | Purpose | Key Details |
|-------|---------|-------------|
| `progship-server` | All game logic | Compiles to WASM, runs in SpacetimeDB, defines tables and reducers |
| `progship-client-sdk` | Type-safe bindings | Auto-generated, provides Rust types for all tables/reducers |
| `progship-client` | Rendering and input | Bevy app, subscribes to tables, renders 3D world, sends player input |
| `progship-core` | *(Legacy)* | Original ECS architecture, now archived |
| `progship-viewer` | *(Experimental)* | Alternative viewer, work in progress |

---

## 4. Server Architecture

The server is a SpacetimeDB module running in a WebAssembly sandbox. It defines the schema (tables) and logic (reducers) for the entire simulation.

### Tables

ProgShip has **28 core tables** organized by domain:

#### Ship Configuration (1 table)
- `ShipConfig`: Singleton holding ship name, deck count, simulation time, time scale

#### People (10 tables)
- `Person`: Identity (name, crew/passenger, player flag)
- `Position`: Current room and x/y/z coordinates
- `Movement`: Active pathfinding (target room, path, speed)
- `Needs`: Hunger, fatigue, social, comfort, hygiene, health, morale
- `Personality`: Big Five traits (openness, conscientiousness, etc.)
- `Skills`: Technical, medical, social, physical skill levels
- `Activity`: Current activity type, start time, duration
- `Crew`: Department, rank, shift, duty station
- `Passenger`: Cabin class, destination, embarkation info
- `ConnectedPlayer`: Maps player identity to their Person ID

#### Spatial (6 tables)
- `Room`: Core spatial container (id, deck, x, y, width, height, room_type)
- `GraphNode`: Pathfinding graph nodes (one per room)
- `GraphEdge`: Pathfinding graph edges (room connections)
- `Door`: Connections between rooms (room_a, room_b, wall sides, position, width)
- `Corridor`: Main circulation corridors (spine, cross-corridors)
- `VerticalShaft`: Elevators and ladders (fixed x/y across all decks)

#### Ship Systems (6 tables)
- `DeckAtmosphere`: Per-deck O2, CO2, humidity, temperature
- `ShipSystem`: Major systems (power, life support, engines)
- `Subsystem`: Children of ship systems
- `SystemComponent`: Physical components in rooms
- `InfraEdge`: Infrastructure dependencies (power flow, air circulation)
- `ShipResources`: Food, water, medical supplies, fuel

#### Maintenance & Tasks (1 table)
- `MaintenanceTask`: Repair tasks for degraded systems

#### Social (3 tables)
- `Relationship`: Pairwise connections (strength, familiarity)
- `Conversation`: Active conversations (topic, state, start time)
- `InConversation`: Join table linking people to conversations

#### Events (1 table)
- `Event`: Fires, hull breaches, medical emergencies, etc.

### Table Relationships

```
Person ─┬─1:1─ Position (person_id FK)
        ├─1:1─ Needs
        ├─1:1─ Personality
        ├─1:1─ Skills
        ├─0:1─ Movement (only if moving)
        ├─0:1─ Activity (only if active)
        ├─0:1─ Crew (if is_crew)
        ├─0:1─ Passenger (if not crew)
        └─M:N─ Conversation (via InConversation)

Position.room_id ──FK──> Room.id

Room ─┬─1:N─ Door (room_a or room_b)
      ├─1:1─ GraphNode
      └─0:N─ SystemComponent

Door ──M:N── Room (room_a, room_b)
```

### Reducers

Reducers are functions called by clients to mutate tables. Think of them as API endpoints.

#### Player Actions
- `client_connected`: Logs player connection, creates ConnectedPlayer entry
- `client_disconnected`: Removes player from ConnectedPlayer table
- `player_join(given_name, family_name, is_crew)`: Creates Person + Position + Needs + Personality
- `player_move(dx, dy)`: Updates player position, handles room transitions
- `player_use_elevator(target_deck)`: Moves player to a different deck via elevator shaft
- `player_use_ladder(direction)`: Moves player up/down one deck via ladder shaft
- `player_interact(target_person_id)`: Interact with another person
- `player_action(action)`: Generic action handler

#### Ship Configuration
- `set_paused(paused)`: Pause/unpause the simulation
- `set_time_scale(scale)`: Adjust simulation speed (time acceleration)

#### Ship Initialization
- `init_ship(name, deck_count, crew_count, passenger_count)`: Main entry point
  - Inserts ShipConfig
  - Runs procedural generation pipeline
  - Spawns NPCs with initial needs/positions

#### Simulation Tickers
- `tick(delta_seconds)`: Main simulation tick, advances all simulation systems

### Generation Pipeline

The `generation.rs` module procedurally creates the ship layout when `init_ship` is called. It follows a **graph-first** approach:

```
┌────────────────────────────────────────────────────────────────┐
│ 1. build_ship_graph()                                          │
│    • Creates GraphNode entries (one per room concept)          │
│    • Creates GraphEdge entries (connections between nodes)     │
│    • Determines room types, counts, and logical relationships  │
└──────────────────────┬─────────────────────────────────────────┘
                       │
┌──────────────────────▼─────────────────────────────────────────┐
│ 2. layout_ship()                                               │
│    • Creates Room tables from graph nodes                      │
│    • Positions rooms on decks with x/y coordinates             │
│    • Creates Corridor tables (main spine, cross-corridors)     │
│    • Creates VerticalShaft tables (elevators and ladders)      │
│    • Creates Door tables connecting rooms and corridors        │
└──────────────────────┬─────────────────────────────────────────┘
                       │
┌──────────────────────▼─────────────────────────────────────────┐
│ 3. generate_ship_systems()                                     │
│    • Creates ShipSystem entries (power, life support, engines) │
│    • Creates Subsystem entries (generators, scrubbers, etc.)   │
│    • Creates SystemComponent entries (physical instances)      │
│    • Creates InfraEdge entries (power/air flow dependencies)   │
└──────────────────────┬─────────────────────────────────────────┘
                       │
┌──────────────────────▼─────────────────────────────────────────┐
│ 4. generate_atmospheres()                                      │
│    • Creates DeckAtmosphere entries (per-deck O2/CO2 tracking) │
│    • Initializes breathable atmosphere on all decks            │
└──────────────────────┬─────────────────────────────────────────┘
                       │
┌──────────────────────▼─────────────────────────────────────────┐
│ 5. generate_crew()                                             │
│    • Creates Person entries for crew members                   │
│    • Assigns departments, shifts, duty stations               │
│    • Creates Position, Needs, Personality, Skills, Crew tables │
└──────────────────────┬─────────────────────────────────────────┘
                       │
┌──────────────────────▼─────────────────────────────────────────┐
│ 6. generate_passengers()                                       │
│    • Creates Person entries for passengers                     │
│    • Assigns cabin classes                                     │
│    • Creates Position, Needs, Personality, Skills, Passenger   │
└────────────────────────────────────────────────────────────────┘
```

**Current Status:**
- Door verification: **0 errors, 0 warnings** across 1,744 doors
- Graph-first approach ensures logical connectivity by design
- Room-to-room doors for logical pairs (galley↔mess, surgery↔hospital)
- Rooms connect primarily to corridors; corridors form circulation spine

### Simulation Systems

Simulation logic runs in `simulation.rs`, called periodically by reducers. Systems are organized into **four tiers** by update frequency:

| Tier | Frequency | Systems | Purpose |
|------|-----------|---------|---------|
| **T0** | 60 Hz | Movement interpolation | Smooth position updates (not yet implemented server-side) |
| **T1** | 1 Hz | Activity state machines | Start/complete activities (eating, sleeping, working) |
| **T2** | 0.1 Hz (10s) | Needs decay, duty scheduling | Hunger/fatigue increase, shift changes |
| **T3** | 0.01 Hz (100s) | Ship systems, atmosphere, events | Power generation, O2/CO2 balance, random emergencies |

#### Implemented Systems

- **Needs System**: Seven needs (hunger, fatigue, social, comfort, hygiene, health, morale) decay over time; activities satisfy them
- **Activity System**: State machine (Idle → Moving → Performing); NPCs pick activities based on highest need
- **Social & Conversations**: NPCs initiate conversations when social need is high; 9 topic types
- **Relationships**: Pairwise strength/familiarity tracking; evolves through interactions
- **Duty & Scheduling**: Three shifts (Alpha, Beta, Gamma); crew assigned to departments
- **Atmosphere**: Per-deck O2/CO2/humidity tracking; people consume O2, produce CO2
- **Ship Systems & Maintenance**: Power, life support, engines degrade; repairs auto-generated
- **Events**: 8 types (fire, hull breach, medical emergency, system failure, resource shortage, altercation, discovery, celebration)
- **Movement**: Grid-based with distance-based door detection; BFS pathfinding through door graph

---

## 5. Client Architecture

The Bevy client is a **thin client**—it performs zero simulation, only rendering and input handling.

### SpacetimeDB Connection and Sync

```rust
// Connection lifecycle (simplified)
fn connect_to_server() {
    let conn = DbConnection::builder()
        .with_uri("http://localhost:3000")
        .with_module_name("progship")
        .on_connect(on_connect_callback)
        .on_disconnect(on_disconnect_callback)
        .build();
    // Store in ConnectionState resource
}

fn on_connect_callback(conn: &DbConnection) {
    // Subscribe to all tables
    conn.subscribe(&["SELECT * FROM room"]);
    conn.subscribe(&["SELECT * FROM person"]);
    // ... etc for all relevant tables
}
```

### Entity Sync Strategy

- **Rooms**: Spawned once at startup, never despawned (static)
  - `sync_rooms` system checks for new rooms and creates Bevy entities
  - Each room gets a PbrBundle mesh (colored rectangle)
- **People**: Rebuilt at **5Hz** to handle joins/leaves
  - `sync_people` system despawns all NPC entities, respawns from Person table
  - **Player entity is preserved** by checking `is_player` flag
  - Each person gets a PbrBundle mesh (colored capsule) positioned in 3D

### Coordinate Mapping

The server uses a 2D grid (x=east/west, y=fore/aft). The client renders in 3D:

```rust
// Example coordinate transformation
// Server coordinates (from Position table)
let server_x = position.x;  // East (+) / West (-)
let server_y = position.y;  // Fore (low) / Aft (high)

// Transform to Bevy 3D world coordinates
let client_x = server_x;           // Same as server X
let client_y = 0.0;                // Height (vertical axis)
let client_z = -server_y;          // INVERTED: Bevy Z opposes server Y

// Cardinal directions in both coordinate systems:
// NORTH = 0 = low server Y (fore)    → Bevy -Z direction
// SOUTH = 1 = high server Y (aft)    → Bevy +Z direction
// EAST  = 2 = high server X (starboard) → Bevy +X direction
// WEST  = 3 = low server X (port)    → Bevy -X direction
```

### Camera System

```rust
// Top-down camera looking down the Y axis
let camera_transform = Transform::from_translation(
    Vec3::new(player_x, camera_height, player_z)  // 80 meters above player
).looking_at(
    Vec3::new(player_x, 0.0, player_z),
    Vec3::NEG_Z  // Up vector points "north" (toward fore)
);
```

- Follows player position at fixed height
- Scroll wheel adjusts `camera_height` (zoom in/out)
- Deck switching (PageUp/PageDown) changes rendered entities

### Input Handling

Input is throttled to avoid flooding the server:

```rust
// Accumulate movement locally
if keys.pressed(KeyCode::KeyW) {
    player_state.pending_dy -= speed * delta;  // Move north (negative Y)
}
// ... other WASD keys

// Send at 20Hz max
player_state.move_send_timer += delta;
if player_state.move_send_timer >= 0.05 {  // 50ms = 20Hz
    if pending_dx != 0.0 || pending_dy != 0.0 {
        player_move(ctx, pending_dx, pending_dy);
        pending_dx = 0.0;
        pending_dy = 0.0;
    }
    player_state.move_send_timer = 0.0;
}
```

Doors and elevators are triggered instantly (no batching):
- `E` key: Check distance to nearest door, call door reducer if close
- Number keys / PageUp/PageDown: Call `player_use_elevator(target_deck)` or `player_use_ladder(direction)`

### UI Overlay

Basic immediate-mode UI (Bevy UI):
- **HUD**: Ship name, deck, player position, simulation time
- **Info Panel**: Hover over entities to see details (person name, room type)
- **Toasts**: Notifications (player joined, door entered, elevator used)

---

## 6. Build & Deploy

### Prerequisites

- **Rust** (stable toolchain): Install via [rustup](https://rustup.rs/)
- **SpacetimeDB CLI**: Install with `curl -fsSL https://install.spacetimedb.com | bash`
- **Python 3.x**: For door verification scripts
- **Linux only**: `sudo apt-get install -y libasound2-dev libudev-dev` (Bevy dependencies)

### Local Development Workflow

#### 1. Start SpacetimeDB

```bash
spacetime start
```

Leave this running in a dedicated terminal.

#### 2. Build and Publish Server

```bash
# Build server WASM module
spacetime build --project-path crates/progship-server

# Publish to local SpacetimeDB (clears existing data)
spacetime publish --clear-database -y --project-path crates/progship-server progship

# Initialize the ship (REQUIRED after each publish)
spacetime call progship init_ship "ISV Prometheus" 21 3000 2000
#                                   └── name        └─decks └─crew └─passengers
```

#### 3. Regenerate Client SDK (if tables/reducers changed)

```bash
spacetime generate --lang rust --out-dir crates/progship-client-sdk/src --project-path crates/progship-server
# Answer 'y' to delete existing lib.rs
# Rename generated mod.rs to lib.rs:
mv crates/progship-client-sdk/src/mod.rs crates/progship-client-sdk/src/lib.rs
```

**Important:** The SDK must be regenerated whenever you add/remove/modify tables or reducers in `progship-server`.

#### 4. Build and Run Client

```bash
cargo build --package progship-client
cargo run --package progship-client
```

The client auto-connects to `http://localhost:3000`.

### PowerShell Automation

For convenience on Windows, use the provided scripts:

```powershell
# Full rebuild + verification
.\scripts\rebuild.ps1

# Quick door verification only
.\scripts\verify.ps1
```

### CI Pipeline

GitHub Actions workflow (`.github/workflows/ci.yml`):

```yaml
jobs:
  lint:
    - cargo fmt --all -- --check
    - cargo clippy --all-targets -- -D warnings
  
  build:
    - spacetime build --project-path crates/progship-server
    - cargo build --package progship-client
  
  test:
    - cargo test --package progship-client
    # (Server tests are lint-only due to WASM constraints)
```

### Deployment (Multiplayer)

For remote multiplayer:

1. **Provision SpacetimeDB instance** (e.g., on SpacetimeDB Cloud or self-hosted)
2. **Publish server module:**
   ```bash
   spacetime publish --project-path crates/progship-server progship -s https://your-spacetimedb-host
   ```
3. **Initialize ship:**
   ```bash
   spacetime call progship init_ship "My Ship" 21 5000 2000 -s https://your-spacetimedb-host
   ```
4. **Clients connect** by changing URI in `connect_to_server()` to point to remote host

---

## 7. How to Extend

### Adding a New Room Type

**Goal:** Add a new facility (e.g., "Hydroponics Bay").

1. **Define constant in `tables.rs`:**
   ```rust
   // In the room_types module
   pub const HYDROPONICS: u8 = 88;  // Pick unused ID in Life Support range (80-89)
   ```

2. **Add to generation logic in `generation.rs`:**
   ```rust
   // In the build_ship_graph or facility manifest
   FacilitySpec {
       name: "Hydroponics Bay",
       room_type: HYDROPONICS,
       target_area: 80.0,
       capacity: 20,
       count: 2,  // 2 bays per deck
       deck_zone: 4,  // Life support zone
       group: 0,
   }
   ```

3. **Update client rendering (optional):**
   ```rust
   // In sync_rooms(), add color mapping
   let color = match room.room_type {
       room_types::HYDROPONICS => Color::srgb(0.2, 0.8, 0.2),  // Green
       // ... existing cases
   };
   ```

4. **Rebuild and verify:**
   ```bash
   spacetime build --project-path crates/progship-server
   spacetime publish --clear-database -y --project-path crates/progship-server progship
   spacetime call progship init_ship "Test" 5 100 50
   python verify_doors.py  # Ensure no connectivity regressions
   ```

### Adding a New Simulation System

**Goal:** Add a "Radiation Exposure" system tracking solar radiation levels.

1. **Add table in `tables.rs`:**
   ```rust
   #[table(name = radiation_exposure, public)]
   pub struct RadiationExposure {
       #[primary_key]
       pub person_id: u64,
       pub accumulated_rads: f32,
       pub shielded: bool,
   }
   ```

2. **Regenerate client SDK:**
   ```bash
   spacetime generate --lang rust --out-dir crates/progship-client-sdk/src --project-path crates/progship-server
   mv crates/progship-client-sdk/src/mod.rs crates/progship-client-sdk/src/lib.rs
   ```

3. **Implement system logic in `simulation.rs`:**
   ```rust
   pub fn tick_radiation(ctx: &ReducerContext, delta_seconds: f32) {
       for person in ctx.db.person().iter() {
           if let Some(position) = ctx.db.position().person_id().find(person.id) {
               if let Some(room) = ctx.db.room().id().find(position.room_id) {
                   // Check if room has shielding
                   let shielded = room.room_type == room_types::QUARTERS || /* ... */;
                   
                   // Update exposure
                   if let Some(mut exposure) = ctx.db.radiation_exposure().person_id().find(person.id) {
                       if !shielded {
                           exposure.accumulated_rads += 0.01 * delta_seconds;
                           ctx.db.radiation_exposure().person_id().update(exposure);
                       }
                   }
               }
           }
       }
   }
   ```

4. **Call from tick reducer in `reducers.rs`:**
   ```rust
   #[reducer]
   pub fn tick(ctx: &ReducerContext, delta_seconds: f32) {
       // ... existing simulation systems
       simulation::tick_radiation(ctx, delta_seconds);
   }
   ```

5. **Test:**
   ```bash
   cargo test --package progship-client  # If you added client-side tests
   cargo clippy --package progship-server -- -D warnings
   ```

### Adding a New Reducer

**Goal:** Add a "use_item" reducer for inventory interactions.

1. **Define in `reducers.rs`:**
   ```rust
   #[reducer]
   pub fn use_item(ctx: &ReducerContext, item_id: u32) {
       // Get player's Person ID from ConnectedPlayer
       let player = ctx.db.connected_player().identity().find(ctx.sender).unwrap();
       let person_id = player.person_id.unwrap();
       
       // Validate player owns item (check Inventory table, not shown here)
       // Apply item effect (heal, satisfy need, etc.)
       
       log::info!("Player {} used item {}", person_id, item_id);
   }
   ```

2. **Regenerate SDK** (as above).

3. **Call from client:**
   ```rust
   // In some client input handler
   if keys.just_pressed(KeyCode::KeyI) {
       use_item(&ctx, 123);  // Item ID
   }
   ```

### Adding a New Client Feature

**Goal:** Add a minimap showing all people on the current deck.

1. **Create Bevy system in `main.rs`:**
   ```rust
   fn render_minimap(
       mut gizmos: Gizmos,
       view_state: Res<ViewState>,
       positions: Query<&Position>,
       // ... other queries
   ) {
       // Draw minimap in corner of screen
       for position in positions.iter() {
           let room = /* fetch room from connection */;
           if room.deck == view_state.current_deck {
               // Draw dot at (position.x, position.y) on minimap
           }
       }
   }
   ```

2. **Add system to app:**
   ```rust
   .add_systems(Update, (
       // ... existing systems
       render_minimap,
   ))
   ```

3. **Test visually:**
   ```bash
   cargo run --package progship-client
   ```

---

## Appendix: Technical Constraints

### SpacetimeDB WASM Sandbox

The server runs in a WebAssembly environment with strict limitations:

**Cannot use:**
- External crates (`rand`, `petgraph`, `lyon`, `noise`)
- File I/O (no reading/writing files)
- Network access (no HTTP calls)
- Threads (single-threaded)
- System calls (no clock, no env vars)

**Can use:**
- Pure Rust standard library (no_std subset)
- SpacetimeDB built-in RNG (`ctx.rng()` or LCG-based pseudo-random)
- Any code you write yourself
- `serde` for serialization (built into SpacetimeDB)

**Implications:**
- All algorithms (BFS, treemap, etc.) are implemented from scratch
- Data files must be embedded as constants, not loaded at runtime
- Random number generation uses Linear Congruential Generator (LCG)

### Coordinate System

- **Server (2D grid)**: x = beam (port-starboard), y = length (fore-aft), 1 cell = 1 meter
- **Client (3D Bevy)**: world_x = game_x, world_y = height, world_z = -game_y
- **Cardinal directions**: NORTH=0=low Y (fore), SOUTH=1=high Y (aft), EAST=2=high X (starboard), WEST=3=low X (port)

### Performance Targets

| Metric | Target | Current Status |
|--------|--------|----------------|
| Agents simulated | 5,000–10,000 | ~150 (early development) |
| Movement tick | 60 Hz | ✅ (client-side) |
| Needs tick | 0.1 Hz | ✅ |
| Client render | 60 fps | ✅ |
| Client input batch | 20 Hz | ✅ |
| People sync (NPC rebuild) | 5 Hz | ✅ |
| Door verification | 0 errors | ✅ |

**Note:** The simulation is currently optimized for ~150 agents during early development. Scaling to 5,000+ agents will require implementing LOD (level of detail) systems, spatial partitioning, and optimized tick frequencies for distant agents.

---

## Further Reading

- **Design Notes**: See `docs/vault/Design/` for deep dives on specific systems
- **Contributing Guide**: `CONTRIBUTING.md` for setup and PR workflow
- **SpacetimeDB Docs**: [https://spacetimedb.com/docs](https://spacetimedb.com/docs)
- **Bevy Engine**: [https://bevyengine.org/learn/](https://bevyengine.org/learn/)

---

**For questions or contributions, see `CONTRIBUTING.md` or open an issue on GitHub.**
