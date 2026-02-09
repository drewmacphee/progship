//! Pure logic extracted from reducers for testability.
//!
//! These functions take plain data and return results without any database
//! access, making them unit-testable without SpacetimeDB.

pub mod actions;
pub mod duty;
pub mod economy;
pub mod health;
pub mod mission;
pub mod movement;
pub mod pathfinding;
pub mod systems;
pub mod utility;
