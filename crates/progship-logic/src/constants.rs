//! Game constants — room types, activity types, shifts, groups, etc.
//!
//! These are simple `u8` constants with no database dependency.
//! Both the SpacetimeDB server and the native simtest use these.

pub mod groups {
    pub const COMMAND: u8 = 0;
    pub const SECURITY: u8 = 1;
    pub const HABITATION: u8 = 2;
    pub const FOOD_SERVICE: u8 = 3;
    pub const MEDICAL: u8 = 4;
    pub const RECREATION: u8 = 5;
    pub const ENGINEERING: u8 = 6;
    pub const WORKSHOP: u8 = 7;
    pub const PROPULSION: u8 = 8;
    pub const LIFE_SUPPORT: u8 = 9;
    pub const CARGO: u8 = 10;
    pub const INFRASTRUCTURE: u8 = 11;
}

pub mod room_types {
    // Command & Administration
    pub const BRIDGE: u8 = 0;
    pub const CONFERENCE: u8 = 1;
    pub const CIC: u8 = 2;
    pub const COMMS_ROOM: u8 = 3;
    pub const CAPTAINS_READY_ROOM: u8 = 4;
    pub const SECURITY_OFFICE: u8 = 5;
    pub const BRIG: u8 = 6;
    pub const ADMIN_OFFICE: u8 = 7;
    pub const OBSERVATORY: u8 = 8;
    // Habitation
    pub const CABIN_SINGLE: u8 = 10;
    pub const CABIN_DOUBLE: u8 = 11;
    pub const FAMILY_SUITE: u8 = 12;
    pub const VIP_SUITE: u8 = 13;
    pub const QUARTERS_CREW: u8 = 14;
    pub const QUARTERS_OFFICER: u8 = 15;
    pub const QUARTERS_PASSENGER: u8 = 16;
    pub const SHARED_BATHROOM: u8 = 17;
    pub const SHARED_LAUNDRY: u8 = 18;
    // Food & Dining
    pub const MESS_HALL: u8 = 20;
    pub const WARDROOM: u8 = 21;
    pub const GALLEY: u8 = 22;
    pub const FOOD_STORAGE_COLD: u8 = 23;
    pub const FOOD_STORAGE_DRY: u8 = 24;
    pub const CAFE: u8 = 25;
    pub const BAKERY: u8 = 26;
    pub const WATER_PURIFICATION: u8 = 27;
    // Medical
    pub const HOSPITAL_WARD: u8 = 30;
    pub const SURGERY: u8 = 31;
    pub const DENTAL_CLINIC: u8 = 32;
    pub const PHARMACY: u8 = 33;
    pub const MENTAL_HEALTH: u8 = 34;
    pub const QUARANTINE: u8 = 35;
    pub const MORGUE: u8 = 36;
    pub const MEDBAY: u8 = 37;
    // Recreation & Morale
    pub const GYM: u8 = 40;
    pub const THEATRE: u8 = 41;
    pub const LIBRARY: u8 = 42;
    pub const CHAPEL: u8 = 43;
    pub const GAME_ROOM: u8 = 44;
    pub const BAR: u8 = 45;
    pub const ART_STUDIO: u8 = 46;
    pub const MUSIC_ROOM: u8 = 47;
    pub const HOLODECK: u8 = 48;
    pub const ARBORETUM: u8 = 49;
    pub const OBSERVATION_LOUNGE: u8 = 50;
    pub const POOL: u8 = 51;
    pub const NURSERY: u8 = 52;
    pub const SCHOOL: u8 = 53;
    pub const RECREATION: u8 = 54;
    pub const LOUNGE: u8 = 55;
    pub const SHOPS: u8 = 56;
    // Engineering & Propulsion
    pub const ENGINEERING: u8 = 60;
    pub const MAIN_ENGINEERING: u8 = 60; // alias for clarity in deck_heights
    pub const REACTOR: u8 = 61;
    pub const BACKUP_REACTOR: u8 = 62;
    pub const ENGINE_ROOM: u8 = 63;
    pub const POWER_DISTRIBUTION: u8 = 64;
    pub const MACHINE_SHOP: u8 = 65;
    pub const ELECTRONICS_LAB: u8 = 66;
    pub const PARTS_STORAGE: u8 = 67;
    pub const FUEL_STORAGE: u8 = 68;
    pub const ROBOTICS_BAY: u8 = 69;
    pub const MAINTENANCE_BAY: u8 = 70;
    pub const COOLING_PLANT: u8 = 71;
    // Life Support
    pub const HYDROPONICS: u8 = 80;
    pub const ATMOSPHERE_PROCESSING: u8 = 81;
    pub const WATER_RECYCLING: u8 = 82;
    pub const WASTE_PROCESSING: u8 = 83;
    pub const ENV_MONITORING: u8 = 84;
    pub const LIFE_SUPPORT: u8 = 85;
    pub const HVAC_CONTROL: u8 = 86;
    // Cargo & Logistics
    pub const CARGO_BAY: u8 = 90;
    pub const STORAGE: u8 = 91;
    pub const ARMORY: u8 = 92;
    pub const SHUTTLE_BAY: u8 = 93;
    pub const AIRLOCK: u8 = 94;
    pub const LABORATORY: u8 = 95;
    // Infrastructure (not placeable rooms)
    pub const CORRIDOR: u8 = 100;
    pub const SERVICE_CORRIDOR: u8 = 101;
    pub const CROSS_CORRIDOR: u8 = 102;
    pub const ELEVATOR_SHAFT: u8 = 110;
    pub const LADDER_SHAFT: u8 = 111;
    pub const SERVICE_ELEVATOR_SHAFT: u8 = 112;
    pub const SERVICE_DECK: u8 = 120;

