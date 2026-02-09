# Contributing to ProgShip

Thank you for your interest in contributing to ProgShip! This guide will help you set up your development environment and understand our workflow.

## Prerequisites

Before you begin, ensure you have the following installed:

### Required Tools

- **Rust** (stable toolchain) — Install via [rustup](https://rustup.rs/)
- **SpacetimeDB CLI** — Install with `curl -fsSL https://install.spacetimedb.com | bash`
  - Verify with `spacetime --version`
- **Python 3.x** — For door verification scripts

### System Dependencies (Linux only)

If you're on Linux, Bevy requires additional system libraries:

```bash
sudo apt-get update && sudo apt-get install -y libasound2-dev libudev-dev
```

On macOS and Windows, no additional system dependencies are required.

## Development Setup

### First-Time Setup

Follow these steps to set up your development environment:

1. **Clone the repository**
   ```bash
   git clone https://github.com/drewmacphee/progship.git
   cd progship
   ```

2. **Start SpacetimeDB**
   ```bash
   spacetime start
   ```
   Leave this running in a separate terminal.

3. **Build the server**
   ```bash
   spacetime build --project-path crates/progship-server
   ```

4. **Publish the server module to local SpacetimeDB**
   ```bash
   spacetime publish --clear-database -y --project-path crates/progship-server progship
   ```

5. **Initialize the ship**
   ```bash
   spacetime call progship init_ship "Test Ship" 21 3000 2000
   **Note:** The SDK is auto-generated. Do not modify files in `crates/progship-client-sdk/` manually.

7. **Build the client**
   ```bash
   cargo build --package progship-client
   ```

### Full Rebuild Script (PowerShell)

For convenience, you can use the provided rebuild script:

```powershell
.\scripts\rebuild.ps1
```

This script:
- Builds the server
- Publishes to SpacetimeDB
- Initializes the ship
- Verifies door placement
- Builds the client

## Running Locally

### Starting the SpacetimeDB Server

```bash
spacetime start
```

Keep this running in a dedicated terminal.

### Building and Publishing the Server Module

```bash
# Build the server WASM module
spacetime build --project-path crates/progship-server

# Publish to local SpacetimeDB
spacetime publish --clear-database -y --project-path crates/progship-server progship

# Initialize the ship (required after publishing)
spacetime call progship init_ship "My Ship" 21 100 50
```

### Running the Client

```bash
cargo run --package progship-client
```

The client will automatically connect to your local SpacetimeDB instance at `http://localhost:3000`.

### Quick Door Verification

After making changes to ship generation, verify door placement:

```powershell
.\scripts\verify.ps1
```

Or manually:

```bash
spacetime -s http://localhost:3000 sql progship "SELECT id, room_type, deck, x, y, width, height FROM room" > rooms_dump.txt
spacetime -s http://localhost:3000 sql progship "SELECT id, room_a, room_b, wall_a, wall_b, door_x, door_y, width FROM door" > doors_dump.txt
python verify_doors.py
```

## Running Tests

### Test Coverage

```bash
# Run client tests (matches CI)
cargo test --package progship-client
```

**Important Notes:**
- Server tests run only linting (Clippy) due to WASM runtime constraints
- Full unit tests are available only for the client
- The CI workflow runs both Clippy and tests

### What Tests Cover

- **Client tests**: Unit tests for client-side logic
- **Server validation**: Clippy linting only (WASM prevents full testing)
- **Integration**: Manual verification via door verification scripts

### Running Individual Test Suites

```bash
# Lint server code
cargo clippy --package progship-server -- -D warnings

# Lint client code
cargo clippy --package progship-client -- -D warnings

# Test client
cargo test --package progship-client
```

## Code Style

### Formatting

All code must be formatted with `rustfmt`:

```bash
cargo fmt --all
```

Check formatting without modifying files:

```bash
cargo fmt --all -- --check
```

### Linting

All code must pass Clippy with zero warnings:

```bash
cargo clippy --all-targets -- -D warnings
```

### Documentation

- All public functions, structs, and modules must have `///` doc comments
- Explain **what** and **why**, not just **how**
- Include examples for complex APIs
- Keep comments minimal — only where logic isn't obvious

### Code Conventions

- New components/tables should prefer `#[derive(Debug, Clone, Serialize, Deserialize)]`
- For new IDs, prefer `u32` where it fits; some existing tables use `u64` primary keys — follow the established type for that table
- Components/tables should remain pure data — logic lives in systems/reducers
- Prefer composition over inheritance
- Return early with descriptive error messages

## Pull Request Process

### Before Opening a PR

1. **Create or reference an issue**
   - Create a new issue if one doesn't exist
   - Reference existing issues with `Fixes #N` in your PR description

2. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/bug-description
   ```

3. **Make your changes**
   - Keep changes focused on a single concern
   - Write tests for new logic
   - Add doc comments to public items

4. **Verify your changes**
   ```bash
   # Format code
   cargo fmt --all
   
   # Run linting
   cargo clippy --all-targets -- -D warnings
   
   # Run tests
   cargo test --all
   
   # Build client
   cargo build --package progship-client
   ```

5. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat: add your feature description"
   # or
   git commit -m "fix: fix bug description"
   ```

### CI Requirements

All PRs must pass continuous integration checks:

- ✅ **Formatting**: `cargo fmt --all -- --check`
- ✅ **Linting (server)**: `cargo clippy --package progship-server -- -D warnings`
- ✅ **Linting (client)**: `cargo clippy --package progship-client -- -D warnings`
- ✅ **Build**: `cargo build --package progship-client`
- ✅ **Tests**: `cargo test --package progship-client`

See `.github/workflows/ci.yml` for the complete CI configuration.

### PR Guidelines

- **One concern per PR** — Don't mix unrelated changes
- **Link issues** — Use `Fixes #N` or `Closes #N` in the PR description
- **Descriptive titles** — Clearly state what the PR does
- **Explain why** — Describe the motivation and context in the PR description
- **Keep PRs small** — Smaller PRs are easier to review and merge
- **Respond to feedback** — Address review comments promptly

### Review Process

1. A maintainer will review your PR
2. Address any requested changes
3. Once approved, a maintainer will merge your PR

## Agent Conventions

If you're working with GitHub Copilot or other AI agents, please reference the agent-specific instructions:

- **Global agent instructions**: `.github/AGENTS.md`
- **Custom agent configurations**: `.github/agents/`
- **Skills**: `.github/skills/`

### Key Agent Rules

- Always write tests for new logic
- Always add doc comments to public items
- Run `cargo test --all` before committing
- Zero Clippy warnings allowed
- If modifying `generation.rs`, note that door verification is needed

### SpacetimeDB Constraints

The server runs in a WASM sandbox with limitations:

- No external crates (except `spacetimedb` and `log`)
- No `rand`, `HashMap`, or filesystem access
- Implement algorithms from scratch if needed
- Use SpacetimeDB's built-in data structures

### Protected Directories

- **DO NOT** modify `crates/progship-client-sdk/` — auto-generated by SpacetimeDB
- **DO NOT** modify `archive/` — read-only reference code
- **DO NOT** commit `.env` files, secrets, or API keys
- **DO NOT** commit binary files (`*.wasm`, `target/`, `save.bin`)
- **DO NOT** commit dump files (`*_dump.txt`)

## Questions?

If you have questions or need help:

1. Check existing issues and discussions
2. Review the documentation in `docs/vault/`
3. Review the architecture in `.github/AGENTS.md`
4. Open a new issue with the `question` label

Thank you for contributing to ProgShip! 🚀
