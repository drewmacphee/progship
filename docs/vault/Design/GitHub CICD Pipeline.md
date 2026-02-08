# GitHub CI/CD Pipeline Plan

Moving ProgShip to GitHub with a full CI/CD pipeline so delegated Copilot agents can work on different workstreams independently and have their PRs automatically validated.

---

## Goals
1. Every PR is automatically built, linted, tested, and verified
2. Copilot coding agents can be assigned issues and produce quality PRs
3. Agents are guided by AGENTS.md and skills to always write tests + docs
4. Eventually: automated game testing catches layout/movement bugs in PRs

---

## Repository Setup

### Branch Strategy
- `main` — stable, always builds
- Feature branches — one per workstream/issue
- PRs require passing CI before merge

### Files to Create

```
progship/
├── .github/
│   ├── AGENTS.md                    # Global agent instructions
│   ├── agents/
│   │   ├── server-agent.md          # SpacetimeDB server specialist
│   │   ├── client-agent.md          # Bevy client specialist
│   │   └── generation-agent.md      # Procgen specialist
│   ├── skills/
│   │   ├── build-server/SKILL.md    # How to build + publish server
│   │   ├── build-client/SKILL.md    # How to build client (SDK dance)
│   │   ├── verify-doors/SKILL.md    # Run door verification pipeline
│   │   └── run-tests/SKILL.md       # cargo test for all crates
│   ├── workflows/
│   │   ├── ci.yml                   # Main CI pipeline
│   │   ├── server-build.yml         # Server-specific build + test
│   │   └── game-test.yml            # Automated game testing (future)
│   └── copilot-instructions.md      # Existing, keep updated
```

---

## CI Pipeline Design

### Tier 1: Every PR (fast, ~2 min)
```yaml
steps:
  - cargo fmt --all -- --check
  - cargo clippy --all-targets -- -D warnings
  - cargo build --package progship-server
  - cargo build --package progship-client
  - cargo test --all
```

### Tier 2: Server Validation (~5 min)
```yaml
steps:
  - spacetime build --project-path crates/progship-server
  - Start local SpacetimeDB instance
  - spacetime publish + init_ship
  - Run door verification (verify_doors.py)
  - Assert: 0 errors, 0 warnings
  - Dump room/door counts as PR comment
```

### Tier 3: Game Testing (future, ~10 min)
```yaml
steps:
  - Build client in headless mode
  - Launch SpacetimeDB + publish + init
  - Run automated walkthrough bot
  - Capture screenshots at key locations
  - Compare against reference images
  - Report pass/fail + diffs as PR artifacts
```

### Runner Requirements
- **Tier 1**: Standard GitHub-hosted runner (ubuntu-latest)
- **Tier 2**: Needs SpacetimeDB CLI + Python — custom Docker image or install step
- **Tier 3**: Needs GPU or `bevy_headless_render` for offscreen rendering

---

## AGENTS.md Design

### Global Instructions (`.github/AGENTS.md`)
Every agent working on this repo must:
1. **Always write tests** — unit tests for new logic, integration tests for systems
2. **Always update docs** — doc comments on public functions, update vault notes if design changes
3. **Run the build** before committing — `cargo build --package progship-server && cargo build --package progship-client`
4. **Run tests** — `cargo test --all`
5. **Follow conventions**:
   - `#[derive(Debug, Clone, Serialize, Deserialize)]` on all components
   - IDs are `u32`, not `String`
   - Components are pure data, logic lives in systems/reducers
   - NORTH=0=low Y, SOUTH=1=high Y, EAST=2=high X, WEST=3=low X
6. **Never modify** `crates/progship-client-sdk/` (auto-generated)
7. **Coordinate system**: 1 grid cell = 1 meter, f32 for local coords

### Specialized Agents

**Server Agent** (`.github/agents/server-agent.md`)
- Scope: `crates/progship-server/`
- Build: `spacetime build --project-path crates/progship-server`
- Test: `cargo test --package progship-server`
- Must verify doors after generation changes
- Understands SpacetimeDB WASM constraints (no external crates)

**Client Agent** (`.github/agents/client-agent.md`)
- Scope: `crates/progship-client/`
- Build: full SDK regeneration dance
- Understands Bevy 0.15 APIs
- Coordinate mapping: world_z = -game_y

**Generation Agent** (`.github/agents/generation-agent.md`)
- Scope: `crates/progship-server/src/generation.rs`
- Must run `verify_doors.py` after any generation change
- Understands grid coordinate system, hull taper, facility manifest
- Must maintain 0 errors, 0 warnings in door verification

---

## Skills Design

### build-server Skill
```markdown
# How to build the SpacetimeDB server module
1. spacetime build --project-path crates/progship-server
2. If build succeeds, module is at target/wasm32-unknown-unknown/release/progship_server.wasm
3. For testing: spacetime start (if not running), then spacetime publish
```

### verify-doors Skill
```markdown
# Run the mathematical door verification pipeline
1. spacetime sql progship "SELECT id, room_type, deck, x, y, width, height FROM room" > rooms_dump.txt
2. spacetime sql progship "SELECT id, room_a, room_b, wall_a, wall_b, door_x, door_y, width FROM door" > doors_dump.txt
3. python verify_doors.py
4. Expected: "0 errors, 0 warnings"
5. If errors found, categorize with: python categorize_errors.py
```

---

## Workstream Isolation

Parallel workstreams that agents can tackle independently:

| Workstream | Branch | Agent | Scope |
|------------|--------|-------|-------|
| Room size fix | `fix/room-sizes` | generation-agent | generation.rs treemap |
| Room type colors | `feat/room-colors` | client-agent | main.rs room_color() |
| Population scale | `feat/population` | server-agent | generation.rs, simulation.rs |
| Per-room atmosphere | `feat/room-atmo` | server-agent | simulation.rs, tables.rs |
| Minimap | `feat/minimap` | client-agent | main.rs new system |
| Economy loop | `feat/economy` | server-agent | simulation.rs new reducer |
| Unit test coverage | `test/coverage` | any agent | tests/ directory |

---

## Migration Steps

### Phase 1: Repository Setup
- [ ] Create GitHub repository (or push existing to remote)
- [ ] Add `.github/workflows/ci.yml` (Tier 1)
- [ ] Add `.github/AGENTS.md`
- [ ] Add `.github/copilot-instructions.md` (update existing)
- [ ] Verify CI passes on main branch
- [ ] Enable branch protection (require CI pass for PR merge)

### Phase 2: Agent Configuration
- [ ] Create specialized agent files in `.github/agents/`
- [ ] Create skill files in `.github/skills/`
- [ ] Test: assign a simple issue to Copilot agent, verify it follows instructions
- [ ] Iterate on AGENTS.md based on agent output quality

### Phase 3: Server Testing in CI
- [ ] Create Docker image or install script for SpacetimeDB CLI in CI
- [ ] Add Tier 2 workflow (server build + publish + verify)
- [ ] Add verify_doors.py to CI as a required check
- [ ] Set up PR comment bot to report door/room counts

### Phase 4: Automated Game Testing
- [ ] Research `bevy_headless_render` compatibility with our Bevy 0.15
- [ ] Build headless test harness (connect to SpacetimeDB, render deck, save image)
- [ ] Create reference screenshots for each deck type
- [ ] Add Tier 3 workflow (headless render + image comparison)
- [ ] Create automated walkthrough bot (spawn, move through doors, verify no teleporting)

---

## See Also
- [[Automated Game Testing]]
- [[Architecture Overview]]
- [[Technical Constraints]]
