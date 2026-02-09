//! Hull dimensions and room size calculations.
//!
//! Constants and functions for computing hull taper and room dimensions.

/// Width of main corridors (spine)
#[allow(dead_code)]
pub const CORRIDOR_WIDTH: f32 = 6.0;

/// Half-width of main corridors
#[allow(dead_code)]
pub const CORRIDOR_HALF: f32 = CORRIDOR_WIDTH / 2.0;

/// Width of service corridors
#[allow(dead_code)]
pub const SERVICE_CORRIDOR_WIDTH: f32 = 3.0;

/// X-position of service corridor center
#[allow(dead_code)]
pub const SERVICE_X: f32 = -(CORRIDOR_HALF + SERVICE_CORRIDOR_WIDTH / 2.0);

/// Ship length in meters
pub const SHIP_LENGTH: usize = 400;

/// Ship beam (width) in meters
pub const SHIP_BEAM: usize = 65;

/// Width of spine corridor in grid cells
pub const SPINE_WIDTH: usize = 3;

/// Width of cross-corridors in grid cells
pub const CROSS_CORRIDOR_WIDTH: usize = 3;

/// Spacing between cross-corridors
pub const CROSS_CORRIDOR_SPACING: usize = 50;

/// Width of service corridor in grid cells
pub const SVC_CORRIDOR_WIDTH: usize = 2;

/// Returns base area for a room type in square meters.
#[allow(dead_code)]
pub fn base_area(function: u8) -> f32 {
    use crate::tables::room_types;
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

/// Compute room dimensions (width, height) from required area.
/// Returns dimensions with aspect ratio between 1:1 and 2:1.
#[allow(dead_code)]
pub fn compute_room_dims(required_area: f32) -> (f32, f32) {
    // Aspect ratio between 1:1 and 2:1
    let w = required_area.sqrt() * 1.2;
    let h = required_area / w;
    (w.max(4.0), h.max(4.0))
}
