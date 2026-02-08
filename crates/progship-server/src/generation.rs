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

// Name pools for generation (deterministic, no rand needed)
const GIVEN_NAMES: &[&str] = &[
    "Alex", "Jordan", "Morgan", "Casey", "Riley", "Quinn", "Avery", "Taylor", "Skyler", "Kai",
    "Rowan", "Sage", "River", "Phoenix", "Eden", "Harper", "Blake", "Logan", "Reese", "Cameron",
    "Dakota", "Emery", "Finley", "Hayden", "Jaden", "Kendall", "Lane", "Marley", "Noel", "Parker",
    "Remy", "Shay", "Tatum", "Val", "Wren", "Zion", "Arden", "Bay", "Cedar", "Drew", "Ellis",
    "Flynn", "Grey", "Hollis", "Indigo", "Jules", "Kit", "Lark", "Milan", "Nico", "Oakley",
    "Peyton", "Raven", "Sol", "Teagan", "Uri", "Vesper", "Winter", "Xen", "Yael", "Zephyr", "Ash",
    "Briar", "Cove", "Dune", "Ever", "Fern", "Glen", "Haven", "Ivy", "Jade", "Kestrel", "Linden",
    "Moss", "North", "Onyx", "Pine", "Rain", "Stone", "Thorn",
];

const FAMILY_NAMES: &[&str] = &[
    "Chen",
    "Nakamura",
    "Petrov",
    "Santos",
    "Kim",
    "Hansen",
    "Okafor",
    "Moreau",
    "Singh",
    "Torres",
    "Andersen",
    "Park",
    "Johansson",
    "Fernandez",
    "Larsson",
    "Novak",
    "Ibrahim",
    "Costa",
    "Yamamoto",
    "Kowalski",
    "Bakker",
    "Tanaka",
    "Müller",
    "Svensson",
    "Rossi",
    "Fischer",
    "Jansen",
    "Dubois",
    "Schmidt",
    "Popov",
    "Mendez",
    "Nguyen",
    "Ali",
    "Jensen",
    "Virtanen",
    "Colombo",
    "Takahashi",
    "Olsen",
    "Nieminen",
    "Bianchi",
    "Wagner",
    "Eriksson",
    "Morel",
    "Ivanov",
    "Ortiz",
    "Reyes",
    "Hoffmann",
    "Nilsson",
    "Russo",
    "Delgado",
    "Berger",
    "Wolf",
    "Richter",
    "Stein",
    "Hahn",
    "Krause",
    "Bauer",
    "Maier",
    "Vogt",
    "Sato",
    "Watanabe",
    "Suzuki",
    "Kato",
    "Yoshida",
    "Yamada",
    "Sasaki",
    "Hayashi",
    "Mori",
    "Ikeda",
    "Abe",
    "Ishikawa",
    "Ogawa",
    "Goto",
    "Hasegawa",
];
#[allow(dead_code)]
const CORRIDOR_WIDTH: f32 = 6.0;
#[allow(dead_code)]
const CORRIDOR_HALF: f32 = CORRIDOR_WIDTH / 2.0;
#[allow(dead_code)]
const SERVICE_CORRIDOR_WIDTH: f32 = 3.0;
#[allow(dead_code)]
const SERVICE_X: f32 = -(CORRIDOR_HALF + SERVICE_CORRIDOR_WIDTH / 2.0);

