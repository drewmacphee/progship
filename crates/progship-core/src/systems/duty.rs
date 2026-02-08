//! Duty system - manages crew shift schedules and duty stations

use hecs::World;
use crate::components::{
    Person, Position, Crew, Shift, Activity, ActivityType, Movement, Room, RoomType, Vec3
};

/// Update crew duty states based on current ship time
/// Should be called at ~1Hz (every simulated second)
pub fn update_duty(world: &mut World, sim_time: f64) {
    let hour = (sim_time % 24.0) as f32;
    
    // Collect crew that need duty state updates
    let mut updates: Vec<(hecs::Entity, bool, u32)> = Vec::new();
    
    for (entity, (_, crew, activity)) in world
        .query::<(&Person, &Crew, &Activity)>()
        .iter()
    {
        let should_be_on_duty = crew.shift.is_active(hour);
        let is_on_duty = activity.activity_type == ActivityType::OnDuty;
        
        // Only update if duty state needs to change
        if should_be_on_duty != is_on_duty {
            // Check if current activity can be interrupted
            if !should_be_on_duty || activity.activity_type.interruptible_for_duty() {
                updates.push((entity, should_be_on_duty, crew.duty_station_id));
            }
        }
    }
    
    // Apply duty state changes
    for (entity, going_on_duty, station_id) in updates {
        if going_on_duty {
            // Start duty shift
            if let Ok(mut activity) = world.get::<&mut Activity>(entity) {
                activity.activity_type = ActivityType::OnDuty;
                activity.started_at = sim_time;
                activity.duration = 8.0; // 8 hour shift
            }
            
            // Move to duty station if we have one
            if station_id > 0 {
                start_movement_to_room(world, entity, station_id);
            }
        } else {
            // End duty shift - go off duty
            if let Ok(mut activity) = world.get::<&mut Activity>(entity) {
                activity.activity_type = ActivityType::OffDuty;
                activity.started_at = sim_time;
                activity.duration = 16.0; // Until next shift (roughly)
            }
        }
    }
}

/// Start movement toward a specific room
fn start_movement_to_room(world: &mut World, entity: hecs::Entity, target_room_id: u32) {
    // Find target room position
    let mut target_pos = None;
    for (room_entity, room) in world.query::<&Room>().iter() {
        if room_entity.id() as u32 == target_room_id {
            target_pos = Some((room.world_x, room.world_y, room.deck_level));
            break;
        }
    }
    
    if let Some((x, y, deck)) = target_pos {
        // Get current room ID
        let current_room_id = world.get::<&Position>(entity)
            .map(|pos| pos.room_id)
            .unwrap_or(0);
        
        // Only start movement if not already at target
        if current_room_id != target_room_id {
            let movement = Movement {
                destination: Vec3::new(x, y, deck as f32),
                final_destination: Vec3::new(x, y, deck as f32),
                speed: 1.2, // Walking speed in m/s
                path: vec![target_room_id],
                path_index: 0,
                next_door_position: None,
                entry_door_positions: Vec::new(),
                exit_door_positions: Vec::new(),
            };
            
            let _ = world.insert_one(entity, movement);
        }
    }
}

/// Assign duty stations to crew based on department and available rooms
pub fn assign_duty_stations(world: &mut World) {
    // Collect rooms by type for assignment
    let mut bridge_rooms: Vec<u32> = Vec::new();
    let mut engineering_rooms: Vec<u32> = Vec::new();
    let mut medical_rooms: Vec<u32> = Vec::new();
    let mut science_rooms: Vec<u32> = Vec::new();
    let mut operations_rooms: Vec<u32> = Vec::new();
    
    for (entity, room) in world.query::<&Room>().iter() {
        let room_id = entity.id() as u32;
        match room.room_type {
            RoomType::Bridge | RoomType::ConferenceRoom => bridge_rooms.push(room_id),
            RoomType::Engineering | RoomType::ReactorRoom | RoomType::MaintenanceBay => engineering_rooms.push(room_id),
            RoomType::Medical => medical_rooms.push(room_id),
            RoomType::Laboratory | RoomType::Observatory => science_rooms.push(room_id),
            RoomType::Cargo | RoomType::Storage => operations_rooms.push(room_id),
            _ => {}
        }
    }
    
    // Assign crew to appropriate rooms
    let mut assignments: Vec<(hecs::Entity, u32)> = Vec::new();
    
    for (entity, (_, crew)) in world.query::<(&Person, &Crew)>().iter() {
        use crate::components::Department;
        
        let rooms = match crew.department {
            Department::Command => &bridge_rooms,
            Department::Engineering => &engineering_rooms,
            Department::Medical => &medical_rooms,
            Department::Science => &science_rooms,
            Department::Security => &bridge_rooms, // Security on bridge
            Department::Operations => &operations_rooms,
            Department::Civilian => &operations_rooms,
        };
        
        if !rooms.is_empty() {
            // Round-robin assignment
            let idx = entity.id() as usize % rooms.len();
            assignments.push((entity, rooms[idx]));
        }
    }
    
    // Apply assignments
    for (entity, station_id) in assignments {
        if let Ok(mut crew) = world.get::<&mut Crew>(entity) {
            crew.duty_station_id = station_id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Department, Rank};

    #[test]
    fn test_shift_is_active() {
        assert!(Shift::Alpha.is_active(8.0));  // 8am
        assert!(!Shift::Alpha.is_active(16.0)); // 4pm
        
        assert!(Shift::Beta.is_active(16.0));  // 4pm
        assert!(!Shift::Beta.is_active(8.0));  // 8am
        
        assert!(Shift::Gamma.is_active(23.0)); // 11pm
        assert!(Shift::Gamma.is_active(3.0));  // 3am
        assert!(!Shift::Gamma.is_active(12.0)); // noon
    }

    #[test]
    fn test_duty_update() {
        let mut world = World::new();
        
        // Create a crew member on Alpha shift at 8am (should be on duty)
        let crew_entity = world.spawn((
            Person,
            Crew::new(Department::Engineering, Rank::Crewman, Shift::Alpha),
            Activity::new(ActivityType::Idle, 0.0, 1.0),
            Position::default(),
        ));
        
        // Simulate at 8am - should go on duty
        update_duty(&mut world, 8.0);
        
        let activity = world.get::<&Activity>(crew_entity).unwrap();
        assert_eq!(activity.activity_type, ActivityType::OnDuty);
    }
}