    /// Returns true if this room type is any kind of sleeping quarters
    pub fn is_quarters(rt: u8) -> bool {
        matches!(
            rt,
            CABIN_SINGLE
                | CABIN_DOUBLE
                | FAMILY_SUITE
                | VIP_SUITE
                | QUARTERS_CREW
                | QUARTERS_OFFICER
                | QUARTERS_PASSENGER
        )
    }
    /// Returns true if this room type is a dining/food area
    pub fn is_dining(rt: u8) -> bool {
        matches!(rt, MESS_HALL | WARDROOM | CAFE | GALLEY)
    }
    /// Returns true if this room type is recreation/social
    pub fn is_recreation(rt: u8) -> bool {
        matches!(
            rt,
            GYM | THEATRE
                | LIBRARY
                | CHAPEL
                | GAME_ROOM
                | BAR
                | ART_STUDIO
                | MUSIC_ROOM
                | HOLODECK
                | ARBORETUM
                | OBSERVATION_LOUNGE
                | POOL
                | NURSERY
                | SCHOOL
                | RECREATION
                | LOUNGE
                | SHOPS
        )
    }
    /// Returns true if this room type is a corridor/infrastructure
    pub fn is_corridor(rt: u8) -> bool {
        rt >= 100
    }
    /// Returns true if this room type is a walkable corridor (not a shaft or service deck)
    pub fn is_plain_corridor(rt: u8) -> bool {
        matches!(rt, CORRIDOR | SERVICE_CORRIDOR | CROSS_CORRIDOR)
    }
    /// Returns true if this room type is a vertical shaft (elevator, ladder, service elevator)
    pub fn is_shaft(rt: u8) -> bool {
        matches!(rt, ELEVATOR_SHAFT | LADDER_SHAFT | SERVICE_ELEVATOR_SHAFT)
    }
    /// Returns true if this room type is a medical facility
    pub fn is_medical(rt: u8) -> bool {
        matches!(
            rt,
            HOSPITAL_WARD | SURGERY | DENTAL_CLINIC | PHARMACY | QUARANTINE
        )
    }
}

