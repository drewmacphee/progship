//! Systems - logic that operates on components

mod movement;
mod needs;
mod activity;
mod wandering;
mod ship_systems;
mod maintenance;
mod social;
mod duty;
mod events;
mod dialogue;

pub use movement::*;
pub use needs::*;
pub use activity::*;
pub use wandering::*;
pub use ship_systems::*;
pub use maintenance::*;
pub use social::*;
pub use duty::*;
pub use events::*;
pub use dialogue::*;
