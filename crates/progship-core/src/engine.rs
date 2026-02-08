//! Simulation engine - main entry point for running the simulation

use hecs::World;
use crate::components::*;
use crate::systems::*;
use crate::generation::{ShipConfig, ShipLayout, generate_ship, generate_crew, generate_passengers};

/// Main simulation engine
pub struct SimulationEngine {
    /// ECS world containing all entities
    pub world: World,
    /// Simulation time in hours since start
    pub sim_time: f64,
    /// Ship layout reference
    pub ship_layout: Option<ShipLayout>,
    /// Ship-wide resources
    pub resources: ShipResources,
    /// Maintenance task queue
    pub maintenance_queue: MaintenanceQueue,
    /// Relationship graph between people
    pub relationships: RelationshipGraph,
    /// Active conversations
    pub conversations: ConversationManager,
    /// Random events system
    pub events: EventManager,
    
    // Update timing
    last_needs_update: f64,
    last_systems_update: f64,
    last_maintenance_update: f64,
    last_social_update: f64,
    last_wandering_update: f64,
    last_duty_update: f64,
    last_events_update: f64,
    
    // Configuration
    time_scale: f32,
}

impl SimulationEngine {
    /// Create a new empty simulation
    pub fn new() -> Self {
        Self {
            world: World::new(),
            sim_time: 0.0,
            ship_layout: None,
            resources: ShipResources::new(),
            maintenance_queue: MaintenanceQueue::new(),
            relationships: RelationshipGraph::new(),
            conversations: ConversationManager::new(),
            events: EventManager::new(),
            last_needs_update: 0.0,
            last_systems_update: 0.0,
            last_maintenance_update: 0.0,
            last_social_update: 0.0,
            last_wandering_update: 0.0,
            last_duty_update: 0.0,
            last_events_update: 0.0,
            time_scale: 1.0,
        }
    }

    /// Generate a complete ship with crew and passengers
    pub fn generate(&mut self, config: ShipConfig) {
        let mut rng = rand::thread_rng();
        
        // Generate ship structure
        let layout = generate_ship(&mut self.world, &config, &mut rng);
        
        // Generate crew
        let _crew = generate_crew(
            &mut self.world,
            config.crew_size,
            &layout.rooms,
            &mut rng,
        );
        
        // Generate passengers
        let _passengers = generate_passengers(
            &mut self.world,
            config.passenger_capacity,
            &layout.rooms,
            &mut rng,
        );
        
        self.ship_layout = Some(layout);
    }

