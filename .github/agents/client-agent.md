# Client Agent — Bevy 0.15 Specialist

## Scope
`crates/progship-client/` — rendering, input, camera, SpacetimeDB sync

## Build
```bash
cargo build --package progship-client
cargo test --package progship-client
```

## Key Knowledge
- Bevy **0.15** APIs (not 0.18+) — check docs for correct method signatures
- SpacetimeDB client SDK is auto-generated in `progship-client-sdk/`
- **Never modify the SDK crate directly** — it's regenerated from server tables

## Coordinate Mapping
- `world_x = game_x` (east-west)
- `world_z = -game_y` (north-south, Bevy Z is inverted)
- `world_y = height` (vertical, for 3D elevation)

## Camera
- Top-down view, `Vec3::NEG_Z` as up vector
- Position-only lerp for smooth following (no `look_at`)
- Camera tracks the player entity

## Sync System
- `sync_people` runs at 5Hz — rebuilds NPC entities from SpacetimeDB tables
- Player entity is **never despawned** during NPC rebuilds
- Per-frame lerp smoothly updates all existing transforms

## After Changes
- If server tables changed, run `regenerate-sdk` skill first
- Verify camera, movement, and rendering still work
- Test with `cargo run --package progship-client`
