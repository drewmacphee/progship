//! Security and access control logic.
//!
//! Doors have an `access_level` field that determines who can pass.
//! This module provides pure functions to check access permissions
//! based on a person's rank, department, and special overrides.
//!
//! # Access Levels
//!
//! | Level | Name | Who Can Pass |
//! |-------|------|-------------|
//! | 0 | Public | Everyone |
//! | 1 | Crew Only | Any crew member (not passengers) |
//! | 2 | Department | Crew in the room's department |
//! | 3 | Officer | Rank ≥ Ensign (4) |
//! | 4 | Captain | Captain only |
//!
//! # Lockdown
//!
//! During emergencies, doors can be locked down. Security personnel
//! and officers can override locked doors.
//!
//! ```
//! use progship_logic::security::{check_access, AccessRequest, AccessResult};
//!
//! let req = AccessRequest {
//!     door_access_level: 1,
//!     is_crew: true,
//!     rank: 2,
//!     department: 1,
//!     door_department: Some(1),
//!     is_lockdown: false,
//! };
//! assert!(check_access(&req).allowed);
//! ```

use serde::{Deserialize, Serialize};

/// Door access levels (matches existing constants in tables.rs).
pub mod access_levels {
    /// Anyone can pass (corridors, common areas, mess halls).
    pub const PUBLIC: u8 = 0;
    /// Crew members only (engineering sections, crew quarters).
    pub const CREW_ONLY: u8 = 1;
    /// Department-specific (e.g., only engineering crew in reactor room).
    pub const DEPARTMENT: u8 = 2;
    /// Officers only (rank ≥ Ensign).
    pub const OFFICER: u8 = 3;
    /// Captain only (bridge command, armory override).
    pub const CAPTAIN: u8 = 4;
}

/// Minimum rank required for officer-level access.
const OFFICER_RANK: u8 = 4; // Ensign

/// Captain rank value.
const CAPTAIN_RANK: u8 = 7;

/// Security department ID (from constants.rs).
const SECURITY_DEPARTMENT: u8 = 4;

/// A request to pass through a door.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRequest {
    /// The door's required access level (0–4).
    pub door_access_level: u8,
    /// Whether the person is crew (vs passenger).
    pub is_crew: bool,
    /// The person's rank (0–7, see constants.rs ranks).
    pub rank: u8,
    /// The person's department (0–6, see constants.rs departments).
    pub department: u8,
    /// The department that "owns" the door's area (if department-restricted).
    pub door_department: Option<u8>,
    /// Whether the ship is in lockdown mode.
    pub is_lockdown: bool,
}

/// Result of an access check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessResult {
    /// Whether access is granted.
    pub allowed: bool,
    /// Reason for denial (if denied).
    pub denial_reason: Option<DenialReason>,
    /// Whether this was an override (officer/captain bypassing normal rules).
    pub is_override: bool,
}

/// Why access was denied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DenialReason {
    /// Passenger trying to enter crew-only area.
    PassengerInCrewArea,
    /// Wrong department for department-restricted door.
    WrongDepartment,
    /// Rank too low for officer area.
    InsufficientRank,
    /// Not the captain for captain-only area.
    CaptainOnly,
    /// Door is locked down and person cannot override.
    Lockdown,
}

