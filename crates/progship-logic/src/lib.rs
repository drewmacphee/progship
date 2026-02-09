//! Pure simulation logic for ProgShip.
//!
//! This crate contains all game logic that is independent of any database,
//! engine, or runtime. Functions take plain data and return results, making
//! them unit-testable and portable across SpacetimeDB (WASM), native CLI
//! tools, and any future engine.

pub mod actions;
pub mod constants;
pub mod duty;
pub mod economy;
pub mod geometry;
pub mod health;
pub mod mission;
pub mod movement;
pub mod pathfinding;
pub mod systems;
pub mod utility;
