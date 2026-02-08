//! SpacetimeDB table definitions for the colony ship simulation.
//!
//! Each table is equivalent to an ECS component from progship-core,
//! but stored in SpacetimeDB for persistence and multiplayer sync.

use spacetimedb::{table, Identity, Timestamp};

// ============================================================================
// SHIP CONFIGURATION
// ============================================================================

/// Ship configuration singleton defining the colony ship's parameters and simulation state.
#[table(name = ship_config, public)]
#[derive(Clone)]
pub struct ShipConfig {
    #[primary_key]
    /// Unique identifier (always 0 for singleton).
    pub id: u32,
    /// Name of the colony ship.
    pub name: String,
    /// Total number of decks on the ship.
    pub deck_count: u32,
    /// Total number of crew members aboard.
    pub crew_count: u32,
    /// Total number of passengers aboard.
    pub passenger_count: u32,
    /// Current simulation time in hours since mission start.
    pub sim_time: f64,
    /// Time acceleration factor (1.0 = real-time).
    pub time_scale: f32,
    /// Whether the simulation is currently paused.
    pub paused: bool,
}

// ============================================================================
// PEOPLE
// ============================================================================

/// Person aboard the colony ship, either crew member, passenger, or player character.
#[table(name = person, public)]
pub struct Person {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this person.
    pub id: u64,
    /// First name of the person.
    pub given_name: String,
    /// Last name of the person.
    pub family_name: String,
    /// Whether this person is a crew member.
    pub is_crew: bool,
    /// Whether this person is a player-controlled character.
    pub is_player: bool,
    /// SpacetimeDB identity of the player controlling this person, if any.
    pub owner_identity: Option<Identity>,
}

/// Physical position of a person within the ship's coordinate system.
#[table(name = position, public)]
pub struct Position {
    #[primary_key]
    /// Foreign key to Person.id.
    pub person_id: u64,
    /// ID of the room the person is currently in.
    pub room_id: u32,
    /// X coordinate in meters (east-west axis).
    pub x: f32,
    /// Y coordinate in meters (fore-aft axis).
    pub y: f32,
    /// Z coordinate in meters (vertical axis, deck height).
    pub z: f32,
}

/// Active movement state for a person navigating toward a destination.
#[table(name = movement, public)]
#[derive(Clone)]
pub struct Movement {
    #[primary_key]
    /// Foreign key to Person.id.
    pub person_id: u64,
    /// ID of the destination room.
    pub target_room_id: u32,
    /// Target X coordinate in meters.
    pub target_x: f32,
    /// Target Y coordinate in meters.
    pub target_y: f32,
    /// Target Z coordinate in meters.
    pub target_z: f32,
    /// Movement speed in meters per second.
    pub speed: f32,
    /// Serialized navigation path as comma-separated room IDs.
    pub path: String,
    /// Current index in the path being traversed.
    pub path_index: u32,
}

/// Physical and psychological needs tracking for a person (0.0 = satisfied, 1.0 = critical).
#[table(name = needs, public)]
pub struct Needs {
    #[primary_key]
    /// Foreign key to Person.id.
    pub person_id: u64,
    /// Hunger level (0.0 = full, 1.0 = starving).
    pub hunger: f32,
    /// Fatigue level (0.0 = rested, 1.0 = exhausted).
    pub fatigue: f32,
    /// Social need level (0.0 = satisfied, 1.0 = lonely).
    pub social: f32,
    /// Comfort need level (0.0 = comfortable, 1.0 = miserable).
    pub comfort: f32,
    /// Hygiene level (0.0 = clean, 1.0 = dirty).
    pub hygiene: f32,
    /// Health status (1.0 = healthy, 0.0 = dead).
    pub health: f32,
    /// Morale level (1.0 = happy, 0.0 = despairing).
    pub morale: f32,
}

/// Big Five personality traits for a person (0.0-1.0 normalized scale).
#[table(name = personality, public)]
pub struct Personality {
    #[primary_key]
    /// Foreign key to Person.id.
    pub person_id: u64,
    /// Openness to experience trait.
    pub openness: f32,
    /// Conscientiousness trait.
    pub conscientiousness: f32,
    /// Extraversion trait.
    pub extraversion: f32,
    /// Agreeableness trait.
    pub agreeableness: f32,
    /// Neuroticism trait.
    pub neuroticism: f32,
}