/// Descriptor for a graph node to be created during build_ship_graph.
#[allow(dead_code)]
struct NodeSpec {
    name: &'static str,
    function: u8,
    capacity: u32,
    group: u8,
    deck_preference: i32,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
// ============================================================================
// STEP 1: BUILD SHIP GRAPH
// ============================================================================

/// Facility manifest entry — describes one kind of room to instantiate.
struct FacilitySpec {
    name: &'static str,
    room_type: u8,
    target_area: f32,
    capacity: u32,
    count: u32,
    deck_zone: u8, // 0=command, 1=hab, 2=services, 3=rec, 4=lifesup, 5=cargo, 6=eng
    group: u8,
}

/// Returns true if two room types should have a direct door between them.
/// Most rooms connect to corridors only; direct room-to-room doors are for
/// logically connected pairs (e.g., galley↔mess, surgery↔hospital).
fn should_have_room_door(a: u8, b: u8) -> bool {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    matches!(
        (lo, hi),
        // Galley connects to mess hall
        (room_types::MESS_HALL, room_types::GALLEY) |
        // Surgery connects to hospital ward
        (room_types::HOSPITAL_WARD, room_types::SURGERY) |
        // Food storage connects to galley
        (room_types::GALLEY, room_types::FOOD_STORAGE_COLD) |
        (room_types::GALLEY, room_types::FOOD_STORAGE_DRY) |
        // Pharmacy connects to hospital ward
        (room_types::HOSPITAL_WARD, room_types::PHARMACY) |
        // Bridge connects to CIC / captain's ready room
        (room_types::BRIDGE, room_types::CIC) |
        (room_types::BRIDGE, room_types::CAPTAINS_READY_ROOM) |
        // Main engineering connects to reactor
        (room_types::ENGINEERING, room_types::REACTOR) |
        (room_types::ENGINEERING, room_types::ENGINE_ROOM) |
        // Shared bathroom connects to cabins
        (room_types::CABIN_SINGLE, room_types::SHARED_BATHROOM) |
        (room_types::CABIN_DOUBLE, room_types::SHARED_BATHROOM) |
        (room_types::QUARTERS_CREW, room_types::SHARED_BATHROOM)
    )
}

/// Deck-zone → deck range mapping.
fn deck_range_for_zone(zone: u8, deck_count: u32) -> (u32, u32) {
    let dc = deck_count;
    match zone {
        0 => (0, core::cmp::min(2, dc)),
        1 => (2, core::cmp::min(10, dc)),
        2 => (
            core::cmp::min(10, dc.saturating_sub(1)),
            core::cmp::min(12, dc),
        ),
        3 => (
            core::cmp::min(12, dc.saturating_sub(1)),
            core::cmp::min(14, dc),
        ),
        4 => (
            core::cmp::min(14, dc.saturating_sub(1)),
            core::cmp::min(17, dc),
        ),
        5 => (
            core::cmp::min(17, dc.saturating_sub(1)),
            core::cmp::min(19, dc),
        ),
        6 => (core::cmp::min(19, dc.saturating_sub(1)), dc),
        _ => (0, dc),
    }
}

fn build_ship_graph(ctx: &ReducerContext, _deck_count: u32) {
    let facility_manifest: Vec<FacilitySpec> = vec![
        // === COMMAND (zone 0) ===
        FacilitySpec {
            name: "Bridge",
            room_type: room_types::BRIDGE,
            target_area: 200.0,
            capacity: 10,
            count: 1,
            deck_zone: 0,
            group: groups::COMMAND,
        },
        FacilitySpec {
            name: "CIC",
            room_type: room_types::CIC,
            target_area: 50.0,
            capacity: 8,
            count: 1,
            deck_zone: 0,
            group: groups::COMMAND,
        },
        FacilitySpec {
            name: "Conference Room",
            room_type: room_types::CONFERENCE,
            target_area: 45.0,
            capacity: 12,
            count: 4,
            deck_zone: 0,
            group: groups::COMMAND,
        },
        FacilitySpec {
            name: "Captain's Ready Room",
            room_type: room_types::CAPTAINS_READY_ROOM,
            target_area: 35.0,
            capacity: 2,
            count: 1,
            deck_zone: 0,
            group: groups::COMMAND,
        },
        FacilitySpec {
            name: "Observatory",
            room_type: room_types::OBSERVATORY,
            target_area: 80.0,
            capacity: 6,
            count: 1,
            deck_zone: 0,
            group: groups::COMMAND,
        },
        FacilitySpec {
            name: "Comms Room",
            room_type: room_types::COMMS_ROOM,
            target_area: 35.0,
            capacity: 4,
            count: 1,
            deck_zone: 0,
            group: groups::COMMAND,
        },
        FacilitySpec {
            name: "Security Office",
            room_type: room_types::SECURITY_OFFICE,
            target_area: 45.0,
            capacity: 6,
            count: 2,
            deck_zone: 0,
            group: groups::COMMAND,
        },
        FacilitySpec {
            name: "Admin Office",
            room_type: room_types::ADMIN_OFFICE,
            target_area: 45.0,
            capacity: 8,
            count: 2,
            deck_zone: 0,
            group: groups::COMMAND,
        },
        FacilitySpec {
            name: "Brig",
            room_type: room_types::BRIG,
            target_area: 40.0,
            capacity: 4,
            count: 1,
            deck_zone: 0,
            group: groups::COMMAND,
        },
        FacilitySpec {
            name: "Officer Quarters",
            room_type: room_types::QUARTERS_OFFICER,
            target_area: 22.0,
            capacity: 1,
            count: 20,
            deck_zone: 0,
            group: groups::COMMAND,
        },
        // === HABITATION (zone 1) ===
        FacilitySpec {
            name: "Single Cabin",
            room_type: room_types::CABIN_SINGLE,
            target_area: 14.0,
            capacity: 1,
            count: 200,
            deck_zone: 1,
            group: groups::PASSENGER,
        },
        FacilitySpec {
            name: "Double Cabin",
            room_type: room_types::CABIN_DOUBLE,
            target_area: 22.0,
            capacity: 2,
            count: 50,
            deck_zone: 1,
            group: groups::PASSENGER,
        },
        FacilitySpec {
            name: "Family Suite",
            room_type: room_types::FAMILY_SUITE,
            target_area: 35.0,
            capacity: 4,
            count: 30,
            deck_zone: 1,
            group: groups::PASSENGER,
        },
        FacilitySpec {
            name: "Shared Bathroom",
            room_type: room_types::SHARED_BATHROOM,
            target_area: 9.0,
            capacity: 4,
            count: 50,
            deck_zone: 1,
            group: groups::PASSENGER,
        },
        FacilitySpec {
            name: "Shared Laundry",
            room_type: room_types::SHARED_LAUNDRY,
            target_area: 18.0,
            capacity: 3,
            count: 10,
            deck_zone: 1,
            group: groups::PASSENGER,
        },
        FacilitySpec {
            name: "Cafe",
            room_type: room_types::CAFE,
            target_area: 50.0,
            capacity: 20,
            count: 8,
            deck_zone: 1,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Crew Quarters",
            room_type: room_types::QUARTERS_CREW,
            target_area: 14.0,
            capacity: 1,
            count: 60,
            deck_zone: 1,
            group: groups::CREW,
        },
        FacilitySpec {
            name: "Crew Lounge",
            room_type: room_types::LOUNGE,
            target_area: 50.0,
            capacity: 20,
            count: 4,
            deck_zone: 1,
            group: groups::CREW,
        },
        FacilitySpec {
            name: "VIP Suite",
            room_type: room_types::VIP_SUITE,
            target_area: 55.0,
            capacity: 2,
            count: 6,
            deck_zone: 1,
            group: groups::PASSENGER,
        },
        // === SERVICES / MEDICAL / FOOD (zone 2) ===
        FacilitySpec {
            name: "Mess Hall",
            room_type: room_types::MESS_HALL,
            target_area: 500.0,
            capacity: 200,
            count: 4,
            deck_zone: 2,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Galley",
            room_type: room_types::GALLEY,
            target_area: 120.0,
            capacity: 6,
            count: 4,
            deck_zone: 2,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Hospital Ward",
            room_type: room_types::HOSPITAL_WARD,
            target_area: 250.0,
            capacity: 20,
            count: 1,
            deck_zone: 2,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Surgery",
            room_type: room_types::SURGERY,
            target_area: 55.0,
            capacity: 4,
            count: 2,
            deck_zone: 2,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Dental Clinic",
            room_type: room_types::DENTAL_CLINIC,
            target_area: 35.0,
            capacity: 4,
            count: 1,
            deck_zone: 2,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Pharmacy",
            room_type: room_types::PHARMACY,
            target_area: 35.0,
            capacity: 3,
            count: 2,
            deck_zone: 2,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Mental Health",
            room_type: room_types::MENTAL_HEALTH,
            target_area: 35.0,
            capacity: 4,
            count: 2,
            deck_zone: 2,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Quarantine",
            room_type: room_types::QUARANTINE,
            target_area: 200.0,
            capacity: 10,
            count: 1,
            deck_zone: 2,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Morgue",
            room_type: room_types::MORGUE,
            target_area: 35.0,
            capacity: 2,
            count: 1,
            deck_zone: 2,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Food Storage Cold",
            room_type: room_types::FOOD_STORAGE_COLD,
            target_area: 120.0,
            capacity: 4,
            count: 2,
            deck_zone: 2,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Food Storage Dry",
            room_type: room_types::FOOD_STORAGE_DRY,
            target_area: 120.0,
            capacity: 4,
            count: 2,
            deck_zone: 2,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Bakery",
            room_type: room_types::BAKERY,
            target_area: 40.0,
            capacity: 4,
            count: 2,
            deck_zone: 2,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Water Purification",
            room_type: room_types::WATER_PURIFICATION,
            target_area: 80.0,
            capacity: 3,
            count: 1,
            deck_zone: 2,
            group: groups::LIFE_SUPPORT,
        },
        FacilitySpec {
            name: "Wardroom",
            room_type: room_types::WARDROOM,
            target_area: 60.0,
            capacity: 20,
            count: 2,
            deck_zone: 2,
            group: groups::COMMAND,
        },
        // === RECREATION / EDUCATION (zone 3) ===
        FacilitySpec {
            name: "Gym",
            room_type: room_types::GYM,
            target_area: 250.0,
            capacity: 30,
            count: 4,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Theatre",
            room_type: room_types::THEATRE,
            target_area: 350.0,
            capacity: 200,
            count: 1,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Library",
            room_type: room_types::LIBRARY,
            target_area: 120.0,
            capacity: 30,
            count: 2,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Bar",
            room_type: room_types::BAR,
            target_area: 50.0,
            capacity: 25,
            count: 4,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Chapel",
            room_type: room_types::CHAPEL,
            target_area: 60.0,
            capacity: 15,
            count: 2,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Game Room",
            room_type: room_types::GAME_ROOM,
            target_area: 50.0,
            capacity: 15,
            count: 3,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Art Studio",
            room_type: room_types::ART_STUDIO,
            target_area: 50.0,
            capacity: 10,
            count: 1,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Music Room",
            room_type: room_types::MUSIC_ROOM,
            target_area: 50.0,
            capacity: 8,
            count: 2,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Pool",
            room_type: room_types::POOL,
            target_area: 250.0,
            capacity: 30,
            count: 1,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Arboretum",
            room_type: room_types::ARBORETUM,
            target_area: 800.0,
            capacity: 50,
            count: 1,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Observation Lounge",
            room_type: room_types::OBSERVATION_LOUNGE,
            target_area: 120.0,
            capacity: 20,
            count: 2,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Nursery",
            room_type: room_types::NURSERY,
            target_area: 60.0,
            capacity: 15,
            count: 2,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "School",
            room_type: room_types::SCHOOL,
            target_area: 60.0,
            capacity: 25,
            count: 4,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Shops",
            room_type: room_types::SHOPS,
            target_area: 50.0,
            capacity: 15,
            count: 4,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Recreation Center",
            room_type: room_types::RECREATION,
            target_area: 120.0,
            capacity: 40,
            count: 2,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        FacilitySpec {
            name: "Holodeck",
            room_type: room_types::HOLODECK,
            target_area: 60.0,
            capacity: 6,
            count: 2,
            deck_zone: 3,
            group: groups::COMMONS,
        },
        // === LIFE SUPPORT (zone 4) ===
        FacilitySpec {
            name: "Hydroponics Bay",
            room_type: room_types::HYDROPONICS,
            target_area: 1000.0,
            capacity: 8,
            count: 4,
            deck_zone: 4,
            group: groups::LIFE_SUPPORT,
        },
        FacilitySpec {
            name: "Atmosphere Processing",
            room_type: room_types::ATMOSPHERE_PROCESSING,
            target_area: 200.0,
            capacity: 4,
            count: 2,
            deck_zone: 4,
            group: groups::LIFE_SUPPORT,
        },
        FacilitySpec {
            name: "Water Recycling",
            room_type: room_types::WATER_RECYCLING,
            target_area: 200.0,
            capacity: 6,
            count: 2,
            deck_zone: 4,
            group: groups::LIFE_SUPPORT,
        },
        FacilitySpec {
            name: "Waste Processing",
            room_type: room_types::WASTE_PROCESSING,
            target_area: 200.0,
            capacity: 4,
            count: 2,
            deck_zone: 4,
            group: groups::LIFE_SUPPORT,
        },
        FacilitySpec {
            name: "Env Monitoring",
            room_type: room_types::ENV_MONITORING,
            target_area: 50.0,
            capacity: 4,
            count: 2,
            deck_zone: 4,
            group: groups::LIFE_SUPPORT,
        },
        FacilitySpec {
            name: "Life Support Center",
            room_type: room_types::LIFE_SUPPORT,
            target_area: 200.0,
            capacity: 8,
            count: 1,
            deck_zone: 4,
            group: groups::LIFE_SUPPORT,
        },
        FacilitySpec {
            name: "HVAC Control",
            room_type: room_types::HVAC_CONTROL,
            target_area: 120.0,
            capacity: 4,
            count: 2,
            deck_zone: 4,
            group: groups::LIFE_SUPPORT,
        },
        FacilitySpec {
            name: "Laboratory",
            room_type: room_types::LABORATORY,
            target_area: 80.0,
            capacity: 10,
            count: 4,
            deck_zone: 4,
            group: groups::LIFE_SUPPORT,
        },
        // === CARGO (zone 5) ===
        FacilitySpec {
            name: "Cargo Bay",
            room_type: room_types::CARGO_BAY,
            target_area: 500.0,
            capacity: 10,
            count: 2,
            deck_zone: 5,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Storage",
            room_type: room_types::STORAGE,
            target_area: 120.0,
            capacity: 4,
            count: 4,
            deck_zone: 5,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Armory",
            room_type: room_types::ARMORY,
            target_area: 50.0,
            capacity: 4,
            count: 1,
            deck_zone: 5,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Shuttle Bay",
            room_type: room_types::SHUTTLE_BAY,
            target_area: 500.0,
            capacity: 10,
            count: 1,
            deck_zone: 5,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Airlock",
            room_type: room_types::AIRLOCK,
            target_area: 40.0,
            capacity: 4,
            count: 4,
            deck_zone: 5,
            group: groups::ENGINEERING,
        },
        // === ENGINEERING (zone 6) ===
        FacilitySpec {
            name: "Reactor",
            room_type: room_types::REACTOR,
            target_area: 500.0,
            capacity: 5,
            count: 1,
            deck_zone: 6,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Backup Reactor",
            room_type: room_types::BACKUP_REACTOR,
            target_area: 250.0,
            capacity: 4,
            count: 1,
            deck_zone: 6,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Engine Room",
            room_type: room_types::ENGINE_ROOM,
            target_area: 500.0,
            capacity: 15,
            count: 1,
            deck_zone: 6,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Main Engineering",
            room_type: room_types::ENGINEERING,
            target_area: 200.0,
            capacity: 15,
            count: 1,
            deck_zone: 6,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Power Distribution",
            room_type: room_types::POWER_DISTRIBUTION,
            target_area: 100.0,
            capacity: 4,
            count: 2,
            deck_zone: 6,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Cooling Plant",
            room_type: room_types::COOLING_PLANT,
            target_area: 120.0,
            capacity: 6,
            count: 2,
            deck_zone: 6,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Maintenance Bay",
            room_type: room_types::MAINTENANCE_BAY,
            target_area: 120.0,
            capacity: 10,
            count: 2,
            deck_zone: 6,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Machine Shop",
            room_type: room_types::MACHINE_SHOP,
            target_area: 100.0,
            capacity: 6,
            count: 1,
            deck_zone: 6,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Electronics Lab",
            room_type: room_types::ELECTRONICS_LAB,
            target_area: 55.0,
            capacity: 6,
            count: 1,
            deck_zone: 6,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Parts Storage",
            room_type: room_types::PARTS_STORAGE,
            target_area: 120.0,
            capacity: 5,
            count: 2,
            deck_zone: 6,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Fuel Storage",
            room_type: room_types::FUEL_STORAGE,
            target_area: 250.0,
            capacity: 4,
            count: 1,
            deck_zone: 6,
            group: groups::ENGINEERING,
        },
        FacilitySpec {
            name: "Robotics Bay",
            room_type: room_types::ROBOTICS_BAY,
            target_area: 55.0,
            capacity: 4,
            count: 1,
            deck_zone: 6,
            group: groups::ENGINEERING,
        },
    ];

    // Expand manifest: one GraphNode per individual room instance
    let mut node_ids: Vec<u64> = Vec::new();
    let mut node_groups: Vec<u8> = Vec::new();
    let mut node_functions: Vec<u8> = Vec::new();
    let mut node_zones: Vec<u8> = Vec::new();

    for spec in &facility_manifest {
        for i in 0..spec.count {
            let name = if spec.count == 1 {
                spec.name.to_string()
            } else {
                format!("{} {}", spec.name, i + 1)
            };
            let area = spec.target_area;
            let node = ctx.db.graph_node().insert(GraphNode {
                id: 0,
                node_type: node_types::ROOM,
                name,
                function: spec.room_type,
                capacity: spec.capacity,
                required_area: area,
                deck_preference: spec.deck_zone as i32,
                group: spec.group,
            });
            node_ids.push(node.id);
            node_groups.push(spec.group);
            node_functions.push(spec.room_type);
            node_zones.push(spec.deck_zone);
        }
    }

    // Intra-zone crew_path edges (connect rooms in same zone, sample to keep edge count manageable)
    for zone in 0..7u8 {
        let zone_ids: Vec<u64> = node_ids
            .iter()
            .zip(node_zones.iter())
            .filter(|(_, z)| **z == zone)
            .map(|(id, _)| *id)
            .collect();
        // Fully connect small groups; for large groups connect each to a few neighbors
        let threshold = 30;
        if zone_ids.len() <= threshold {
            for i in 0..zone_ids.len() {
                for j in (i + 1)..zone_ids.len() {
                    ctx.db.graph_edge().insert(GraphEdge {
                        id: 0,
                        from_node: zone_ids[i],
                        to_node: zone_ids[j],
                        edge_type: edge_types::CREW_PATH,
                        weight: 1.0,
                        bidirectional: true,
                    });
                }
            }
        } else {
            // Ring + short-range links
            for i in 0..zone_ids.len() {
                let next = (i + 1) % zone_ids.len();
                ctx.db.graph_edge().insert(GraphEdge {
                    id: 0,
                    from_node: zone_ids[i],
                    to_node: zone_ids[next],
                    edge_type: edge_types::CREW_PATH,
                    weight: 1.0,
                    bidirectional: true,
                });
                // Skip-3 link for connectivity
                let skip = (i + 3) % zone_ids.len();
                if skip != next && skip != i {
                    ctx.db.graph_edge().insert(GraphEdge {
                        id: 0,
                        from_node: zone_ids[i],
                        to_node: zone_ids[skip],
                        edge_type: edge_types::CREW_PATH,
                        weight: 1.0,
                        bidirectional: true,
                    });
                }
            }
        }
    }

    // Cross-zone crew paths: connect adjacent zones
    for z in 0..6u8 {
        let z_ids: Vec<u64> = node_ids
            .iter()
            .zip(node_zones.iter())
            .filter(|(_, zz)| **zz == z)
            .map(|(id, _)| *id)
            .collect();
        let z1_ids: Vec<u64> = node_ids
            .iter()
            .zip(node_zones.iter())
            .filter(|(_, zz)| **zz == z + 1)
            .map(|(id, _)| *id)
            .collect();
        if let (Some(&a), Some(&b)) = (z_ids.first(), z1_ids.first()) {
            ctx.db.graph_edge().insert(GraphEdge {
                id: 0,
                from_node: a,
                to_node: b,
                edge_type: edge_types::CREW_PATH,
                weight: 2.0,
                bidirectional: true,
            });
        }
        if let (Some(&a), Some(&b)) = (z_ids.last(), z1_ids.last()) {
            ctx.db.graph_edge().insert(GraphEdge {
                id: 0,
                from_node: a,
                to_node: b,
                edge_type: edge_types::CREW_PATH,
                weight: 2.0,
                bidirectional: true,
            });
        }
    }

    // Infrastructure edges
    let find_by_func = |func: u8| -> Option<u64> {
        node_ids
            .iter()
            .zip(node_functions.iter())
            .find(|(_, f)| **f == func)
            .map(|(id, _)| *id)
    };

    let reactor_node = find_by_func(room_types::REACTOR);
    let eng_node = find_by_func(room_types::ENGINEERING);
    let water_node = find_by_func(room_types::WATER_RECYCLING);
    let hvac_node = find_by_func(room_types::HVAC_CONTROL);
    let comms_node = find_by_func(room_types::COMMS_ROOM);
    let bridge_node = find_by_func(room_types::BRIDGE);
    let cic_node = find_by_func(room_types::CIC);

    // POWER: Reactor -> Engineering -> every other room
    if let (Some(reactor), Some(eng)) = (reactor_node, eng_node) {
        ctx.db.graph_edge().insert(GraphEdge {
            id: 0,
            from_node: reactor,
            to_node: eng,
            edge_type: edge_types::POWER,
            weight: 100.0,
            bidirectional: false,
        });
        for &nid in &node_ids {
            if nid != reactor && nid != eng {
                ctx.db.graph_edge().insert(GraphEdge {
                    id: 0,
                    from_node: eng,
                    to_node: nid,
                    edge_type: edge_types::POWER,
                    weight: 10.0,
                    bidirectional: false,
                });
            }
        }
    }

    // WATER: Water Recycling -> habitable rooms (sample to keep edge count sane)
    if let Some(water) = water_node {
        for &nid in &node_ids {
            if nid != water {
                let func = node_functions[node_ids.iter().position(|&x| x == nid).unwrap_or(0)];
                if room_types::is_quarters(func)
                    || room_types::is_dining(func)
                    || func == room_types::HYDROPONICS
                    || func == room_types::HOSPITAL_WARD
                {
                    ctx.db.graph_edge().insert(GraphEdge {
                        id: 0,
                        from_node: water,
                        to_node: nid,
                        edge_type: edge_types::WATER,
                        weight: 5.0,
                        bidirectional: false,
                    });
                }
            }
        }
    }

    // HVAC: HVAC Control -> all rooms (sample: only first 200 to keep manageable)
    if let Some(hvac) = hvac_node {
        let mut hvac_count = 0u32;
        for &nid in &node_ids {
            if nid != hvac && hvac_count < 200 {
                ctx.db.graph_edge().insert(GraphEdge {
                    id: 0,
                    from_node: hvac,
                    to_node: nid,
                    edge_type: edge_types::HVAC,
                    weight: 1.0,
                    bidirectional: false,
                });
                hvac_count += 1;
            }
        }
    }

    // DATA: Comms -> Bridge, CIC, Engineering
    if let Some(comms) = comms_node {
        let data_targets: Vec<u64> = [bridge_node, cic_node, eng_node]
            .iter()
            .filter_map(|n| *n)
            .collect();
        for &t in &data_targets {
            ctx.db.graph_edge().insert(GraphEdge {
                id: 0,
                from_node: comms,
                to_node: t,
                edge_type: edge_types::DATA,
                weight: 1.0,
                bidirectional: false,
            });
        }
    }
}
// ============================================================================
// STEP 2: LAYOUT SHIP — Infrastructure-First Treemap Fill
// ============================================================================

// Grid cell values
const CELL_EMPTY: u8 = 0;
const CELL_MAIN_CORRIDOR: u8 = 1;
const CELL_SERVICE_CORRIDOR: u8 = 2;
const CELL_SHAFT: u8 = 3;
const CELL_ROOM_BASE: u8 = 10; // room N = CELL_ROOM_BASE + N (wraps at 246)

// Ship geometry constants
const SHIP_LENGTH: usize = 400;
const SHIP_BEAM: usize = 65;
const SPINE_WIDTH: usize = 3;
const CROSS_CORRIDOR_WIDTH: usize = 3;
const CROSS_CORRIDOR_SPACING: usize = 50;
const SVC_CORRIDOR_WIDTH: usize = 2;

/// Seeded LCG random number generator (no external crate).
struct SimpleRng {
    state: u64,
}
impl SimpleRng {
    fn from_name(name: &str) -> Self {
        let mut hash: u64 = 5381;
        for b in name.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(b as u64);
        }
        Self { state: hash }
    }
    fn next_f32(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.state >> 33) as f32) / (u32::MAX as f32)
    }
    #[allow(dead_code)]
    fn next_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
    #[allow(dead_code)]
    fn next_usize(&mut self, min: usize, max: usize) -> usize {
        if max <= min {
            return min;
        }
        let f = self.next_f32();
        let range = max - min;
        min + (f * range as f32) as usize
    }
}

/// Room request for treemap placement.
#[derive(Clone)]
struct RoomRequest {
    node_id: u64,
    name: String,
    room_type: u8,
    target_area: f32,
    capacity: u32,
    group: u8,
}

/// Placed room result from treemap.
#[allow(dead_code)]
struct PlacedRoom {
    room_id: u32,
    node_id: u64,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    room_type: u8,
}

