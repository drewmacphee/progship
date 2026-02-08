# Agent Best Practices

How to configure GitHub Copilot coding agents to produce high-quality, tested, documented PRs for ProgShip.

---

## AGENTS.md Configuration

### Location
- `.github/AGENTS.md` — global instructions for ALL agents
- `.github/agents/<name>.md` — specialized agent personas

### What Goes in AGENTS.md

```markdown
---
name: progship
description: ProgShip colony ship simulation development
---

# Project
Rust simulation engine using SpacetimeDB (server/WASM) + Bevy (client).

# Build Commands
- Server: spacetime build --project-path crates/progship-server
- Client: cargo build --package progship-client
- Tests: cargo test --all
- Lint: cargo clippy --all-targets -- -D warnings
- Format: cargo fmt --all -- --check

# MANDATORY for every PR
1. All new functions must have unit tests
2. All public functions must have doc comments (/// style)
3. Run `cargo test --all` before committing — must pass
4. Run `cargo clippy` — no warnings allowed
5. If generation.rs was modified, note in PR that door verification is needed

# Conventions
- Derive Debug, Clone, Serialize, Deserialize on all components
- IDs are u32 for performance
- Components are pure data — logic in systems/reducers
- Coordinate system: NORTH=0=low Y, SOUTH=1=high Y, EAST=2=high X, WEST=3=low X
- 1 grid cell = 1 meter
- f32 for local coordinates

# DO NOT
- Modify crates/progship-client-sdk/ (auto-generated)
- Add external crates to progship-server (WASM sandbox restriction)
- Modify archive/ directory (read-only reference)
- Commit .env files, secrets, or API keys
- Commit binary files (save.bin, .wasm, target/)

# Code Style
- Minimal comments — only where logic isn't obvious
- Prefer composition over inheritance
- Use Option<Entity> for optional relationships
- Error handling: return early with descriptive messages
```

---

## Skills Configuration

### Location
`.github/skills/<skill-name>/SKILL.md`

### Skills to Create

**build-and-test**
```markdown
---
name: build-and-test
description: Build all crates and run tests
---
# Steps
1. cargo fmt --all -- --check
2. cargo clippy --all-targets -- -D warnings
3. spacetime build --project-path crates/progship-server
4. cargo build --package progship-client
5. cargo test --all
```

**verify-generation**
```markdown
---
name: verify-generation
description: Verify ship generation produces valid layout
---
# Prerequisites
- SpacetimeDB running locally (spacetime start)
# Steps
1. spacetime build --project-path crates/progship-server
2. spacetime publish --clear-database -y --project-path crates/progship-server progship -s http://localhost:3000
3. spacetime call progship init_ship '"Test Ship"' 21 100 50 -s http://localhost:3000
4. spacetime sql progship "SELECT id, room_type, deck, x, y, width, height FROM room" -s http://localhost:3000 > rooms_dump.txt
5. spacetime sql progship "SELECT id, room_a, room_b, wall_a, wall_b, door_x, door_y, width FROM door" -s http://localhost:3000 > doors_dump.txt
6. python verify_doors.py
7. Expected output: "0 errors, 0 warnings"
```

**regenerate-sdk**
```markdown
---
name: regenerate-sdk
description: Regenerate client SDK after server table/reducer changes
---
# Steps
1. spacetime generate --lang rust --out-dir crates/progship-client-sdk/src --project-path crates/progship-server
2. Answer 'y' when prompted to delete lib.rs
3. Rename: mv crates/progship-client-sdk/src/mod.rs crates/progship-client-sdk/src/lib.rs
4. cargo build --package progship-client
```

---

## Agent Specialization

### Server Agent
```markdown
---
name: server-agent
description: SpacetimeDB server module specialist
target: github-copilot
---
You work on crates/progship-server/. You understand:
- SpacetimeDB tables, reducers, and WASM constraints
- No external crates allowed (implement algorithms from scratch)
- RNG via reducer context, not rand crate
- Grid coordinate system (1 cell = 1 meter)

When modifying generation.rs:
- Always run the verify-generation skill
- Maintain 0 errors, 0 warnings in door verification
- Test with multiple seeds if adding randomized logic

When modifying reducers.rs:
- Test door traversal edge cases (small rooms, embedded doors)
- Verify movement doesn't teleport players
```

### Client Agent
```markdown
---
name: client-agent
description: Bevy 0.15 client specialist
target: github-copilot
---
You work on crates/progship-client/. You understand:
- Bevy 0.15 API (not 0.18+)
- SpacetimeDB SDK (auto-generated in progship-client-sdk)
- Coordinate mapping: world_x = game_x, world_z = -game_y, world_y = height
- Camera: top-down, Vec3::NEG_Z up vector

After changes:
- Run regenerate-sdk skill if server tables changed
- Verify camera, movement, and rendering still work
- Don't modify the SDK crate directly
```

### Generation Agent
```markdown
---
name: generation-agent
description: Procedural ship generation specialist
target: github-copilot
---
You work on crates/progship-server/src/generation.rs. You understand:
- Infrastructure-first layout (corridors → shafts → zones → rooms → doors)
- Squarified treemap packer for room placement
- Grid stamp system (grid[x][y], 1m cells)
- Facility manifest with room specs per deck zone
- Door placement via grid adjacency scanning

CRITICAL: After ANY change to generation:
1. Build server: spacetime build --project-path crates/progship-server
2. Run verify-generation skill
3. Assert 0 errors in verify_doors.py output
4. Note room/door count changes in PR description
```

---

## What Makes a Good Agent PR

1. **Focused** — one issue, one branch, one concern
2. **Tested** — unit tests for new logic, existing tests still pass
3. **Documented** — doc comments on new public items, PR description explains why
4. **Verified** — build passes, clippy clean, door verification (if applicable)
5. **Small** — prefer many small PRs over one massive change

---

## Anti-Patterns to Prevent

| Problem | Prevention |
|---------|------------|
| Agent modifies auto-generated SDK | AGENTS.md: "Never modify progship-client-sdk" |
| Agent adds external crate to server | AGENTS.md: "No external crates (WASM sandbox)" |
| Agent skips tests | AGENTS.md: "MANDATORY: unit tests for new logic" |
| Agent breaks door verification | Generation agent: "Assert 0 errors after changes" |
| Agent guesses entity IDs | AGENTS.md: "IDs are u32, not String" |
| Agent uses wrong coordinate convention | AGENTS.md: "NORTH=0=low Y, SOUTH=1=high Y" |

---

## See Also
- [[GitHub CICD Pipeline]] — how CI validates agent PRs
- [[Automated Game Testing]] — testing methods agents should use
- [[Architecture Overview]] — project structure agents need to understand