/// Professional skill levels for a person (0.0-1.0 normalized scale).
#[table(name = skills, public)]
pub struct Skills {
    #[primary_key]
    /// Foreign key to Person.id.
    pub person_id: u64,
    /// Engineering skill level.
    pub engineering: f32,
    /// Medical skill level.
    pub medical: f32,
    /// Piloting skill level.
    pub piloting: f32,
    /// Science skill level.
    pub science: f32,
    /// Social skill level.
    pub social: f32,
    /// Combat skill level.
    pub combat: f32,
}

/// Current activity state for a person's scheduled behavior.
#[table(name = activity, public)]
#[derive(Clone)]
pub struct Activity {
    #[primary_key]
    /// Foreign key to Person.id.
    pub person_id: u64,
    /// Type of activity (see activity_types module).
    pub activity_type: u8,
    /// Simulation time when this activity started in hours.
    pub started_at: f64,
    /// Planned duration of the activity in hours.
    pub duration: f32,
    /// Room where the activity takes place, if applicable.
    pub target_room_id: Option<u32>,
}

/// Crew-specific information for personnel assigned to ship operations.
#[table(name = crew, public)]
pub struct Crew {
    #[primary_key]
    /// Foreign key to Person.id.
    pub person_id: u64,
    /// Department assignment (see departments module).
    pub department: u8,
    /// Rank within the crew hierarchy (see ranks module).
    pub rank: u8,
    /// Assigned duty shift (see shifts module).
    pub shift: u8,
    /// Room ID where this crew member is stationed.
    pub duty_station_id: u32,
    /// Whether the crew member is currently on duty.
    pub on_duty: bool,
}

/// Passenger-specific information for civilians traveling aboard the colony ship.
#[table(name = passenger, public)]
pub struct Passenger {
    #[primary_key]
    /// Foreign key to Person.id.
    pub person_id: u64,
    /// Cabin class for accommodation (see cabin_classes module).
    pub cabin_class: u8,
    /// Destination colony or station.
    pub destination: String,
    /// Passenger's profession or occupation.
    pub profession: String,
}

// ============================================================================
// SHIP STRUCTURE
// ============================================================================

/// Physical room or compartment aboard the ship with spatial properties.
#[table(name = room, public)]
pub struct Room {
    #[primary_key]
    /// Unique identifier for this room.
    pub id: u32,
    /// Foreign key to GraphNode (logical representation).
    pub node_id: u64,
    /// Human-readable name of the room.
    pub name: String,
    /// Type of room (see room_types module).
    pub room_type: u8,
    /// Deck number where this room is located.
    pub deck: i32,
    /// X coordinate of room's bottom-left corner in meters.
    pub x: f32,
    /// Y coordinate of room's bottom-left corner in meters.
    pub y: f32,
    /// Width of the room in meters (east-west).
    pub width: f32,
    /// Height of the room in meters (fore-aft).
    pub height: f32,
    /// Maximum occupancy capacity of the room.
    pub capacity: u32,
}

/// Logical graph node representing any functional entity in the ship's network.
#[table(name = graph_node, public)]
#[derive(Clone)]
pub struct GraphNode {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this graph node.
    pub id: u64,
    /// Type of node (see node_types module).
    pub node_type: u8,
    /// Human-readable name of this node.
    pub name: String,
    /// Functional role of this node (see room_types module).
    pub function: u8,
    /// Maximum capacity or occupancy for this node.
    pub capacity: u32,
    /// Required physical area in square meters.
    pub required_area: f32,
    /// Preferred deck for placement (-1 = no preference, 0+ = specific deck number).
    pub deck_preference: i32,
    /// Functional group classification (see groups module).
    pub group: u8,
}