/// Rectangular zone on the grid where rooms can be placed.
struct GridZone {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

/// Squarified treemap: packs weighted rectangles into a zone.
/// Returns (original_index, x, y, w, h) for each room.
fn squarified_treemap(
    rooms: &[(f32, usize)], // (area_weight, original_index)
    zone_x: usize,
    zone_y: usize,
    zone_w: usize,
    zone_h: usize,
) -> Vec<(usize, usize, usize, usize, usize)> {
    if rooms.is_empty() || zone_w == 0 || zone_h == 0 {
        return Vec::new();
    }
    if rooms.len() == 1 {
        return vec![(rooms[0].1, zone_x, zone_y, zone_w, zone_h)];
    }

    let total_weight: f32 = rooms.iter().map(|(w, _)| *w).sum();
    let zone_area = (zone_w * zone_h) as f32;
    if total_weight <= 0.0 || zone_area <= 0.0 {
        return Vec::new();
    }

    // Normalize weights to sum to zone_area
    let scale = zone_area / total_weight;
    let normalized: Vec<(f32, usize)> = rooms.iter().map(|(w, idx)| (w * scale, *idx)).collect();

    let mut result = Vec::new();
    let mut remaining = &normalized[..];
    let mut cx = zone_x;
    let mut cy = zone_y;
    let mut cw = zone_w;
    let mut ch = zone_h;

    while !remaining.is_empty() && cw > 0 && ch > 0 {
        // Lay out along the shorter dimension
        let layout_vertical = cw <= ch; // strip runs along y if vertical, along x if horizontal
        let strip_len = if layout_vertical { ch } else { cw };
        let strip_breadth = if layout_vertical { cw } else { ch };

        // Greedily add rooms to the current strip, maximizing worst aspect ratio
        let _remaining_area: f32 = remaining.iter().map(|(a, _)| *a).sum();
        let mut best_count = 1;
        let mut best_worst_ratio = f32::MAX;

        for count in 1..=remaining.len() {
            let strip_area: f32 = remaining[..count].iter().map(|(a, _)| *a).sum();
            let strip_thickness = (strip_area / strip_len as f32).ceil() as usize;
            let strip_thickness = strip_thickness.max(1).min(strip_breadth);

            // Compute aspect ratios for rooms in this strip
            let mut worst_ratio: f32 = 0.0;
            let mut _pos = 0.0_f32;
            for (area, _) in &remaining[..count] {
                let room_len = if strip_thickness > 0 {
                    *area / strip_thickness as f32
                } else {
                    *area
                };
                let room_len = room_len.max(1.0);
                let r = if room_len > strip_thickness as f32 {
                    room_len / strip_thickness as f32
                } else {
                    strip_thickness as f32 / room_len
                };
                if r > worst_ratio {
                    worst_ratio = r;
                }
                _pos += room_len;
            }

            if count == 1 || worst_ratio <= best_worst_ratio {
                best_worst_ratio = worst_ratio;
                best_count = count;
            } else {
                break; // Adding more rooms makes aspect ratio worse
            }
        }

        // Lay out best_count rooms in the strip
        let strip_rooms = &remaining[..best_count];
        let strip_area: f32 = strip_rooms.iter().map(|(a, _)| *a).sum();
        let strip_thickness = if strip_len > 0 {
            (strip_area / strip_len as f32).ceil() as usize
        } else {
            1
        };
        let strip_thickness = strip_thickness.max(1).min(strip_breadth);

        let mut pos = 0usize;
        for (i, (area, idx)) in strip_rooms.iter().enumerate() {
            let room_len = if i == best_count - 1 {
                // Last room takes remaining space
                strip_len.saturating_sub(pos)
            } else if strip_thickness > 0 {
                (*area / strip_thickness as f32).round() as usize
            } else {
                1
            };
            let room_len = room_len.max(1).min(strip_len.saturating_sub(pos));

            if room_len == 0 {
                continue;
            }

            let (rx, ry, rw, rh) = if layout_vertical {
                (cx, cy + pos, strip_thickness, room_len)
            } else {
                (cx + pos, cy, room_len, strip_thickness)
            };

            if rw > 0 && rh > 0 {
                result.push((*idx, rx, ry, rw, rh));
            }
            pos += room_len;
        }

        // Advance past this strip
        if layout_vertical {
            cx += strip_thickness;
            cw = cw.saturating_sub(strip_thickness);
        } else {
            cy += strip_thickness;
            ch = ch.saturating_sub(strip_thickness);
        }

        remaining = &remaining[best_count..];
    }

    result
}

/// Scan the grid for contiguous rectangular zones of CELL_EMPTY.
fn find_empty_zones(grid: &[Vec<u8>], width: usize, height: usize) -> Vec<GridZone> {
    // Simple row-run based approach: scan rows, find horizontal runs of empty,
    // then merge vertically adjacent runs with matching x-ranges.
    let mut zones: Vec<GridZone> = Vec::new();

    // Track which cells are already claimed by a zone
    let mut claimed = vec![vec![false; height]; width];

    for x in 0..width {
        for y in 0..height {
            if grid[x][y] != CELL_EMPTY || claimed[x][y] {
                continue;
            }

            // Find the widest run starting at (x, y)
            let mut run_w = 0;
            while x + run_w < width && grid[x + run_w][y] == CELL_EMPTY && !claimed[x + run_w][y] {
                run_w += 1;
            }
            if run_w < 3 {
                continue;
            } // too narrow for a room

            // Extend downward while the same x-range is all empty
            let mut run_h = 1;
            'outer: while y + run_h < height {
                for xx in x..(x + run_w) {
                    if grid[xx][y + run_h] != CELL_EMPTY || claimed[xx][y + run_h] {
                        break 'outer;
                    }
                }
                run_h += 1;
            }

            if run_h < 3 {
                continue;
            } // too short

            // Claim these cells
            for xx in x..(x + run_w) {
                for yy in y..(y + run_h) {
                    claimed[xx][yy] = true;
                }
            }

            zones.push(GridZone {
                x,
                y,
                w: run_w,
                h: run_h,
            });
        }
    }

    // Sort largest-first
    zones.sort_by(|a, b| (b.w * b.h).cmp(&(a.w * a.h)));
    zones
}

