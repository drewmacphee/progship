//! Facility specifications and deck zone mapping.
//!
//! Defines the complete manifest of ship facilities (rooms) with their properties,
//! and maps facility zones to deck ranges for vertical ship organization.
//!
//! Room data is loaded from `data/facility_manifest.json` at compile time via
//! `include_str!()`. To add or modify room types, edit the JSON file — no code
//! changes required.

use serde::Deserialize;

/// Facility manifest entry — describes one kind of room to instantiate.
///
/// Deserialized from `data/facility_manifest.json`.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct FacilitySpec {
    pub name: String,
    pub room_type: u8,
    pub target_area: f32,
    pub capacity: u32,
    pub count: u32,
    pub deck_zone: u8, // 0=command, 1=hab, 2=services, 3=rec, 4=lifesup, 5=cargo, 6=eng
    pub group: u8,
    #[serde(default)]
    pub placement: String, // "none", "hull_facing", "interior", "aft", "forward"
}

/// Deck-zone → deck range mapping.
/// Proportionally distributes zones across the available deck count.
pub(super) fn deck_range_for_zone(zone: u8, deck_count: u32) -> (u32, u32) {
    let dc = deck_count;
    // Zone weights: how many "slices" each zone ideally occupies (out of 20)
    // 0=command(2), 1=hab(8), 2=services(2), 3=rec(2), 4=lifesup(3), 5=cargo(1), 6=eng(2)
    let weights: [u32; 7] = [2, 8, 2, 2, 3, 1, 2];
    let total_weight: u32 = weights.iter().sum(); // 20

    // Compute cumulative boundaries scaled to deck_count
    let mut boundaries = [0u32; 8];
    let mut accum = 0u32;
    for (i, &w) in weights.iter().enumerate() {
        accum += w;
        // Round to nearest deck, ensuring last boundary = deck_count
        boundaries[i + 1] = if i == 6 {
            dc
        } else {
            ((accum as f32 / total_weight as f32) * dc as f32).round() as u32
        };
    }

    let z = zone.min(6) as usize;
    let lo = boundaries[z].min(dc);
    let hi = boundaries[z + 1].min(dc);
    // Ensure every zone gets at least 1 deck (share with neighbor if needed)
    if lo >= hi && dc > 0 {
        let clamped_lo = lo.min(dc - 1);
        (clamped_lo, (clamped_lo + 1).min(dc))
    } else {
        (lo, hi)
    }
}

/// Returns the complete facility manifest for ship generation.
///
/// Loaded from `data/facility_manifest.json` embedded at compile time.
pub(super) fn get_facility_manifest() -> Vec<FacilitySpec> {
    const MANIFEST_JSON: &str = include_str!("../../../../data/facility_manifest.json");
    serde_json::from_str(MANIFEST_JSON).expect("facility_manifest.json is invalid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::{groups, room_types};

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
    fn test_json_room_types_match_constants() {
        let manifest = get_facility_manifest();
        let bridge = manifest.iter().find(|f| f.name == "Bridge").unwrap();
        assert_eq!(bridge.room_type, room_types::BRIDGE);
        assert_eq!(bridge.group, groups::COMMAND);

        let reactor = manifest.iter().find(|f| f.name == "Reactor").unwrap();
        assert_eq!(reactor.room_type, room_types::REACTOR);
        assert_eq!(reactor.group, groups::PROPULSION);

        let cabin = manifest.iter().find(|f| f.name == "Single Cabin").unwrap();
        assert_eq!(cabin.room_type, room_types::CABIN_SINGLE);
        assert_eq!(cabin.group, groups::HABITATION);

        let hydro = manifest
            .iter()
            .find(|f| f.name == "Hydroponics Bay")
            .unwrap();
        assert_eq!(hydro.room_type, room_types::HYDROPONICS);
        assert_eq!(hydro.group, groups::LIFE_SUPPORT);

        let hospital = manifest.iter().find(|f| f.name == "Hospital Ward").unwrap();
        assert_eq!(hospital.group, groups::MEDICAL);

        let cargo = manifest.iter().find(|f| f.name == "Cargo Bay").unwrap();
        assert_eq!(cargo.group, groups::CARGO);

        let gym = manifest.iter().find(|f| f.name == "Gym").unwrap();
        assert_eq!(gym.group, groups::RECREATION);

        let mess = manifest.iter().find(|f| f.name == "Mess Hall").unwrap();
        assert_eq!(mess.group, groups::FOOD_SERVICE);

        let brig = manifest.iter().find(|f| f.name == "Brig").unwrap();
        assert_eq!(brig.group, groups::SECURITY);

        let shop = manifest.iter().find(|f| f.name == "Machine Shop").unwrap();
        assert_eq!(shop.group, groups::WORKSHOP);
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
