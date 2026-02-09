//! ProgShip Server - SpacetimeDB Module
//!
//! Colony ship simulation running as a SpacetimeDB module.
//! All simulation logic runs here as reducers; clients are thin renderers.

mod generation;
mod reducers;
mod simulation;
mod tables;

pub use reducers::*;
pub use tables::*;
