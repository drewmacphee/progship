//! SpacetimeDB table definitions for the colony ship simulation.
//!
//! Each table is equivalent to an ECS component from progship-core,
//! but stored in SpacetimeDB for persistence and multiplayer sync.

use spacetimedb::{table, Identity, Timestamp};

// ============================================================================
// SHIP CONFIGURATION
// ============================================================================

/// Ship configuration singleton (id always 0)
#[table(name = ship_config, public)]
#[derive(Clone)]
pub struct ShipConfig {
    #[primary_key]
    pub id: u32,
    pub name: String,
    pub deck_count: u32,
    pub crew_count: u32,
    pub passenger_count: u32,
    pub sim_time: f64,      // Simulation time in hours
    pub time_scale: f32,
    pub paused: bool,
}

// ============================================================================
// PEOPLE
// ============================================================================

/// Person - crew member, passenger, or player character
#[table(name = person, public)]
pub struct Person {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub given_name: String,
    pub family_name: String,
    pub is_crew: bool,
    pub is_player: bool,
    pub owner_identity: Option<Identity>,
}

/// Position in the ship
#[table(name = position, public)]
pub struct Position {
    #[primary_key]
    pub person_id: u64,
    pub room_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Active movement toward a destination
#[table(name = movement, public)]
#[derive(Clone)]
pub struct Movement {
    #[primary_key]
    pub person_id: u64,
    pub target_room_id: u32,
    pub target_x: f32,
    pub target_y: f32,
    pub target_z: f32,
    pub speed: f32,
    /// Serialized path as comma-separated room IDs
    pub path: String,
    pub path_index: u32,
}

/// Physical and psychological needs (0.0 = satisfied, 1.0 = critical)
#[table(name = needs, public)]
pub struct Needs {
    #[primary_key]
    pub person_id: u64,
    pub hunger: f32,
    pub fatigue: f32,
    pub social: f32,
    pub comfort: f32,
    pub hygiene: f32,
    pub health: f32,     // 1.0 = healthy, 0.0 = dead
    pub morale: f32,     // 1.0 = happy, 0.0 = despairing
}

/// Big Five personality traits (0.0-1.0 each)
#[table(name = personality, public)]
pub struct Personality {
    #[primary_key]
    pub person_id: u64,
    pub openness: f32,
    pub conscientiousness: f32,
    pub extraversion: f32,
    pub agreeableness: f32,
    pub neuroticism: f32,
}

/// Skill levels (0.0-1.0 each)
#[table(name = skills, public)]
pub struct Skills {
    #[primary_key]
    pub person_id: u64,
    pub engineering: f32,
    pub medical: f32,
    pub piloting: f32,
    pub science: f32,
    pub social: f32,
    pub combat: f32,
}

/// Current activity state
#[table(name = activity, public)]
#[derive(Clone)]
pub struct Activity {
    #[primary_key]
    pub person_id: u64,
    pub activity_type: u8,  // ActivityType as u8
    pub started_at: f64,
    pub duration: f32,      // hours
    pub target_room_id: Option<u32>,
}

/// Crew-specific data
#[table(name = crew, public)]
pub struct Crew {
    #[primary_key]
    pub person_id: u64,
    pub department: u8,     // Department as u8
    pub rank: u8,           // Rank as u8
    pub shift: u8,          // Shift as u8
    pub duty_station_id: u32,
    pub on_duty: bool,
}

/// Passenger-specific data
#[table(name = passenger, public)]
pub struct Passenger {
    #[primary_key]
    pub person_id: u64,
    pub cabin_class: u8,    // CabinClass as u8
    pub destination: String,
    pub profession: String,
}

// ============================================================================
// SHIP STRUCTURE
// ============================================================================

/// Room/compartment on the ship
#[table(name = room, public)]
pub struct Room {
    #[primary_key]
    pub id: u32,
    pub node_id: u64,           // FK → GraphNode
    pub name: String,
    pub room_type: u8,
    pub deck: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub capacity: u32,
}

/// Logical graph node — every "thing" in the ship
#[table(name = graph_node, public)]
#[derive(Clone)]
pub struct GraphNode {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub node_type: u8,          // node_types::*
    pub name: String,
    pub function: u8,           // room_types::* (what role this serves)
    pub capacity: u32,
    pub required_area: f32,     // computed from function + capacity + subsystem count
    pub deck_preference: i32,   // -1 = no preference, 0-5 = soft preference
    pub group: u8,              // groups::* (command, engineering, etc.)
}

/// Logical graph edge — connection between two nodes
#[table(name = graph_edge, public)]
#[derive(Clone)]
pub struct GraphEdge {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub from_node: u64,
    pub to_node: u64,
    pub edge_type: u8,          // edge_types::*
    pub weight: f32,
    pub bidirectional: bool,
}

/// Door connecting two rooms with explicit wall info
#[table(name = door, public)]
pub struct Door {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub room_a: u32,
    pub room_b: u32,
    pub wall_a: u8,             // wall_sides::* (which wall of room_a)
    pub wall_b: u8,             // wall_sides::* (which wall of room_b)
    pub position_along_wall: f32, // DEPRECATED — use door_x/door_y instead
    pub width: f32,             // door opening width (default 3.0)
    pub access_level: u8,       // access_levels::*
    pub door_x: f32,            // absolute world X position of door center
    pub door_y: f32,            // absolute world Y position of door center
}

/// Generated corridor — first-class entity
#[table(name = corridor, public)]
pub struct Corridor {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub deck: i32,
    pub corridor_type: u8,      // corridor_types::*
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub length: f32,
    pub orientation: u8,        // 0=horizontal, 1=vertical
    pub carries: u8,            // bitmask: carries_flags::*
}

/// Elevator or ladder shaft spanning multiple decks
#[table(name = vertical_shaft, public)]
pub struct VerticalShaft {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub shaft_type: u8,         // shaft_types::*
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub decks_served: String,   // comma-separated: "0,1,2,3,4,5"
    pub width: f32,
    pub height: f32,
}

/// Per-deck atmosphere state
#[table(name = deck_atmosphere, public)]
pub struct DeckAtmosphere {
    #[primary_key]
    pub deck: i32,
    pub oxygen: f32,        // 0.0-1.0 (nominal ~0.21)
    pub co2: f32,           // 0.0-1.0 (danger > 0.04)
    pub humidity: f32,      // 0.0-1.0 (comfort 0.4-0.6)
    pub temperature: f32,   // Celsius (comfort 20-24)
    pub pressure: f32,      // kPa (nominal ~101)
}

// ============================================================================
// SHIP SYSTEMS & RESOURCES
// ============================================================================

/// Ship system — top-level category (Power, Life Support, etc.)
/// Health/status computed from child subsystems.
#[table(name = ship_system, public)]
#[derive(Clone)]
pub struct ShipSystem {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub name: String,
    pub system_type: u8,
    pub overall_health: f32,    // computed: avg of subsystem health
    pub overall_status: u8,     // computed: worst subsystem status
    pub priority: u8,           // power_priorities::*
}

/// Subsystem — functional unit within a system (e.g. O2 Generator within Life Support)
#[table(name = subsystem, public)]
#[derive(Clone)]
pub struct Subsystem {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub system_id: u64,
    pub name: String,
    pub subsystem_type: u8,
    pub health: f32,
    pub status: u8,
    pub node_id: u64,           // FK → GraphNode (which logical node this lives in)
    pub power_draw: f32,// kW required
    pub crew_required: u8,
}

/// Individual component within a subsystem (pump, valve, sensor, etc.)
#[table(name = system_component, public)]
#[derive(Clone)]
pub struct SystemComponent {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub subsystem_id: u64,
    pub name: String,
    pub component_type: u8,
    pub health: f32,
    pub status: u8,
    pub position_x: f32,
    pub position_y: f32,
    pub maintenance_interval_hours: f32,
    pub last_maintenance: f64,
}

/// Physical infrastructure routing (power cable, water pipe, etc.)
#[table(name = infra_edge, public)]
#[derive(Clone)]
pub struct InfraEdge {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub graph_edge_id: u64,     // FK → GraphEdge
    pub edge_type: u8,          // infra_types::*
    pub corridor_id: u64,       // FK → Corridor (which corridor carries this)
    pub capacity: f32,
    pub current_flow: f32,
    pub health: f32,
}

/// Ship-wide resource storage (singleton, id=0)
#[table(name = ship_resources, public)]
pub struct ShipResources {
    #[primary_key]
    pub id: u32,
    pub power: f32,
    pub water: f32,
    pub oxygen: f32,
    pub food: f32,
    pub fuel: f32,
    pub spare_parts: f32,
    // Capacities
    pub power_cap: f32,
    pub water_cap: f32,
    pub oxygen_cap: f32,
    pub food_cap: f32,
    pub fuel_cap: f32,
    pub spare_parts_cap: f32,
}

/// Active maintenance task targeting a specific component
#[table(name = maintenance_task, public)]
#[derive(Clone)]
pub struct MaintenanceTask {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub component_id: u64,      // FK → SystemComponent
    pub subsystem_id: u64,      // FK → Subsystem (denormalized for quick lookup)
    pub assigned_crew_id: Option<u64>,
    pub priority: f32,
    pub progress: f32,          // 0.0-1.0
    pub created_at: f64,
    pub required_skill: u8,
    pub duration_hours: f32,
}

// ============================================================================
// SOCIAL
// ============================================================================

/// Relationship between two people
#[table(name = relationship, public)]
pub struct Relationship {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub person_a: u64,
    pub person_b: u64,
    pub relationship_type: u8, // RelationshipType as u8
    pub strength: f32,         // -1.0 to 1.0
    pub familiarity: f32,      // 0.0 to 1.0
    pub last_interaction: f64,
}

/// Active conversation
#[table(name = conversation, public)]
#[derive(Clone)]
pub struct Conversation {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub topic: u8,             // ConversationTopic as u8
    pub state: u8,             // ConversationState as u8
    pub started_at: f64,
    pub participant_a: u64,
    pub participant_b: u64,
}

/// Marker: person is currently in a conversation
#[table(name = in_conversation, public)]
pub struct InConversation {
    #[primary_key]
    pub person_id: u64,
    pub conversation_id: u64,
}

// ============================================================================
// EVENTS
// ============================================================================

/// Active event (emergency, celebration, etc.)
#[table(name = event, public)]
#[derive(Clone)]
pub struct Event {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub event_type: u8,        // EventType as u8
    pub room_id: u32,
    pub started_at: f64,
    pub duration: f32,
    pub state: u8,             // EventState as u8
    pub responders_needed: u8,
    pub responders_assigned: u8,
    pub severity: f32,         // 0.0-1.0
}

// ============================================================================
// PLAYERS
// ============================================================================

/// Connected player session
#[table(name = connected_player, public)]
pub struct ConnectedPlayer {
    #[primary_key]
    pub identity: Identity,
    pub person_id: Option<u64>,
    pub connected_at: Timestamp,
}

// ============================================================================
// ENUM CONSTANTS
// ============================================================================

pub mod node_types {
    pub const ROOM: u8 = 0;
    pub const CORRIDOR: u8 = 1;
    pub const ELEVATOR: u8 = 2;
    pub const LADDER: u8 = 3;
    pub const SYSTEM: u8 = 4;
    pub const SUBSYSTEM: u8 = 5;
}

pub mod edge_types {
    pub const CREW_PATH: u8 = 0;
    pub const POWER: u8 = 1;
    pub const WATER: u8 = 2;
    pub const COOLANT: u8 = 3;
    pub const HVAC: u8 = 4;
    pub const DATA: u8 = 5;
    pub const STRUCTURAL: u8 = 6;
}

pub mod wall_sides {
    pub const NORTH: u8 = 0;
    pub const SOUTH: u8 = 1;
    pub const EAST: u8 = 2;
    pub const WEST: u8 = 3;
}

pub mod corridor_types {
    pub const MAIN: u8 = 0;
    pub const SERVICE: u8 = 1;
    pub const BRANCH: u8 = 2;
}

pub mod shaft_types {
    pub const ELEVATOR: u8 = 0;
    pub const SERVICE_ELEVATOR: u8 = 1;
    pub const LADDER: u8 = 2;
}

pub mod access_levels {
    pub const PUBLIC: u8 = 0;
    pub const CREW_ONLY: u8 = 1;
    pub const RESTRICTED: u8 = 2;
}

pub mod groups {
    pub const COMMAND: u8 = 0;
    pub const ENGINEERING: u8 = 1;
    pub const LIFE_SUPPORT: u8 = 2;
    pub const COMMONS: u8 = 3;
    pub const CREW: u8 = 4;
    pub const PASSENGER: u8 = 5;
    pub const INFRASTRUCTURE: u8 = 6;
}

pub mod infra_types {
    pub const POWER_CABLE: u8 = 0;
    pub const WATER_PIPE: u8 = 1;
    pub const COOLANT_PIPE: u8 = 2;
    pub const HVAC_DUCT: u8 = 3;
    pub const DATA_CABLE: u8 = 4;
}

pub mod carries_flags {
    pub const CREW_PATH: u8 = 1;
    pub const POWER: u8 = 2;
    pub const WATER: u8 = 4;
    pub const HVAC: u8 = 8;
    pub const DATA: u8 = 16;
    pub const COOLANT: u8 = 32;
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
    pub const SERVICE_DECK: u8 = 120;

