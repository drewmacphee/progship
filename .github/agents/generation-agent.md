# Generation Agent — Procedural Ship Generation Specialist

## Scope
`crates/progship-server/src/generation.rs` — the ship layout pipeline

## Build & Verify
```bash
spacetime build --project-path crates/progship-server
# Then run verify-generation skill (see skills/)
```

## Key Knowledge
- **Infrastructure-first layout**: corridors → shafts → zones → rooms → doors
- **Squarified treemap** packer for room placement within zones
- **Grid stamp system**: `grid[x][y]`, 1m cells, stamped for collision/adjacency
- **Facility manifest**: room specs (type, count, min/max size) per deck zone
- **Door placement**: grid adjacency scanning between rooms sharing walls
- `should_have_room_door()` filters which room pairs get direct doors

## Pipeline Order
1. Hull dimensions (deck width/height from taper)
2. Central corridor (full width, 3m tall)
3. Elevator shafts + ladder shafts
4. Port/starboard zones
5. Squarified treemap room packing per zone
6. Grid stamping (rooms, corridors, shafts)
7. Corridor doors (rooms touching corridor)
8. Room-to-room doors (filtered by `should_have_room_door`)
9. Shaft doors (elevator/ladder access)

## CRITICAL: After ANY Change
1. Build server: `spacetime build --project-path crates/progship-server`
2. Publish + init: `spacetime publish --clear-database -y --project-path crates/progship-server progship`
3. Init ship: `spacetime call progship init_ship '"Test Ship"' 21 100 50`
4. Dump + verify:
   ```bash
   spacetime sql progship "SELECT id, room_type, deck, x, y, width, height FROM room" > rooms_dump.txt
   spacetime sql progship "SELECT id, room_a, room_b, wall_a, wall_b, door_x, door_y, width FROM door" > doors_dump.txt
   python verify_doors.py
   ```
5. Assert: **0 errors, 0 warnings**
6. Note room/door count changes in PR description