/// Returns a short label for a room type, suitable for minimap and door signage.
/// Uses ASCII-safe abbreviations guaranteed to render in any font.
pub fn room_type_icon(rt: u8) -> &'static str {
    match rt {
        // Command & Administration
        room_types::BRIDGE => "BR",
        room_types::CONFERENCE => "CF",
        room_types::CIC => "CIC",
        room_types::COMMS_ROOM => "COM",
        room_types::CAPTAINS_READY_ROOM => "CPT",
        room_types::SECURITY_OFFICE => "SEC",
        room_types::BRIG => "BRG",
        room_types::ADMIN_OFFICE => "ADM",
        room_types::OBSERVATORY => "OBS",
        // Habitation
        room_types::CABIN_SINGLE => "C1",
        room_types::CABIN_DOUBLE => "C2",
        room_types::FAMILY_SUITE => "FS",
        room_types::VIP_SUITE => "VIP",
        room_types::QUARTERS_CREW => "CQ",
        room_types::QUARTERS_OFFICER => "OQ",
        room_types::QUARTERS_PASSENGER => "PQ",
        room_types::SHARED_BATHROOM => "WC",
        room_types::SHARED_LAUNDRY => "LA",
        // Food & Dining
        room_types::MESS_HALL => "MH",
        room_types::WARDROOM => "WR",
        room_types::GALLEY => "GL",
        room_types::FOOD_STORAGE_COLD => "CS",
        room_types::FOOD_STORAGE_DRY => "DS",
        room_types::CAFE => "CA",
        room_types::BAKERY => "BK",
        room_types::WATER_PURIFICATION => "WP",
        // Medical
        room_types::HOSPITAL_WARD => "H+",
        room_types::SURGERY => "SU",
        room_types::DENTAL_CLINIC => "DT",
        room_types::PHARMACY => "RX",
        room_types::MENTAL_HEALTH => "MH",
        room_types::QUARANTINE => "QT",
        room_types::MORGUE => "MG",
        room_types::MEDBAY => "M+",
        // Recreation & Morale
        room_types::GYM => "GY",
        room_types::THEATRE => "TH",
        room_types::LIBRARY => "LB",
        room_types::CHAPEL => "CH",
        room_types::GAME_ROOM => "GM",
        room_types::BAR => "BA",
        room_types::ART_STUDIO => "AR",
        room_types::MUSIC_ROOM => "MU",
        room_types::HOLODECK => "HD",
        room_types::ARBORETUM => "AB",
        room_types::OBSERVATION_LOUNGE => "OL",
        room_types::POOL => "PL",
        room_types::NURSERY => "NU",
        room_types::SCHOOL => "SC",
        room_types::RECREATION => "RC",
        room_types::LOUNGE => "LG",
        room_types::SHOPS => "SH",
        // Engineering & Propulsion
        room_types::ENGINEERING => "EN",
        room_types::REACTOR => "RX!",
        room_types::BACKUP_REACTOR => "BR!",
        room_types::ENGINE_ROOM => "ENG",
        room_types::POWER_DISTRIBUTION => "PD",
        room_types::MACHINE_SHOP => "MS",
        room_types::ELECTRONICS_LAB => "EL",
        room_types::PARTS_STORAGE => "PS",
        room_types::FUEL_STORAGE => "FU",
        room_types::ROBOTICS_BAY => "RB",
        room_types::MAINTENANCE_BAY => "MB",
        room_types::COOLING_PLANT => "CP",
        // Life Support
        room_types::HYDROPONICS => "HY",
        room_types::ATMOSPHERE_PROCESSING => "AT",
        room_types::WATER_RECYCLING => "WR",
        room_types::WASTE_PROCESSING => "WS",
        room_types::ENV_MONITORING => "EM",
        room_types::LIFE_SUPPORT => "LS",
        room_types::HVAC_CONTROL => "HV",
        // Cargo & Logistics
        room_types::CARGO_BAY => "CG",
        room_types::STORAGE => "ST",
        room_types::ARMORY => "AM",
        room_types::SHUTTLE_BAY => "SB",
        room_types::AIRLOCK => "AL",
        room_types::LABORATORY => "LAB",
        // Infrastructure
        room_types::ELEVATOR_SHAFT => "EV",
        room_types::LADDER_SHAFT => "LD",
        room_types::SERVICE_ELEVATOR_SHAFT => "SE",
        _ => "",
    }
}