fn layout_ship(ctx: &ReducerContext, deck_count: u32) {
    let ship_name = ctx
        .db
        .ship_config()
        .id()
        .find(0)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let _rng = SimpleRng::from_name(&ship_name);
    let nodes: Vec<GraphNode> = ctx.db.graph_node().iter().collect();

    // Build per-deck-zone room request lists from graph nodes
    let mut zone_requests: Vec<Vec<RoomRequest>> = vec![Vec::new(); 7];
    for node in &nodes {
        let zone = (node.deck_preference as u8).min(6);
        zone_requests[zone as usize].push(RoomRequest {
            node_id: node.id,
            name: node.name.clone(),
            room_type: node.function,
            target_area: node.required_area,
            capacity: node.capacity,
            group: node.group,
        });
    }
    // Sort each zone's requests: largest rooms first for better treemap packing
    for zr in zone_requests.iter_mut() {
        zr.sort_by(|a, b| {
            b.target_area
                .partial_cmp(&a.target_area)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
    }

    let mut room_id_counter: u32 = 0;
    let mut next_id = || {
        let id = room_id_counter;
        room_id_counter += 1;
        id
    };

    // Per-deck shaft positions are computed inside the deck loop below

    /// Spine segment info for a deck: (room_id, y_start, y_end)
    struct SpineSegment {
        room_id: u32,
        y_start: usize,
        y_end: usize,
    }

    /// Cross-corridor Room info: (room_id, y_start)
    struct CrossCorridorRoom {
        room_id: u32,
        y_start: usize,
    }

    for deck in 0..deck_count as i32 {
        // Hull taper per deck
        let hull_width: usize = match deck as u32 {
            0..=1 => 40,
            d if d >= deck_count.saturating_sub(2) => 50,
            _ => SHIP_BEAM,
        };
        let hull_length: usize = match deck as u32 {
            0..=1 => 200,
            d if d >= deck_count.saturating_sub(2) => 300,
            _ => SHIP_LENGTH,
        };

        // Shaft positions relative to THIS deck's hull
        let deck_spine_cx = hull_width / 2;
        // Place elevators adjacent to spine (just outside it on starboard side)
        let fore_elev_deck = (deck_spine_cx + 2, 10usize, 3usize, 3usize);
        let aft_elev_deck = (
            deck_spine_cx + 2,
            if hull_length > 20 {
                hull_length - 14
            } else {
                hull_length / 2
            },
            3,
            3,
        );
        let svc_elev_deck = (
            hull_width.saturating_sub(5),
            100usize.min(hull_length.saturating_sub(5)),
            2,
            2,
        );
        let ladders_deck: Vec<(usize, usize, usize, usize)> = [50, 150, 250, 350]
            .iter()
            .filter(|&&ly| ly + 2 <= hull_length)
            .map(|&ly| (hull_width.saturating_sub(4), ly, 2, 2))
            .collect();

        // Allocate grid: grid[x][y], size [hull_width][hull_length]
        let mut grid: Vec<Vec<u8>> = vec![vec![CELL_EMPTY; hull_length]; hull_width];

        // ---- Step 1: Stamp corridor skeleton ----

        // Main spine: SPINE_WIDTH cells wide, centered, full length
        let spine_left = hull_width / 2 - SPINE_WIDTH / 2;
        let spine_right = spine_left + SPINE_WIDTH;
        for x in spine_left..spine_right.min(hull_width) {
            for y in 0..hull_length {
                grid[x][y] = CELL_MAIN_CORRIDOR;
            }
        }

        // Compute service corridor boundary early (needed by cross-corridors)
        let svc_left = hull_width.saturating_sub(SVC_CORRIDOR_WIDTH);

        // Cross-corridors: CROSS_CORRIDOR_WIDTH cells wide, horizontal, every CROSS_CORRIDOR_SPACING
        // Only span from x=0 to svc_left (stop before service corridor)
        let mut cross_corridor_ys: Vec<usize> = Vec::new();
        let mut cy = CROSS_CORRIDOR_SPACING;
        while cy + CROSS_CORRIDOR_WIDTH <= hull_length {
            for x in 0..svc_left {
                for dy in 0..CROSS_CORRIDOR_WIDTH {
                    let yy = cy + dy;
                    if yy < hull_length {
                        // Don't overwrite shaft cells (will be stamped later, but we
                        // pre-check to keep cross-corridor Room bounds accurate)
                        grid[x][yy] = CELL_MAIN_CORRIDOR;
                    }
                }
            }
            cross_corridor_ys.push(cy);
            cy += CROSS_CORRIDOR_SPACING;
        }

        // FIX 1: Create SEGMENTED spine Room entries (one per section between cross-corridors)
        // Boundaries are: 0, cross1_start, cross1_end, cross2_start, ..., hull_length
        let mut spine_segments: Vec<SpineSegment> = Vec::new();
        {
            let mut seg_start = 0usize;
            for &ccy in &cross_corridor_ys {
                // Spine segment from seg_start to ccy (just before cross-corridor)
                if ccy > seg_start {
                    let seg_len = ccy - seg_start;
                    let sid = next_id();
                    let seg_cy = seg_start as f32 + seg_len as f32 / 2.0;
                    ctx.db.room().insert(Room {
                        id: sid,
                        node_id: 0,
                        name: format!("Deck {} Spine Seg {}", deck + 1, spine_segments.len() + 1),
                        room_type: room_types::CORRIDOR,
                        deck,
                        x: (spine_left + spine_right) as f32 / 2.0,
                        y: seg_cy,
                        width: SPINE_WIDTH as f32,
                        height: seg_len as f32,
                        capacity: 50,
                    });
                    spine_segments.push(SpineSegment {
                        room_id: sid,
                        y_start: seg_start,
                        y_end: ccy,
                    });
                }
                // Skip past the cross-corridor band (seg_start advances after it)
                seg_start = ccy + CROSS_CORRIDOR_WIDTH;
            }
            // Final segment after last cross-corridor to hull end
            if seg_start < hull_length {
                let seg_len = hull_length - seg_start;
                let sid = next_id();
                let seg_cy = seg_start as f32 + seg_len as f32 / 2.0;
                ctx.db.room().insert(Room {
                    id: sid,
                    node_id: 0,
                    name: format!("Deck {} Spine Seg {}", deck + 1, spine_segments.len() + 1),
                    room_type: room_types::CORRIDOR,
                    deck,
                    x: (spine_left + spine_right) as f32 / 2.0,
                    y: seg_cy,
                    width: SPINE_WIDTH as f32,
                    height: seg_len as f32,
                    capacity: 50,
                });
                spine_segments.push(SpineSegment {
                    room_id: sid,
                    y_start: seg_start,
                    y_end: hull_length,
                });
            }
        }

        // Corridor table entry for full spine (rendering uses Corridor table)
        ctx.db.corridor().insert(Corridor {
            id: 0,
            deck,
            corridor_type: corridor_types::MAIN,
            x: (spine_left + spine_right) as f32 / 2.0,
            y: hull_length as f32 / 2.0,
            width: SPINE_WIDTH as f32,
            length: hull_length as f32,
            orientation: 1,
            carries: carries_flags::CREW_PATH | carries_flags::POWER | carries_flags::DATA,
        });

        // Doors connecting adjacent spine segments (through cross-corridors)
        // Each spine segment's SOUTH wall connects to the next segment's NORTH wall.
        // The door is at the spine's center X and at the boundary Y between segments.
        let spine_center_x = (spine_left + spine_right) as f32 / 2.0;
        for _i in 0..spine_segments.len().saturating_sub(1) {
            // Spine segments connect through cross-corridor rooms, not directly.
            // seg_a SOUTH → cross-corridor NORTH, cross-corridor SOUTH → seg_b NORTH
        }

        // FIX 2: Create Room entries for each cross-corridor
        // Width limited to svc_left (does not extend into service corridor zone)
        // Shafts may sit inside the cross-corridor — that overlap is tolerated
        let mut cross_rooms: Vec<CrossCorridorRoom> = Vec::new();
        for (ci, &ccy) in cross_corridor_ys.iter().enumerate() {
            let cross_cy_f = ccy as f32 + CROSS_CORRIDOR_WIDTH as f32 / 2.0;
            let crid = next_id();
            let cross_width = svc_left as f32;
            ctx.db.room().insert(Room {
                id: crid,
                node_id: 0,
                name: format!("Deck {} Cross-Corridor {}", deck + 1, ci + 1),
                room_type: room_types::CROSS_CORRIDOR,
                deck,
                x: cross_width / 2.0,
                y: cross_cy_f,
                width: cross_width,
                height: CROSS_CORRIDOR_WIDTH as f32,
                capacity: 20,
            });
            ctx.db.corridor().insert(Corridor {
                id: 0,
                deck,
                corridor_type: corridor_types::BRANCH,
                x: cross_width / 2.0,
                y: cross_cy_f,
                width: cross_width,
                length: CROSS_CORRIDOR_WIDTH as f32,
                orientation: 0,
                carries: carries_flags::CREW_PATH,
            });
            cross_rooms.push(CrossCorridorRoom {
                room_id: crid,
                y_start: ccy,
            });

            // Door from cross-corridor to adjacent spine segments
            // The cross-corridor sits between spine segment i and i+1
            // Connect to the segment that ends at ccy (shared edge at y=ccy)
            // and segment that starts at ccy+CROSS_CORRIDOR_WIDTH (shared edge there)
            for seg in &spine_segments {
                if seg.y_end == ccy {
                    // Spine segment's south edge at y=ccy, cross-corridor's north edge at y=ccy
                    // Door at spine center X, boundary Y = ccy
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: crid,
                        room_b: seg.room_id,
                        wall_a: wall_sides::NORTH,
                        wall_b: wall_sides::SOUTH,
                        position_along_wall: 0.5,
                        width: SPINE_WIDTH as f32,
                        access_level: access_levels::PUBLIC,
                        door_x: spine_center_x,
                        door_y: ccy as f32,
                    });
                }
                if seg.y_start == ccy + CROSS_CORRIDOR_WIDTH {
                    // Cross-corridor's south edge at y=ccy+width, spine segment's north edge there
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: crid,
                        room_b: seg.room_id,
                        wall_a: wall_sides::SOUTH,
                        wall_b: wall_sides::NORTH,
                        position_along_wall: 0.5,
                        width: SPINE_WIDTH as f32,
                        access_level: access_levels::PUBLIC,
                        door_x: spine_center_x,
                        door_y: (ccy + CROSS_CORRIDOR_WIDTH) as f32,
                    });
                }
            }
        }

        // Service corridor: SVC_CORRIDOR_WIDTH cells wide, along starboard (right) edge
        // (svc_left already computed above before cross-corridors)
        for x in svc_left..hull_width {
            for y in 0..hull_length {
                grid[x][y] = CELL_SERVICE_CORRIDOR;
            }
        }
        let svc_rid = next_id();
        ctx.db.room().insert(Room {
            id: svc_rid,
            node_id: 0,
            name: format!("Deck {} Service Corridor", deck + 1),
            room_type: room_types::SERVICE_CORRIDOR,
            deck,
            x: (svc_left as f32 + hull_width as f32) / 2.0,
            y: hull_length as f32 / 2.0,
            width: SVC_CORRIDOR_WIDTH as f32,
            height: hull_length as f32,
            capacity: 4,
        });
        ctx.db.corridor().insert(Corridor {
            id: 0,
            deck,
            corridor_type: corridor_types::SERVICE,
            x: (svc_left as f32 + hull_width as f32) / 2.0,
            y: hull_length as f32 / 2.0,
            width: SVC_CORRIDOR_WIDTH as f32,
            length: hull_length as f32,
            orientation: 1,
            carries: carries_flags::POWER
                | carries_flags::WATER
                | carries_flags::HVAC
                | carries_flags::COOLANT,
        });

        // Door connecting service corridor to each cross-corridor
        for cr in &cross_rooms {
            // Service corridor's west edge at x=svc_left, cross-corridor's east side
            // Door at the shared boundary X=svc_left, centered in the cross-corridor Y range
            let cr_cy = cr.y_start as f32 + CROSS_CORRIDOR_WIDTH as f32 / 2.0;
            ctx.db.door().insert(Door {
                id: 0,
                room_a: svc_rid,
                room_b: cr.room_id,
                wall_a: wall_sides::WEST,
                wall_b: wall_sides::EAST,
                position_along_wall: 0.5,
                width: 2.0,
                access_level: access_levels::CREW_ONLY,
                door_x: svc_left as f32,
                door_y: cr_cy,
            });
        }

        // Helper closures for finding corridor segments by Y coordinate
        let find_spine_segment = |y: usize| -> Option<&SpineSegment> {
            spine_segments
                .iter()
                .find(|s| y >= s.y_start && y < s.y_end)
        };
        let find_cross_room = |y: usize| -> Option<&CrossCorridorRoom> {
            cross_rooms
                .iter()
                .find(|c| y >= c.y_start && y < c.y_start + CROSS_CORRIDOR_WIDTH)
        };

        // ---- Step 2: Stamp vertical shaft anchors ----
        let all_shafts: Vec<(usize, usize, usize, usize, u8, u8, &str, bool)> = {
            let mut v = Vec::new();
            v.push((
                fore_elev_deck.0,
                fore_elev_deck.1,
                fore_elev_deck.2,
                fore_elev_deck.3,
                shaft_types::ELEVATOR,
                room_types::ELEVATOR_SHAFT,
                "Fore Elevator",
                true,
            ));
            v.push((
                aft_elev_deck.0,
                aft_elev_deck.1,
                aft_elev_deck.2,
                aft_elev_deck.3,
                shaft_types::ELEVATOR,
                room_types::ELEVATOR_SHAFT,
                "Aft Elevator",
                true,
            ));
            v.push((
                svc_elev_deck.0,
                svc_elev_deck.1,
                svc_elev_deck.2,
                svc_elev_deck.3,
                shaft_types::SERVICE_ELEVATOR,
                room_types::ELEVATOR_SHAFT,
                "Service Elevator",
                false,
            ));
            for (li, &(lx, ly, lw, lh)) in ladders_deck.iter().enumerate() {
                v.push((
                    lx,
                    ly,
                    lw,
                    lh,
                    shaft_types::LADDER,
                    room_types::LADDER_SHAFT,
                    match li {
                        0 => "Ladder A",
                        1 => "Ladder B",
                        2 => "Ladder C",
                        _ => "Ladder D",
                    },
                    false,
                ));
            }
            v
        };

        for &(sx, sy, sw, sh, _shaft_type, srt, sname, is_main) in &all_shafts {
            if sx + sw > hull_width || sy + sh > hull_length {
                continue;
            }

            for xx in sx..(sx + sw) {
                for yy in sy..(sy + sh) {
                    grid[xx][yy] = CELL_SHAFT;
                }
            }

            let rid = next_id();
            ctx.db.room().insert(Room {
                id: rid,
                node_id: 0,
                name: format!("{} D{}", sname, deck + 1),
                room_type: srt,
                deck,
                x: sx as f32 + sw as f32 / 2.0,
                y: sy as f32 + sh as f32 / 2.0,
                width: sw as f32,
                height: sh as f32,
                capacity: if is_main { 6 } else { 2 },
            });

            // Connect shaft to adjacent corridor via shared edge
            let access = if is_main {
                access_levels::PUBLIC
            } else {
                access_levels::CREW_ONLY
            };
            let shaft_cy = sy + sh / 2;
            let shaft_cx = sx + sw / 2;
            let shaft_center_x = sx as f32 + sw as f32 / 2.0;
            let shaft_center_y = sy as f32 + sh as f32 / 2.0;

            // First: check if shaft overlaps a cross-corridor (shaft sits inside it)
            // If so, connect to it at the shaft's north or south edge
            let mut connected = false;
            for cr in &cross_rooms {
                let cr_end = cr.y_start + CROSS_CORRIDOR_WIDTH;
                // Shaft overlaps cross-corridor if their Y ranges intersect
                if sy < cr_end && sy + sh > cr.y_start {
                    // Connect via shaft's WEST edge to the cross-corridor.
                    // Shaft is embedded inside the corridor — no corridor wall at this boundary.
                    // wall_a=WEST creates gap in shaft wall; wall_b=255 skips corridor gap.
                    let boundary_x = sx as f32;
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: rid,
                        room_b: cr.room_id,
                        wall_a: wall_sides::WEST,
                        wall_b: 255,
                        position_along_wall: 0.5,
                        width: sh.min(CROSS_CORRIDOR_WIDTH) as f32,
                        access_level: access,
                        door_x: boundary_x,
                        door_y: shaft_center_y,
                    });
                    connected = true;
                    break;
                }
            }

            // Then check all 4 edges for adjacent corridor cells in the grid

            // SOUTH edge of shaft (y + sh): check if corridor is below
            if sy + sh < hull_length {
                let test_y = sy + sh;
                let test_x = shaft_cx.min(hull_width - 1);
                if grid[test_x][test_y] == CELL_MAIN_CORRIDOR
                    || test_y < hull_length
                        && grid[test_x.min(hull_width - 1)][test_y] == CELL_MAIN_CORRIDOR
                {
                    let target = find_spine_segment(test_y).or_else(|| {
                        // Check if it's in a cross-corridor
                        None
                    });
                    if let Some(seg) = target {
                        let boundary_y = (sy + sh) as f32;
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: rid,
                            room_b: seg.room_id,
                            wall_a: wall_sides::SOUTH,
                            wall_b: wall_sides::NORTH,
                            position_along_wall: 0.5,
                            width: sw as f32,
                            access_level: access,
                            door_x: shaft_center_x,
                            door_y: boundary_y,
                        });
                        connected = true;
                    }
                }
            }

            // NORTH edge of shaft (y - 1): check if corridor is above
            if sy > 0 && !connected {
                let test_y = sy - 1;
                let test_x = shaft_cx.min(hull_width - 1);
                if grid[test_x][test_y] == CELL_MAIN_CORRIDOR {
                    if let Some(seg) = find_spine_segment(test_y) {
                        let boundary_y = sy as f32;
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: rid,
                            room_b: seg.room_id,
                            wall_a: wall_sides::NORTH,
                            wall_b: wall_sides::SOUTH,
                            position_along_wall: 0.5,
                            width: sw as f32,
                            access_level: access,
                            door_x: shaft_center_x,
                            door_y: boundary_y,
                        });
                        connected = true;
                    }
                }
            }

            // EAST edge of shaft (x + sw): check if corridor is to the right
            if sx + sw < hull_width && !connected {
                let test_x = sx + sw;
                let test_y = shaft_cy.min(hull_length - 1);
                let cell = grid[test_x][test_y];
                if cell == CELL_MAIN_CORRIDOR || cell == CELL_SERVICE_CORRIDOR {
                    let boundary_x = (sx + sw) as f32;
                    let target_id = if cell == CELL_MAIN_CORRIDOR {
                        find_spine_segment(test_y).map(|s| s.room_id)
                    } else {
                        Some(svc_rid)
                    };
                    if let Some(tid) = target_id {
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: rid,
                            room_b: tid,
                            wall_a: wall_sides::EAST,
                            wall_b: wall_sides::WEST,
                            position_along_wall: 0.5,
                            width: sh as f32,
                            access_level: access,
                            door_x: boundary_x,
                            door_y: shaft_center_y,
                        });
                        connected = true;
                    }
                }
            }

            // WEST edge of shaft (x - 1): check if corridor is to the left
            if sx > 0 && !connected {
                let test_x = sx - 1;
                let test_y = shaft_cy.min(hull_length - 1);
                let cell = grid[test_x][test_y];
                if cell == CELL_MAIN_CORRIDOR || cell == CELL_SERVICE_CORRIDOR {
                    let boundary_x = sx as f32;
                    let target_id = if cell == CELL_MAIN_CORRIDOR {
                        find_spine_segment(test_y).map(|s| s.room_id)
                    } else {
                        Some(svc_rid)
                    };
                    if let Some(tid) = target_id {
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: rid,
                            room_b: tid,
                            wall_a: wall_sides::WEST,
                            wall_b: wall_sides::EAST,
                            position_along_wall: 0.5,
                            width: sh as f32,
                            access_level: access,
                            door_x: boundary_x,
                            door_y: shaft_center_y,
                        });
                        connected = true;
                    }
                }
            }

            // Shaft is either connected via cross-corridor overlap, edge adjacency, or remains isolated
        }

        // ---- Step 3: Find empty rectangular zones ----
        let zones = find_empty_zones(&grid, hull_width, hull_length);

        // ---- Step 4: Determine which rooms go on this deck ----
        let mut deck_room_requests: Vec<RoomRequest> = Vec::new();
        for zone_idx in 0..7u8 {
            let (lo, hi) = deck_range_for_zone(zone_idx, deck_count);
            if (deck as u32) >= lo && (deck as u32) < hi {
                let zone_deck_count = hi.saturating_sub(lo).max(1);
                let deck_offset = (deck as u32).saturating_sub(lo);
                let zone_reqs = &zone_requests[zone_idx as usize];
                let total_rooms = zone_reqs.len();
                let per_deck = total_rooms / zone_deck_count as usize;
                let extra = total_rooms % zone_deck_count as usize;
                let start = deck_offset as usize * per_deck + (deck_offset as usize).min(extra);
                let count = per_deck + if (deck_offset as usize) < extra { 1 } else { 0 };
                for i in start..(start + count).min(total_rooms) {
                    let rr = &zone_reqs[i];
                    deck_room_requests.push(RoomRequest {
                        node_id: rr.node_id,
                        name: rr.name.clone(),
                        room_type: rr.room_type,
                        target_area: rr.target_area,
                        capacity: rr.capacity,
                        group: rr.group,
                    });
                }
            }
        }

        if deck_room_requests.is_empty() {
            continue;
        }

        // ---- Step 5: Assign rooms to zones using squarified treemap ----
        // FIX 3: Distribute rooms PROPORTIONALLY across zones by area (not greedy)
        let mut placed_rooms: Vec<PlacedRoom> = Vec::new();
        let total_zone_area: f32 = zones
            .iter()
            .filter(|z| (z.w * z.h) as f32 >= 9.0)
            .map(|z| (z.w * z.h) as f32)
            .sum();
        let _total_room_area: f32 = deck_room_requests.iter().map(|r| r.target_area).sum();

        // Pre-allocate room counts per zone proportional to zone area
        let usable_zones: Vec<&GridZone> =
            zones.iter().filter(|z| (z.w * z.h) as f32 >= 9.0).collect();
        let mut rooms_per_zone: Vec<usize> = Vec::new();
        let mut allocated = 0usize;
        for (zi, zone) in usable_zones.iter().enumerate() {
            let zone_area = (zone.w * zone.h) as f32;
            let fraction = if total_zone_area > 0.0 {
                zone_area / total_zone_area
            } else {
                0.0
            };
            let room_count = if zi == usable_zones.len() - 1 {
                deck_room_requests.len().saturating_sub(allocated)
            } else {
                (fraction * deck_room_requests.len() as f32).round() as usize
            };
            let room_count = room_count.min(deck_room_requests.len().saturating_sub(allocated));
            rooms_per_zone.push(room_count);
            allocated += room_count;
        }

        let mut request_cursor = 0usize;
        for (zi, zone) in usable_zones.iter().enumerate() {
            if request_cursor >= deck_room_requests.len() {
                break;
            }
            let count = rooms_per_zone[zi];
            if count == 0 {
                continue;
            }

            let end = (request_cursor + count).min(deck_room_requests.len());
            let mut batch: Vec<(f32, usize)> = Vec::new();
            for i in request_cursor..end {
                batch.push((deck_room_requests[i].target_area, i));
            }
            request_cursor = end;

            if batch.is_empty() {
                continue;
            }

            let placements = squarified_treemap(&batch, zone.x, zone.y, zone.w, zone.h);

            for (orig_idx, rx, ry, rw, rh) in placements {
                if rw < 2 || rh < 2 {
                    continue;
                }
                let rr = &deck_room_requests[orig_idx];

                let cell_val = CELL_ROOM_BASE + (placed_rooms.len() % 246) as u8;
                for xx in rx..(rx + rw).min(hull_width) {
                    for yy in ry..(ry + rh).min(hull_length) {
                        if grid[xx][yy] == CELL_EMPTY {
                            grid[xx][yy] = cell_val;
                        }
                    }
                }

                let rid = next_id();
                ctx.db.room().insert(Room {
                    id: rid,
                    node_id: rr.node_id,
                    name: format!("{} D{}", rr.name, deck + 1),
                    room_type: rr.room_type,
                    deck,
                    x: rx as f32 + rw as f32 / 2.0,
                    y: ry as f32 + rh as f32 / 2.0,
                    width: rw as f32,
                    height: rh as f32,
                    capacity: rr.capacity,
                });

                placed_rooms.push(PlacedRoom {
                    room_id: rid,
                    node_id: rr.node_id,
                    x: rx,
                    y: ry,
                    w: rw,
                    h: rh,
                    room_type: rr.room_type,
                });
            }
        }

        // ---- Step 6: Generate doors ----
        let mut door_set: Vec<(u32, u32, u8)> = Vec::new();

        for pr in &placed_rooms {
            // Compute absolute door positions from shared edges.
            // Room grid coords: pr.x, pr.y, pr.w, pr.h
            // Room center: (pr.x + pr.w/2, pr.y + pr.h/2)
            let room_center_y = pr.y as f32 + pr.h as f32 / 2.0;
            let room_center_x = pr.x as f32 + pr.w as f32 / 2.0;

            // WEST edge (x - 1): room's west wall touches corridor to its left
            if pr.x > 0 {
                let test_x = pr.x - 1;
                let mid_y = pr.y + pr.h / 2;
                if mid_y < hull_length && test_x < hull_width {
                    let cell = grid[test_x][mid_y];
                    // Shared edge at x = pr.x (room's west boundary)
                    let boundary_x = pr.x as f32;
                    if cell == CELL_MAIN_CORRIDOR {
                        if let Some(seg) = find_spine_segment(mid_y) {
                            let key = (pr.room_id, seg.room_id, wall_sides::WEST);
                            if !door_set.iter().any(|k| *k == key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: seg.room_id,
                                    wall_a: wall_sides::WEST,
                                    wall_b: wall_sides::EAST,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: boundary_x,
                                    door_y: room_center_y,
                                });
                                door_set.push(key);
                            }
                        } else if let Some(cr) = find_cross_room(mid_y) {
                            let key = (pr.room_id, cr.room_id, wall_sides::WEST);
                            if !door_set.iter().any(|k| *k == key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: cr.room_id,
                                    wall_a: wall_sides::WEST,
                                    wall_b: wall_sides::EAST,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: boundary_x,
                                    door_y: room_center_y,
                                });
                                door_set.push(key);
                            }
                        }
                    } else if cell == CELL_SERVICE_CORRIDOR {
                        let key = (pr.room_id, svc_rid, wall_sides::WEST);
                        if !door_set.iter().any(|k| *k == key) {
                            ctx.db.door().insert(Door {
                                id: 0,
                                room_a: pr.room_id,
                                room_b: svc_rid,
                                wall_a: wall_sides::WEST,
                                wall_b: wall_sides::EAST,
                                position_along_wall: 0.5,
                                width: 2.0,
                                access_level: access_levels::CREW_ONLY,
                                door_x: boundary_x,
                                door_y: room_center_y,
                            });
                            door_set.push(key);
                        }
                    }
                }
            }
            // EAST edge (x + w): room's east wall touches corridor to its right
            {
                let test_x = pr.x + pr.w;
                let mid_y = pr.y + pr.h / 2;
                if test_x < hull_width && mid_y < hull_length {
                    let cell = grid[test_x][mid_y];
                    // Shared edge at x = pr.x + pr.w (room's east boundary)
                    let boundary_x = (pr.x + pr.w) as f32;
                    if cell == CELL_MAIN_CORRIDOR {
                        if let Some(seg) = find_spine_segment(mid_y) {
                            let key = (pr.room_id, seg.room_id, wall_sides::EAST);
                            if !door_set.iter().any(|k| *k == key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: seg.room_id,
                                    wall_a: wall_sides::EAST,
                                    wall_b: wall_sides::WEST,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: boundary_x,
                                    door_y: room_center_y,
                                });
                                door_set.push(key);
                            }
                        } else if let Some(cr) = find_cross_room(mid_y) {
                            let key = (pr.room_id, cr.room_id, wall_sides::EAST);
                            if !door_set.iter().any(|k| *k == key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: cr.room_id,
                                    wall_a: wall_sides::EAST,
                                    wall_b: wall_sides::WEST,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: boundary_x,
                                    door_y: room_center_y,
                                });
                                door_set.push(key);
                            }
                        }
                    } else if cell == CELL_SERVICE_CORRIDOR {
                        let key = (pr.room_id, svc_rid, wall_sides::EAST);
                        if !door_set.iter().any(|k| *k == key) {
                            ctx.db.door().insert(Door {
                                id: 0,
                                room_a: pr.room_id,
                                room_b: svc_rid,
                                wall_a: wall_sides::EAST,
                                wall_b: wall_sides::WEST,
                                position_along_wall: 0.5,
                                width: 2.0,
                                access_level: access_levels::CREW_ONLY,
                                door_x: boundary_x,
                                door_y: room_center_y,
                            });
                            door_set.push(key);
                        }
                    }
                }
            }
            // NORTH edge (y - 1): room's north wall touches corridor above
            if pr.y > 0 {
                let test_y = pr.y - 1;
                let mid_x = pr.x + pr.w / 2;
                if mid_x < hull_width && test_y < hull_length {
                    let cell = grid[mid_x][test_y];
                    // Shared edge at y = pr.y (room's north boundary — low Y = fore)
                    let boundary_y = pr.y as f32;
                    if cell == CELL_MAIN_CORRIDOR {
                        if let Some(seg) = find_spine_segment(test_y) {
                            let key = (pr.room_id, seg.room_id, wall_sides::NORTH);
                            if !door_set.iter().any(|k| *k == key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: seg.room_id,
                                    wall_a: wall_sides::NORTH,
                                    wall_b: wall_sides::SOUTH,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: room_center_x,
                                    door_y: boundary_y,
                                });
                                door_set.push(key);
                            }
                        } else if let Some(cr) = find_cross_room(test_y) {
                            let key = (pr.room_id, cr.room_id, wall_sides::NORTH);
                            if !door_set.iter().any(|k| *k == key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: cr.room_id,
                                    wall_a: wall_sides::NORTH,
                                    wall_b: wall_sides::SOUTH,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: room_center_x,
                                    door_y: boundary_y,
                                });
                                door_set.push(key);
                            }
                        }
                    }
                }
            }
            // SOUTH edge (y + h): room's south wall touches corridor below
            {
                let test_y = pr.y + pr.h;
                let mid_x = pr.x + pr.w / 2;
                if test_y < hull_length && mid_x < hull_width {
                    let cell = grid[mid_x][test_y];
                    // Shared edge at y = pr.y + pr.h (room's south boundary)
                    let boundary_y = (pr.y + pr.h) as f32;
                    if cell == CELL_MAIN_CORRIDOR {
                        if let Some(seg) = find_spine_segment(test_y) {
                            let key = (pr.room_id, seg.room_id, wall_sides::SOUTH);
                            if !door_set.iter().any(|k| *k == key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: seg.room_id,
                                    wall_a: wall_sides::SOUTH,
                                    wall_b: wall_sides::NORTH,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: room_center_x,
                                    door_y: boundary_y,
                                });
                                door_set.push(key);
                            }
                        } else if let Some(cr) = find_cross_room(test_y) {
                            let key = (pr.room_id, cr.room_id, wall_sides::SOUTH);
                            if !door_set.iter().any(|k| *k == key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: cr.room_id,
                                    wall_a: wall_sides::SOUTH,
                                    wall_b: wall_sides::NORTH,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: room_center_x,
                                    door_y: boundary_y,
                                });
                                door_set.push(key);
                            }
                        }
                    }
                }
            }
        }

        // Room-to-room doors: only for specific adjacent pairings that make logical sense
        // (e.g., galley↔mess hall, surgery↔hospital). Most rooms connect via corridors only.
        for i in 0..placed_rooms.len() {
            for j in (i + 1)..placed_rooms.len() {
                let a = &placed_rooms[i];
                let b = &placed_rooms[j];

                // Only connect rooms that should have direct internal doors
                if !should_have_room_door(a.room_type, b.room_type) {
                    continue;
                }

                // A's east edge touches B's west edge
                let boundary_x_ab = a.x + a.w;
                if boundary_x_ab == b.x
                    && boundary_x_ab > 0
                    && boundary_x_ab < hull_width
                    && a.y < b.y + b.h
                    && b.y < a.y + a.h
                {
                    let overlap_y0 = core::cmp::max(a.y, b.y);
                    let overlap_y1 = core::cmp::min(a.y + a.h, b.y + b.h);
                    if overlap_y1 > overlap_y0 {
                        let boundary_x = (a.x + a.w) as f32;
                        let mid_y = (overlap_y0 + overlap_y1) as f32 / 2.0;
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: a.room_id,
                            room_b: b.room_id,
                            wall_a: wall_sides::EAST,
                            wall_b: wall_sides::WEST,
                            position_along_wall: 0.5,
                            width: 2.0,
                            access_level: access_levels::CREW_ONLY,
                            door_x: boundary_x,
                            door_y: mid_y,
                        });
                    }
                } else if b.x + b.w == a.x
                    && a.x > 0
                    && a.x < hull_width
                    && a.y < b.y + b.h
                    && b.y < a.y + a.h
                {
                    let overlap_y0 = core::cmp::max(a.y, b.y);
                    let overlap_y1 = core::cmp::min(a.y + a.h, b.y + b.h);
                    if overlap_y1 > overlap_y0 {
                        let boundary_x = a.x as f32;
                        let mid_y = (overlap_y0 + overlap_y1) as f32 / 2.0;
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: a.room_id,
                            room_b: b.room_id,
                            wall_a: wall_sides::WEST,
                            wall_b: wall_sides::EAST,
                            position_along_wall: 0.5,
                            width: 2.0,
                            access_level: access_levels::CREW_ONLY,
                            door_x: boundary_x,
                            door_y: mid_y,
                        });
                    }
                }
                // A's south edge touches B's north edge
                let boundary_y_ab = a.y + a.h;
                if boundary_y_ab == b.y
                    && boundary_y_ab > 0
                    && boundary_y_ab < hull_length
                    && a.x < b.x + b.w
                    && b.x < a.x + a.w
                {
                    let overlap_x0 = core::cmp::max(a.x, b.x);
                    let overlap_x1 = core::cmp::min(a.x + a.w, b.x + b.w);
                    if overlap_x1 > overlap_x0 {
                        let boundary_y = (a.y + a.h) as f32;
                        let mid_x = (overlap_x0 + overlap_x1) as f32 / 2.0;
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: a.room_id,
                            room_b: b.room_id,
                            wall_a: wall_sides::SOUTH,
                            wall_b: wall_sides::NORTH,
                            position_along_wall: 0.5,
                            width: 2.0,
                            access_level: access_levels::CREW_ONLY,
                            door_x: mid_x,
                            door_y: boundary_y,
                        });
                    }
                } else if b.y + b.h == a.y
                    && a.y > 0
                    && a.y < hull_length
                    && a.x < b.x + b.w
                    && b.x < a.x + a.w
                {
                    let overlap_x0 = core::cmp::max(a.x, b.x);
                    let overlap_x1 = core::cmp::min(a.x + a.w, b.x + b.w);
                    if overlap_x1 > overlap_x0 {
                        let boundary_y = a.y as f32;
                        let mid_x = (overlap_x0 + overlap_x1) as f32 / 2.0;
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: a.room_id,
                            room_b: b.room_id,
                            wall_a: wall_sides::NORTH,
                            wall_b: wall_sides::SOUTH,
                            position_along_wall: 0.5,
                            width: 2.0,
                            access_level: access_levels::CREW_ONLY,
                            door_x: mid_x,
                            door_y: boundary_y,
                        });
                    }
                }
            }
        }

        // Force-connect orphan rooms: only if room actually borders a corridor cell
        for pr in &placed_rooms {
            let has_door = door_set.iter().any(|&(a, _, _)| a == pr.room_id);
            if has_door {
                continue;
            }

            // Check all 4 edges for adjacent corridor cells
            let mut connected = false;

            // West edge: check cell at (pr.x - 1, mid_y)
            if pr.x > 0 {
                let test_x = pr.x - 1;
                let mid_y = pr.y + pr.h / 2;
                if test_x < hull_width && mid_y < hull_length {
                    let cell = grid[test_x][mid_y];
                    if cell == CELL_MAIN_CORRIDOR || cell == CELL_SERVICE_CORRIDOR {
                        let target = if cell == CELL_MAIN_CORRIDOR {
                            find_spine_segment(mid_y)
                                .map(|s| s.room_id)
                                .or_else(|| find_cross_room(mid_y).map(|c| c.room_id))
                        } else {
                            Some(svc_rid)
                        };
                        if let Some(tid) = target {
                            let bx = pr.x as f32;
                            if bx > 0.5 && (bx as usize) < hull_width {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: tid,
                                    wall_a: wall_sides::WEST,
                                    wall_b: wall_sides::EAST,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: bx,
                                    door_y: pr.y as f32 + pr.h as f32 / 2.0,
                                });
                                connected = true;
                            }
                        }
                    }
                }
            }
            // East edge
            if !connected {
                let test_x = pr.x + pr.w;
                let mid_y = pr.y + pr.h / 2;
                if test_x < hull_width && mid_y < hull_length {
                    let cell = grid[test_x][mid_y];
                    if cell == CELL_MAIN_CORRIDOR || cell == CELL_SERVICE_CORRIDOR {
                        let target = if cell == CELL_MAIN_CORRIDOR {
                            find_spine_segment(mid_y)
                                .map(|s| s.room_id)
                                .or_else(|| find_cross_room(mid_y).map(|c| c.room_id))
                        } else {
                            Some(svc_rid)
                        };
                        if let Some(tid) = target {
                            let bx = (pr.x + pr.w) as f32;
                            if bx > 0.5 && (bx as usize) < hull_width {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: tid,
                                    wall_a: wall_sides::EAST,
                                    wall_b: wall_sides::WEST,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: bx,
                                    door_y: pr.y as f32 + pr.h as f32 / 2.0,
                                });
                                connected = true;
                            }
                        }
                    }
                }
            }
            // North edge
            if !connected && pr.y > 0 {
                let test_y = pr.y - 1;
                let mid_x = pr.x + pr.w / 2;
                if mid_x < hull_width && test_y < hull_length {
                    let cell = grid[mid_x][test_y];
                    if cell == CELL_MAIN_CORRIDOR {
                        let target = find_spine_segment(test_y)
                            .map(|s| s.room_id)
                            .or_else(|| find_cross_room(test_y).map(|c| c.room_id));
                        if let Some(tid) = target {
                            let by = pr.y as f32;
                            if by > 0.5 {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: tid,
                                    wall_a: wall_sides::NORTH,
                                    wall_b: wall_sides::SOUTH,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: pr.x as f32 + pr.w as f32 / 2.0,
                                    door_y: by,
                                });
                                connected = true;
                            }
                        }
                    }
                }
            }
            // South edge
            if !connected {
                let test_y = pr.y + pr.h;
                let mid_x = pr.x + pr.w / 2;
                if test_y < hull_length && mid_x < hull_width {
                    let cell = grid[mid_x][test_y];
                    if cell == CELL_MAIN_CORRIDOR {
                        let target = find_spine_segment(test_y)
                            .map(|s| s.room_id)
                            .or_else(|| find_cross_room(test_y).map(|c| c.room_id));
                        if let Some(tid) = target {
                            let by = (pr.y + pr.h) as f32;
                            if (by as usize) < hull_length {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: tid,
                                    wall_a: wall_sides::SOUTH,
                                    wall_b: wall_sides::NORTH,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: pr.x as f32 + pr.w as f32 / 2.0,
                                    door_y: by,
                                });
                                connected = true;
                            }
                        }
                    }
                }
            }
            // If still not connected, this room is truly isolated — skip it
            let _ = connected;
        }

        // ASCII dump for debugging
        {
            let mut dump = format!(
                "Deck {} grid ({}x{}, {} rooms, {} spine segs, {} cross-corridors):\n",
                deck + 1,
                hull_width,
                hull_length,
                placed_rooms.len(),
                spine_segments.len(),
                cross_rooms.len()
            );
            let max_rows = hull_length.min(60);
            for y in 0..max_rows {
                for x in 0..hull_width {
                    let ch = match grid[x][y] {
                        CELL_EMPTY => '.',
                        CELL_MAIN_CORRIDOR => '=',
                        CELL_SERVICE_CORRIDOR => '-',
                        CELL_SHAFT => '#',
                        v if v >= CELL_ROOM_BASE => {
                            let idx = (v - CELL_ROOM_BASE) % 26;
                            (b'A' + idx) as char
                        }
                        _ => '.',
                    };
                    dump.push(ch);
                }
                dump.push('\n');
            }
            if hull_length > max_rows {
                dump.push_str(&format!("... ({} more rows)\n", hull_length - max_rows));
            }
            log::info!("{}", dump);
        }
    }

    // ---- Step 7: Create VerticalShaft entries and cross-deck doors ----
    let decks_str = (0..deck_count)
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(",");

    // Use standard-deck positions for VerticalShaft entries (visual markers)
    let std_spine_cx = SHIP_BEAM / 2;
    struct ShaftDef {
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        shaft_type: u8,
        name: &'static str,
        is_main: bool,
    }
    let shaft_defs = [
        ShaftDef {
            x: std_spine_cx + 2,
            y: 10,
            w: 3,
            h: 3,
            shaft_type: shaft_types::ELEVATOR,
            name: "Fore Elevator",
            is_main: true,
        },
        ShaftDef {
            x: std_spine_cx + 2,
            y: SHIP_LENGTH - 14,
            w: 3,
            h: 3,
            shaft_type: shaft_types::ELEVATOR,
            name: "Aft Elevator",
            is_main: true,
        },
        ShaftDef {
            x: SHIP_BEAM - 5,
            y: 100,
            w: 2,
            h: 2,
            shaft_type: shaft_types::SERVICE_ELEVATOR,
            name: "Service Elevator",
            is_main: false,
        },
        ShaftDef {
            x: SHIP_BEAM - 4,
            y: 50,
            w: 2,
            h: 2,
            shaft_type: shaft_types::LADDER,
            name: "Ladder A",
            is_main: false,
        },
        ShaftDef {
            x: SHIP_BEAM - 4,
            y: 150,
            w: 2,
            h: 2,
            shaft_type: shaft_types::LADDER,
            name: "Ladder B",
            is_main: false,
        },
        ShaftDef {
            x: SHIP_BEAM - 4,
            y: 250,
            w: 2,
            h: 2,
            shaft_type: shaft_types::LADDER,
            name: "Ladder C",
            is_main: false,
        },
        ShaftDef {
            x: SHIP_BEAM - 4,
            y: 350,
            w: 2,
            h: 2,
            shaft_type: shaft_types::LADDER,
            name: "Ladder D",
            is_main: false,
        },
    ];

    for sd in &shaft_defs {
        ctx.db.vertical_shaft().insert(VerticalShaft {
            id: 0,
            shaft_type: sd.shaft_type,
            name: sd.name.to_string(),
            x: sd.x as f32 + sd.w as f32 / 2.0,
            y: sd.y as f32 + sd.h as f32 / 2.0,
            decks_served: decks_str.clone(),
            width: sd.w as f32,
            height: sd.h as f32,
        });

        // Cross-deck doors between consecutive deck shaft rooms
        // Find shaft rooms by name pattern across decks
        let mut shaft_rooms_across_decks: Vec<u32> = Vec::new();
        for d in 0..deck_count {
            let search_name = format!("{} D{}", sd.name, d + 1);
            // Look up room by name match
            for room in ctx.db.room().iter() {
                if room.name == search_name {
                    shaft_rooms_across_decks.push(room.id);
                    break;
                }
            }
        }

        for i in 0..shaft_rooms_across_decks.len().saturating_sub(1) {
            let access = if sd.is_main {
                access_levels::PUBLIC
            } else {
                access_levels::CREW_ONLY
            };
            // Use actual room positions (they vary per deck due to hull taper)
            let room_a_id = shaft_rooms_across_decks[i];
            let room_b_id = shaft_rooms_across_decks[i + 1];
            if let (Some(ra), Some(rb)) = (
                ctx.db.room().id().find(room_a_id),
                ctx.db.room().id().find(room_b_id),
            ) {
                // Cross-deck door: midpoint between the two rooms' centers
                let mid_x = (ra.x + rb.x) / 2.0;
                let mid_y = (ra.y + rb.y) / 2.0;
                ctx.db.door().insert(Door {
                    id: 0,
                    room_a: room_a_id,
                    room_b: room_b_id,
                    wall_a: wall_sides::SOUTH,
                    wall_b: wall_sides::NORTH,
                    position_along_wall: 0.5,
                    width: 2.0,
                    access_level: access,
                    door_x: mid_x,
                    door_y: mid_y,
                });
            }
        }
    }
}
// ============================================================================
// STEP 3: GENERATE SHIP SYSTEMS
// ============================================================================

