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
