//! Dynamic facility manifest — system-driven room generation.
//!
//! Replaces hardcoded room counts with a pipeline that derives room
//! requirements from:
//! - Selected system variants (each requires specific room types)
//! - Population size (quarters, dining, medical scale with people)
//! - Budget class (premium = more recreation, larger rooms)
//! - Fixed infrastructure (command, security always present)
//!
//! Room counts use four scaling modes:
//! - Fixed: always N rooms regardless of population
//! - PerPopulation: 1 room per N people
//! - PerSystem: 1 room per installed system of a type
//! - PerDeck: 1 room per habitable deck

use serde::{Deserialize, Serialize};

use crate::config::SystemSelection;
use crate::constants::{groups, room_types};
use crate::population::PopulationProfile;
use crate::systems::*;

/// A single room requirement in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomRequirement {
    pub room_type: u8,
    pub name: String,
    pub count: u32,
    pub target_area: f32,
    pub capacity: u32,
    pub group: u8,
    pub deck_zone: DeckZone,
}

/// Where on the ship this room type should be placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeckZone {
    /// Forward decks (command, sensors).
    Forward,
    /// Central decks (habitation, services).
    Central,
    /// Aft decks (engineering, propulsion).
    Aft,
    /// Any deck.
    Any,
}

/// Generate the complete facility manifest from mission parameters.
#[allow(clippy::vec_init_then_push)]
pub fn generate_manifest(
    systems: &SystemSelection,
    population: &PopulationProfile,
    budget_class: u8,
) -> Vec<RoomRequirement> {
    let pop = population.departure_total;
    let crew = population.total_crew;
    let passengers = population.total_passengers;

    let mut manifest = Vec::new();

    // ── Command (Fixed) ───────────────────────────────────────
    manifest.push(room(
        room_types::BRIDGE,
        "Bridge",
        1,
        80.0,
        20,
        groups::COMMAND,
        DeckZone::Forward,
    ));
    manifest.push(room(
        room_types::CIC,
        "Combat Information Center",
        1,
        60.0,
        15,
        groups::COMMAND,
        DeckZone::Forward,
    ));
    manifest.push(room(
        room_types::CONFERENCE,
        "Conference Room",
        1 + pop / 2000,
        50.0,
        30,
        groups::COMMAND,
        DeckZone::Forward,
    ));
    manifest.push(room(
        room_types::CAPTAINS_READY_ROOM,
        "Captain's Ready Room",
        1,
        25.0,
        2,
        groups::COMMAND,
        DeckZone::Forward,
    ));
    manifest.push(room(
        room_types::COMMS_ROOM,
        "Communications Room",
        1,
        30.0,
        8,
        groups::COMMAND,
        DeckZone::Forward,
    ));
    manifest.push(room(
        room_types::ADMIN_OFFICE,
        "Admin Office",
        1 + pop / 3000,
        30.0,
        10,
        groups::COMMAND,
        DeckZone::Forward,
    ));
    manifest.push(room(
        room_types::OBSERVATORY,
        "Observatory",
        1,
        60.0,
        20,
        groups::COMMAND,
        DeckZone::Forward,
    ));

    // ── Security (Fixed + per-pop) ────────────────────────────
    manifest.push(room(
        room_types::SECURITY_OFFICE,
        "Security Office",
        1 + pop / 3000,
        40.0,
        10,
        groups::SECURITY,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::BRIG,
        "Brig",
        1,
        30.0,
        8,
        groups::SECURITY,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::ARMORY,
        "Armory",
        1,
        25.0,
        4,
        groups::SECURITY,
        DeckZone::Central,
    ));

    // ── Habitation (per-pop) ──────────────────────────────────
    // Crew quarters: ~4 crew per room
    let crew_quarters = (crew / 4).max(1);
    manifest.push(room(
        room_types::QUARTERS_CREW,
        "Crew Quarters",
        crew_quarters,
        16.0,
        4,
        groups::HABITATION,
        DeckZone::Central,
    ));

    // Officer quarters: 1 per ~20 crew (single occupancy)
    let officer_quarters = (crew / 20).max(1);
    manifest.push(room(
        room_types::QUARTERS_OFFICER,
        "Officer Quarters",
        officer_quarters,
        20.0,
        1,
        groups::HABITATION,
        DeckZone::Central,
    ));

    // Passenger cabins based on budget
    if passengers > 0 {
        match budget_class {
            1 => {
                // Austere: mostly shared quarters
                manifest.push(room(
                    room_types::QUARTERS_PASSENGER,
                    "Passenger Quarters",
                    passengers / 6 + 1,
                    20.0,
                    6,
                    groups::HABITATION,
                    DeckZone::Central,
                ));
            }
            3 => {
                // Premium: mix of doubles and suites
                let doubles = passengers / 3;
                let suites = passengers / 10;
                manifest.push(room(
                    room_types::CABIN_DOUBLE,
                    "Double Cabin",
                    doubles.max(1),
                    18.0,
                    2,
                    groups::HABITATION,
                    DeckZone::Central,
                ));
                manifest.push(room(
                    room_types::FAMILY_SUITE,
                    "Family Suite",
                    suites.max(1),
                    35.0,
                    4,
                    groups::HABITATION,
                    DeckZone::Central,
                ));
                manifest.push(room(
                    room_types::VIP_SUITE,
                    "VIP Suite",
                    (passengers / 50).max(1),
                    50.0,
                    2,
                    groups::HABITATION,
                    DeckZone::Central,
                ));
            }
            _ => {
                // Standard: mix of singles and doubles
                let singles = passengers / 4;
                let doubles = passengers / 4;
                manifest.push(room(
                    room_types::CABIN_SINGLE,
                    "Single Cabin",
                    singles.max(1),
                    12.0,
                    1,
                    groups::HABITATION,
                    DeckZone::Central,
                ));
                manifest.push(room(
                    room_types::CABIN_DOUBLE,
                    "Double Cabin",
                    doubles.max(1),
                    18.0,
                    2,
                    groups::HABITATION,
                    DeckZone::Central,
                ));
            }
        }
    }

    // Shared facilities: 1 bathroom per 20 people, 1 laundry per 50
    manifest.push(room(
        room_types::SHARED_BATHROOM,
        "Shared Bathroom",
        (pop / 20).max(1),
        15.0,
        6,
        groups::HABITATION,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::SHARED_LAUNDRY,
        "Shared Laundry",
        (pop / 50).max(1),
        20.0,
        4,
        groups::HABITATION,
        DeckZone::Central,
    ));

    // ── Food Service (per-pop) ────────────────────────────────
    manifest.push(room(
        room_types::MESS_HALL,
        "Mess Hall",
        1 + pop / 500,
        120.0,
        100,
        groups::FOOD_SERVICE,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::GALLEY,
        "Galley",
        1 + pop / 1000,
        40.0,
        10,
        groups::FOOD_SERVICE,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::WARDROOM,
        "Wardroom",
        1,
        40.0,
        20,
        groups::FOOD_SERVICE,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::CAFE,
        "Café",
        (pop / 1000).max(1),
        35.0,
        25,
        groups::FOOD_SERVICE,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::BAKERY,
        "Bakery",
        1,
        25.0,
        5,
        groups::FOOD_SERVICE,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::FOOD_STORAGE_COLD,
        "Cold Storage",
        1 + pop / 2000,
        30.0,
        2,
        groups::FOOD_SERVICE,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::FOOD_STORAGE_DRY,
        "Dry Storage",
        1 + pop / 2000,
        30.0,
        2,
        groups::FOOD_SERVICE,
        DeckZone::Central,
    ));

    // ── Medical (per-pop + system-driven) ─────────────────────
    let med_variant = MedicalVariant::all()
        .iter()
        .find(|v| **v as u8 == systems.medical);
    let med_rooms = match med_variant {
        Some(MedicalVariant::AdvancedHospital) | Some(MedicalVariant::AutoDoc) => 2,
        _ => 1,
    };
    manifest.push(room(
        room_types::HOSPITAL_WARD,
        "Hospital Ward",
        med_rooms,
        60.0,
        20,
        groups::MEDICAL,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::SURGERY,
        "Surgery",
        1,
        40.0,
        5,
        groups::MEDICAL,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::DENTAL_CLINIC,
        "Dental Clinic",
        (pop / 2000).max(1),
        20.0,
        3,
        groups::MEDICAL,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::PHARMACY,
        "Pharmacy",
        1,
        15.0,
        3,
        groups::MEDICAL,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::MENTAL_HEALTH,
        "Mental Health",
        (pop / 2000).max(1),
        20.0,
        4,
        groups::MEDICAL,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::QUARANTINE,
        "Quarantine",
        1,
        30.0,
        8,
        groups::MEDICAL,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::MORGUE,
        "Morgue",
        1,
        15.0,
        0,
        groups::MEDICAL,
        DeckZone::Central,
    ));

    // ── Recreation (per-pop, budget scaled) ───────────────────
    let rec_mult = match budget_class {
        1 => 0.5_f32,
        3 => 1.5,
        _ => 1.0,
    };
    let rec_count = |base: u32| -> u32 { ((base as f32 * rec_mult).ceil() as u32).max(1) };

    manifest.push(room(
        room_types::GYM,
        "Gymnasium",
        rec_count(1 + pop / 1000),
        80.0,
        40,
        groups::RECREATION,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::THEATRE,
        "Theatre",
        rec_count(1),
        100.0,
        200,
        groups::RECREATION,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::LIBRARY,
        "Library",
        rec_count(1),
        50.0,
        30,
        groups::RECREATION,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::BAR,
        "Bar",
        rec_count(pop / 1500),
        40.0,
        30,
        groups::RECREATION,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::CHAPEL,
        "Chapel",
        1,
        40.0,
        40,
        groups::RECREATION,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::GAME_ROOM,
        "Game Room",
        rec_count(pop / 2000),
        35.0,
        20,
        groups::RECREATION,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::ART_STUDIO,
        "Art Studio",
        rec_count(1),
        30.0,
        15,
        groups::RECREATION,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::MUSIC_ROOM,
        "Music Room",
        rec_count(1),
        25.0,
        10,
        groups::RECREATION,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::ARBORETUM,
        "Arboretum",
        rec_count(1),
        150.0,
        50,
        groups::RECREATION,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::OBSERVATION_LOUNGE,
        "Observation Lounge",
        rec_count(1),
        60.0,
        30,
        groups::RECREATION,
        DeckZone::Central,
    ));

    if budget_class >= 2 {
        manifest.push(room(
            room_types::POOL,
            "Pool",
            1,
            80.0,
            30,
            groups::RECREATION,
            DeckZone::Central,
        ));
        manifest.push(room(
            room_types::LOUNGE,
            "Lounge",
            (pop / 2000).max(1),
            40.0,
            20,
            groups::RECREATION,
            DeckZone::Central,
        ));
        manifest.push(room(
            room_types::SHOPS,
            "Shops",
            (pop / 3000).max(1),
            30.0,
            10,
            groups::RECREATION,
            DeckZone::Central,
        ));
    }
    if budget_class >= 3 {
        manifest.push(room(
            room_types::HOLODECK,
            "Holodeck",
            (pop / 2000).max(1),
            50.0,
            10,
            groups::RECREATION,
            DeckZone::Central,
        ));
    }

    manifest.push(room(
        room_types::NURSERY,
        "Nursery",
        (pop / 2000).max(1),
        30.0,
        15,
        groups::RECREATION,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::SCHOOL,
        "School",
        (pop / 1000).max(1),
        50.0,
        30,
        groups::RECREATION,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::RECREATION,
        "Recreation Center",
        rec_count(1),
        60.0,
        40,
        groups::RECREATION,
        DeckZone::Central,
    ));

    // ── Engineering (system-driven) ───────────────────────────
    manifest.push(room(
        room_types::ENGINEERING,
        "Main Engineering",
        1,
        100.0,
        15,
        groups::ENGINEERING,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::POWER_DISTRIBUTION,
        "Power Distribution",
        1,
        30.0,
        4,
        groups::ENGINEERING,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::ELECTRONICS_LAB,
        "Electronics Lab",
        1,
        30.0,
        6,
        groups::ENGINEERING,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::COOLING_PLANT,
        "Cooling Plant",
        1,
        40.0,
        4,
        groups::ENGINEERING,
        DeckZone::Aft,
    ));

    // ── Workshop ──────────────────────────────────────────────
    manifest.push(room(
        room_types::MACHINE_SHOP,
        "Machine Shop",
        1,
        50.0,
        8,
        groups::WORKSHOP,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::MAINTENANCE_BAY,
        "Maintenance Bay",
        1 + pop / 3000,
        40.0,
        6,
        groups::WORKSHOP,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::ROBOTICS_BAY,
        "Robotics Bay",
        1,
        35.0,
        4,
        groups::WORKSHOP,
        DeckZone::Aft,
    ));

    // ── Propulsion (system-driven) ────────────────────────────
    // Reactor count depends on power system
    let reactor_count = match PowerVariant::all()
        .iter()
        .find(|v| **v as u8 == systems.power)
    {
        Some(PowerVariant::SolarArray) | Some(PowerVariant::RTG) => 0,
        _ => 1,
    };
    if reactor_count > 0 {
        manifest.push(room(
            room_types::REACTOR,
            "Reactor",
            reactor_count,
            80.0,
            10,
            groups::PROPULSION,
            DeckZone::Aft,
        ));
    }
    manifest.push(room(
        room_types::BACKUP_REACTOR,
        "Backup Reactor",
        1,
        30.0,
        4,
        groups::PROPULSION,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::ENGINE_ROOM,
        "Engine Room",
        1,
        80.0,
        10,
        groups::PROPULSION,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::FUEL_STORAGE,
        "Fuel Storage",
        1 + pop / 5000,
        50.0,
        2,
        groups::PROPULSION,
        DeckZone::Aft,
    ));

    // ── Life Support (system-driven) ──────────────────────────
    let hydro_count = match FoodVariant::all()
        .iter()
        .find(|v| **v as u8 == systems.food)
    {
        Some(FoodVariant::BasicHydroponics) | Some(FoodVariant::AdvancedAeroponics) => {
            2 + pop / 1000
        }
        _ => 1,
    };
    manifest.push(room(
        room_types::HYDROPONICS,
        "Hydroponics Bay",
        hydro_count,
        80.0,
        8,
        groups::LIFE_SUPPORT,
        DeckZone::Central,
    ));
    manifest.push(room(
        room_types::ATMOSPHERE_PROCESSING,
        "Atmosphere Processing",
        1 + pop / 2000,
        40.0,
        6,
        groups::LIFE_SUPPORT,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::WATER_RECYCLING,
        "Water Recycling",
        1,
        35.0,
        4,
        groups::LIFE_SUPPORT,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::WATER_PURIFICATION,
        "Water Purification",
        1,
        25.0,
        3,
        groups::LIFE_SUPPORT,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::WASTE_PROCESSING,
        "Waste Processing",
        1,
        30.0,
        4,
        groups::LIFE_SUPPORT,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::ENV_MONITORING,
        "Environmental Monitoring",
        1,
        20.0,
        4,
        groups::LIFE_SUPPORT,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::LIFE_SUPPORT,
        "Life Support Center",
        1,
        40.0,
        6,
        groups::LIFE_SUPPORT,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::HVAC_CONTROL,
        "HVAC Control",
        1,
        20.0,
        3,
        groups::LIFE_SUPPORT,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::LABORATORY,
        "Laboratory",
        1,
        40.0,
        8,
        groups::LIFE_SUPPORT,
        DeckZone::Central,
    ));

    // ── Cargo (per-pop) ───────────────────────────────────────
    manifest.push(room(
        room_types::CARGO_BAY,
        "Cargo Bay",
        2 + pop / 2000,
        100.0,
        4,
        groups::CARGO,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::STORAGE,
        "Storage",
        2 + pop / 1500,
        40.0,
        2,
        groups::CARGO,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::PARTS_STORAGE,
        "Parts Storage",
        1,
        30.0,
        2,
        groups::CARGO,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::SHUTTLE_BAY,
        "Shuttle Bay",
        1,
        120.0,
        10,
        groups::CARGO,
        DeckZone::Aft,
    ));
    manifest.push(room(
        room_types::AIRLOCK,
        "Airlock",
        2,
        15.0,
        4,
        groups::CARGO,
        DeckZone::Any,
    ));

    manifest
}