fn generate_ship_systems(ctx: &ReducerContext) {
    let insert_system = |name: &str, sys_type: u8, priority: u8| -> u64 {
        ctx.db
            .ship_system()
            .insert(ShipSystem {
                id: 0,
                name: name.to_string(),
                system_type: sys_type,
                overall_health: 1.0,
                overall_status: system_statuses::NOMINAL,
                priority,
            })
            .id
    };

    // Find node_id by room_type from the GraphNode entries
    let find_node = |func: u8| -> u64 {
        ctx.db
            .graph_node()
            .iter()
            .find(|n| n.function == func)
            .map(|n| n.id)
            .unwrap_or(0)
    };

    let reactor_node = find_node(room_types::REACTOR);
    let engineering_node = find_node(room_types::ENGINEERING);
    let power_dist_node = find_node(room_types::POWER_DISTRIBUTION);
    let ls_node = find_node(room_types::LIFE_SUPPORT);
    let cooling_node = find_node(room_types::COOLING_PLANT);
    let hvac_node = find_node(room_types::HVAC_CONTROL);
    let water_node = find_node(room_types::WATER_RECYCLING);
    let waste_node = find_node(room_types::WASTE_PROCESSING);
    let hydro_node = find_node(room_types::HYDROPONICS);
    let galley_node = find_node(room_types::GALLEY);
    let bridge_node = find_node(room_types::BRIDGE);
    let comms_node = find_node(room_types::COMMS_ROOM);
    let medical_node = find_node(room_types::HOSPITAL_WARD);

    let insert_subsystem = |system_id: u64,
                            name: &str,
                            sub_type: u8,
                            node_id: u64,
                            power_draw: f32,
                            crew_req: u8|
     -> u64 {
        ctx.db
            .subsystem()
            .insert(Subsystem {
                id: 0,
                system_id,
                name: name.to_string(),
                subsystem_type: sub_type,
                health: 1.0,
                status: system_statuses::NOMINAL,
                node_id,
                power_draw,
                crew_required: crew_req,
            })
            .id
    };

    let insert_component =
        |subsystem_id: u64, name: &str, comp_type: u8, px: f32, py: f32, maint_hours: f32| {
            ctx.db.system_component().insert(SystemComponent {
                id: 0,
                subsystem_id,
                name: name.to_string(),
                component_type: comp_type,
                health: 1.0,
                status: system_statuses::NOMINAL,
                position_x: px,
                position_y: py,
                maintenance_interval_hours: maint_hours,
                last_maintenance: 0.0,
            });
        };

    // Find the first service corridor for infra edge routing
    let svc_corridor_id = ctx
        .db
        .corridor()
        .iter()
        .find(|c| c.corridor_type == corridor_types::SERVICE)
        .map(|c| c.id)
        .unwrap_or(0);

    // Helper: create GraphEdge + InfraEdge for system connections
    let insert_infra = |from_node: u64, to_node: u64, etype: u8, infra: u8, capacity: f32| {
        let ge = ctx.db.graph_edge().insert(GraphEdge {
            id: 0,
            from_node,
            to_node,
            edge_type: etype,
            weight: capacity,
            bidirectional: false,
        });
        ctx.db.infra_edge().insert(InfraEdge {
            id: 0,
            graph_edge_id: ge.id,
            edge_type: infra,
            corridor_id: svc_corridor_id,
            capacity,
            current_flow: capacity,
            health: 1.0,
        });
    };

    // ---- POWER SYSTEM ----
    let power_sys = insert_system(
        "Power System",
        system_types::POWER,
        power_priorities::CRITICAL,
    );

    let reactor_core = insert_subsystem(
        power_sys,
        "Reactor Core",
        subsystem_types::REACTOR_CORE,
        reactor_node,
        0.0,
        2,
    );
    insert_component(
        reactor_core,
        "Primary Fuel Injector",
        component_types::FUEL_INJECTOR,
        -2.0,
        0.0,
        500.0,
    );
    insert_component(
        reactor_core,
        "Secondary Fuel Injector",
        component_types::FUEL_INJECTOR,
        2.0,
        0.0,
        500.0,
    );
    insert_component(
        reactor_core,
        "Containment Coil A",
        component_types::CONTAINMENT_COIL,
        -1.0,
        -2.0,
        1000.0,
    );
    insert_component(
        reactor_core,
        "Containment Coil B",
        component_types::CONTAINMENT_COIL,
        1.0,
        -2.0,
        1000.0,
    );
    insert_component(
        reactor_core,
        "Core Temperature Sensor",
        component_types::SENSOR,
        0.0,
        0.0,
        200.0,
    );

    let fuel_inj = insert_subsystem(
        power_sys,
        "Fuel Injection System",
        subsystem_types::FUEL_INJECTION,
        reactor_node,
        2.0,
        1,
    );
    insert_component(
        fuel_inj,
        "Fuel Pump",
        component_types::PUMP,
        -1.0,
        1.0,
        300.0,
    );
    insert_component(
        fuel_inj,
        "Flow Regulator",
        component_types::REGULATOR,
        1.0,
        1.0,
        400.0,
    );

    let containment = insert_subsystem(
        power_sys,
        "Magnetic Containment",
        subsystem_types::MAGNETIC_CONTAINMENT,
        reactor_node,
        15.0,
        1,
    );
    insert_component(
        containment,
        "Containment Field Generator",
        component_types::GENERATOR,
        0.0,
        -1.0,
        800.0,
    );
    insert_component(
        containment,
        "Field Strength Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        200.0,
    );

    let reactor_cool = insert_subsystem(
        power_sys,
        "Reactor Cooling",
        subsystem_types::REACTOR_COOLING,
        cooling_node,
        10.0,
        1,
    );
    insert_component(
        reactor_cool,
        "Primary Coolant Pump",
        component_types::PUMP,
        -2.0,
        0.0,
        250.0,
    );
    insert_component(
        reactor_cool,
        "Backup Coolant Pump",
        component_types::PUMP,
        2.0,
        0.0,
        250.0,
    );
    insert_component(
        reactor_cool,
        "Coolant Temperature Sensor",
        component_types::SENSOR,
        0.0,
        0.0,
        150.0,
    );

    let power_bus = insert_subsystem(
        power_sys,
        "Primary Power Bus",
        subsystem_types::PRIMARY_POWER_BUS,
        power_dist_node,
        1.0,
        1,
    );
    insert_component(
        power_bus,
        "Main Transformer",
        component_types::TRANSFORMER,
        -1.0,
        0.0,
        600.0,
    );
    insert_component(
        power_bus,
        "Bus Circuit Breaker",
        component_types::CIRCUIT_BREAKER,
        1.0,
        0.0,
        400.0,
    );

    let deck_dist = insert_subsystem(
        power_sys,
        "Deck Distribution",
        subsystem_types::DECK_DISTRIBUTION,
        power_dist_node,
        1.0,
        1,
    );
    insert_component(
        deck_dist,
        "Distribution Panel",
        component_types::CIRCUIT_BREAKER,
        0.0,
        -1.0,
        350.0,
    );
    insert_component(
        deck_dist,
        "Load Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        200.0,
    );

    let emerg_bus = insert_subsystem(
        power_sys,
        "Emergency Power Bus",
        subsystem_types::EMERGENCY_BUS,
        power_dist_node,
        0.5,
        1,
    );
    insert_component(
        emerg_bus,
        "Emergency Breaker",
        component_types::CIRCUIT_BREAKER,
        0.0,
        0.0,
        300.0,
    );

    let emerg_gen1 = insert_subsystem(
        power_sys,
        "Emergency Generator 1",
        subsystem_types::EMERGENCY_GENERATOR,
        engineering_node,
        0.0,
        1,
    );
    insert_component(
        emerg_gen1,
        "Generator Motor",
        component_types::GENERATOR,
        0.0,
        0.0,
        500.0,
    );
    let emerg_gen2 = insert_subsystem(
        power_sys,
        "Emergency Generator 2",
        subsystem_types::EMERGENCY_GENERATOR,
        engineering_node,
        0.0,
        1,
    );
    insert_component(
        emerg_gen2,
        "Generator Motor",
        component_types::GENERATOR,
        0.0,
        0.0,
        500.0,
    );

    // ---- LIFE SUPPORT ----
    let ls_sys = insert_system(
        "Life Support",
        system_types::LIFE_SUPPORT,
        power_priorities::CRITICAL,
    );

    let o2_gen = insert_subsystem(
        ls_sys,
        "O2 Generation",
        subsystem_types::O2_GENERATION,
        ls_node,
        20.0,
        1,
    );
    insert_component(
        o2_gen,
        "Electrolysis Cell A",
        component_types::GENERATOR,
        -2.0,
        0.0,
        400.0,
    );
    insert_component(
        o2_gen,
        "Electrolysis Cell B",
        component_types::GENERATOR,
        2.0,
        0.0,
        400.0,
    );
    insert_component(
        o2_gen,
        "O2 Level Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        150.0,
    );

    let co2_scrub = insert_subsystem(
        ls_sys,
        "CO2 Scrubbers",
        subsystem_types::CO2_SCRUBBING,
        ls_node,
        12.0,
        1,
    );
    insert_component(
        co2_scrub,
        "Scrubber Filter A",
        component_types::FILTER,
        -1.0,
        0.0,
        200.0,
    );
    insert_component(
        co2_scrub,
        "Scrubber Filter B",
        component_types::FILTER,
        1.0,
        0.0,
        200.0,
    );
    insert_component(
        co2_scrub,
        "CO2 Sensor",
        component_types::SENSOR,
        0.0,
        0.0,
        150.0,
    );

    let air_circ = insert_subsystem(
        ls_sys,
        "Air Circulation",
        subsystem_types::AIR_CIRCULATION,
        hvac_node,
        8.0,
        1,
    );
    insert_component(
        air_circ,
        "Circulation Fan A",
        component_types::FAN,
        -1.0,
        0.0,
        300.0,
    );
    insert_component(
        air_circ,
        "Circulation Fan B",
        component_types::FAN,
        1.0,
        0.0,
        300.0,
    );
    insert_component(
        air_circ,
        "Airflow Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        150.0,
    );

    let heat_ex = insert_subsystem(
        ls_sys,
        "Heat Exchangers",
        subsystem_types::HEAT_EXCHANGE,
        cooling_node,
        6.0,
        1,
    );
    insert_component(
        heat_ex,
        "Heat Exchanger Unit",
        component_types::HEAT_EXCHANGER,
        0.0,
        0.0,
        500.0,
    );
    insert_component(
        heat_ex,
        "Temperature Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        150.0,
    );

    let coolant_pump = insert_subsystem(
        ls_sys,
        "Coolant Pumps",
        subsystem_types::COOLANT_PUMP,
        cooling_node,
        5.0,
        1,
    );
    insert_component(
        coolant_pump,
        "Main Coolant Pump",
        component_types::PUMP,
        0.0,
        0.0,
        250.0,
    );
    insert_component(
        coolant_pump,
        "Coolant Valve",
        component_types::VALVE,
        1.0,
        0.0,
        300.0,
    );

    let radiator = insert_subsystem(
        ls_sys,
        "Radiator Panels",
        subsystem_types::RADIATOR_PANEL,
        cooling_node,
        0.0,
        0,
    );
    insert_component(
        radiator,
        "Radiator Panel Array",
        component_types::HEAT_EXCHANGER,
        0.0,
        0.0,
        600.0,
    );

    let pressure = insert_subsystem(
        ls_sys,
        "Pressure Management",
        subsystem_types::PRESSURE_MANAGEMENT,
        ls_node,
        3.0,
        1,
    );
    insert_component(
        pressure,
        "Pressure Regulator",
        component_types::REGULATOR,
        0.0,
        -1.0,
        350.0,
    );
    insert_component(
        pressure,
        "Bulkhead Seal Actuator",
        component_types::ACTUATOR,
        0.0,
        1.0,
        400.0,
    );
    insert_component(
        pressure,
        "Pressure Sensor",
        component_types::SENSOR,
        1.0,
        0.0,
        150.0,
    );

    // ---- WATER SYSTEM ----
    let water_sys = insert_system(
        "Water System",
        system_types::WATER_RECYCLING,
        power_priorities::NORMAL,
    );

    let water_filt = insert_subsystem(
        water_sys,
        "Water Filtration",
        subsystem_types::WATER_FILTRATION,
        water_node,
        8.0,
        1,
    );
    insert_component(
        water_filt,
        "Filtration Membrane",
        component_types::FILTER,
        0.0,
        -1.0,
        200.0,
    );
    insert_component(
        water_filt,
        "Sediment Filter",
        component_types::FILTER,
        0.0,
        1.0,
        150.0,
    );

    let water_dist_sub = insert_subsystem(
        water_sys,
        "Water Distillation",
        subsystem_types::WATER_DISTILLATION,
        water_node,
        10.0,
        1,
    );
    insert_component(
        water_dist_sub,
        "Distillation Column",
        component_types::HEAT_EXCHANGER,
        0.0,
        0.0,
        400.0,
    );
    insert_component(
        water_dist_sub,
        "Distillation Heater",
        component_types::GENERATOR,
        1.0,
        0.0,
        350.0,
    );

    let uv_purify = insert_subsystem(
        water_sys,
        "UV Purification",
        subsystem_types::UV_PURIFICATION,
        water_node,
        4.0,
        0,
    );
    insert_component(
        uv_purify,
        "UV Lamp Array",
        component_types::LAMP,
        0.0,
        0.0,
        300.0,
    );
    insert_component(
        uv_purify,
        "UV Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        150.0,
    );

    let water_store = insert_subsystem(
        water_sys,
        "Water Storage Tanks",
        subsystem_types::WATER_STORAGE,
        water_node,
        1.0,
        0,
    );
    insert_component(
        water_store,
        "Main Tank",
        component_types::TANK,
        -1.0,
        0.0,
        800.0,
    );
    insert_component(
        water_store,
        "Level Sensor",
        component_types::SENSOR,
        1.0,
        0.0,
        200.0,
    );

    let water_pump = insert_subsystem(
        water_sys,
        "Water Distribution",
        subsystem_types::WATER_DISTRIBUTION,
        water_node,
        5.0,
        1,
    );
    insert_component(
        water_pump,
        "Distribution Pump",
        component_types::PUMP,
        0.0,
        0.0,
        250.0,
    );
    insert_component(
        water_pump,
        "Pressure Valve",
        component_types::VALVE,
        1.0,
        0.0,
        300.0,
    );

    let waste_proc = insert_subsystem(
        water_sys,
        "Waste Processing",
        subsystem_types::WASTE_PROCESSING,
        waste_node,
        6.0,
        1,
    );
    insert_component(
        waste_proc,
        "Bioreactor",
        component_types::TANK,
        -1.0,
        0.0,
        500.0,
    );
    insert_component(
        waste_proc,
        "Solids Separator",
        component_types::FILTER,
        1.0,
        0.0,
        300.0,
    );

    // ---- FOOD PRODUCTION ----
    let food_sys = insert_system(
        "Food Production",
        system_types::FOOD_PRODUCTION,
        power_priorities::NORMAL,
    );

    let growth = insert_subsystem(
        food_sys,
        "Growth Chambers",
        subsystem_types::GROWTH_CHAMBER,
        hydro_node,
        12.0,
        2,
    );
    insert_component(
        growth,
        "Grow Bed A",
        component_types::TANK,
        -2.0,
        0.0,
        600.0,
    );
    insert_component(growth, "Grow Bed B", component_types::TANK, 2.0, 0.0, 600.0);
    insert_component(
        growth,
        "Soil Moisture Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        100.0,
    );

    let nutrients = insert_subsystem(
        food_sys,
        "Nutrient Mixer",
        subsystem_types::NUTRIENT_MIXER,
        hydro_node,
        3.0,
        1,
    );
    insert_component(
        nutrients,
        "Nutrient Pump",
        component_types::PUMP,
        0.0,
        0.0,
        200.0,
    );
    insert_component(
        nutrients,
        "pH Sensor",
        component_types::SENSOR,
        1.0,
        0.0,
        100.0,
    );

    let grow_light = insert_subsystem(
        food_sys,
        "Grow Lighting",
        subsystem_types::GROW_LIGHTING,
        hydro_node,
        15.0,
        0,
    );
    insert_component(
        grow_light,
        "LED Array A",
        component_types::LAMP,
        -1.0,
        0.0,
        400.0,
    );
    insert_component(
        grow_light,
        "LED Array B",
        component_types::LAMP,
        1.0,
        0.0,
        400.0,
    );

    let food_proc = insert_subsystem(
        food_sys,
        "Food Processing",
        subsystem_types::FOOD_PROCESSING,
        galley_node,
        5.0,
        2,
    );
    insert_component(
        food_proc,
        "Processing Unit",
        component_types::MOTOR,
        0.0,
        0.0,
        350.0,
    );

    let cold_store = insert_subsystem(
        food_sys,
        "Cold Storage",
        subsystem_types::COLD_STORAGE,
        galley_node,
        8.0,
        0,
    );
    insert_component(
        cold_store,
        "Refrigeration Compressor",
        component_types::COMPRESSOR,
        0.0,
        0.0,
        400.0,
    );
    insert_component(
        cold_store,
        "Temperature Sensor",
        component_types::SENSOR,
        1.0,
        0.0,
        100.0,
    );

    // ---- PROPULSION ----
    let prop_sys = insert_system(
        "Propulsion",
        system_types::PROPULSION,
        power_priorities::HIGH,
    );

    let thrust = insert_subsystem(
        prop_sys,
        "Thrust Chambers",
        subsystem_types::THRUST_CHAMBER,
        engineering_node,
        0.0,
        2,
    );
    insert_component(
        thrust,
        "Thrust Nozzle A",
        component_types::NOZZLE,
        -2.0,
        0.0,
        700.0,
    );
    insert_component(
        thrust,
        "Thrust Nozzle B",
        component_types::NOZZLE,
        2.0,
        0.0,
        700.0,
    );
    insert_component(
        thrust,
        "Thrust Sensor",
        component_types::SENSOR,
        0.0,
        0.0,
        200.0,
    );

    let fuel_pump_sub = insert_subsystem(
        prop_sys,
        "Fuel Pumps",
        subsystem_types::FUEL_PUMP,
        engineering_node,
        5.0,
        1,
    );
    insert_component(
        fuel_pump_sub,
        "Primary Fuel Pump",
        component_types::PUMP,
        0.0,
        0.0,
        300.0,
    );
    insert_component(
        fuel_pump_sub,
        "Fuel Flow Valve",
        component_types::VALVE,
        1.0,
        0.0,
        300.0,
    );

    let nozzle_act = insert_subsystem(
        prop_sys,
        "Nozzle Actuators",
        subsystem_types::NOZZLE_ACTUATOR,
        engineering_node,
        3.0,
        1,
    );
    insert_component(
        nozzle_act,
        "Gimbal Actuator A",
        component_types::ACTUATOR,
        -1.0,
        0.0,
        400.0,
    );
    insert_component(
        nozzle_act,
        "Gimbal Actuator B",
        component_types::ACTUATOR,
        1.0,
        0.0,
        400.0,
    );

    // ---- NAVIGATION ----
    let nav_sys = insert_system(
        "Navigation",
        system_types::NAVIGATION,
        power_priorities::CRITICAL,
    );

    let star_track = insert_subsystem(
        nav_sys,
        "Star Trackers",
        subsystem_types::STAR_TRACKER,
        bridge_node,
        4.0,
        1,
    );
    insert_component(
        star_track,
        "Star Tracker Camera",
        component_types::SCANNER_HEAD,
        0.0,
        0.0,
        300.0,
    );
    insert_component(
        star_track,
        "Image Processor",
        component_types::PROCESSOR,
        1.0,
        0.0,
        200.0,
    );

    let gyro = insert_subsystem(
        nav_sys,
        "Gyroscopes",
        subsystem_types::GYROSCOPE,
        bridge_node,
        3.0,
        0,
    );
    insert_component(
        gyro,
        "Gyroscope Unit A",
        component_types::MOTOR,
        -1.0,
        0.0,
        500.0,
    );
    insert_component(
        gyro,
        "Gyroscope Unit B",
        component_types::MOTOR,
        1.0,
        0.0,
        500.0,
    );

    let att_thrust = insert_subsystem(
        nav_sys,
        "Attitude Thrusters",
        subsystem_types::ATTITUDE_THRUSTER,
        engineering_node,
        2.0,
        0,
    );
    insert_component(
        att_thrust,
        "Thruster Pack Fore",
        component_types::NOZZLE,
        -1.0,
        0.0,
        400.0,
    );
    insert_component(
        att_thrust,
        "Thruster Pack Aft",
        component_types::NOZZLE,
        1.0,
        0.0,
        400.0,
    );

    // ---- COMMUNICATIONS ----
    let comms_sys = insert_system(
        "Communications",
        system_types::COMMUNICATIONS,
        power_priorities::HIGH,
    );

    let antenna = insert_subsystem(
        comms_sys,
        "Antenna Array",
        subsystem_types::ANTENNA_ARRAY,
        comms_node,
        5.0,
        1,
    );
    insert_component(
        antenna,
        "Primary Antenna",
        component_types::ANTENNA,
        -1.0,
        0.0,
        600.0,
    );
    insert_component(
        antenna,
        "Secondary Antenna",
        component_types::ANTENNA,
        1.0,
        0.0,
        600.0,
    );

    let sig_proc = insert_subsystem(
        comms_sys,
        "Signal Processors",
        subsystem_types::SIGNAL_PROCESSOR,
        comms_node,
        3.0,
        1,
    );
    insert_component(
        sig_proc,
        "Signal Processor Unit",
        component_types::PROCESSOR,
        0.0,
        0.0,
        300.0,
    );
    insert_component(
        sig_proc,
        "Encryption Module",
        component_types::PROCESSOR,
        1.0,
        0.0,
        400.0,
    );

    let intercom = insert_subsystem(
        comms_sys,
        "Intercom Network",
        subsystem_types::INTERCOM_NETWORK,
        comms_node,
        2.0,
        0,
    );
    insert_component(
        intercom,
        "Intercom Hub",
        component_types::PROCESSOR,
        0.0,
        0.0,
        250.0,
    );

    let data_back = insert_subsystem(
        comms_sys,
        "Data Backbone",
        subsystem_types::DATA_BACKBONE,
        comms_node,
        3.0,
        0,
    );
    insert_component(
        data_back,
        "Network Switch A",
        component_types::PROCESSOR,
        -1.0,
        0.0,
        350.0,
    );
    insert_component(
        data_back,
        "Network Switch B",
        component_types::PROCESSOR,
        1.0,
        0.0,
        350.0,
    );

    // ---- GRAVITY ----
    let grav_sys = insert_system(
        "Gravity System",
        system_types::GRAVITY,
        power_priorities::NORMAL,
    );

    let grav_ctrl = insert_subsystem(
        grav_sys,
        "Gravity Controller",
        subsystem_types::GRAVITY_CONTROLLER,
        engineering_node,
        5.0,
        1,
    );
    insert_component(
        grav_ctrl,
        "Central Controller",
        component_types::PROCESSOR,
        0.0,
        0.0,
        400.0,
    );

    let grav_plate = insert_subsystem(
        grav_sys,
        "Gravity Plates",
        subsystem_types::GRAVITY_PLATE,
        engineering_node,
        50.0,
        0,
    );
    insert_component(
        grav_plate,
        "Gravity Emitter Array",
        component_types::GRAVITY_EMITTER,
        0.0,
        0.0,
        800.0,
    );

    let dampener = insert_subsystem(
        grav_sys,
        "Inertial Dampeners",
        subsystem_types::INERTIAL_DAMPENER,
        engineering_node,
        15.0,
        0,
    );
    insert_component(
        dampener,
        "Dampener Compensator",
        component_types::CAPACITOR,
        0.0,
        0.0,
        500.0,
    );

    // ---- MEDICAL ----
    let med_sys = insert_system(
        "Medical Systems",
        system_types::MEDICAL,
        power_priorities::HIGH,
    );

    let diag = insert_subsystem(
        med_sys,
        "Diagnostic Scanner",
        subsystem_types::DIAGNOSTIC_SCANNER,
        medical_node,
        4.0,
        1,
    );
    insert_component(
        diag,
        "Body Scanner",
        component_types::SCANNER_HEAD,
        0.0,
        0.0,
        300.0,
    );
    insert_component(
        diag,
        "Scanner Display",
        component_types::DISPLAY,
        1.0,
        0.0,
        200.0,
    );

    let lab = insert_subsystem(
        med_sys,
        "Lab Analyzer",
        subsystem_types::LAB_ANALYZER,
        medical_node,
        3.0,
        1,
    );
    insert_component(
        lab,
        "Chemical Analyzer",
        component_types::PROCESSOR,
        0.0,
        0.0,
        250.0,
    );

    let surgery_sub = insert_subsystem(
        med_sys,
        "Surgical Suite",
        subsystem_types::SURGICAL_SUITE,
        medical_node,
        8.0,
        2,
    );
    insert_component(
        surgery_sub,
        "Surgical Arm",
        component_types::ACTUATOR,
        -1.0,
        0.0,
        400.0,
    );
    insert_component(
        surgery_sub,
        "Surgical Display",
        component_types::DISPLAY,
        1.0,
        0.0,
        200.0,
    );

    let cryo = insert_subsystem(
        med_sys,
        "Cryo Pods",
        subsystem_types::CRYO_POD,
        medical_node,
        6.0,
        0,
    );
    insert_component(
        cryo,
        "Cryo Pod A",
        component_types::COMPRESSOR,
        -1.0,
        0.0,
        600.0,
    );
    insert_component(
        cryo,
        "Cryo Pod B",
        component_types::COMPRESSOR,
        1.0,
        0.0,
        600.0,
    );

    // ---- INFRASTRUCTURE EDGES (resource flow graph) ----
    // Power flow
    insert_infra(
        reactor_node,
        power_dist_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        100.0,
    );
    insert_infra(
        power_dist_node,
        engineering_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        90.0,
    );
    insert_infra(
        engineering_node,
        ls_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        40.0,
    );
    insert_infra(
        engineering_node,
        cooling_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        20.0,
    );
    insert_infra(
        engineering_node,
        hvac_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        10.0,
    );
    insert_infra(
        engineering_node,
        water_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        15.0,
    );
    insert_infra(
        engineering_node,
        hydro_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        15.0,
    );
    insert_infra(
        engineering_node,
        galley_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        10.0,
    );
    insert_infra(
        engineering_node,
        bridge_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        10.0,
    );
    insert_infra(
        engineering_node,
        comms_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        8.0,
    );
    insert_infra(
        engineering_node,
        medical_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        12.0,
    );

    // Coolant flow
    insert_infra(
        reactor_node,
        cooling_node,
        edge_types::COOLANT,
        infra_types::COOLANT_PIPE,
        50.0,
    );
    insert_infra(
        cooling_node,
        ls_node,
        edge_types::COOLANT,
        infra_types::COOLANT_PIPE,
        30.0,
    );

    // Water flow
    insert_infra(
        waste_node,
        water_node,
        edge_types::WATER,
        infra_types::WATER_PIPE,
        30.0,
    );
    insert_infra(
        water_node,
        galley_node,
        edge_types::WATER,
        infra_types::WATER_PIPE,
        10.0,
    );
    insert_infra(
        water_node,
        hydro_node,
        edge_types::WATER,
        infra_types::WATER_PIPE,
        10.0,
    );
    insert_infra(
        water_node,
        medical_node,
        edge_types::WATER,
        infra_types::WATER_PIPE,
        5.0,
    );

    // HVAC flow
    insert_infra(
        hvac_node,
        ls_node,
        edge_types::HVAC,
        infra_types::HVAC_DUCT,
        40.0,
    );
    insert_infra(
        hvac_node,
        bridge_node,
        edge_types::HVAC,
        infra_types::HVAC_DUCT,
        10.0,
    );
    insert_infra(
        hvac_node,
        medical_node,
        edge_types::HVAC,
        infra_types::HVAC_DUCT,
        10.0,
    );

    // Data connections
    insert_infra(
        comms_node,
        bridge_node,
        edge_types::DATA,
        infra_types::DATA_CABLE,
        10.0,
    );
    insert_infra(
        comms_node,
        engineering_node,
        edge_types::DATA,
        infra_types::DATA_CABLE,
        10.0,
    );
    insert_infra(
        comms_node,
        medical_node,
        edge_types::DATA,
        infra_types::DATA_CABLE,
        5.0,
    );
}
// ============================================================================
// STEP 4: GENERATE ATMOSPHERES
// ============================================================================

