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

use graph::build_ship_graph;
use infrastructure::layout_ship;
use people::{generate_crew, generate_passengers};
use systems::{generate_atmospheres, generate_ship_systems};

const CORRIDOR_WIDTH: f32 = 6.0;
const CORRIDOR_HALF: f32 = CORRIDOR_WIDTH / 2.0;
const SERVICE_CORRIDOR_WIDTH: f32 = 3.0;
const SERVICE_X: f32 = -(CORRIDOR_HALF + SERVICE_CORRIDOR_WIDTH / 2.0);

/// Descriptor for a graph node to be created during build_ship_graph.
struct NodeSpec {
    name: &'static str,
    function: u8,
    capacity: u32,
    group: u8,
    deck_preference: i32,
}

fn base_area(function: u8) -> f32 {
    match function {
        room_types::BRIDGE | room_types::ENGINEERING | room_types::REACTOR => 200.0,
        room_types::MESS_HALL => 500.0,
        room_types::ARBORETUM => 800.0,
        room_types::THEATRE => 350.0,
        room_types::HYDROPONICS => 1000.0,
        room_types::CARGO_BAY | room_types::SHUTTLE_BAY | room_types::ENGINE_ROOM => 500.0,
        room_types::HOSPITAL_WARD | room_types::QUARANTINE => 200.0,
        room_types::GYM | room_types::POOL => 250.0,
        room_types::GALLEY | room_types::LIBRARY | room_types::OBSERVATION_LOUNGE => 120.0,
        room_types::POWER_DISTRIBUTION | room_types::MACHINE_SHOP => 100.0,
        room_types::ATMOSPHERE_PROCESSING
        | room_types::WATER_RECYCLING
        | room_types::WASTE_PROCESSING
        | room_types::LIFE_SUPPORT => 200.0,
        room_types::CABIN_SINGLE => 14.0,
        room_types::CABIN_DOUBLE | room_types::QUARTERS_OFFICER => 22.0,
        room_types::FAMILY_SUITE | room_types::QUARTERS_PASSENGER => 35.0,
        room_types::VIP_SUITE => 55.0,
        room_types::SHARED_BATHROOM => 9.0,
        room_types::SHARED_LAUNDRY => 18.0,
        room_types::CAFE
        | room_types::BAR
        | room_types::GAME_ROOM
        | room_types::ART_STUDIO
        | room_types::MUSIC_ROOM => 50.0,
        room_types::CONFERENCE | room_types::SECURITY_OFFICE | room_types::ADMIN_OFFICE => 45.0,
        room_types::PHARMACY
        | room_types::CIC
        | room_types::COMMS_ROOM
        | room_types::CAPTAINS_READY_ROOM
        | room_types::DENTAL_CLINIC
        | room_types::MENTAL_HEALTH
        | room_types::MORGUE
        | room_types::MEDBAY => 35.0,
        room_types::SURGERY | room_types::ELECTRONICS_LAB | room_types::ROBOTICS_BAY => 55.0,
        room_types::NURSERY | room_types::SCHOOL | room_types::CHAPEL | room_types::HOLODECK => {
            60.0
        }
        room_types::BAKERY | room_types::BRIG | room_types::AIRLOCK => 40.0,
        room_types::ARMORY | room_types::ENV_MONITORING => 50.0,
        room_types::FUEL_STORAGE | room_types::BACKUP_REACTOR => 250.0,
        room_types::FOOD_STORAGE_COLD
        | room_types::FOOD_STORAGE_DRY
        | room_types::PARTS_STORAGE
        | room_types::STORAGE => 120.0,
        room_types::LABORATORY | room_types::OBSERVATORY => 80.0,
        _ => 40.0,
    }
}

fn compute_room_dims(required_area: f32) -> (f32, f32) {
    // Aspect ratio between 1:1 and 2:1
    let w = required_area.sqrt() * 1.2;
    let h = required_area / w;
    (w.max(4.0), h.max(4.0))
}

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
        death_count: 0,
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