/// Logical graph edge representing a connection between two nodes in the ship's network.
#[table(name = graph_edge, public)]
#[derive(Clone)]
pub struct GraphEdge {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this edge.
    pub id: u64,
    /// Foreign key to the source GraphNode.
    pub from_node: u64,
    /// Foreign key to the target GraphNode.
    pub to_node: u64,
    /// Type of connection (see edge_types module).
    pub edge_type: u8,
    /// Weight or cost of traversing this edge.
    pub weight: f32,
    /// Whether the edge can be traversed in both directions.
    pub bidirectional: bool,
}

/// Physical door connecting two adjacent rooms with spatial and access properties.
#[table(name = door, public)]
pub struct Door {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this door.
    pub id: u64,
    /// Foreign key to first Room.id connected by this door.
    pub room_a: u32,
    /// Foreign key to second Room.id connected by this door.
    pub room_b: u32,
    /// Wall side in room_a where door is located (see wall_sides module).
    pub wall_a: u8,
    /// Wall side in room_b where door is located (see wall_sides module).
    pub wall_b: u8,
    /// DEPRECATED: Position along wall (use door_x/door_y instead).
    pub position_along_wall: f32,
    /// Width of the door opening in meters (default 3.0).
    pub width: f32,
    /// Required access level to traverse (see access_levels module).
    pub access_level: u8,
    /// Absolute world X coordinate of door center in meters.
    pub door_x: f32,
    /// Absolute world Y coordinate of door center in meters.
    pub door_y: f32,
}

/// Procedurally generated corridor providing primary navigation paths between rooms.
#[table(name = corridor, public)]
pub struct Corridor {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this corridor.
    pub id: u64,
    /// Deck number where this corridor is located.
    pub deck: i32,
    /// Type of corridor (see corridor_types module).
    pub corridor_type: u8,
    /// X coordinate of corridor's bottom-left corner in meters.
    pub x: f32,
    /// Y coordinate of corridor's bottom-left corner in meters.
    pub y: f32,
    /// Width of the corridor in meters.
    pub width: f32,
    /// Length of the corridor in meters.
    pub length: f32,
    /// Orientation of the corridor (0=horizontal, 1=vertical).
    pub orientation: u8,
    /// Infrastructure types carried by this corridor (bitmask, see carries_flags module).
    pub carries: u8,
}

/// Vertical shaft for elevators or ladders connecting multiple decks.
#[table(name = vertical_shaft, public)]
pub struct VerticalShaft {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this vertical shaft.
    pub id: u64,
    /// Type of vertical transport (see shaft_types module).
    pub shaft_type: u8,
    /// Human-readable name of the shaft.
    pub name: String,
    /// X coordinate of shaft center in meters.
    pub x: f32,
    /// Y coordinate of shaft center in meters.
    pub y: f32,
    /// Comma-separated list of deck numbers served (e.g., "0,1,2,3,4,5").
    pub decks_served: String,
    /// Width of the shaft in meters.
    pub width: f32,
    /// Height of the shaft in meters.
    pub height: f32,
}

/// Atmospheric conditions and life support status for a single deck.
#[table(name = deck_atmosphere, public)]
pub struct DeckAtmosphere {
    #[primary_key]
    /// Deck number for this atmosphere record.
    pub deck: i32,
    /// Oxygen concentration (0.0-1.0, nominal ~0.21).
    pub oxygen: f32,
    /// CO2 concentration (0.0-1.0, danger > 0.04).
    pub co2: f32,
    /// Relative humidity (0.0-1.0, comfort 0.4-0.6).
    pub humidity: f32,
    /// Temperature in degrees Celsius (comfort 20-24).
    pub temperature: f32,
    /// Air pressure in kilopascals (nominal ~101).
    pub pressure: f32,
}

// ============================================================================
// SHIP SYSTEMS & RESOURCES
// ============================================================================

/// Top-level ship system category aggregating subsystems (e.g., Power, Life Support).
#[table(name = ship_system, public)]
#[derive(Clone)]
pub struct ShipSystem {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this ship system.
    pub id: u64,
    /// Human-readable name of the system.
    pub name: String,
    /// Type of ship system (see system_types module).
    pub system_type: u8,
    /// Overall health computed as average of child subsystem health (0.0-1.0).
    pub overall_health: f32,
    /// Overall status computed as worst child subsystem status (see system_statuses module).
    pub overall_status: u8,
    /// Power priority for resource allocation (see power_priorities module).
    pub priority: u8,
}

