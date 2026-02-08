//! ProgShip Core - Colony Ship Simulation Engine
//!
//! An ECS-based simulation of a deep space colony ship with thousands of
//! crew and passengers, each with their own needs, duties, and social lives.
//!
//! # Architecture
//!
//! The simulation uses an Entity Component System (ECS) architecture via `hecs`:
//! - **Entities**: People, rooms, ship systems, conversations
//! - **Components**: Pure data attached to entities (Position, Needs, Crew, etc.)
//! - **Systems**: Logic that queries and updates components
//!
//! # Example
//!
//! ```rust,no_run
//! use progship_core::prelude::*;
//! use progship_core::generation::ShipConfig;
//!
//! let mut engine = SimulationEngine::new();
//!
//! // Generate a ship with crew
//! engine.generate(ShipConfig::default());
//!
//! // Run simulation
//! loop {
//!     engine.update(1.0 / 60.0); // 60 FPS
//! }
//! ```

pub mod components;
pub mod systems;
pub mod generation;
pub mod engine;
pub mod persistence;

/// Commonly used types for convenient importing
pub mod prelude {
    pub use crate::components::*;
    pub use crate::engine::SimulationEngine;
}
