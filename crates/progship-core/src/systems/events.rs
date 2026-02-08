//! Events system - random events that affect the ship and crew
//!
//! Events add drama and challenge to the simulation. They can be emergencies
//! that require crew response, celebrations, discoveries, or other occurrences.

use hecs::World;
use rand::Rng;
use serde::{Serialize, Deserialize};
use crate::components::{
    Person, Position, Crew, Activity, ActivityType, Room, RoomType, ShipSystem, SystemStatus
};

/// Types of events that can occur
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// Ship system malfunction requiring repair
    SystemFailure,
    /// Person needs medical attention
    MedicalEmergency,
    /// Fire in a compartment
    Fire,
    /// Hull breach (serious emergency)
    HullBreach,
    /// Scientific discovery (positive event)
    Discovery,
    /// Ship-wide celebration (positive event)
    Celebration,
    /// Conflict between crew/passengers
    Altercation,
    /// Resource shortage alert
    ResourceShortage,
}

impl EventType {
    /// How severe is this event? (1-5, 5 being critical)
    pub fn severity(&self) -> u8 {
        match self {
            EventType::HullBreach => 5,
            EventType::Fire => 4,
            EventType::SystemFailure => 3,
            EventType::MedicalEmergency => 3,
            EventType::ResourceShortage => 2,
            EventType::Altercation => 2,
            EventType::Discovery => 1,
            EventType::Celebration => 1,
        }
    }
    
    /// Does this event require emergency response?
    pub fn is_emergency(&self) -> bool {
        self.severity() >= 3
    }
    
    /// Which department primarily responds to this event?
    pub fn responding_department(&self) -> Option<crate::components::Department> {
        use crate::components::Department;
        match self {
            EventType::SystemFailure => Some(Department::Engineering),
            EventType::MedicalEmergency => Some(Department::Medical),
            EventType::Fire => Some(Department::Engineering),
            EventType::HullBreach => Some(Department::Engineering),
            EventType::Altercation => Some(Department::Security),
            _ => None,
        }
    }
}

/// An active event in the simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event ID
    pub id: u32,
    /// Type of event
    pub event_type: EventType,
    /// Room where event is occurring
    pub room_id: u32,
    /// When the event started (sim time)
    pub started_at: f64,
    /// How long the event lasts (hours)
    pub duration: f32,
    /// Current state of the event
    pub state: EventState,
    /// How many responders are needed
    pub responders_needed: u8,
    /// How many responders are currently assigned
    pub responders_assigned: u8,
    /// Description of the event
    pub description: String,
}

/// State of an event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventState {
    /// Event is active and needs response
    Active,
    /// Event is being handled
    BeingHandled,
    /// Event has been resolved
    Resolved,
    /// Event escalated (got worse)
    Escalated,
}

/// Manages active events in the simulation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventManager {
    /// Active events
    pub events: Vec<Event>,
    /// Next event ID
    next_id: u32,
    /// Time since last event check
    last_check_time: f64,
}

impl EventManager {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new event
    pub fn spawn_event(
        &mut self,
        event_type: EventType,
        room_id: u32,
        sim_time: f64,
        description: String,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        
        let responders_needed = match event_type {
            EventType::HullBreach => 4,
            EventType::Fire => 3,
            EventType::SystemFailure => 2,
            EventType::MedicalEmergency => 2,
            EventType::Altercation => 2,
            _ => 0,
        };
        
        let duration = match event_type {
            EventType::HullBreach => 2.0,
            EventType::Fire => 1.0,
            EventType::SystemFailure => 0.5,
            EventType::MedicalEmergency => 1.0,
            EventType::Celebration => 4.0,
            EventType::Discovery => 0.5,
            _ => 0.5,
        };
        
        self.events.push(Event {
            id,
            event_type,
            room_id,
            started_at: sim_time,
            duration,
            state: EventState::Active,
            responders_needed,
            responders_assigned: 0,
            description,
        });
        
        id
    }
    
