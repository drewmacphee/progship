//! Simulation tick systems - all game logic that runs each tick.
//!
//! Systems are called by the `tick` reducer at appropriate frequencies.

mod activities;
mod atmosphere;
mod death;
mod duty;
mod events;
mod maintenance;
mod movement;
mod needs;
mod ship_systems;
mod social;
mod wandering;

// Re-export all public tick functions
pub use activities::tick_activities;
pub use atmosphere::tick_atmosphere;
pub use death::tick_death;
pub use duty::tick_duty;
pub use events::tick_events;
pub use maintenance::tick_maintenance;
pub use movement::tick_movement;
pub use needs::tick_needs;
pub use ship_systems::tick_ship_systems;
pub use social::tick_social;
pub use wandering::tick_wandering;