/// Functional subsystem within a parent ship system (e.g., O2 Generator in Life Support).
#[table(name = subsystem, public)]
#[derive(Clone)]
pub struct Subsystem {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this subsystem.
    pub id: u64,
    /// Foreign key to parent ShipSystem.id.
    pub system_id: u64,
    /// Human-readable name of the subsystem.
    pub name: String,
    /// Type of subsystem (see subsystem_types module).
    pub subsystem_type: u8,
    /// Current health of the subsystem (0.0-1.0).
    pub health: f32,
    /// Current operational status (see system_statuses module).
    pub status: u8,
    /// Foreign key to GraphNode where this subsystem is physically located.
    pub node_id: u64,
    /// Power consumption in kilowatts.
    pub power_draw: f32,
    /// Number of crew required to operate this subsystem.
    pub crew_required: u8,
}

/// Individual physical component within a subsystem (pump, valve, sensor, etc.).
#[table(name = system_component, public)]
#[derive(Clone)]
pub struct SystemComponent {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this component.
    pub id: u64,
    /// Foreign key to parent Subsystem.id.
    pub subsystem_id: u64,
    /// Human-readable name of the component.
    pub name: String,
    /// Type of component (see component_types module).
    pub component_type: u8,
    /// Current health of the component (0.0-1.0).
    pub health: f32,
    /// Current operational status (see system_statuses module).
    pub status: u8,
    /// X coordinate of component location in meters.
    pub position_x: f32,
    /// Y coordinate of component location in meters.
    pub position_y: f32,
    /// Required maintenance interval in hours.
    pub maintenance_interval_hours: f32,
    /// Simulation time when last maintenance was performed.
    pub last_maintenance: f64,
}

/// Physical infrastructure routing element (power cable, water pipe, etc.) within corridors.
#[table(name = infra_edge, public)]
#[derive(Clone)]
pub struct InfraEdge {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this infrastructure edge.
    pub id: u64,
    /// Foreign key to logical GraphEdge.id.
    pub graph_edge_id: u64,
    /// Type of infrastructure (see infra_types module).
    pub edge_type: u8,
    /// Foreign key to Corridor.id where this infrastructure runs.
    pub corridor_id: u64,
    /// Maximum capacity of this infrastructure element.
    pub capacity: f32,
    /// Current flow or load through this element.
    pub current_flow: f32,
    /// Health of the infrastructure element (0.0-1.0).
    pub health: f32,
}

/// Ship-wide resource storage tracking current levels and maximum capacities (singleton, id=0).
#[table(name = ship_resources, public)]
pub struct ShipResources {
    #[primary_key]
    /// Unique identifier (always 0 for singleton).
    pub id: u32,
    /// Current power reserves in kilowatt-hours.
    pub power: f32,
    /// Current water reserves in cubic meters.
    pub water: f32,
    /// Current oxygen reserves in kilograms.
    pub oxygen: f32,
    /// Current food reserves in kilograms.
    pub food: f32,
    /// Current fuel reserves in kilograms.
    pub fuel: f32,
    /// Current spare parts inventory in units.
    pub spare_parts: f32,
    /// Maximum power storage capacity in kilowatt-hours.
    pub power_cap: f32,
    /// Maximum water storage capacity in cubic meters.
    pub water_cap: f32,
    /// Maximum oxygen storage capacity in kilograms.
    pub oxygen_cap: f32,
    /// Maximum food storage capacity in kilograms.
    pub food_cap: f32,
    /// Maximum fuel storage capacity in kilograms.
    pub fuel_cap: f32,
    /// Maximum spare parts storage capacity in units.
    pub spare_parts_cap: f32,
}

