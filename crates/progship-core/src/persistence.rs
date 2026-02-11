//! Save/Load functionality for persisting simulation state
//!
//! Uses bincode for efficient binary serialization of the entire simulation.
//! Components are serialized individually then reconstructed on load.

use hecs::World;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

use crate::components::*;
use crate::generation::ShipLayout;
use crate::systems::{ConversationManager, MaintenanceQueue, RelationshipGraph, ShipResources};

/// Version number for save file format (increment when format changes)
const SAVE_VERSION: u32 = 1;

/// Serializable snapshot of the simulation state
#[derive(Serialize, Deserialize)]
pub struct SaveData {
    /// Save format version
    pub version: u32,
    /// Simulation time in hours
    pub sim_time: f64,
    /// Time scale
    pub time_scale: f32,
    /// Ship layout info
    pub ship_layout: Option<SerializableShipLayout>,
    /// Ship resources
    pub resources: ShipResources,
    /// Maintenance queue
    pub maintenance_queue: MaintenanceQueue,
    /// Relationships
    pub relationships: RelationshipGraph,
    /// Conversations
    pub conversations: ConversationManager,
    /// Events
    pub events: crate::systems::EventManager,
    /// All entities with their components
    pub entities: Vec<SerializableEntity>,
}

/// Serializable version of ShipLayout (without Entity references)
#[derive(Serialize, Deserialize, Clone)]
pub struct SerializableShipLayout {
    pub name: String,
    pub ship_length: f32,
    pub ship_width: f32,
    pub room_count: usize,
    pub deck_count: usize,
    pub elevator_count: usize,
}

impl From<&ShipLayout> for SerializableShipLayout {
    fn from(layout: &ShipLayout) -> Self {
        Self {
            name: layout.name.clone(),
            ship_length: layout.ship_length,
            ship_width: layout.ship_width,
            room_count: layout.rooms.len(),
            deck_count: layout.decks.len(),
            elevator_count: layout.elevators.len(),
        }
    }
}

/// All possible components for an entity, serialized as optionals
#[derive(Serialize, Deserialize, Default)]
pub struct SerializableEntity {
    // Core
    pub person: Option<Person>,
    pub position: Option<Position>,
    pub movement: Option<Movement>,
    pub needs: Option<Needs>,
    pub name: Option<Name>,

    // Role
    pub crew: Option<Crew>,
    pub passenger: Option<Passenger>,

    // Behavior
    pub activity: Option<Activity>,
    pub personality: Option<Personality>,
    pub skills: Option<Skills>,
    pub in_conversation: Option<InConversation>,

    // Ship structure
    pub room: Option<Room>,
    pub room_connections: Option<RoomConnections>,
    pub deck: Option<Deck>,

    // Systems
    pub ship_system: Option<ShipSystem>,
    pub resource_flow: Option<ResourceFlow>,
    pub maintenance_task: Option<MaintenanceTask>,
}

/// Extract all entities from a world into serializable form
fn serialize_entities(world: &World) -> Vec<SerializableEntity> {
    let mut entities = Vec::new();

    // Get all entities
    for entity in world.iter() {
        let mut se = SerializableEntity::default();
        let entity_ref = world.entity(entity.entity()).unwrap();

        // Extract each component type (dereference Ref to clone)
        if let Some(c) = entity_ref.get::<&Person>() {
            se.person = Some((*c).clone());
        }
        if let Some(c) = entity_ref.get::<&Position>() {
            se.position = Some(*c);
        }
        if let Some(c) = entity_ref.get::<&Movement>() {
            se.movement = Some((*c).clone());
        }
        if let Some(c) = entity_ref.get::<&Needs>() {
            se.needs = Some(*c);
        }
        if let Some(c) = entity_ref.get::<&Name>() {
            se.name = Some((*c).clone());
        }
        if let Some(c) = entity_ref.get::<&Crew>() {
            se.crew = Some((*c).clone());
        }
        if let Some(c) = entity_ref.get::<&Passenger>() {
            se.passenger = Some((*c).clone());
        }
        if let Some(c) = entity_ref.get::<&Activity>() {
            se.activity = Some((*c).clone());
        }
        if let Some(c) = entity_ref.get::<&Personality>() {
            se.personality = Some(*c);
        }
        if let Some(c) = entity_ref.get::<&Skills>() {
            se.skills = Some(*c);
        }
        if let Some(c) = entity_ref.get::<&InConversation>() {
            se.in_conversation = Some(*c);
        }
        if let Some(c) = entity_ref.get::<&Room>() {
            se.room = Some((*c).clone());
        }
        if let Some(c) = entity_ref.get::<&RoomConnections>() {
            se.room_connections = Some((*c).clone());
        }
        if let Some(c) = entity_ref.get::<&Deck>() {
            se.deck = Some((*c).clone());
        }
        if let Some(c) = entity_ref.get::<&ShipSystem>() {
            se.ship_system = Some((*c).clone());
        }
        if let Some(c) = entity_ref.get::<&ResourceFlow>() {
            se.resource_flow = Some((*c).clone());
        }
        if let Some(c) = entity_ref.get::<&MaintenanceTask>() {
            se.maintenance_task = Some((*c).clone());
        }

        entities.push(se);
    }

    entities
}