pub mod deck_heights {
    use super::room_types;

    /// Minimum deck height in meters (floor-to-ceiling for standard rooms).
    pub const MIN_DECK_HEIGHT: f32 = 3.5;

    /// Standard personnel door opening height in meters.
    pub const STANDARD_DOOR_HEIGHT: f32 = 2.4;

    /// Equipment / large-access door opening height in meters.
    pub const EQUIPMENT_DOOR_HEIGHT: f32 = 3.0;

    /// Returns the ceiling height for a given room type, in meters.
    ///
    /// Multi-deck rooms (reactor, engine room, cargo bay, shuttle bay) get
    /// double height. All other rooms use the base deck height.
    pub fn room_ceiling_height(room_type: u8) -> f32 {
        let span = room_deck_span(room_type) as f32;
        MIN_DECK_HEIGHT * span
    }

    /// Returns how many decks a room of this type spans (1, 2, or 3).
    pub fn room_deck_span(room_type: u8) -> u8 {
        match room_type {
            room_types::SHUTTLE_BAY => 3,
            room_types::REACTOR
            | room_types::ENGINE_ROOM
            | room_types::CARGO_BAY
            | room_types::MAIN_ENGINEERING
            | room_types::BACKUP_REACTOR
            | room_types::FUEL_STORAGE
            | room_types::HYDROPONICS
            | room_types::ARBORETUM
            | room_types::THEATRE
            | room_types::POOL => 2,
            _ => 1,
        }
    }

    /// Returns the door opening height for a doorway between two room types.
    ///
    /// Equipment-height doors are used when either side is an engineering,
    /// cargo, maintenance, or shuttle room. All others get standard height.
    pub fn door_opening_height(rt_a: u8, rt_b: u8) -> f32 {
        if is_equipment_door_room(rt_a) || is_equipment_door_room(rt_b) {
            EQUIPMENT_DOOR_HEIGHT
        } else {
            STANDARD_DOOR_HEIGHT
        }
    }

    /// Returns true if doors to/from this room type should use equipment height.
    fn is_equipment_door_room(rt: u8) -> bool {
        matches!(
            rt,
            room_types::ENGINEERING
                | room_types::REACTOR
                | room_types::BACKUP_REACTOR
                | room_types::ENGINE_ROOM
                | room_types::MACHINE_SHOP
                | room_types::FUEL_STORAGE
                | room_types::ROBOTICS_BAY
                | room_types::MAINTENANCE_BAY
                | room_types::COOLING_PLANT
                | room_types::CARGO_BAY
                | room_types::SHUTTLE_BAY
                | room_types::AIRLOCK
        )
    }
}

/// Room placement constraints for generation.
///
/// Encoded as u8 for compactness and serialization.
pub mod placement {
    use super::room_types;

    pub const NONE: u8 = 0;
    pub const HULL_FACING: u8 = 1; // Must touch ship exterior (perimeter ring)
    pub const INTERIOR: u8 = 2; // Must NOT touch hull (protected/shielded)
    pub const AFT: u8 = 3; // Prefer aft (high-Y) section
    pub const FORWARD: u8 = 4; // Prefer forward (low-Y) section