    /// Update the simulation by delta_seconds
    pub fn update(&mut self, delta_seconds: f32) {
        let scaled_delta = delta_seconds * self.time_scale;
        let delta_hours = scaled_delta as f64 / 3600.0;
        self.sim_time += delta_hours;

        // T0: Movement (every frame)
        movement_system(&mut self.world, scaled_delta);

        // T1: Activity (every frame, but checks internal timing)
        activity_system(&mut self.world, self.sim_time, delta_hours as f32);
        
        // T1: Wandering for idle people (throttled to 10Hz to reduce allocations)
        let wandering_interval = 0.1 / 3600.0; // 0.1 seconds in hours
        if self.sim_time - self.last_wandering_update >= wandering_interval {
            let room_entities: &[hecs::Entity] = self.ship_layout
                .as_ref()
                .map(|l| l.rooms.as_slice())
                .unwrap_or(&[]);
            wandering_system(&mut self.world, room_entities);
            self.last_wandering_update = self.sim_time;
        }

        // T2: Needs (0.1 Hz - every 10 seconds)
        let needs_interval = 10.0 / 3600.0; // 10 seconds in hours
        if self.sim_time - self.last_needs_update >= needs_interval {
            let elapsed = (self.sim_time - self.last_needs_update) as f32;
            needs_system(&mut self.world, elapsed);
            self.last_needs_update = self.sim_time;
        }

        // T3: Ship systems (0.01 Hz - every 100 seconds)
        let systems_interval = 100.0 / 3600.0;
        if self.sim_time - self.last_systems_update >= systems_interval {
            let elapsed = (self.sim_time - self.last_systems_update) as f32;
            ship_systems_system(&mut self.world, &mut self.resources, elapsed);
            self.last_systems_update = self.sim_time;
        }
        
        // T3: Maintenance (0.01 Hz - every 100 seconds, with ship systems)
        let maintenance_interval = 100.0 / 3600.0;
        if self.sim_time - self.last_maintenance_update >= maintenance_interval {
            let elapsed = (self.sim_time - self.last_maintenance_update) as f32;
            
            // Generate tasks for damaged systems
            generate_maintenance_tasks(&self.world, &mut self.maintenance_queue, self.sim_time);
            
            // Assign available crew
            assign_maintenance_crew(&self.world, &mut self.maintenance_queue);
            
            // Progress repairs
            progress_maintenance(&mut self.world, &mut self.maintenance_queue, elapsed);
            
            self.last_maintenance_update = self.sim_time;
        }
        
        // T2: Social interactions (0.1 Hz - every 10 seconds)
        let social_interval = 10.0 / 3600.0;
        if self.sim_time - self.last_social_update >= social_interval {
            let elapsed = (self.sim_time - self.last_social_update) as f32;
            let room_entities: &[hecs::Entity] = self.ship_layout.as_ref()
                .map(|l| l.rooms.as_slice())
                .unwrap_or(&[]);
            
            social_system(
                &mut self.world,
                &mut self.conversations,
                &mut self.relationships,
                room_entities,
                self.sim_time,
                elapsed,
            );
            
            self.last_social_update = self.sim_time;
        }
        
        // T2: Crew duty schedules (0.1 Hz - every 10 seconds)
        let duty_interval = 10.0 / 3600.0;
        if self.sim_time - self.last_duty_update >= duty_interval {
            update_duty(&mut self.world, self.sim_time);
            self.last_duty_update = self.sim_time;
        }
        
        // T3: Random events (0.01 Hz - every 100 seconds)  
        let events_interval = 100.0 / 3600.0;
        if self.sim_time - self.last_events_update >= events_interval {
            let mut rng = rand::thread_rng();
            
            generate_random_events(&self.world, &mut self.events, self.sim_time, &mut rng);
            dispatch_emergency_responders(&mut self.world, &mut self.events, self.sim_time);
            
            self.last_events_update = self.sim_time;
        }
    }

    /// Set time scale (1.0 = real-time, 2.0 = 2x speed, etc.)
    pub fn set_time_scale(&mut self, scale: f32) {
        self.time_scale = scale.max(0.0);
    }

    /// Get current time scale
    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    /// Get current simulation time in hours
    pub fn sim_time(&self) -> f64 {
        self.sim_time
    }

    /// Get current hour of day (0-24)
    pub fn hour_of_day(&self) -> f32 {
        (self.sim_time % 24.0) as f32
    }

    /// Count total people in simulation
    pub fn person_count(&self) -> usize {
        self.world.query::<&Person>().iter().count()
    }

    /// Count rooms
    pub fn room_count(&self) -> usize {
        self.world.query::<&Room>().iter().count()
    }

    /// Count crew members
    pub fn crew_count(&self) -> usize {
        self.world.query::<(&Person, &Crew)>().iter().count()
    }

    /// Count passengers
    pub fn passenger_count(&self) -> usize {
        self.world.query::<(&Person, &Passenger)>().iter().count()
    }

    /// Get all people in a specific room
    pub fn people_in_room(&self, room_id: u32) -> Vec<hecs::Entity> {
        self.world
            .query::<(&Person, &Position)>()
            .iter()
            .filter(|(_, (_, pos))| pos.room_id == room_id)
            .map(|(entity, _)| entity)
            .collect()
    }

    /// Find people with urgent needs
    pub fn people_with_urgent_needs(&self, threshold: f32) -> Vec<(hecs::Entity, NeedType)> {
        find_urgent_needs(&self.world, threshold)
    }

    /// Save simulation state to a writer
    pub fn save<W: std::io::Write>(&self, writer: W) -> Result<(), crate::persistence::SaveError> {
        crate::persistence::save_simulation(
            writer,
            &self.world,
            self.sim_time,
            self.time_scale,
            self.ship_layout.as_ref(),
            &self.resources,
            &self.maintenance_queue,
            &self.relationships,
            &self.conversations,
            &self.events,
        )
    }