fn generate_atmospheres(ctx: &ReducerContext, deck_count: u32) {
    for deck in 0..deck_count as i32 {
        ctx.db.deck_atmosphere().insert(DeckAtmosphere {
            deck,
            oxygen: 0.21,
            co2: 0.0004,
            humidity: 0.45,
            temperature: 22.0,
            pressure: 101.3,
        });
    }
}

// ============================================================================
// STEP 5: GENERATE CREW
// ============================================================================

fn generate_crew(ctx: &ReducerContext, count: u32) {
    let dept_cycle = [
        departments::ENGINEERING,
        departments::MEDICAL,
        departments::SCIENCE,
        departments::SECURITY,
        departments::OPERATIONS,
        departments::COMMAND,
    ];

    for i in 0..count {
        let given_idx = i as usize % GIVEN_NAMES.len();
        let family_idx = (i as usize / GIVEN_NAMES.len() + i as usize * 7) % FAMILY_NAMES.len();

        let person_id = ctx
            .db
            .person()
            .insert(Person {
                id: 0,
                given_name: GIVEN_NAMES[given_idx].to_string(),
                family_name: FAMILY_NAMES[family_idx].to_string(),
                is_crew: true,
                is_player: false,
                owner_identity: None,
            })
            .id;

        let dept = dept_cycle[i as usize % dept_cycle.len()];
        let rank = if i < 3 {
            ranks::LIEUTENANT
        } else if i < 10 {
            ranks::SPECIALIST
        } else {
            ranks::CREWMAN
        };
        let shift = (i % 3) as u8;

        // Assign duty station based on department
        let duty_room_type = match dept {
            departments::ENGINEERING => room_types::ENGINEERING,
            departments::MEDICAL => room_types::HOSPITAL_WARD,
            departments::SCIENCE => room_types::LABORATORY,
            departments::SECURITY => room_types::SECURITY_OFFICE,
            departments::COMMAND => room_types::BRIDGE,
            _ => room_types::CORRIDOR,
        };
        let duty_station_id = ctx
            .db
            .room()
            .iter()
            .find(|r| r.room_type == duty_room_type)
            .map(|r| r.id)
            .unwrap_or(0);

        // Place crew in their duty station room
        let spawn_room = ctx
            .db
            .room()
            .id()
            .find(duty_station_id)
            .or_else(|| ctx.db.room().id().find(0));
        let (sx, sy, sw, sh, spawn_rid) = spawn_room
            .map(|r| (r.x, r.y, r.width, r.height, r.id))
            .unwrap_or((0.0, 0.0, 6.0, 50.0, 0));
        let spread_x = (i % 2) as f32 * 2.0 - 1.0;
        let spread_y = (i as f32 / 2.0).rem_euclid(sh - 2.0) - (sh / 2.0 - 1.0);
        ctx.db.position().insert(Position {
            person_id,
            room_id: spawn_rid,
            x: sx + spread_x.clamp(-sw / 2.0 + 0.5, sw / 2.0 - 0.5),
            y: sy + spread_y.clamp(-sh / 2.0 + 0.5, sh / 2.0 - 0.5),
            z: 0.0,
        });

        ctx.db.needs().insert(Needs {
            person_id,
            hunger: 0.15 + (i % 5) as f32 * 0.05,
            fatigue: 0.2 + (i % 4) as f32 * 0.05,
            social: 0.3 + (i % 3) as f32 * 0.1,
            comfort: 0.1 + (i % 6) as f32 * 0.03,
            hygiene: 0.1 + (i % 7) as f32 * 0.02,
            health: 1.0,
            morale: 0.7 + (i % 5) as f32 * 0.05,
        });

        let base = (i as f32 * 0.618033988) % 1.0;
        ctx.db.personality().insert(Personality {
            person_id,
            openness: 0.3 + base * 0.4,
            conscientiousness: 0.4 + ((base * 3.0) % 1.0) * 0.3,
            extraversion: 0.3 + ((base * 5.0) % 1.0) * 0.4,
            agreeableness: 0.4 + ((base * 7.0) % 1.0) * 0.3,
            neuroticism: 0.2 + ((base * 11.0) % 1.0) * 0.3,
        });

        ctx.db.crew().insert(Crew {
            person_id,
            department: dept,
            rank,
            shift,
            duty_station_id,
            on_duty: shift == shifts::ALPHA,
        });

        let (eng, med, pilot, sci, soc, combat) = match dept {
            departments::ENGINEERING => (0.7, 0.1, 0.2, 0.3, 0.2, 0.1),
            departments::MEDICAL => (0.1, 0.8, 0.1, 0.4, 0.5, 0.1),
            departments::SCIENCE => (0.3, 0.2, 0.1, 0.8, 0.3, 0.1),
            departments::SECURITY => (0.2, 0.2, 0.2, 0.1, 0.3, 0.8),
            departments::COMMAND => (0.3, 0.2, 0.5, 0.3, 0.6, 0.3),
            _ => (0.3, 0.2, 0.2, 0.2, 0.3, 0.2),
        };
        ctx.db.skills().insert(Skills {
            person_id,
            engineering: eng,
            medical: med,
            piloting: pilot,
            science: sci,
            social: soc,
            combat,
        });

        ctx.db.activity().insert(Activity {
            person_id,
            activity_type: activity_types::IDLE,
            started_at: 0.0,
            duration: 0.5,
            target_room_id: None,
        });
    }
}