    /// Returns the placement constraint for a room type.
    pub fn room_placement(room_type: u8) -> u8 {
        match room_type {
            // Hull-facing: viewports, launch doors, exhaust, antennas, radiators, venting
            room_types::OBSERVATORY
            | room_types::OBSERVATION_LOUNGE
            | room_types::COMMS_ROOM
            | room_types::VIP_SUITE
            | room_types::SHUTTLE_BAY
            | room_types::AIRLOCK
            | room_types::CARGO_BAY
            | room_types::FUEL_STORAGE
            | room_types::COOLING_PLANT => HULL_FACING,

            // Interior/protected: shielding, security, contamination containment
            room_types::REACTOR
            | room_types::CIC
            | room_types::BRIG
            | room_types::ARMORY
            | room_types::WATER_PURIFICATION
            | room_types::QUARANTINE
            | room_types::WATER_RECYCLING
            | room_types::WASTE_PROCESSING
            | room_types::HOLODECK => INTERIOR,

            // Aft section: propulsion, engineering
            room_types::ENGINE_ROOM | room_types::ENGINEERING => AFT,

            // Forward: command
            room_types::BRIDGE => FORWARD,

            _ => NONE,
        }
    }

    /// Parses a placement string from the manifest JSON.
    pub fn from_str(s: &str) -> u8 {
        match s {
            "hull_facing" => HULL_FACING,
            "interior" => INTERIOR,
            "aft" => AFT,
            "forward" => FORWARD,
            _ => NONE,
        }
    }
}

pub mod shifts {
    pub const ALPHA: u8 = 0; // 0600-1400
    pub const BETA: u8 = 1; // 1400-2200
    pub const GAMMA: u8 = 2; // 2200-0600
}

pub mod activity_types {
    pub const IDLE: u8 = 0;
    pub const WORKING: u8 = 1;
    pub const EATING: u8 = 2;
    pub const SLEEPING: u8 = 3;
    pub const SOCIALIZING: u8 = 4;
    pub const RELAXING: u8 = 5;
    pub const HYGIENE: u8 = 6;
    pub const TRAVELING: u8 = 7;
    pub const MAINTENANCE: u8 = 8;
    pub const ON_DUTY: u8 = 9;
    pub const OFF_DUTY: u8 = 10;
    pub const EMERGENCY: u8 = 11;
    pub const EXERCISING: u8 = 12;
}

pub mod departments {
    pub const COMMAND: u8 = 0;
    pub const ENGINEERING: u8 = 1;
    pub const MEDICAL: u8 = 2;
    pub const SCIENCE: u8 = 3;
    pub const SECURITY: u8 = 4;
    pub const OPERATIONS: u8 = 5;
    pub const CIVILIAN: u8 = 6;
}

pub mod ranks {
    pub const CREWMAN: u8 = 0;
    pub const SPECIALIST: u8 = 1;
    pub const PETTY: u8 = 2;
    pub const CHIEF: u8 = 3;
    pub const ENSIGN: u8 = 4;
    pub const LIEUTENANT: u8 = 5;
    pub const COMMANDER: u8 = 6;
    pub const CAPTAIN: u8 = 7;
}

pub mod system_types {
    pub const POWER: u8 = 0;
    pub const LIFE_SUPPORT: u8 = 1;
    pub const PROPULSION: u8 = 2;
    pub const NAVIGATION: u8 = 3;
    pub const COMMUNICATIONS: u8 = 4;
    pub const WEAPONS: u8 = 5;
    pub const SHIELDS: u8 = 6;
    pub const MEDICAL: u8 = 7;
    pub const FOOD_PRODUCTION: u8 = 8;
    pub const WATER_RECYCLING: u8 = 9;
    pub const GRAVITY: u8 = 10;
}

pub mod event_types {
    pub const SYSTEM_FAILURE: u8 = 0;
    pub const MEDICAL_EMERGENCY: u8 = 1;
    pub const FIRE: u8 = 2;
    pub const HULL_BREACH: u8 = 3;
    pub const DISCOVERY: u8 = 4;
    pub const CELEBRATION: u8 = 5;
    pub const ALTERCATION: u8 = 6;
    pub const RESOURCE_SHORTAGE: u8 = 7;
    pub const DEATH: u8 = 8;
}

