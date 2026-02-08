//! Component definitions for the ECS simulation.
//!
//! Components are pure data structs attached to entities.
//! They have no behavior - that lives in systems.

mod common;
mod people;
mod ship;
mod social;

pub use common::*;
pub use people::*;
pub use ship::*;
pub use social::*;