    /// Returns true if this room type is any kind of sleeping quarters
    pub fn is_quarters(rt: u8) -> bool {
        matches!(rt, CABIN_SINGLE | CABIN_DOUBLE | FAMILY_SUITE | VIP_SUITE
            | QUARTERS_CREW | QUARTERS_OFFICER | QUARTERS_PASSENGER)
    }
    /// Returns true if this room type is a dining/food area
    pub fn is_dining(rt: u8) -> bool {
        matches!(rt, MESS_HALL | WARDROOM | CAFE | GALLEY)
    }
    /// Returns true if this room type is recreation/social
    pub fn is_recreation(rt: u8) -> bool {
        matches!(rt, GYM | THEATRE | LIBRARY | CHAPEL | GAME_ROOM | BAR
            | ART_STUDIO | MUSIC_ROOM | HOLODECK | ARBORETUM
            | OBSERVATION_LOUNGE | POOL | NURSERY | SCHOOL | RECREATION | LOUNGE | SHOPS)
    }
    /// Returns true if this room type is a corridor/infrastructure
    pub fn is_corridor(rt: u8) -> bool {
        rt >= 100
    }
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

pub mod shifts {
    pub const ALPHA: u8 = 0;  // 0600-1400
    pub const BETA: u8 = 1;   // 1400-2200
    pub const GAMMA: u8 = 2;  // 2200-0600
}

pub mod cabin_classes {
    pub const FIRST: u8 = 0;
    pub const STANDARD: u8 = 1;
    pub const STEERAGE: u8 = 2;
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

pub mod system_statuses {
    pub const NOMINAL: u8 = 0;
    pub const DEGRADED: u8 = 1;
    pub const CRITICAL: u8 = 2;
    pub const OFFLINE: u8 = 3;
    pub const DESTROYED: u8 = 4;
}

pub mod power_priorities {
    pub const CRITICAL: u8 = 0;     // Life support, navigation — last to lose power
    pub const HIGH: u8 = 1;         // Medical, comms
    pub const NORMAL: u8 = 2;       // Gravity, water recycling, food production
    pub const LOW: u8 = 3;          // Recreation, non-essential
}

pub mod subsystem_types {
    // Power subsystems
    pub const REACTOR_CORE: u8 = 0;
    pub const FUEL_INJECTION: u8 = 1;
    pub const MAGNETIC_CONTAINMENT: u8 = 2;
    pub const REACTOR_COOLING: u8 = 3;
    pub const PRIMARY_POWER_BUS: u8 = 4;
    pub const DECK_DISTRIBUTION: u8 = 5;
    pub const EMERGENCY_BUS: u8 = 6;
    pub const EMERGENCY_GENERATOR: u8 = 7;
    // Life support subsystems
    pub const O2_GENERATION: u8 = 10;
    pub const CO2_SCRUBBING: u8 = 11;
    pub const AIR_CIRCULATION: u8 = 12;
    pub const HEAT_EXCHANGE: u8 = 13;
    pub const COOLANT_PUMP: u8 = 14;
    pub const RADIATOR_PANEL: u8 = 15;
    pub const PRESSURE_MANAGEMENT: u8 = 16;
    // Water subsystems
    pub const WATER_FILTRATION: u8 = 20;
    pub const WATER_DISTILLATION: u8 = 21;
    pub const UV_PURIFICATION: u8 = 22;
    pub const WATER_STORAGE: u8 = 23;
    pub const WATER_DISTRIBUTION: u8 = 24;
    pub const WASTE_PROCESSING: u8 = 25;
    // Food production subsystems
    pub const GROWTH_CHAMBER: u8 = 30;
    pub const NUTRIENT_MIXER: u8 = 31;
    pub const GROW_LIGHTING: u8 = 32;
    pub const FOOD_PROCESSING: u8 = 33;
    pub const COLD_STORAGE: u8 = 34;
    // Propulsion subsystems
    pub const THRUST_CHAMBER: u8 = 40;
    pub const FUEL_PUMP: u8 = 41;
    pub const NOZZLE_ACTUATOR: u8 = 42;
    // Navigation subsystems
    pub const STAR_TRACKER: u8 = 50;
    pub const GYROSCOPE: u8 = 51;
    pub const ATTITUDE_THRUSTER: u8 = 52;
    // Communications subsystems
    pub const ANTENNA_ARRAY: u8 = 60;
    pub const SIGNAL_PROCESSOR: u8 = 61;
    pub const INTERCOM_NETWORK: u8 = 62;
    pub const DATA_BACKBONE: u8 = 63;
    // Gravity subsystems
    pub const GRAVITY_PLATE: u8 = 70;
    pub const GRAVITY_CONTROLLER: u8 = 71;
    pub const INERTIAL_DAMPENER: u8 = 72;
    // Medical subsystems
    pub const DIAGNOSTIC_SCANNER: u8 = 80;
    pub const LAB_ANALYZER: u8 = 81;
    pub const SURGICAL_SUITE: u8 = 82;
    pub const CRYO_POD: u8 = 83;
}

pub mod component_types {
    pub const PUMP: u8 = 0;
    pub const VALVE: u8 = 1;
    pub const SENSOR: u8 = 2;
    pub const PROCESSOR: u8 = 3;
    pub const MOTOR: u8 = 4;
    pub const GENERATOR: u8 = 5;
    pub const HEAT_EXCHANGER: u8 = 6;
    pub const FILTER: u8 = 7;
    pub const COMPRESSOR: u8 = 8;
    pub const FAN: u8 = 9;
    pub const LAMP: u8 = 10;
    pub const ACTUATOR: u8 = 11;
    pub const CONTAINMENT_COIL: u8 = 12;
    pub const FUEL_INJECTOR: u8 = 13;
    pub const CAPACITOR: u8 = 14;
    pub const TRANSFORMER: u8 = 15;
    pub const CIRCUIT_BREAKER: u8 = 16;
    pub const ANTENNA: u8 = 17;
    pub const DISPLAY: u8 = 18;
    pub const GRAVITY_EMITTER: u8 = 19;
    pub const SCANNER_HEAD: u8 = 20;
    pub const NOZZLE: u8 = 21;
    pub const TANK: u8 = 22;
    pub const SEAL: u8 = 23;
    pub const REGULATOR: u8 = 24;
}

pub mod relationship_types {
    pub const STRANGER: u8 = 0;
    pub const ACQUAINTANCE: u8 = 1;
    pub const COLLEAGUE: u8 = 2;
    pub const FRIEND: u8 = 3;
    pub const CLOSE_FRIEND: u8 = 4;
    pub const ROMANTIC: u8 = 5;
    pub const FAMILY: u8 = 6;
    pub const RIVAL: u8 = 7;
    pub const ENEMY: u8 = 8;
}

pub mod conversation_topics {
    pub const GREETING: u8 = 0;
    pub const WORK: u8 = 1;
    pub const GOSSIP: u8 = 2;
    pub const PERSONAL: u8 = 3;
    pub const COMPLAINT: u8 = 4;
    pub const REQUEST: u8 = 5;
    pub const FLIRTATION: u8 = 6;
    pub const ARGUMENT: u8 = 7;
    pub const FAREWELL: u8 = 8;
}

pub mod conversation_states {
    pub const ACTIVE: u8 = 0;
    pub const PAUSED: u8 = 1;
    pub const ENDED: u8 = 2;
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
}

pub mod event_states {
    pub const ACTIVE: u8 = 0;
    pub const BEING_HANDLED: u8 = 1;
    pub const RESOLVED: u8 = 2;
    pub const ESCALATED: u8 = 3;
}

pub mod skill_types {
    pub const ENGINEERING: u8 = 0;
    pub const MEDICAL: u8 = 1;
    pub const PILOTING: u8 = 2;
    pub const SCIENCE: u8 = 3;
    pub const SOCIAL: u8 = 4;
    pub const COMBAT: u8 = 5;
}