/// Active maintenance task assigned to repair or service a system component.
#[table(name = maintenance_task, public)]
#[derive(Clone)]
pub struct MaintenanceTask {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this maintenance task.
    pub id: u64,
    /// Foreign key to SystemComponent.id being serviced.
    pub component_id: u64,
    /// Foreign key to Subsystem.id (denormalized for quick lookup).
    pub subsystem_id: u64,
    /// Foreign key to Person.id of assigned crew member, if any.
    pub assigned_crew_id: Option<u64>,
    /// Priority level of this task (higher is more urgent).
    pub priority: f32,
    /// Task completion progress (0.0-1.0).
    pub progress: f32,
    /// Simulation time when this task was created.
    pub created_at: f64,
    /// Required skill type to perform this task (see skill_types module).
    pub required_skill: u8,
    /// Estimated duration to complete task in hours.
    pub duration_hours: f32,
}

// ============================================================================
// SOCIAL
// ============================================================================

/// Social relationship between two people aboard the ship.
#[table(name = relationship, public)]
pub struct Relationship {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this relationship.
    pub id: u64,
    /// Foreign key to first Person.id.
    pub person_a: u64,
    /// Foreign key to second Person.id.
    pub person_b: u64,
    /// Type of relationship (see relationship_types module).
    pub relationship_type: u8,
    /// Relationship strength (-1.0 = hostile, 1.0 = close).
    pub strength: f32,
    /// Familiarity level (0.0 = strangers, 1.0 = well-known).
    pub familiarity: f32,
    /// Simulation time of last social interaction.
    pub last_interaction: f64,
}

/// Active conversation between two people with topic and state tracking.
#[table(name = conversation, public)]
#[derive(Clone)]
pub struct Conversation {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this conversation.
    pub id: u64,
    /// Conversation topic (see conversation_topics module).
    pub topic: u8,
    /// Conversation state (see conversation_states module).
    pub state: u8,
    /// Simulation time when this conversation started.
    pub started_at: f64,
    /// Foreign key to first Person.id participating.
    pub participant_a: u64,
    /// Foreign key to second Person.id participating.
    pub participant_b: u64,
}

/// Marker table indicating a person is currently engaged in a conversation.
#[table(name = in_conversation, public)]
pub struct InConversation {
    #[primary_key]
    /// Foreign key to Person.id.
    pub person_id: u64,
    /// Foreign key to Conversation.id.
    pub conversation_id: u64,
}

// ============================================================================
// EVENTS
// ============================================================================

/// Active ship event such as emergency, celebration, or incident.
#[table(name = event, public)]
#[derive(Clone)]
pub struct Event {
    #[primary_key]
    #[auto_inc]
    /// Unique identifier for this event.
    pub id: u64,
    /// Type of event (see event_types module).
    pub event_type: u8,
    /// Room where the event is taking place.
    pub room_id: u32,
    /// Simulation time when this event started.
    pub started_at: f64,
    /// Duration of the event in hours.
    pub duration: f32,
    /// Current state of the event (see event_states module).
    pub state: u8,
    /// Number of responders needed to handle this event.
    pub responders_needed: u8,
    /// Number of responders currently assigned to this event.
    pub responders_assigned: u8,
    /// Severity level of the event (0.0 = minor, 1.0 = critical).
    pub severity: f32,
}

// ============================================================================
// PLAYERS
// ============================================================================

/// Active player connection session to the SpacetimeDB server.
#[table(name = connected_player, public)]
pub struct ConnectedPlayer {
    #[primary_key]
    /// SpacetimeDB identity of the connected player.
    pub identity: Identity,
    /// Foreign key to Person.id controlled by this player, if assigned.
    pub person_id: Option<u64>,
    /// Timestamp when the player connected to the server.
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
    pub const ALPHA: u8 = 0; // 0600-1400
    pub const BETA: u8 = 1; // 1400-2200
    pub const GAMMA: u8 = 2; // 2200-0600
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
    pub const CRITICAL: u8 = 0; // Life support, navigation — last to lose power
    pub const HIGH: u8 = 1; // Medical, comms
    pub const NORMAL: u8 = 2; // Gravity, water recycling, food production
    pub const LOW: u8 = 3; // Recreation, non-essential
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