/// Check whether a person can pass through a door.
///
/// During lockdown, only security crew and officers (rank ≥ Ensign) can
/// pass through any door. The captain can always pass through any door.
pub fn check_access(req: &AccessRequest) -> AccessResult {
    // Captain always passes
    if req.rank >= CAPTAIN_RANK {
        return AccessResult {
            allowed: true,
            denial_reason: None,
            is_override: req.door_access_level > access_levels::PUBLIC || req.is_lockdown,
        };
    }

    // Lockdown check — only security crew and officers can override
    if req.is_lockdown {
        let can_override =
            req.is_crew && (req.department == SECURITY_DEPARTMENT || req.rank >= OFFICER_RANK);
        if !can_override {
            return AccessResult {
                allowed: false,
                denial_reason: Some(DenialReason::Lockdown),
                is_override: false,
            };
        }
        // If they can override lockdown, still check normal access below
    }

    match req.door_access_level {
        access_levels::PUBLIC => AccessResult {
            allowed: true,
            denial_reason: None,
            is_override: false,
        },

        access_levels::CREW_ONLY => {
            if req.is_crew {
                AccessResult {
                    allowed: true,
                    denial_reason: None,
                    is_override: false,
                }
            } else {
                AccessResult {
                    allowed: false,
                    denial_reason: Some(DenialReason::PassengerInCrewArea),
                    is_override: false,
                }
            }
        }

        access_levels::DEPARTMENT => {
            if !req.is_crew {
                return AccessResult {
                    allowed: false,
                    denial_reason: Some(DenialReason::PassengerInCrewArea),
                    is_override: false,
                };
            }
            // Officers can enter any department area
            if req.rank >= OFFICER_RANK {
                return AccessResult {
                    allowed: true,
                    denial_reason: None,
                    is_override: true,
                };
            }
            // Security can enter any department area (patrol access)
            if req.department == SECURITY_DEPARTMENT {
                return AccessResult {
                    allowed: true,
                    denial_reason: None,
                    is_override: true,
                };
            }
            // Check department match
            match req.door_department {
                Some(dept) if dept == req.department => AccessResult {
                    allowed: true,
                    denial_reason: None,
                    is_override: false,
                },
                _ => AccessResult {
                    allowed: false,
                    denial_reason: Some(DenialReason::WrongDepartment),
                    is_override: false,
                },
            }
        }

        access_levels::OFFICER => {
            if req.is_crew && req.rank >= OFFICER_RANK {
                AccessResult {
                    allowed: true,
                    denial_reason: None,
                    is_override: false,
                }
            } else {
                AccessResult {
                    allowed: false,
                    denial_reason: Some(DenialReason::InsufficientRank),
                    is_override: false,
                }
            }
        }

        access_levels::CAPTAIN => AccessResult {
            allowed: false,
            denial_reason: Some(DenialReason::CaptainOnly),
            is_override: false,
        },

        // Unknown access level — deny by default
        _ => AccessResult {
            allowed: false,
            denial_reason: Some(DenialReason::InsufficientRank),
            is_override: false,
        },
    }
}

/// Determine the appropriate access level for a room type.
///
/// Maps room types to their default door access level based on
/// the room's function and security requirements.
pub fn default_access_for_room(room_type: u8) -> u8 {
    use crate::constants::room_types as rt;
    match room_type {
        // Public areas — corridors, recreation, dining, quarters
        rt::CORRIDOR
        | rt::SERVICE_CORRIDOR
        | rt::CROSS_CORRIDOR
        | rt::MESS_HALL
        | rt::WARDROOM
        | rt::CAFE
        | rt::LOUNGE
        | rt::GYM
        | rt::LIBRARY
        | rt::CHAPEL
        | rt::OBSERVATION_LOUNGE
        | rt::THEATRE
        | rt::BAR
        | rt::GAME_ROOM
        | rt::ART_STUDIO
        | rt::MUSIC_ROOM
        | rt::HOLODECK
        | rt::ARBORETUM
        | rt::POOL
        | rt::NURSERY
        | rt::SCHOOL
        | rt::RECREATION
        | rt::SHOPS
        | rt::CABIN_SINGLE
        | rt::CABIN_DOUBLE
        | rt::FAMILY_SUITE
        | rt::VIP_SUITE
        | rt::QUARTERS_CREW
        | rt::QUARTERS_OFFICER
        | rt::QUARTERS_PASSENGER
        | rt::SHARED_BATHROOM
        | rt::SHARED_LAUNDRY => access_levels::PUBLIC,

        // Crew-only areas
        rt::GALLEY
        | rt::FOOD_STORAGE_COLD
        | rt::FOOD_STORAGE_DRY
        | rt::ADMIN_OFFICE
        | rt::CONFERENCE
        | rt::STORAGE
        | rt::CARGO_BAY
        | rt::SHUTTLE_BAY
        | rt::AIRLOCK
        | rt::BAKERY
        | rt::PARTS_STORAGE
        | rt::ELEVATOR_SHAFT
        | rt::LADDER_SHAFT
        | rt::SERVICE_ELEVATOR_SHAFT
        | rt::SERVICE_DECK => access_levels::CREW_ONLY,

        // Department-restricted — engineering
        rt::ENGINEERING
        | rt::REACTOR
        | rt::BACKUP_REACTOR
        | rt::ENGINE_ROOM
        | rt::POWER_DISTRIBUTION
        | rt::MACHINE_SHOP
        | rt::ELECTRONICS_LAB
        | rt::FUEL_STORAGE
        | rt::ROBOTICS_BAY
        | rt::MAINTENANCE_BAY
        | rt::COOLING_PLANT => access_levels::DEPARTMENT,

        // Department-restricted — medical
        rt::HOSPITAL_WARD
        | rt::SURGERY
        | rt::DENTAL_CLINIC
        | rt::PHARMACY
        | rt::MENTAL_HEALTH
        | rt::QUARANTINE
        | rt::MORGUE
        | rt::MEDBAY => access_levels::DEPARTMENT,

        // Department-restricted — science / life support
        rt::LABORATORY
        | rt::HYDROPONICS
        | rt::ATMOSPHERE_PROCESSING
        | rt::WATER_RECYCLING
        | rt::WATER_PURIFICATION
        | rt::WASTE_PROCESSING
        | rt::ENV_MONITORING
        | rt::LIFE_SUPPORT
        | rt::HVAC_CONTROL
        | rt::COMMS_ROOM => access_levels::DEPARTMENT,

        // Officer areas
        rt::BRIDGE | rt::CIC | rt::CAPTAINS_READY_ROOM | rt::OBSERVATORY => access_levels::OFFICER,

        // High security
        rt::ARMORY | rt::SECURITY_OFFICE | rt::BRIG => access_levels::OFFICER,

        // Default: crew only for unknown rooms
        _ => access_levels::CREW_ONLY,
    }
}