/// Rebuild a world from serialized entities
fn deserialize_entities(world: &mut World, entities: Vec<SerializableEntity>) {
    for se in entities {
        // Build component tuple dynamically
        // We need to spawn with the right component combination
        spawn_entity(world, se);
    }
}

/// Spawn an entity with all its components
fn spawn_entity(world: &mut World, se: SerializableEntity) {
    // Start with a base entity and add components
    let entity = world.spawn(());

    if let Some(c) = se.person {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.position {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.movement {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.needs {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.name {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.crew {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.passenger {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.activity {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.personality {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.skills {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.in_conversation {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.room {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.room_connections {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.deck {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.ship_system {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.resource_flow {
        let _ = world.insert_one(entity, c);
    }
    if let Some(c) = se.maintenance_task {
        let _ = world.insert_one(entity, c);
    }
}

/// Save the complete simulation to a writer
pub fn save_simulation<W: Write>(
    writer: W,
    world: &World,
    sim_time: f64,
    time_scale: f32,
    ship_layout: Option<&ShipLayout>,
    resources: &ShipResources,
    maintenance_queue: &MaintenanceQueue,
    relationships: &RelationshipGraph,
    conversations: &ConversationManager,
    events: &crate::systems::EventManager,
) -> Result<(), SaveError> {
    let entities = serialize_entities(world);

    let save_data = SaveData {
        version: SAVE_VERSION,
        sim_time,
        time_scale,
        ship_layout: ship_layout.map(SerializableShipLayout::from),
        resources: resources.clone(),
        maintenance_queue: maintenance_queue.clone(),
        relationships: relationships.clone(),
        conversations: conversations.clone(),
        events: events.clone(),
        entities,
    };

    bincode::serialize_into(writer, &save_data)?;
    Ok(())
}

/// Load a simulation from a reader
pub fn load_simulation<R: Read>(reader: R) -> Result<LoadedSimulation, SaveError> {
    let save_data: SaveData = bincode::deserialize_from(reader)?;

    if save_data.version != SAVE_VERSION {
        return Err(SaveError::VersionMismatch {
            expected: SAVE_VERSION,
            found: save_data.version,
        });
    }

    let mut world = World::new();
    deserialize_entities(&mut world, save_data.entities);

    Ok(LoadedSimulation {
        world,
        sim_time: save_data.sim_time,
        time_scale: save_data.time_scale,
        ship_layout_info: save_data.ship_layout,
        resources: save_data.resources,
        maintenance_queue: save_data.maintenance_queue,
        relationships: save_data.relationships,
        conversations: save_data.conversations,
        events: save_data.events,
    })
}

/// Result of loading a simulation
pub struct LoadedSimulation {
    pub world: World,
    pub sim_time: f64,
    pub time_scale: f32,
    pub ship_layout_info: Option<SerializableShipLayout>,
    pub resources: ShipResources,
    pub maintenance_queue: MaintenanceQueue,
    pub relationships: RelationshipGraph,
    pub conversations: ConversationManager,
    pub events: crate::systems::EventManager,
}

/// Errors that can occur during save/load
#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    Bincode(Box<bincode::ErrorKind>),
    VersionMismatch { expected: u32, found: u32 },
}

impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> Self {
        SaveError::Io(e)
    }
}

impl From<Box<bincode::ErrorKind>> for SaveError {
    fn from(e: Box<bincode::ErrorKind>) -> Self {
        SaveError::Bincode(e)
    }
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "IO error: {}", e),
            SaveError::Bincode(e) => write!(f, "Serialization error: {}", e),
            SaveError::VersionMismatch { expected, found } => {
                write!(
                    f,
                    "Save version mismatch: expected {}, found {}",
                    expected, found
                )
            }
        }
    }
}

impl std::error::Error for SaveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::SimulationEngine;
    use crate::generation::ShipConfig;

    #[test]
    fn test_save_load_roundtrip() {
        // Create and populate a simulation
        let mut engine = SimulationEngine::new();
        engine.generate(ShipConfig {
            name: "Test Ship".to_string(),
            num_decks: 2,
            rooms_per_deck: 5,
            passenger_capacity: 50,
            crew_size: 20,
            ship_length: 100.0,
            ship_width: 20.0,
        });

        // Run a few updates
        for _ in 0..10 {
            engine.update(1.0 / 60.0);
        }

        let original_time = engine.sim_time;
        let original_people = engine.crew_count() + engine.passenger_count();

        // Save
        let mut save_buffer = Vec::new();
        engine.save(&mut save_buffer).expect("Save failed");

        println!("Save size: {} bytes", save_buffer.len());

        // Load into new engine
        let mut loaded_engine = SimulationEngine::new();
        loaded_engine.load(&save_buffer[..]).expect("Load failed");

        // Verify
        assert!((loaded_engine.sim_time - original_time).abs() < 0.001);
        assert_eq!(
            loaded_engine.crew_count() + loaded_engine.passenger_count(),
            original_people
        );
    }
}