    /// Load simulation state from a reader
    pub fn load<R: std::io::Read>(&mut self, reader: R) -> Result<(), crate::persistence::SaveError> {
        let loaded = crate::persistence::load_simulation(reader)?;
        
        self.world = loaded.world;
        self.sim_time = loaded.sim_time;
        self.time_scale = loaded.time_scale;
        self.resources = loaded.resources;
        self.maintenance_queue = loaded.maintenance_queue;
        self.relationships = loaded.relationships;
        self.conversations = loaded.conversations;
        self.events = loaded.events;
        
        // Rebuild ship layout from loaded entities
        if let Some(layout_info) = loaded.ship_layout_info {
            self.rebuild_ship_layout(layout_info);
        }
        
        // Reset update timers
        self.last_needs_update = self.sim_time;
        self.last_systems_update = self.sim_time;
        self.last_maintenance_update = self.sim_time;
        self.last_social_update = self.sim_time;
        self.last_wandering_update = self.sim_time;
        
        Ok(())
    }

    /// Rebuild ship layout entity references from loaded world
    fn rebuild_ship_layout(&mut self, layout_info: crate::persistence::SerializableShipLayout) {
        let mut rooms = Vec::new();
        let mut decks = Vec::new();
        let mut elevators = Vec::new();
        
        // Collect room entities
        for (entity, _room) in self.world.query::<&Room>().iter() {
            rooms.push(entity);
        }
        
        // Collect deck entities
        for (entity, _deck) in self.world.query::<&Deck>().iter() {
            decks.push(entity);
        }
        
        // Sort rooms by deck level and position for consistent ordering
        rooms.sort_by(|a, b| {
            let room_a = self.world.get::<&Room>(*a).ok();
            let room_b = self.world.get::<&Room>(*b).ok();
            match (room_a, room_b) {
                (Some(ra), Some(rb)) => {
                    ra.deck_level.cmp(&rb.deck_level)
                        .then(ra.world_x.partial_cmp(&rb.world_x).unwrap_or(std::cmp::Ordering::Equal))
                }
                _ => std::cmp::Ordering::Equal,
            }
        });
        
        // Collect elevator rooms (rooms that appear on multiple decks)
        // For simplicity, find rooms with room_type Elevator
        for (entity, room) in self.world.query::<&Room>().iter() {
            if room.room_type == RoomType::Elevator {
                elevators.push(entity);
            }
        }
        
        self.ship_layout = Some(crate::generation::ShipLayout {
            name: layout_info.name,
            rooms,
            decks,
            elevators,
            ship_length: layout_info.ship_length,
            ship_width: layout_info.ship_width,
        });
    }
}

impl Default for SimulationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = SimulationEngine::new();
        assert_eq!(engine.person_count(), 0);
        assert_eq!(engine.sim_time(), 0.0);
    }

    #[test]
    fn test_engine_generation() {
        let mut engine = SimulationEngine::new();
        
        let config = ShipConfig {
            num_decks: 2,
            rooms_per_deck: 3,
            crew_size: 10,
            passenger_capacity: 20,
            ..Default::default()
        };
        
        engine.generate(config);
        
        assert_eq!(engine.crew_count(), 10);
        assert_eq!(engine.passenger_count(), 20);
        assert_eq!(engine.person_count(), 30);
    }

    #[test]
    fn test_engine_update() {
        let mut engine = SimulationEngine::new();
        
        let config = ShipConfig {
            num_decks: 1,
            rooms_per_deck: 2,
            crew_size: 5,
            passenger_capacity: 5,
            ..Default::default()
        };
        
        engine.generate(config);
        
        // Simulate 1 hour
        for _ in 0..3600 {
            engine.update(1.0); // 1 second per frame
        }
        
        assert!((engine.sim_time() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_time_scale() {
        let mut engine = SimulationEngine::new();
        engine.set_time_scale(2.0);
        
        engine.update(1.0); // 1 real second = 2 sim seconds
        
        let expected_hours = 2.0 / 3600.0;
        assert!((engine.sim_time() - expected_hours).abs() < 0.0001);
    }
}
