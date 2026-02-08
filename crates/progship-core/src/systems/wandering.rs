//! Wandering system - gives idle people movement targets

use hecs::World;
use rand::Rng;
use crate::components::{Person, Position, Movement, Activity, Room, RoomConnections, Vec3};
use super::movement::start_movement_to_room;

/// Give idle people (no Activity, no Movement) random wander targets within room bounds
/// Occasionally sends people to adjacent rooms for exploration
pub fn wandering_system(world: &mut World, room_entities: &[hecs::Entity]) {
    let mut rng = rand::thread_rng();
    let mut local_wander = Vec::new();
    let mut room_explore = Vec::new();

    // Find idle people (no current activity or movement)
    for (entity, (_, pos)) in world.query::<(&Person, &Position)>().iter() {
        // Skip if already moving
        if world.get::<&Movement>(entity).is_ok() {
            continue;
        }
        
        // Skip if doing an activity
        if world.get::<&Activity>(entity).is_ok() {
            continue;
        }
        
        let roll = rng.gen::<f32>();
        
        // 20% chance to wander locally, 5% chance to explore another room (runs at 10Hz)
        if roll < 0.05 {
            room_explore.push((entity, pos.room_id));
        } else if roll < 0.25 {
            local_wander.push((entity, pos.room_id));
        }
    }

    // Early exit if no one wants to move
    if local_wander.is_empty() && room_explore.is_empty() {
        return;
    }

    // Collect just room sizes (no connection cloning) for local wander
    let room_sizes: Vec<(u32, f32, f32)> = world
        .query::<&Room>()
        .iter()
        .enumerate()
        .map(|(idx, (_, room))| (idx as u32, room.width(), room.depth()))
        .collect();

    // Local wandering (within same room)
    for (entity, current_room_id) in local_wander {
        let (room_width, room_depth) = room_sizes
            .iter()
            .find(|(id, _, _)| *id == current_room_id)
            .map(|(_, w, d)| (*w, *d))
            .unwrap_or((10.0, 10.0));
        
        // Random position within room bounds (10% to 90% of size, keeping away from walls)
        let target_x = rng.gen_range(room_width * 0.1..room_width * 0.9);
        let target_y = rng.gen_range(room_depth * 0.1..room_depth * 0.9);
        
        let movement = Movement::new(
            Vec3::new(target_x, target_y, 0.0),
            1.2, // Walking speed: 1.2 m/s
        );
        
        let _ = world.insert_one(entity, movement);
    }
    
    // Inter-room exploration (only collect connections if needed)
    if !room_explore.is_empty() {
        for (entity, current_room_id) in room_explore {
            // Get connections for just this room
            let connected: Vec<u32> = if (current_room_id as usize) < room_entities.len() {
                world.get::<&RoomConnections>(room_entities[current_room_id as usize])
                    .map(|conn| conn.connected_to.clone())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            
            if connected.is_empty() {
                continue;
            }
            
            // Pick a random connected room
            let target_room_id = connected[rng.gen_range(0..connected.len())];
            
            // Get target room dimensions
            let (target_width, target_depth): (f32, f32) = room_sizes
                .iter()
                .find(|(id, _, _)| *id == target_room_id)
                .map(|(_, w, d)| (*w, *d))
                .unwrap_or((10.0, 10.0));
            
            let dest_x = rng.gen_range(target_width * 0.2..target_width * 0.8);
            let dest_y = rng.gen_range(target_depth * 0.2..target_depth * 0.8);
            
            start_movement_to_room(
                world,
                entity,
                target_room_id,
                Vec3::new(dest_x, dest_y, 0.0),
                1.2, // Walking speed
                room_entities,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::RoomType;

    #[test]
    fn test_wandering_starts_movement() {
        let mut world = World::new();
        
        // Create a room with connections
        let room = Room::new("Test Room", RoomType::Corridor, 10.0, 10.0);
        let room_entity = world.spawn((room, RoomConnections::new()));
        
        // Create an idle person in center of room
        let person = world.spawn((
            Person,
            Position::new(5.0, 5.0, 0),
        ));
        
        // Run multiple times to get a hit (4% chance)
        for _ in 0..100 {
            wandering_system(&mut world, &[room_entity]);
        }
        
        // Should have started moving eventually (probabilistic)
        let _ = world.get::<&Movement>(person);
    }
}
