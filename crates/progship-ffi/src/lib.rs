//! C FFI bindings for ProgShip simulation engine
//!
//! This crate provides a C-compatible API for integrating the ProgShip simulation
//! with game engines like Godot, Unity, Unreal, or any language with C FFI support.
//!
//! # Basic Usage (C)
//! ```c
//! #include "progship.h"
//!
//! // Create simulation
//! ProgShipHandle sim = progship_create();
//! progship_generate(sim, 5, 10, 4000, 1000);
//!
//! // Game loop
//! while (running) {
//!     progship_update(sim, delta_time);
//!     
//!     // Query people
//!     uint32_t count = progship_person_count(sim);
//!     for (uint32_t i = 0; i < count; i++) {
//!         ProgShipPerson person;
//!         if (progship_get_person(sim, i, &person)) {
//!             // Use person.world_x, person.world_y, person.room_id, etc.
//!         }
//!     }
//! }
//!
//! // Cleanup
//! progship_destroy(sim);
//! ```

use progship_core::engine::SimulationEngine;
use progship_core::generation::ShipConfig;
use progship_core::components::{Position, Person, Crew, Room, Needs, Vec3};

/// Opaque handle to the simulation engine
pub type ProgShipHandle = *mut SimulationEngine;

/// Person data returned to C
#[repr(C)]
pub struct ProgShipPerson {
    /// Index of this person (0 to person_count-1)
    pub index: u32,
    /// World X coordinate
    pub world_x: f32,
    /// World Y coordinate  
    pub world_y: f32,
    /// Room ID the person is in
    pub room_id: u32,
    /// Deck level (0-indexed)
    pub deck_level: i32,
    /// 1 if crew, 0 if passenger
    pub is_crew: u8,
    /// Hunger need (0.0 = satisfied, 1.0 = starving)
    pub hunger: f32,
    /// Fatigue need (0.0 = rested, 1.0 = exhausted)
    pub fatigue: f32,
    /// Social need (0.0 = satisfied, 1.0 = lonely)
    pub social: f32,
}

/// Room data returned to C
#[repr(C)]
pub struct ProgShipRoom {
    /// Room ID (index)
    pub id: u32,
    /// World X position (center)
    pub world_x: f32,
    /// World Y position (center)
    pub world_y: f32,
    /// Room width in meters
    pub width: f32,
    /// Room depth in meters
    pub depth: f32,
    /// Deck level
    pub deck_level: i32,
    /// Room type (see RoomType enum values)
    pub room_type: u8,
}

/// Simulation statistics
#[repr(C)]
pub struct ProgShipStats {
    /// Current simulation time in hours
    pub sim_time_hours: f64,
    /// Number of crew members
    pub crew_count: u32,
    /// Number of passengers
    pub passenger_count: u32,
    /// Number of rooms
    pub room_count: u32,
    /// Number of active conversations
    pub conversation_count: u32,
    /// Number of pending maintenance tasks
    pub maintenance_count: u32,
    /// Current time scale
    pub time_scale: f32,
}

// ============================================================================
// Lifecycle Functions
// ============================================================================

/// Create a new simulation engine
/// 
/// Returns a handle that must be freed with `progship_destroy`
#[no_mangle]
pub extern "C" fn progship_create() -> ProgShipHandle {
    Box::into_raw(Box::new(SimulationEngine::new()))
}

/// Destroy a simulation engine and free its memory
#[no_mangle]
pub extern "C" fn progship_destroy(handle: ProgShipHandle) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

/// Generate a ship with the specified parameters
/// 
/// # Parameters
/// - `num_decks`: Number of decks (1-10 recommended)
/// - `rooms_per_deck`: Rooms per deck (5-20 recommended)
/// - `passenger_capacity`: Number of passengers to generate
/// - `crew_size`: Number of crew members to generate
#[no_mangle]
pub extern "C" fn progship_generate(
    handle: ProgShipHandle,
    num_decks: u32,
    rooms_per_deck: u32,
    passenger_capacity: u32,
    crew_size: u32,
) {
    if handle.is_null() {
        return;
    }
    
    let sim = unsafe { &mut *handle };
    let config = ShipConfig {
        name: "Colony Ship".to_string(),
        num_decks,
        rooms_per_deck,
        passenger_capacity,
        crew_size,
        ship_length: 200.0,
        ship_width: 40.0,
    };
    sim.generate(config);
}

// ============================================================================
// Simulation Control
// ============================================================================

/// Update the simulation by delta_seconds (in real time)
/// 
/// The actual simulation time advanced depends on the time scale.
#[no_mangle]
pub extern "C" fn progship_update(handle: ProgShipHandle, delta_seconds: f32) {
    if handle.is_null() {
        return;
    }
    
    let sim = unsafe { &mut *handle };
    sim.update(delta_seconds);
}

/// Set the time scale (1.0 = real-time, 10.0 = 10x speed)
#[no_mangle]
pub extern "C" fn progship_set_time_scale(handle: ProgShipHandle, scale: f32) {
    if handle.is_null() {
        return;
    }
    
    let sim = unsafe { &mut *handle };
    sim.set_time_scale(scale);
}

/// Get current time scale
#[no_mangle]
pub extern "C" fn progship_get_time_scale(handle: ProgShipHandle) -> f32 {
    if handle.is_null() {
        return 0.0;
    }
    
    let sim = unsafe { &*handle };
    sim.time_scale()
}

// ============================================================================
// Query Functions
// ============================================================================

