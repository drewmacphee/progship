# Skill: Verify Generation

Verify that ship generation produces a valid layout with correct doors.

## Prerequisites
- SpacetimeDB CLI installed (`spacetime` command available)
- SpacetimeDB running locally (`spacetime start`)
- Python 3 installed

## Steps
1. Build server:
   ```bash
   spacetime build --project-path crates/progship-server
   ```
2. Publish with clean database:
   ```bash
   spacetime publish --clear-database -y --project-path crates/progship-server progship -s http://localhost:3000
   ```
3. Initialize ship:
   ```bash
   spacetime call progship init_ship '"Test Ship"' 21 100 50 -s http://localhost:3000
   ```
4. Dump rooms:
   ```bash
   spacetime sql progship "SELECT id, room_type, deck, x, y, width, height FROM room" -s http://localhost:3000 > rooms_dump.txt
   ```
5. Dump doors:
   ```bash
   spacetime sql progship "SELECT id, room_a, room_b, wall_a, wall_b, door_x, door_y, width FROM door" -s http://localhost:3000 > doors_dump.txt
   ```
6. Run verification:
   ```bash
   python verify_doors.py
   ```

## Expected Output
```
0 errors, 0 warnings
```

## If Errors Found
- Run `python categorize_errors.py` for detailed breakdown
- Fix generation code and repeat from step 1
