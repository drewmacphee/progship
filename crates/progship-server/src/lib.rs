//! ProgShip Server - SpacetimeDB Module
//!
//! Colony ship simulation running as a SpacetimeDB module.
//! All simulation logic runs here as reducers; clients are thin renderers.

// TODO: Fix these clippy lints incrementally
#![allow(clippy::needless_range_loop)]
#![allow(clippy::type_complexity)]
#![allow(dead_code)]
#![allow(unused_assignments)]

mod generation;
mod reducers;
mod simulation;
mod tables;

pub use reducers::*;
pub use tables::*;
