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
