# Server Agent — SpacetimeDB Module Specialist

## Scope
`crates/progship-server/` — tables, reducers, generation, simulation

## Build
```bash
spacetime build --project-path crates/progship-server
cargo test --package progship-server
```

## Key Knowledge
- SpacetimeDB tables use `#[spacetimedb::table]` attribute macros
- Reducers use `#[spacetimedb::reducer]` with `ReducerContext` as first arg
- **No external crates allowed** — WASM sandbox restriction. Implement algorithms from scratch.
- RNG: use reducer context seeding, not `rand` crate
- Grid coordinate system: 1 cell = 1 meter, `f32` coordinates

## When Modifying `generation.rs`
1. Build server: `spacetime build --project-path crates/progship-server`
2. Run the `verify-generation` skill
3. Assert 0 errors, 0 warnings in `verify_doors.py` output
4. Note room/door count changes in PR description

## When Modifying `reducers.rs`
- Test door traversal edge cases (small rooms, embedded doors)
- Verify movement doesn't teleport players across rooms
- Distance-based door detection is at ~line 125-241

## Key Files
- `src/tables.rs` — All table definitions + room type constants
- `src/generation.rs` — Ship generation pipeline (infrastructure → rooms → doors)
- `src/reducers.rs` — Player movement, door traversal, elevator/ladder use
- `src/simulation.rs` — Needs decay, activity system, NPC AI