fn room(
    room_type: u8,
    name: &str,
    count: u32,
    target_area: f32,
    capacity: u32,
    group: u8,
    deck_zone: DeckZone,
) -> RoomRequirement {
    RoomRequirement {
        room_type,
        name: name.to_string(),
        count,
        target_area,
        capacity,
        group,
        deck_zone,
    }
}

/// Total area required by all rooms in the manifest.
pub fn total_area(manifest: &[RoomRequirement]) -> f32 {
    manifest
        .iter()
        .map(|r| r.target_area * r.count as f32)
        .sum()
}

/// Total room count in the manifest.
pub fn total_rooms(manifest: &[RoomRequirement]) -> u32 {
    manifest.iter().map(|r| r.count).sum()
}

/// Estimate the number of habitable decks needed.
///
/// Based on total area / deck area (hull_width × hull_length).
pub fn estimate_deck_count(manifest: &[RoomRequirement], hull_width: f32, hull_length: f32) -> u32 {
    let deck_area = hull_width * hull_length;
    if deck_area <= 0.0 {
        return 1;
    }
    let area = total_area(manifest);
    // 70% packing efficiency (corridors, walls, etc.)
    let usable_area_per_deck = deck_area * 0.7;
    ((area / usable_area_per_deck).ceil() as u32).max(3) // minimum 3 decks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{select_systems, SystemOverrides};
    use crate::mission::MissionConfig;
    use crate::population::compute_population;

    fn default_manifest() -> Vec<RoomRequirement> {
        let config = MissionConfig::default();
        let systems = select_systems(&config, &SystemOverrides::default());
        let pop = compute_population(&config, &systems);
        generate_manifest(&systems, &pop, config.budget_class)
    }

    #[test]
    fn test_manifest_has_rooms() {
        let m = default_manifest();
        assert!(!m.is_empty());
        assert!(total_rooms(&m) > 50, "should have many rooms");
    }

    #[test]
    fn test_manifest_has_bridge() {
        let m = default_manifest();
        assert!(
            m.iter().any(|r| r.room_type == room_types::BRIDGE),
            "must have a bridge"
        );
    }

    #[test]
    fn test_manifest_has_quarters() {
        let m = default_manifest();
        let quarters = m
            .iter()
            .filter(|r| room_types::is_quarters(r.room_type))
            .count();
        assert!(quarters > 0, "must have quarters");
    }

    #[test]
    fn test_manifest_has_medical() {
        let m = default_manifest();
        let medical = m.iter().filter(|r| r.group == groups::MEDICAL).count();
        assert!(medical >= 3, "should have multiple medical rooms");
    }

    #[test]
    fn test_total_area_positive() {
        let m = default_manifest();
        let area = total_area(&m);
        assert!(area > 1000.0, "total area should be substantial: {area}");
    }

    #[test]
    fn test_deck_count_reasonable() {
        let m = default_manifest();
        let decks = estimate_deck_count(&m, 65.0, 400.0);
        assert!(decks >= 3, "should need at least 3 decks");
        assert!(decks <= 100, "should not need 100 decks");
    }

    #[test]
    fn test_all_rooms_have_group() {
        let m = default_manifest();
        for r in &m {
            assert!(
                r.group <= groups::INFRASTRUCTURE,
                "room {} has invalid group {}",
                r.name,
                r.group
            );
        }
    }

    #[test]
    fn test_premium_has_more_recreation() {
        let config = MissionConfig::default();
        let systems = select_systems(&config, &SystemOverrides::default());
        let pop = compute_population(&config, &systems);

        let austere = generate_manifest(&systems, &pop, 1);
        let premium = generate_manifest(&systems, &pop, 3);

        let rec_a: u32 = austere
            .iter()
            .filter(|r| r.group == groups::RECREATION)
            .map(|r| r.count)
            .sum();
        let rec_p: u32 = premium
            .iter()
            .filter(|r| r.group == groups::RECREATION)
            .map(|r| r.count)
            .sum();
        assert!(
            rec_p >= rec_a,
            "premium should have at least as much recreation"
        );
    }

    #[test]
    fn test_premium_has_holodeck() {
        let config = MissionConfig::default();
        let systems = select_systems(&config, &SystemOverrides::default());
        let pop = compute_population(&config, &systems);
        let m = generate_manifest(&systems, &pop, 3);
        assert!(
            m.iter().any(|r| r.room_type == room_types::HOLODECK),
            "premium should have holodeck"
        );
    }

    #[test]
    fn test_larger_pop_more_rooms() {
        let config = MissionConfig::default();
        let systems = select_systems(&config, &SystemOverrides::default());

        let small_pop = PopulationProfile {
            departure_total: 500,
            total_crew: 100,
            total_passengers: 400,
            arrival_target: 1000,
            estimated_arrival: 1000,
            department_crew: crate::population::DepartmentCrew {
                command: 10,
                engineering: 20,
                medical: 10,
                science: 10,
                security: 10,
                operations: 20,
                civilian: 20,
            },
            genetic_diversity_ok: true,
        };
        let large_pop = PopulationProfile {
            departure_total: 5000,
            total_crew: 1000,
            total_passengers: 4000,
            arrival_target: 10000,
            estimated_arrival: 10000,
            department_crew: crate::population::DepartmentCrew {
                command: 50,
                engineering: 200,
                medical: 100,
                science: 50,
                security: 50,
                operations: 200,
                civilian: 350,
            },
            genetic_diversity_ok: true,
        };

        let small = generate_manifest(&systems, &small_pop, 2);
        let large = generate_manifest(&systems, &large_pop, 2);

        assert!(
            total_rooms(&large) > total_rooms(&small),
            "larger pop should need more rooms"
        );
    }
}
