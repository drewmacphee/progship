# Agent Instructions

## Project Overview

**ProgShip** is a real-time simulation engine for deep space colony ships with 5,000+ crew and passengers. Built in Rust using Entity Component System (ECS) architecture for performance and scalability.

## Technology Stack

- **Language**: Rust
- **ECS**: hecs (core simulation), Bevy (visualization)
- **Serialization**: serde, bincode
- **Server**: SpacetimeDB (tables + reducers, compiled to WASM)
- **Client**: Bevy 0.15 (rendering, input, camera)
- **SDK**: Auto-generated Rust bindings (`progship-client-sdk`)

## Repository Structure

```
progship/
├── crates/
│   ├── progship-logic/        # Pure simulation logic (portable, no engine deps)
│   ├── progship-core/         # ECS simulation engine (hecs)
│   ├── progship-server/       # SpacetimeDB module (tables, reducers)
│   ├── progship-client-sdk/   # Auto-generated SDK (DO NOT modify)
│   ├── progship-client/       # Bevy 0.15 multiplayer client
│   ├── progship-viewer/       # Bevy offline ship visualizer
│   └── progship-simtest/      # Headless simulation test harness
├── data/                      # Static JSON data files
├── docs/                      # Documentation
├── scripts/                   # Build/verify scripts
└── archive/                   # Old Python/Godot code (read-only)
```

## Git Workflow — MANDATORY

**NEVER commit directly to master.** All changes must follow this workflow:

1. Create a feature branch: `git checkout -b feature/description`
2. Make changes, commit to the branch
3. Run checks: `cargo fmt --all`, `cargo clippy -- -D warnings`, `cargo test --workspace --exclude progship-server --exclude progship-client`
4. Push the branch: `git push -u origin feature/description`
5. Create a PR: `gh pr create --title "..." --body "..."`
6. Wait for CI to pass
7. Merge via squash: `gh pr merge N --squash --delete-branch`

This applies to ALL changes — even single-line fixes, doc updates, or config tweaks.

### Branch Naming

| Prefix | Use Case | Example |
|--------|----------|---------|
| `feature/` | New features, modules | `feature/disease-system` |
| `fix/` | Bug fixes | `fix/lod-doctest-import` |
| `docs/` | Documentation only | `docs/update-readme` |
| `chore/` | Config, tooling, maintenance | `chore/add-workspace-member` |

## Build & Test Commands

```bash
# Build all crates
cargo build --release

# Build only the simulation logic
cargo build --package progship-logic

# Run tests (exclude WASM-only server and client)
cargo test --workspace --exclude progship-server --exclude progship-client

# Run headless simulation harness
cargo run -p progship-simtest

# Run offline ship viewer
cargo run -p progship-viewer --release

# Lint and format
cargo clippy -- -D warnings
cargo fmt --all
```

## Architecture

### progship-logic (Pure Logic — Portable)
23 modules of pure functions and data structures with zero engine dependencies.
Used by both the ECS core and SpacetimeDB server.

Key modules: actions, archetypes, atmosphere, config, constants, conversation,
cylinder, duty, economy, geometry, health, lod, manifest, mission, movement,
pathfinding, population, security, service_decks, ship_config, skills, supplies,
systems, utility

### progship-core (ECS Simulation)
- **Components**: Position, Movement, Needs, Crew, Passenger, Activity, Personality, Skills
- **Systems**: movement (60Hz), activity (1Hz), needs (0.1Hz), ship systems (0.01Hz)
- **Generation**: Procedural ship layout pipeline

### Server (SpacetimeDB)
- **Tables**: Room, Door, Person, ShipSystem, etc.
- **Reducers**: Player movement, door traversal, ship init
- Cannot be tested natively on Windows (WASM-only symbols)

### Client (Bevy 0.15)
- Thin client — subscribes to SpacetimeDB tables, renders state
- Requires SpacetimeDB server running at localhost:3000

### Client SDK Regeneration — MANDATORY

When **any** server table schema changes (fields added/removed/reordered in `tables.rs`),
the client SDK **must** be regenerated. Stale SDK causes silent deserialization bugs
(fields shift, wrong data in wrong fields).

```bash
# Regenerate SDK from server module
spacetime generate --lang rust --out-dir crates/progship-client-sdk/src --project-path crates/progship-server

# IMPORTANT: rename mod.rs → lib.rs (spacetime generates mod.rs, crate needs lib.rs)
mv crates/progship-client-sdk/src/mod.rs crates/progship-client-sdk/src/lib.rs

# Rebuild client to verify
cargo build -p progship-client --release
```

### Key Constants
- `NORTH=0` (low Y), `SOUTH=1` (high Y), `EAST=2` (high X), `WEST=3` (low X)
- 1 grid cell = 1 meter
- IDs are `u32` for performance

## Code Conventions

- `#[derive(Debug, Clone, Serialize, Deserialize)]` on all components/tables
- Components/tables are pure data — logic in systems/reducers
- Never modify `progship-client-sdk/` (auto-generated)
- Minimal comments — only where logic isn't obvious
- Unit tests for all new logic
- Doc comments (`///`) on all public functions