/// Patrol route types for security crew.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatrolType {
    /// Walk through public corridors and common areas.
    PublicAreas,
    /// Check crew-only and restricted areas.
    RestrictedAreas,
    /// Respond to a specific incident location.
    IncidentResponse,
}

/// Generate a list of room types that a security patrol should visit.
pub fn patrol_room_types(patrol: PatrolType) -> Vec<u8> {
    use crate::constants::room_types as rt;
    match patrol {
        PatrolType::PublicAreas => vec![
            rt::CORRIDOR,
            rt::MESS_HALL,
            rt::LOUNGE,
            rt::ARBORETUM,
            rt::SHOPS,
            rt::OBSERVATION_LOUNGE,
        ],
        PatrolType::RestrictedAreas => vec![
            rt::ENGINEERING,
            rt::REACTOR,
            rt::ARMORY,
            rt::CARGO_BAY,
            rt::SHUTTLE_BAY,
            rt::BRIDGE,
        ],
        PatrolType::IncidentResponse => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crew_request(rank: u8, department: u8, access_level: u8) -> AccessRequest {
        AccessRequest {
            door_access_level: access_level,
            is_crew: true,
            rank,
            department,
            door_department: Some(1), // Engineering
            is_lockdown: false,
        }
    }

    fn passenger_request(access_level: u8) -> AccessRequest {
        AccessRequest {
            door_access_level: access_level,
            is_crew: false,
            rank: 0,
            department: 6, // Civilian
            door_department: None,
            is_lockdown: false,
        }
    }

    #[test]
    fn public_door_allows_everyone() {
        assert!(check_access(&crew_request(0, 1, 0)).allowed);
        assert!(check_access(&passenger_request(0)).allowed);
    }

    #[test]
    fn crew_only_blocks_passengers() {
        let result = check_access(&passenger_request(1));
        assert!(!result.allowed);
        assert_eq!(
            result.denial_reason,
            Some(DenialReason::PassengerInCrewArea)
        );
    }

    #[test]
    fn crew_only_allows_any_crew() {
        assert!(check_access(&crew_request(0, 5, 1)).allowed); // Crewman, Operations
    }

    #[test]
    fn department_blocks_wrong_department() {
        let result = check_access(&crew_request(0, 3, 2)); // Science trying Engineering door
        assert!(!result.allowed);
        assert_eq!(result.denial_reason, Some(DenialReason::WrongDepartment));
    }

    #[test]
    fn department_allows_matching_department() {
        let result = check_access(&crew_request(0, 1, 2)); // Engineering crew, Engineering door
        assert!(result.allowed);
        assert!(!result.is_override);
    }

    #[test]
    fn officer_overrides_department() {
        let result = check_access(&crew_request(5, 3, 2)); // Lieutenant, Science, Engineering door
        assert!(result.allowed);
        assert!(result.is_override);
    }

    #[test]
    fn security_overrides_department() {
        let result = check_access(&crew_request(0, 4, 2)); // Crewman, Security, Engineering door
        assert!(result.allowed);
        assert!(result.is_override);
    }

    #[test]
    fn officer_area_blocks_low_rank() {
        let result = check_access(&crew_request(3, 0, 3)); // Chief, Command
        assert!(!result.allowed);
        assert_eq!(result.denial_reason, Some(DenialReason::InsufficientRank));
    }

    #[test]
    fn officer_area_allows_ensign() {
        assert!(check_access(&crew_request(4, 0, 3)).allowed);
    }

    #[test]
    fn captain_only_blocks_officers() {
        let result = check_access(&crew_request(6, 0, 4)); // Commander
        assert!(!result.allowed);
        assert_eq!(result.denial_reason, Some(DenialReason::CaptainOnly));
    }

    #[test]
    fn captain_always_passes() {
        for level in 0..=4 {
            let mut req = crew_request(7, 0, level);
            req.is_lockdown = false;
            assert!(
                check_access(&req).allowed,
                "captain blocked at level {level}"
            );
        }
    }

    #[test]
    fn captain_passes_during_lockdown() {
        let mut req = crew_request(7, 0, 2);
        req.is_lockdown = true;
        let result = check_access(&req);
        assert!(result.allowed);
        assert!(result.is_override);
    }

    #[test]
    fn lockdown_blocks_regular_crew() {
        let mut req = crew_request(0, 1, 0); // Public door
        req.is_lockdown = true;
        let result = check_access(&req);
        assert!(!result.allowed);
        assert_eq!(result.denial_reason, Some(DenialReason::Lockdown));
    }

    #[test]
    fn lockdown_allows_security() {
        let mut req = crew_request(0, 4, 0); // Security crewman
        req.is_lockdown = true;
        assert!(check_access(&req).allowed);
    }

    #[test]
    fn lockdown_allows_officers() {
        let mut req = crew_request(5, 1, 0); // Lieutenant, Engineering
        req.is_lockdown = true;
        assert!(check_access(&req).allowed);
    }

    #[test]
    fn lockdown_blocks_passengers() {
        let mut req = passenger_request(0);
        req.is_lockdown = true;
        let result = check_access(&req);
        assert!(!result.allowed);
        assert_eq!(result.denial_reason, Some(DenialReason::Lockdown));
    }

    #[test]
    fn default_access_public_rooms() {
        use crate::constants::room_types as rt;
        assert_eq!(default_access_for_room(rt::CORRIDOR), access_levels::PUBLIC);
        assert_eq!(
            default_access_for_room(rt::MESS_HALL),
            access_levels::PUBLIC
        );
        assert_eq!(default_access_for_room(rt::GYM), access_levels::PUBLIC);
    }

    #[test]
    fn default_access_department_rooms() {
        use crate::constants::room_types as rt;
        assert_eq!(
            default_access_for_room(rt::REACTOR),
            access_levels::DEPARTMENT
        );
        assert_eq!(
            default_access_for_room(rt::HOSPITAL_WARD),
            access_levels::DEPARTMENT
        );
        assert_eq!(
            default_access_for_room(rt::LABORATORY),
            access_levels::DEPARTMENT
        );
    }

    #[test]
    fn default_access_officer_rooms() {
        use crate::constants::room_types as rt;
        assert_eq!(default_access_for_room(rt::BRIDGE), access_levels::OFFICER);
    }

    #[test]
    fn patrol_public_areas() {
        let types = patrol_room_types(PatrolType::PublicAreas);
        assert!(!types.is_empty());
        assert!(types.contains(&crate::constants::room_types::CORRIDOR));
    }

    #[test]
    fn patrol_restricted_areas() {
        let types = patrol_room_types(PatrolType::RestrictedAreas);
        assert!(!types.is_empty());
        assert!(types.contains(&crate::constants::room_types::REACTOR));
    }

    #[test]
    fn acceptance_passenger_cannot_enter_crew_area() {
        let result = check_access(&passenger_request(1));
        assert!(!result.allowed, "passenger should not enter crew-only area");
    }

    #[test]
    fn acceptance_lockdown_seals_doors() {
        // Regular crew blocked during lockdown
        let mut req = crew_request(2, 1, 0);
        req.is_lockdown = true;
        assert!(
            !check_access(&req).allowed,
            "lockdown should seal doors for regular crew"
        );

        // Security can still pass
        let mut sec = crew_request(0, 4, 0);
        sec.is_lockdown = true;
        assert!(
            check_access(&sec).allowed,
            "security should override lockdown"
        );
    }
}
