//! Facility specifications and deck zone mapping.
//!
//! Defines the complete manifest of ship facilities (rooms) with their properties,
//! and maps facility zones to deck ranges for vertical ship organization.

use crate::tables::{groups, room_types};

/// Facility manifest entry — describes one kind of room to instantiate.
pub(super) struct FacilitySpec {
    pub name: &'static str,
    pub room_type: u8,
    pub target_area: f32,
    pub capacity: u32,
    pub count: u32,
    pub deck_zone: u8, // 0=command, 1=hab, 2=services, 3=rec, 4=lifesup, 5=cargo, 6=eng
    pub group: u8,
}

/// Deck-zone → deck range mapping.
pub(super) fn deck_range_for_zone(zone: u8, deck_count: u32) -> (u32, u32) {
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

/// Returns the complete facility manifest for ship generation.
pub(super) fn get_facility_manifest() -> Vec<FacilitySpec> {
    vec![
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_facility_manifest_not_empty() {
        let manifest = get_facility_manifest();
        assert!(
            !manifest.is_empty(),
            "Facility manifest should not be empty"
        );
        assert!(manifest.len() > 20, "Ship should have many facility types");
    }

    #[test]
    fn test_all_facilities_have_valid_specs() {
        let manifest = get_facility_manifest();

        for (i, spec) in manifest.iter().enumerate() {
            assert!(!spec.name.is_empty(), "Facility {} should have a name", i);
            assert!(
                spec.target_area > 0.0,
                "Facility {} '{}' should have positive area",
                i,
                spec.name
            );
            assert!(
                spec.count > 0,
                "Facility {} '{}' should have count > 0",
                i,
                spec.name
            );
            assert!(
                spec.deck_zone < 10,
                "Facility {} '{}' should have valid deck zone",
                i,
                spec.name
            );
        }
    }

    #[test]
    fn test_facility_room_counts() {
        let manifest = get_facility_manifest();

        // Count total rooms
        let total_rooms: u32 = manifest.iter().map(|f| f.count).sum();
        assert!(
            total_rooms >= 50,
            "Ship should have at least 50 rooms total"
        );

        // Check for key facilities
        let bridge_count = manifest.iter().filter(|f| f.name == "Bridge").count();
        assert_eq!(bridge_count, 1, "Should have exactly one Bridge definition");

        let mess_halls: u32 = manifest
            .iter()
            .filter(|f| f.name == "Mess Hall")
            .map(|f| f.count)
            .sum();
        assert!(mess_halls > 0, "Should have at least one mess hall");
    }

    #[test]
    fn test_deck_zone_ranges_valid() {
        let deck_count = 20;

        for zone in 0..7 {
            let (start, end) = deck_range_for_zone(zone, deck_count);
            assert!(
                start < deck_count,
                "Zone {} start deck {} should be less than deck count",
                zone,
                start
            );
            assert!(
                end <= deck_count,
                "Zone {} end deck {} should be <= deck count",
                zone,
                end
            );
            assert!(start < end, "Zone {} should have start < end", zone);
        }
    }

    #[test]
    fn test_deck_zone_command_at_top() {
        let deck_count = 20;
        let (start, end) = deck_range_for_zone(0, deck_count); // Command zone

        assert_eq!(start, 0, "Command zone should start at deck 0");
        assert!(end <= 3, "Command zone should be in upper decks");
    }

    #[test]
    fn test_deck_zone_engineering_at_bottom() {
        let deck_count = 20;
        let (start, end) = deck_range_for_zone(6, deck_count); // Engineering zone

        assert!(start >= 19, "Engineering zone should be in lower decks");
        assert_eq!(end, deck_count, "Engineering zone should extend to bottom");
    }

    #[test]
    fn test_facility_zones_match_manifest() {
        let manifest = get_facility_manifest();

        // Verify all deck_zone values are used
        let mut zones_used = vec![false; 7];
        for spec in &manifest {
            if spec.deck_zone < 7 {
                zones_used[spec.deck_zone as usize] = true;
            }
        }

        for (i, used) in zones_used.iter().enumerate() {
            assert!(*used, "Deck zone {} should have at least one facility", i);
        }
    }
}