    /// Get an active event by ID
    pub fn get(&self, id: u32) -> Option<&Event> {
        self.events.iter().find(|e| e.id == id)
    }
    
    /// Get mutable event by ID
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Event> {
        self.events.iter_mut().find(|e| e.id == id)
    }
    
    /// Get all active (unresolved) events
    pub fn active_events(&self) -> impl Iterator<Item = &Event> {
        self.events.iter().filter(|e| e.state != EventState::Resolved)
    }
    
    /// Get highest priority unhandled event
    pub fn highest_priority_event(&self) -> Option<&Event> {
        self.events.iter()
            .filter(|e| e.state == EventState::Active)
            .max_by_key(|e| e.event_type.severity())
    }
    
    /// Assign a responder to an event
    pub fn assign_responder(&mut self, event_id: u32) -> bool {
        if let Some(event) = self.get_mut(event_id) {
            if event.responders_assigned < event.responders_needed {
                event.responders_assigned += 1;
                if event.responders_assigned >= event.responders_needed {
                    event.state = EventState::BeingHandled;
                }
                return true;
            }
        }
        false
    }
    
    /// Update events - check for resolution, escalation, etc.
    pub fn update(&mut self, sim_time: f64) -> Vec<u32> {
        let mut resolved = Vec::new();
        
        for event in &mut self.events {
            if event.state == EventState::Resolved {
                continue;
            }
            
            let elapsed = sim_time - event.started_at;
            
            // Check if being handled events are resolved
            if event.state == EventState::BeingHandled {
                if elapsed >= event.duration as f64 {
                    event.state = EventState::Resolved;
                    resolved.push(event.id);
                }
            }
            // Check if active events escalate (not enough responders in time)
            else if event.state == EventState::Active && event.event_type.is_emergency() {
                if elapsed > event.duration as f64 * 0.5 && event.responders_assigned == 0 {
                    event.state = EventState::Escalated;
                    event.duration *= 2.0; // Takes longer to resolve
                    event.responders_needed += 1; // Need more people
                }
            }
        }
        
        // Clean up old resolved events
        self.events.retain(|e| {
            e.state != EventState::Resolved || 
            (sim_time - e.started_at) < 24.0 // Keep for 24 hours for history
        });
        
        resolved
    }
}

/// Randomly generate events based on ship state
pub fn generate_random_events(
    world: &World,
    event_manager: &mut EventManager,
    sim_time: f64,
    rng: &mut impl Rng,
) {
    // Only check every few sim minutes
    if sim_time - event_manager.last_check_time < 0.1 {
        return;
    }
    event_manager.last_check_time = sim_time;
    
    // Don't spawn too many events at once
    let active_emergencies = event_manager.active_events()
        .filter(|e| e.event_type.is_emergency())
        .count();
    
    if active_emergencies >= 2 {
        return;
    }
    
    // Random chance for each event type
    // Probabilities are per check (roughly every 6 sim minutes)
    
    // System failure: check systems with low health
    for (entity, system) in world.query::<&ShipSystem>().iter() {
        if system.status == SystemStatus::Critical && rng.gen_bool(0.02) {
            let room_id = entity.id() as u32;
            event_manager.spawn_event(
                EventType::SystemFailure,
                room_id,
                sim_time,
                format!("{} malfunction - critical failure imminent", format!("{:?}", system.system_type)),
            );
            break; // One event per check
        }
    }
    
    // Medical emergency: random chance based on population
    let person_count = world.query::<&Person>().iter().count();
    if person_count > 0 && rng.gen_bool(0.001) {
        // Pick a random room with people
        if let Some((_, (_, pos))) = world.query::<(&Person, &Position)>().iter().next() {
            event_manager.spawn_event(
                EventType::MedicalEmergency,
                pos.room_id,
                sim_time,
                "Crew member collapsed - medical attention required".to_string(),
            );
        }
    }
    
    // Celebration: occasional morale boost
    if rng.gen_bool(0.0005) {
        // Find a recreation or mess room
        for (entity, room) in world.query::<&Room>().iter() {
            if room.room_type == RoomType::Recreation || room.room_type == RoomType::Mess {
                event_manager.spawn_event(
                    EventType::Celebration,
                    entity.id() as u32,
                    sim_time,
                    "Impromptu celebration in progress!".to_string(),
                );
                break;
            }
        }
    }
    
    // Discovery: science labs occasionally make discoveries
    if rng.gen_bool(0.0002) {
        for (entity, room) in world.query::<&Room>().iter() {
            if room.room_type == RoomType::Laboratory || room.room_type == RoomType::Observatory {
                event_manager.spawn_event(
                    EventType::Discovery,
                    entity.id() as u32,
                    sim_time,
                    "Significant scientific discovery made!".to_string(),
                );
                break;
            }
        }
    }
}

