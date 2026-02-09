//! Ship, crew, and passenger generation reducers.
//!
//! Graph-first ship layout pipeline:
//!   1. build_ship_graph      -- creates GraphNode + GraphEdge entries
//!   2. layout_ship           -- creates Room, Corridor, Door, VerticalShaft from graph
//!   3. generate_ship_systems -- creates ShipSystem, Subsystem, SystemComponent, InfraEdge
//!   4. generate_atmospheres  -- per-deck atmosphere state
//!   5. generate_crew         -- crew members
//!   6. generate_passengers   -- passengers

use crate::tables::*;
use spacetimedb::{reducer, ReducerContext, Table};

mod doors;
mod facilities;
mod graph;
mod hull;
mod infrastructure;
mod people;
mod systems;
mod treemap;
mod zones;

// Re-export public items needed by reducers.rs and lib.rs
pub use graph::build_ship_graph;
pub use infrastructure::layout_ship;
pub use people::{generate_crew, generate_passengers};
pub use systems::{generate_atmospheres, generate_ship_systems};

/// Initialize a full ship with rooms, crew, passengers, systems, and atmosphere
#[reducer]
pub fn init_ship(
    ctx: &ReducerContext,
    name: String,
    deck_count: u32,
    crew_count: u32,
    passenger_count: u32,
) {
    log::info!(
        "Initializing ship: {} ({} decks, {} crew, {} passengers)",
        name,
        deck_count,
        crew_count,
        passenger_count
    );

    if ctx.db.ship_config().id().find(0).is_some() {
        log::warn!("Ship already initialized!");
        return;
    }

    // Ship config
    ctx.db.ship_config().insert(ShipConfig {
        id: 0,
        name: name.clone(),
        deck_count,
        crew_count,
        passenger_count,
        sim_time: 0.0,
        time_scale: 1.0,
        paused: false,
    });

    // Resources (singleton)
    ctx.db.ship_resources().insert(ShipResources {
        id: 0,
        power: 10000.0,
        water: 50000.0,
        oxygen: 20000.0,
        food: 30000.0,
        fuel: 100000.0,
        spare_parts: 5000.0,
        power_cap: 15000.0,
        water_cap: 60000.0,
        oxygen_cap: 25000.0,
        food_cap: 40000.0,
        fuel_cap: 120000.0,
        spare_parts_cap: 8000.0,
    });

    build_ship_graph(ctx, deck_count);
    layout_ship(ctx, deck_count);
    generate_ship_systems(ctx);
    generate_atmospheres(ctx, deck_count);
    generate_crew(ctx, crew_count);
    generate_passengers(ctx, passenger_count, deck_count);

    log::info!(
        "Ship '{}' initialized with {} people",
        name,
        crew_count + passenger_count
    );
}