/// Get simulation statistics
#[no_mangle]
pub extern "C" fn progship_get_stats(handle: ProgShipHandle, stats: *mut ProgShipStats) -> bool {
    if handle.is_null() || stats.is_null() {
        return false;
    }
    
    let sim = unsafe { &*handle };
    let out = unsafe { &mut *stats };
    
    out.sim_time_hours = sim.sim_time;
    out.crew_count = sim.crew_count() as u32;
    out.passenger_count = sim.passenger_count() as u32;
    out.room_count = sim.ship_layout.as_ref().map(|l| l.rooms.len() as u32).unwrap_or(0);
    out.conversation_count = sim.conversations.active_count() as u32;
    out.maintenance_count = sim.maintenance_queue.tasks.len() as u32;
    out.time_scale = sim.time_scale();
    
    true
}

/// Get the total number of people (crew + passengers)
#[no_mangle]
pub extern "C" fn progship_person_count(handle: ProgShipHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    
    let sim = unsafe { &*handle };
    (sim.crew_count() + sim.passenger_count()) as u32
}

/// Get person data by index
/// 
/// Returns true if successful, false if index out of bounds
#[no_mangle]
pub extern "C" fn progship_get_person(
    handle: ProgShipHandle,
    index: u32,
    person: *mut ProgShipPerson,
) -> bool {
    if handle.is_null() || person.is_null() {
        return false;
    }
    
    let sim = unsafe { &*handle };
    let out = unsafe { &mut *person };
    
    let mut current_idx: u32 = 0;
    
    for (entity, (_, pos)) in sim.world.query::<(&Person, &Position)>().iter() {
        if current_idx == index {
            out.index = index;
            out.room_id = pos.room_id;
            
            // Get world position from room
            let (world_x, world_y, deck_level) = get_world_position(sim, pos);
            out.world_x = world_x;
            out.world_y = world_y;
            out.deck_level = deck_level;
            
            // Check if crew or passenger
            out.is_crew = if sim.world.get::<&Crew>(entity).is_ok() { 1 } else { 0 };
            
            // Get needs
            if let Ok(needs) = sim.world.get::<&Needs>(entity) {
                out.hunger = needs.hunger;
                out.fatigue = needs.fatigue;
                out.social = needs.social;
            } else {
                out.hunger = 0.0;
                out.fatigue = 0.0;
                out.social = 0.0;
            }
            
            return true;
        }
        current_idx += 1;
    }
    
    false
}

/// Get the number of rooms
#[no_mangle]
pub extern "C" fn progship_room_count(handle: ProgShipHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    
    let sim = unsafe { &*handle };
    sim.ship_layout.as_ref().map(|l| l.rooms.len() as u32).unwrap_or(0)
}

/// Get room data by index
#[no_mangle]
pub extern "C" fn progship_get_room(
    handle: ProgShipHandle,
    index: u32,
    room: *mut ProgShipRoom,
) -> bool {
    if handle.is_null() || room.is_null() {
        return false;
    }
    
    let sim = unsafe { &*handle };
    let layout = match &sim.ship_layout {
        Some(l) => l,
        None => return false,
    };
    
    if index as usize >= layout.rooms.len() {
        return false;
    }
    
    let room_entity = layout.rooms[index as usize];
    let room_data = match sim.world.get::<&Room>(room_entity) {
        Ok(r) => r,
        Err(_) => return false,
    };
    
    let out = unsafe { &mut *room };
    out.id = index;
    out.world_x = room_data.world_x;
    out.world_y = room_data.world_y;
    out.width = room_data.width();
    out.depth = room_data.depth();
    out.deck_level = room_data.deck_level;
    out.room_type = room_data.room_type as u8;
    
    true
}

/// Get the number of decks
#[no_mangle]
pub extern "C" fn progship_deck_count(handle: ProgShipHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    
    let sim = unsafe { &*handle };
    sim.ship_layout.as_ref().map(|l| l.decks.len() as u32).unwrap_or(0)
}

/// Get ship dimensions
#[no_mangle]
pub extern "C" fn progship_get_ship_dimensions(
    handle: ProgShipHandle,
    length: *mut f32,
    width: *mut f32,
) -> bool {
    if handle.is_null() {
        return false;
    }
    
    let sim = unsafe { &*handle };
    let layout = match &sim.ship_layout {
        Some(l) => l,
        None => return false,
    };
    
    if !length.is_null() {
        unsafe { *length = layout.ship_length; }
    }
    if !width.is_null() {
        unsafe { *width = layout.ship_width; }
    }
    
    true
}

/// Get the current simulation time as hours since start
#[no_mangle]
pub extern "C" fn progship_get_sim_time(handle: ProgShipHandle) -> f64 {
    if handle.is_null() {
        return 0.0;
    }
    
    let sim = unsafe { &*handle };
    sim.sim_time
}

/// Get the current hour of day (0-23)
#[no_mangle]
pub extern "C" fn progship_get_hour_of_day(handle: ProgShipHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    
    let sim = unsafe { &*handle };
    sim.hour_of_day() as u32
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_world_position(sim: &SimulationEngine, pos: &Position) -> (f32, f32, i32) {
    let layout = match &sim.ship_layout {
        Some(l) => l,
        None => return (pos.local.x, pos.local.y, 0),
    };
    
    if (pos.room_id as usize) >= layout.rooms.len() {
        return (pos.local.x, pos.local.y, 0);
    }
    
    let room_entity = layout.rooms[pos.room_id as usize];
    match sim.world.get::<&Room>(room_entity) {
        Ok(room) => {
            let world: Vec3 = room.local_to_world(pos.local);
            (world.x, world.y, room.deck_level)
        }
        Err(_) => (pos.local.x, pos.local.y, 0),
    }
}