// ============================================================================
// STEP 6: GENERATE PASSENGERS
// ============================================================================

fn generate_passengers(ctx: &ReducerContext, count: u32, _deck_count: u32) {
    let professions = [
        "Colonist",
        "Scientist",
        "Engineer",
        "Teacher",
        "Doctor",
        "Artist",
        "Farmer",
        "Merchant",
        "Writer",
        "Architect",
    ];

    // Find passenger quarters room
    let passenger_room_id = ctx
        .db
        .room()
        .iter()
        .find(|r| r.room_type == room_types::QUARTERS_PASSENGER)
        .map(|r| r.id)
        .unwrap_or(0);

    for i in 0..count {
        let given_idx = (i as usize + 40) % GIVEN_NAMES.len();
        let family_idx = (i as usize * 13 + 5) % FAMILY_NAMES.len();

        let person_id = ctx
            .db
            .person()
            .insert(Person {
                id: 0,
                given_name: GIVEN_NAMES[given_idx].to_string(),
                family_name: FAMILY_NAMES[family_idx].to_string(),
                is_crew: false,
                is_player: false,
                owner_identity: None,
            })
            .id;

        let (rx, ry, rw, rh) = ctx
            .db
            .room()
            .id()
            .find(passenger_room_id)
            .map(|r| (r.x, r.y, r.width, r.height))
            .unwrap_or((0.0, 0.0, 24.0, 18.0));
        let spread_x = ((i as f32 * 1.7) % (rw - 2.0)) - (rw / 2.0 - 1.0);
        let spread_y = ((i as f32 * 2.3) % (rh - 2.0)) - (rh / 2.0 - 1.0);
        ctx.db.position().insert(Position {
            person_id,
            room_id: passenger_room_id,
            x: rx + spread_x,
            y: ry + spread_y,
            z: 0.0,
        });

        ctx.db.needs().insert(Needs {
            person_id,
            hunger: 0.2 + (i % 4) as f32 * 0.05,
            fatigue: 0.15 + (i % 5) as f32 * 0.04,
            social: 0.4 + (i % 3) as f32 * 0.1,
            comfort: 0.2 + (i % 6) as f32 * 0.03,
            hygiene: 0.15 + (i % 7) as f32 * 0.02,
            health: 1.0,
            morale: 0.7 + (i % 4) as f32 * 0.06,
        });

        let base = ((i + 40) as f32 * 0.618033988) % 1.0;
        ctx.db.personality().insert(Personality {
            person_id,
            openness: 0.4 + base * 0.3,
            conscientiousness: 0.3 + ((base * 3.0) % 1.0) * 0.4,
            extraversion: 0.4 + ((base * 5.0) % 1.0) * 0.3,
            agreeableness: 0.5 + ((base * 7.0) % 1.0) * 0.2,
            neuroticism: 0.2 + ((base * 11.0) % 1.0) * 0.4,
        });

        let cabin = if i < count / 10 {
            cabin_classes::FIRST
        } else if i < count / 2 {
            cabin_classes::STANDARD
        } else {
            cabin_classes::STEERAGE
        };

        ctx.db.passenger().insert(Passenger {
            person_id,
            cabin_class: cabin,
            destination: "Kepler-442b".to_string(),
            profession: professions[i as usize % professions.len()].to_string(),
        });

        ctx.db.skills().insert(Skills {
            person_id,
            engineering: 0.1 + ((i as f32 * 0.3) % 0.3),
            medical: 0.1 + ((i as f32 * 0.2) % 0.2),
            piloting: 0.05,
            science: 0.2 + ((i as f32 * 0.25) % 0.3),
            social: 0.3 + ((i as f32 * 0.15) % 0.3),
            combat: 0.05,
        });

        ctx.db.activity().insert(Activity {
            person_id,
            activity_type: activity_types::IDLE,
            started_at: 0.0,
            duration: 0.5,
            target_room_id: None,
        });
    }
}
