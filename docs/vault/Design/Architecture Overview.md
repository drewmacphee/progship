# Architecture Overview

## Tech Stack
- **Server**: SpacetimeDB (Rust → WASM module)
  - All simulation logic runs as "reducers" inside the database
  - Auto-persistence — no save system needed, survives crashes
  - Server-authoritative — prevents cheating in multiplayer
  - Real-time client subscriptions via WebSocket
- **Client**: Bevy 0.15 (Rust, 3D rendering)
  - Thin client — just renders, sends inputs
  - SpacetimeDB SDK (direct, not bevy_spacetimedb plugin)
  - Top-down camera, 3D room/person rendering
- **Client SDK**: Auto-generated from server tables via `spacetime generate`

## Why SpacetimeDB?
Decision made early — multiplayer is a goal. SpacetimeDB gives us:
- Shared world state (all players see same 5,000+ NPCs)
- Atomic transactions (no duping/exploiting)
- Rust-native (our simulation is already Rust)
- No separate game server needed

Trade-offs accepted:
- WASM sandbox — no file I/O, no external crates (`rand`, `petgraph`, etc.)
- Single-player still runs SpacetimeDB locally
- Different programming model (tables + reducers, not traditional ECS)

## Project Structure
```
progship/
├── crates/
│   ├── progship-server/       # SpacetimeDB module (ALL simulation)
│   │   └── src/
│   │       ├── lib.rs         # Entry point, table re-exports
│   │       ├── tables.rs      # All table definitions + constants
│   │       ├── generation.rs  # Ship/crew/passenger procedural generation
│   │       ├── simulation.rs  # Tick-based systems (needs, activities, atmosphere)
│   │       └── reducers.rs    # Player actions, movement, elevator/ladder use
│   │
│   ├── progship-client/       # Bevy thin client
│   │   └── src/main.rs        # Rendering, input, camera, sync
│   │
│   └── progship-client-sdk/   # Auto-generated SpacetimeDB bindings
│
├── docs/vault/                # This Obsidian vault
├── data/                      # Static JSON data files
├── tests/                     # Integration tests
├── archive/                   # Old Python/Godot code (read-only reference)
└── verify_doors.py            # Mathematical door verification pipeline
```

## Data Flow
```
Player Input (WASD, E, F, etc.)
    → Bevy captures input
    → Calls SpacetimeDB reducer (e.g., player_move)
    → Server validates, updates tables
    → Subscription pushes update to all clients
    → Bevy syncs entities from table data
    → Render
```

## Build & Deploy
```bash
# Server
spacetime build --project-path crates/progship-server
spacetime publish --clear-database -y --project-path crates/progship-server progship -s http://localhost:3000
spacetime call progship init_ship '"ISV Prometheus"' 21 100 50 -s http://localhost:3000

# Client SDK regeneration (after server changes)
spacetime generate --lang rust --out-dir crates/progship-client-sdk/src --project-path crates/progship-server
# Answer 'y' to delete lib.rs, then:
mv crates/progship-client-sdk/src/mod.rs crates/progship-client-sdk/src/lib.rs

# Client
cargo build --package progship-client
cargo run --package progship-client
```

## See Also
- [[Simulation Systems]]
- [[Procedural Generation]]
- [[Technical Constraints]]
