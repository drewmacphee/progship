# Skill: Build and Test

Build all crates, run linting, and execute tests.

## Steps
1. Check formatting: `cargo fmt --all -- --check`
2. Run clippy: `cargo clippy --all-targets -- -D warnings`
3. Build server: `spacetime build --project-path crates/progship-server`
4. Build client: `cargo build --package progship-client`
5. Run all tests: `cargo test --all`

## Expected
- All steps exit with code 0
- No clippy warnings
- No formatting issues
- All tests pass
