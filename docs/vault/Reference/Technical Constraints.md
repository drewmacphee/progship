# Technical Constraints

Hard limits and gotchas that affect implementation decisions.

---

## SpacetimeDB WASM Sandbox

The server module compiles to WebAssembly and runs inside SpacetimeDB. This means:

### ❌ Cannot Use
- **External crates** — no `rand`, `petgraph`, `lyon`, `noise`, etc.
- **File I/O** — no reading/writing files from reducers
- **Network access** — no HTTP calls, no external APIs
- **Threads** — single-threaded execution
- **System calls** — no clock, no env vars

### ✅ Can Use
- Pure Rust standard library (no_std compatible subset)
- SpacetimeDB's built-in RNG (`ctx.rng()` or similar)
- Any code you write yourself (no dependency restrictions on your own code)
- `serde` for serialization (built into SpacetimeDB)

### Implications
- All algorithms must be implemented from scratch (BFS, treemap, etc.)
- RNG must use SpacetimeDB's provided seed system
- Data files must be embedded as constants, not loaded at runtime
- No `petgraph` for pathfinding → manual BFS through Door table

---

## SpacetimeDB SQL Limitations

The `spacetime sql` CLI does NOT support:
- `ORDER BY`
- `COUNT(*)`
- `GROUP BY`
- `LIKE`
- Aggregate functions

Workaround: dump raw data, process with Python scripts (e.g., `verify_doors.py`).

---

## Coordinate System

### Server (Game Logic)
- 2D grid: `grid[x][y]`
- x = beam (port → starboard, 0 = port)
- y = length (fore → aft, 0 = fore)
- 1 grid cell = 1 meter
- **Cardinal directions**: NORTH=0=low Y (fore), SOUTH=1=high Y (aft), EAST=2=high X (starboard), WEST=3=low X (port)

### Client (Bevy 3D)
- world_x = game_x
- world_z = -game_y (Bevy's Z is inverted relative to our Y)
- world_y = height (up)
- Camera: top-down, looking down Y axis, up vector = Vec3::NEG_Z

### Precision
- f32 for all local coordinates (positions, room dimensions, door positions)
- f64 reserved for future orbital/travel calculations (not used yet)

---

## Performance Targets

| Metric | Target | Current |
|--------|--------|---------|
| Agents simulated | 5,000–10,000 | ~150 |
| Movement tick | 60 Hz | ✅ |
| Needs tick | 0.1 Hz | ✅ |
| Client render | 60 fps | ✅ |
| Client input batch | 20 Hz | ✅ |
| People sync (NPC rebuild) | 5 Hz | ✅ |
| Door count verification | 0 errors | ✅ |

---

## Build Pipeline

The SDK regeneration dance (required after any server table/reducer change):
```bash
spacetime build --project-path crates/progship-server
spacetime publish --clear-database -y --project-path crates/progship-server progship -s http://localhost:3000
spacetime call progship init_ship '"ISV Prometheus"' 21 100 50 -s http://localhost:3000
spacetime generate --lang rust --out-dir crates/progship-client-sdk/src --project-path crates/progship-server
# Answer 'y' to delete lib.rs
# Rename mod.rs → lib.rs
cargo build --package progship-client
```

This is annoying and error-prone. A desktop shortcut exists for the full pipeline.

---

## See Also
- [[Architecture Overview]]
- [[Procedural Generation]]
