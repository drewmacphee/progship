# Agent Instructions

## Project Overview

**ProgShip** is a real-time colony ship simulation with 5,000+ crew and passengers. SpacetimeDB server (Rust → WASM) handles all game logic; Bevy 0.15 client is a thin renderer.

## Technology Stack

- **Language**: Rust
- **Server**: SpacetimeDB (tables + reducers, compiled to WASM)
- **Client**: Bevy 0.15 (3D rendering, input, camera)
- **SDK**: Auto-generated Rust bindings (`progship-client-sdk`)

## Repository Structure

```
progship/
├── crates/
│   ├── progship-server/       # SpacetimeDB module (tables, reducers, generation)
│   ├── progship-client-sdk/   # Auto-generated SDK (DO NOT modify)
│   └── progship-client/       # Bevy 0.15 thin client
├── scripts/                   # Build/verify PowerShell scripts
├── docs/vault/                # Obsidian design notes
├── verify_doors.py            # Mathematical door verification
├── categorize_errors.py       # Error categorization for door issues
├── archive/                   # Old Python/Godot code (read-only)
└── data/                      # Static JSON data files
```

## Build & Test Commands

```bash
# Build server (SpacetimeDB WASM module)
spacetime build --project-path crates/progship-server

# Build client (Bevy)
cargo build --package progship-client

# Run all tests
cargo test --all

# Lint and format
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check

# Full rebuild + verify (PowerShell)
.\scripts\rebuild.ps1

# Quick door verification
.\scripts\verify.ps1
```

## Architecture

### Server (SpacetimeDB)
- **Tables**: Room, Door, Person, ShipSystem, etc. (defined in `tables.rs`)
- **Reducers**: Player movement, door traversal, ship init (`reducers.rs`)
- **Generation**: Procedural ship layout pipeline (`generation.rs`)
- **Simulation**: Needs decay, activity system, NPC AI (`simulation.rs`)

### Client (Bevy 0.15)
- Thin client — subscribes to SpacetimeDB tables, renders state
- `sync_people` rebuilds NPC entities at 5Hz; player entity preserved
- Camera: top-down, `Vec3::NEG_Z` up vector
- Coordinate mapping: `world_x = game_x`, `world_z = -game_y`

### Key Constants
- `NORTH=0` (low Y), `SOUTH=1` (high Y), `EAST=2` (high X), `WEST=3` (low X)
- 1 grid cell = 1 meter
- IDs are `u32` for performance

## Code Conventions

- `#[derive(Debug, Clone, Serialize, Deserialize)]` on all components/tables
- Components/tables are pure data — logic in systems/reducers
- No external crates in server (WASM sandbox)
- Never modify `progship-client-sdk/` (auto-generated)
- Minimal comments — only where logic isn't obvious
- Unit tests for all new logic
- Doc comments (`///`) on all public functions
