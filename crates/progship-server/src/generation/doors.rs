//! Door connectivity rules for direct room-to-room connections.
//!
//! Most rooms connect only to corridors. This module defines the special cases
//! where direct doors between rooms make logical sense (e.g., galley↔mess hall).

use crate::tables::room_types;

/// Returns true if two room types should have a direct door between them.
/// Most rooms connect to corridors only; direct room-to-room doors are for
/// logically connected pairs (e.g., galley↔mess, surgery↔hospital).
pub(super) fn should_have_room_door(a: u8, b: u8) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_galley_connects_to_mess_hall() {
        assert!(should_have_room_door(
            room_types::GALLEY,
            room_types::MESS_HALL
        ));
        assert!(should_have_room_door(
            room_types::MESS_HALL,
            room_types::GALLEY
        ));
    }

    #[test]
    fn test_surgery_connects_to_hospital() {
        assert!(should_have_room_door(
            room_types::SURGERY,
            room_types::HOSPITAL_WARD
        ));
        assert!(should_have_room_door(
            room_types::HOSPITAL_WARD,
            room_types::SURGERY
        ));
    }

    #[test]
    fn test_galley_connects_to_food_storage() {
        assert!(should_have_room_door(
            room_types::GALLEY,
            room_types::FOOD_STORAGE_COLD
        ));
        assert!(should_have_room_door(
            room_types::GALLEY,
            room_types::FOOD_STORAGE_DRY
        ));
        assert!(should_have_room_door(
            room_types::FOOD_STORAGE_COLD,
            room_types::GALLEY
        ));
        assert!(should_have_room_door(
            room_types::FOOD_STORAGE_DRY,
            room_types::GALLEY
        ));
    }

    #[test]
    fn test_pharmacy_connects_to_hospital() {
        assert!(should_have_room_door(
            room_types::PHARMACY,
            room_types::HOSPITAL_WARD
        ));
        assert!(should_have_room_door(
            room_types::HOSPITAL_WARD,
            room_types::PHARMACY
        ));
    }

    #[test]
    fn test_bridge_connects_to_command() {
        assert!(should_have_room_door(room_types::BRIDGE, room_types::CIC));
        assert!(should_have_room_door(
            room_types::BRIDGE,
            room_types::CAPTAINS_READY_ROOM
        ));
        assert!(should_have_room_door(room_types::CIC, room_types::BRIDGE));
        assert!(should_have_room_door(
            room_types::CAPTAINS_READY_ROOM,
            room_types::BRIDGE
        ));
    }

    #[test]
    fn test_engineering_connects_to_power() {
        assert!(should_have_room_door(
            room_types::ENGINEERING,
            room_types::REACTOR
        ));
        assert!(should_have_room_door(
            room_types::ENGINEERING,
            room_types::ENGINE_ROOM
        ));
        assert!(should_have_room_door(
            room_types::REACTOR,
            room_types::ENGINEERING
        ));
        assert!(should_have_room_door(
            room_types::ENGINE_ROOM,
            room_types::ENGINEERING
        ));
    }

    #[test]
    fn test_cabins_connect_to_shared_bathroom() {
        assert!(should_have_room_door(
            room_types::CABIN_SINGLE,
            room_types::SHARED_BATHROOM
        ));
        assert!(should_have_room_door(
            room_types::CABIN_DOUBLE,
            room_types::SHARED_BATHROOM
        ));
        assert!(should_have_room_door(
            room_types::QUARTERS_CREW,
            room_types::SHARED_BATHROOM
        ));
        assert!(should_have_room_door(
            room_types::SHARED_BATHROOM,
            room_types::CABIN_SINGLE
        ));
        assert!(should_have_room_door(
            room_types::SHARED_BATHROOM,
            room_types::CABIN_DOUBLE
        ));
        assert!(should_have_room_door(
            room_types::SHARED_BATHROOM,
            room_types::QUARTERS_CREW
        ));
    }

    #[test]
    fn test_unrelated_rooms_no_door() {
        // Most rooms should not have direct doors
        assert!(!should_have_room_door(
            room_types::BRIDGE,
            room_types::GALLEY
        ));
        assert!(!should_have_room_door(
            room_types::CABIN_SINGLE,
            room_types::ENGINEERING
        ));
        assert!(!should_have_room_door(room_types::GYM, room_types::REACTOR));
        assert!(!should_have_room_door(
            room_types::LIBRARY,
            room_types::HOSPITAL_WARD
        ));
    }

    #[test]
    fn test_order_independence() {
        // Function should be symmetric - order of arguments shouldn't matter
        let pairs = vec![
            (room_types::GALLEY, room_types::MESS_HALL),
            (room_types::SURGERY, room_types::HOSPITAL_WARD),
            (room_types::BRIDGE, room_types::CIC),
        ];

        for (a, b) in pairs {
            assert_eq!(
                should_have_room_door(a, b),
                should_have_room_door(b, a),
                "should_have_room_door({}, {}) should equal should_have_room_door({}, {})",
                a,
                b,
                b,
                a
            );
        }
    }

    #[test]
    fn test_same_room_no_door() {
        // A room shouldn't have a door to itself
        assert!(!should_have_room_door(
            room_types::BRIDGE,
            room_types::BRIDGE
        ));
        assert!(!should_have_room_door(
            room_types::GALLEY,
            room_types::GALLEY
        ));
        assert!(!should_have_room_door(
            room_types::CABIN_SINGLE,
            room_types::CABIN_SINGLE
        ));
    }
}
