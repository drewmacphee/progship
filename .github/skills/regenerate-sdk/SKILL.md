# Skill: Regenerate SDK

Regenerate the SpacetimeDB client SDK after server table or reducer changes.

## When to Use
- After adding/modifying tables in `crates/progship-server/src/tables.rs`
- After adding/modifying reducers in `crates/progship-server/src/reducers.rs`

## Prerequisites
- SpacetimeDB CLI installed
- Server builds successfully: `spacetime build --project-path crates/progship-server`

## Steps
1. Generate SDK:
   ```bash
   spacetime generate --lang rust --out-dir crates/progship-client-sdk/src --project-path crates/progship-server
   ```
2. When prompted to delete `lib.rs`, answer `y`
3. Rename generated module file:
   ```bash
   # PowerShell
   Move-Item -Force crates/progship-client-sdk/src/mod.rs crates/progship-client-sdk/src/lib.rs
   # Unix
   mv crates/progship-client-sdk/src/mod.rs crates/progship-client-sdk/src/lib.rs
   ```
4. Verify client builds:
   ```bash
   cargo build --package progship-client
   ```

## Important
- **Never manually edit** files in `crates/progship-client-sdk/` — they are overwritten by this process
- Commit the regenerated SDK files as part of your PR
