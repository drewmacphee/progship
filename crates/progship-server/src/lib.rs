//! ProgShip Server - SpacetimeDB Module
//!
//! Colony ship simulation running as a SpacetimeDB module.
//! All simulation logic runs here as reducers; clients are thin renderers.

mod tables;
mod generation;
mod simulation;
mod reducers;

pub use tables::*;
pub use reducers::*;
