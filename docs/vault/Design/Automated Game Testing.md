# Automated Game Testing

Research into methods for automatically verifying game correctness in CI — catching layout bugs, movement issues, and visual regressions without manual playtesting.

---

## Testing Pyramid for ProgShip

```
         ┌─────────────────┐
         │  Visual / E2E   │  Headless render + image comparison
         │  (slow, fragile) │  Automated walkthrough bot
         ├─────────────────┤
         │  Integration     │  SpacetimeDB publish + verify_doors.py
         │  (medium)        │  Reducer call sequences
         ├─────────────────┤
         │  Unit Tests      │  cargo test (pure logic)
         │  (fast, stable)  │  Property-based generation tests
         └─────────────────┘
```

---

## Layer 1: Unit Tests (cargo test)

**What to test**:
- Generation logic: given a seed, does layout produce valid rooms?
- Needs decay math: does hunger increase at correct rate?
- Door detection: given player position and door, does traversal trigger correctly?
- Pathfinding: does BFS find path between connected rooms?
- Facility manifest: do room counts match expectations?

**How**:
- Standard `#[test]` functions in `tests/` or inline `#[cfg(test)]` modules
- No SpacetimeDB needed — test pure functions extracted from reducers
- Property-based: run generation with 100 random seeds, assert invariants hold

**Invariants to test**:
- Every room has at least one door
- Every door connects two rooms that exist
- Door positions are within room boundaries
- No room overlaps another room (except shaft-in-corridor by design)
- All corridors are connected (BFS from any corridor reaches all others)
- Room counts match facility manifest quantities

---

## Layer 2: Integration Tests (SpacetimeDB in CI)

**What to test**:
- Full generation pipeline: `init_ship` produces valid world state
- Door verification: `verify_doors.py` returns 0 errors
- Reducer correctness: `player_move` doesn't teleport, respects walls
- Elevator/ladder traversal: deck changes work correctly

**How**:
- Install SpacetimeDB CLI in CI runner
- `spacetime start` → `spacetime build` → `spacetime publish` → `spacetime call init_ship`
- Run verification scripts against live DB
- Call reducers with test inputs, query results

**Challenges**:
- SpacetimeDB needs a running instance (not just a library import)
- CI runner needs SpacetimeDB binary — may need custom Docker image
- Tests are slower (seconds, not milliseconds)

---

## Layer 3: Visual / E2E Tests (Future)

### 3a: Headless Rendering

**Goal**: Render a deck view without a GPU/display, compare to reference image.

**Tools**:
- `bevy_headless_render` crate — offscreen rendering to image buffer
- Bevy's built-in `headless_renderer.rs` example
- Image comparison: pixel diff with tolerance threshold

**Pipeline**:
1. Build client with headless feature flag
2. Connect to SpacetimeDB (local instance in CI)
3. Render deck 0 top-down view → save as `deck0_actual.png`
4. Compare against `tests/references/deck0_expected.png`
5. Fail if pixel diff > threshold (e.g., 5%)
6. Upload diff image as CI artifact for manual review

**Challenges**:
- Bevy 0.15 vs `bevy_headless_render` version compatibility
- Procedural generation means layout differs per seed — must fix seed
- Color/lighting differences between platforms
- Setting up headless Bevy without windowing dependencies on CI

### 3b: Automated Walkthrough Bot

**Goal**: A bot that spawns into the game, walks through doors, and verifies it doesn't teleport or get stuck.

**How**:
1. Connect test client to SpacetimeDB
2. Call `player_join` reducer
3. For each door on current deck:
   a. Move player toward door position
   b. Step through door
   c. Verify new position is in expected destination room
   d. Verify position is near door (not teleported to room center)
4. Use elevators to change decks, repeat
5. Report: X doors tested, Y passed, Z failed (with details)

**This can run WITHOUT rendering** — pure SpacetimeDB reducer calls:
```rust
// Pseudocode
player_join("test_bot");
for door in all_doors_on_deck(0) {
    move_player_to(door.position);
    step_through(door);
    assert!(player_in_room(door.dest_room));
    assert!(player_near(door.exit_position, tolerance=2.0));
}
```

**This is the highest-value automated test** — catches the exact teleportation/wall bugs we've been fighting.

---

## Property-Based Generation Testing

Run generation with many seeds, assert invariants hold:

```rust
#[test]
fn generation_invariants_hold_across_seeds() {
    for seed in 0..100 {
        let world = generate_ship(seed, 21, 100, 50);
        
        // Every room has at least one door
        for room in &world.rooms {
            assert!(world.doors.iter().any(|d| 
                d.room_a == room.id || d.room_b == room.id
            ), "Room {} has no doors", room.id);
        }
        
        // No door positions outside room boundaries
        for door in &world.doors {
            let room_a = find_room(&world.rooms, door.room_a);
            assert!(door.door_x >= room_a.x - 1.0 
                && door.door_x <= room_a.x + room_a.width + 1.0);
        }
        
        // All corridors connected via BFS
        assert!(all_corridors_connected(&world));
    }
}
```

**Challenge**: Generation currently requires SpacetimeDB context (`ReducerContext`). Need to either:
- Extract pure generation logic into testable functions
- Or run tests against a SpacetimeDB instance

---

## Seeded Reproducibility

**Critical for automated testing**: Same seed must produce same layout.

Current state: Generation uses `ctx.rng()` which should be deterministic per seed.

To verify:
```bash
# Generate twice with same params
spacetime call progship init_ship '"Test"' 21 100 50
# Dump rooms
spacetime sql progship "SELECT * FROM room" > run1.txt
# Clear and regenerate
spacetime publish --clear-database -y ...
spacetime call progship init_ship '"Test"' 21 100 50
spacetime sql progship "SELECT * FROM room" > run2.txt
# Compare
diff run1.txt run2.txt  # Should be identical
```

---

## Reference Links

- [Bevy headless example](https://github.com/bevyengine/bevy/blob/main/examples/app/headless.rs)
- [Bevy headless renderer](https://github.com/bevyengine/bevy/blob/main/examples/app/headless_renderer.rs)
- [bevy_headless_render crate](https://lib.rs/crates/bevy_headless_render)
- [Game Testing Frameworks Guide 2025](https://generalistprogrammer.com/tutorials/game-testing-frameworks-complete-automation-guide-2025)
- [SpacetimeDB CI/CD](https://deepwiki.com/clockworklabs/SpacetimeDB/8.2-testing-framework-and-cicd)

---

## Priority Order

1. **Unit tests** (now) — extract pure functions, test with `cargo test`
2. **Door verification in CI** (soon) — already have `verify_doors.py`
3. **Walkthrough bot** (next) — highest value for catching movement bugs
4. **Headless rendering** (later) — visual regression, most complex to set up

---

## See Also
- [[GitHub CICD Pipeline]] — CI workflow design
- [[Technical Constraints]] — WASM sandbox, build pipeline
- [[Open Problems]] — the bugs this testing would catch