#[cfg(test)]
mod tests {
    use super::deck_heights::*;
    use super::room_types;

    #[test]
    fn standard_rooms_single_deck() {
        assert_eq!(room_deck_span(room_types::BRIDGE), 1);
        assert_eq!(room_deck_span(room_types::CABIN_SINGLE), 1);
        assert_eq!(room_deck_span(room_types::CORRIDOR), 1);
        assert_eq!(room_deck_span(room_types::GYM), 1);
    }

    #[test]
    fn multi_deck_rooms() {
        assert_eq!(room_deck_span(room_types::REACTOR), 2);
        assert_eq!(room_deck_span(room_types::ENGINE_ROOM), 2);
        assert_eq!(room_deck_span(room_types::CARGO_BAY), 2);
        assert_eq!(room_deck_span(room_types::SHUTTLE_BAY), 3);
        assert_eq!(room_deck_span(room_types::ENGINEERING), 2);
        assert_eq!(room_deck_span(room_types::BACKUP_REACTOR), 2);
        assert_eq!(room_deck_span(room_types::FUEL_STORAGE), 2);
        assert_eq!(room_deck_span(room_types::HYDROPONICS), 2);
        assert_eq!(room_deck_span(room_types::ARBORETUM), 2);
        assert_eq!(room_deck_span(room_types::THEATRE), 2);
        assert_eq!(room_deck_span(room_types::POOL), 2);
    }

    #[test]
    fn ceiling_heights() {
        assert!((room_ceiling_height(room_types::BRIDGE) - 3.5).abs() < 0.001);
        assert!((room_ceiling_height(room_types::REACTOR) - 7.0).abs() < 0.001);
        assert!((room_ceiling_height(room_types::CORRIDOR) - 3.5).abs() < 0.001);
        assert!((room_ceiling_height(room_types::SHUTTLE_BAY) - 10.5).abs() < 0.001);
    }

    #[test]
    fn door_heights_standard() {
        let h = door_opening_height(room_types::CORRIDOR, room_types::CABIN_SINGLE);
        assert!((h - STANDARD_DOOR_HEIGHT).abs() < 0.001);
    }

    #[test]
    fn door_heights_equipment() {
        let h = door_opening_height(room_types::CORRIDOR, room_types::CARGO_BAY);
        assert!((h - EQUIPMENT_DOOR_HEIGHT).abs() < 0.001);
        let h2 = door_opening_height(room_types::ENGINE_ROOM, room_types::CORRIDOR);
        assert!((h2 - EQUIPMENT_DOOR_HEIGHT).abs() < 0.001);
    }

    #[test]
    fn door_heights_both_standard() {
        let h = door_opening_height(room_types::MESS_HALL, room_types::GALLEY);
        assert!((h - STANDARD_DOOR_HEIGHT).abs() < 0.001);
    }

    #[test]
    fn placement_constraints() {
        use super::placement;
        assert_eq!(
            placement::room_placement(room_types::SHUTTLE_BAY),
            placement::HULL_FACING
        );
        assert_eq!(
            placement::room_placement(room_types::REACTOR),
            placement::INTERIOR
        );
        assert_eq!(
            placement::room_placement(room_types::ENGINE_ROOM),
            placement::AFT
        );
        assert_eq!(
            placement::room_placement(room_types::BRIDGE),
            placement::FORWARD
        );
        assert_eq!(
            placement::room_placement(room_types::CABIN_SINGLE),
            placement::NONE
        );
    }

    #[test]
    fn placement_from_str() {
        use super::placement;
        assert_eq!(placement::from_str("hull_facing"), placement::HULL_FACING);
        assert_eq!(placement::from_str("interior"), placement::INTERIOR);
        assert_eq!(placement::from_str("aft"), placement::AFT);
        assert_eq!(placement::from_str("forward"), placement::FORWARD);
        assert_eq!(placement::from_str("none"), placement::NONE);
        assert_eq!(placement::from_str(""), placement::NONE);
    }
}