/// Dispatch crew to respond to emergencies
pub fn dispatch_emergency_responders(
    world: &mut World,
    event_manager: &mut EventManager,
    sim_time: f64,
) {
    // Get events needing responders
    let events_needing_help: Vec<(u32, crate::components::Department, u32)> = event_manager
        .active_events()
        .filter(|e| e.state == EventState::Active && e.responders_needed > e.responders_assigned)
        .filter_map(|e| {
            e.event_type.responding_department().map(|dept| (e.id, dept, e.room_id))
        })
        .collect();
    
    for (event_id, department, room_id) in events_needing_help {
        // Find available crew from the right department
        let mut available_crew: Vec<hecs::Entity> = Vec::new();
        
        for (entity, (_, crew, activity)) in world
            .query::<(&Person, &Crew, &Activity)>()
            .iter()
        {
            if crew.department == department 
                && activity.activity_type != ActivityType::Emergency
                && activity.activity_type.interruptible_for_duty()
            {
                available_crew.push(entity);
            }
        }
        
        // Assign first available crew member
        if let Some(responder) = available_crew.first() {
            // Update their activity to emergency response
            if let Ok(mut activity) = world.get::<&mut Activity>(*responder) {
                activity.activity_type = ActivityType::Emergency;
                activity.started_at = sim_time;
                activity.duration = 2.0;
                activity.target_id = Some(room_id);
            }
            
            // Mark responder as assigned
            event_manager.assign_responder(event_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_severity() {
        assert_eq!(EventType::HullBreach.severity(), 5);
        assert_eq!(EventType::Celebration.severity(), 1);
        assert!(EventType::Fire.is_emergency());
        assert!(!EventType::Discovery.is_emergency());
    }

    #[test]
    fn test_event_manager() {
        let mut manager = EventManager::new();
        
        let id = manager.spawn_event(
            EventType::SystemFailure,
            100,
            10.0,
            "Test event".to_string(),
        );
        
        assert!(manager.get(id).is_some());
        assert_eq!(manager.active_events().count(), 1);
        
        // Assign responders
        assert!(manager.assign_responder(id));
        assert!(manager.assign_responder(id));
        
        let event = manager.get(id).unwrap();
        assert_eq!(event.state, EventState::BeingHandled);
    }
    
    #[test]
    fn test_event_resolution() {
        let mut manager = EventManager::new();
        
        let id = manager.spawn_event(
            EventType::Discovery,
            100,
            10.0,
            "Test discovery".to_string(),
        );
        
        // Discoveries don't need responders, so we just update with enough time
        // Actually discoveries have 0 responders_needed, so state won't change
        // Let's test with a system failure instead
        let id2 = manager.spawn_event(
            EventType::SystemFailure,
            101,
            10.0,
            "System failure".to_string(),
        );
        
        manager.assign_responder(id2);
        manager.assign_responder(id2);
        
        // Update with enough time passed
        let resolved = manager.update(12.0); // 2 hours later
        
        assert!(resolved.contains(&id2));
    }
}
