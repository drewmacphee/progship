//! Movement system - updates positions for entities with Movement component

use hecs::World;
use crate::components::{Position, Movement, Room, Vec3};

/// Move entities toward their destinations (handles inter-room paths)
pub fn movement_system(world: &mut World, delta_seconds: f32) {
    // Reusable buffer - sized for typical use
    let mut updates: Vec<(hecs::Entity, Position, Option<Movement>)> = Vec::with_capacity(256);

    // Collect updates (can't mutate while iterating)
    for (entity, (pos, movement)) in world.query::<(&Position, &Movement)>().iter() {
        let result = process_movement(pos, movement, delta_seconds);
        updates.push((entity, result.0, result.1));
    }

    // Apply updates
    for (entity, new_pos, new_movement) in updates {
        if let Ok(mut pos) = world.get::<&mut Position>(entity) {
            *pos = new_pos;
        }
        
        match new_movement {
            Some(mov) => {
                // Update movement component
                if let Ok(mut m) = world.get::<&mut Movement>(entity) {
                    *m = mov;
                }
            }
            None => {
                // Remove movement - arrived
                let _ = world.remove_one::<Movement>(entity);
            }
        }
    }
}

/// Process movement for a single entity, returns new position and optionally updated movement
fn process_movement(pos: &Position, movement: &Movement, delta_seconds: f32) -> (Position, Option<Movement>) {
    let current = pos.local;
    let target = movement.destination;
    
    let diff = target - current;
    let distance = diff.length();
    let step = movement.speed * delta_seconds;
    
    // Check if we've arrived (or will arrive this frame)
    if distance < 0.1 || step >= distance {
        // Arrived at current target
        let mut new_pos = Position {
            local: target,
            room: pos.room,
            room_id: pos.room_id,
        };
        
        // Check if we have more rooms in path
        if !movement.path.is_empty() && movement.path_index < movement.path.len() - 1 {
            // Move to next room in path
            let next_room_id = movement.path[movement.path_index + 1];
            new_pos.room_id = next_room_id;
            
            // Use door position for entering the room (stored in movement)
            let entry_pos = movement.next_door_position.unwrap_or(Vec3::new(0.0, 0.0, 0.0));
            new_pos.local = entry_pos;
            
            let is_final_room = movement.path_index + 1 == movement.path.len() - 1;
            let next_destination = if is_final_room {
                // Final room - go to final destination
                movement.final_destination
            } else {
                // Intermediate room - go to exit door
                movement.exit_door_positions.get(movement.path_index + 1)
                    .copied()
                    .unwrap_or(Vec3::new(5.0, 0.0, 0.0))
            };
            
            let new_movement = Movement {
                destination: next_destination,
                speed: movement.speed,
                path: movement.path.clone(),
                path_index: movement.path_index + 1,
                final_destination: movement.final_destination,
                next_door_position: movement.entry_door_positions.get(movement.path_index + 2).copied(),
                entry_door_positions: movement.entry_door_positions.clone(),
                exit_door_positions: movement.exit_door_positions.clone(),
            };
            
            (new_pos, Some(new_movement))
        } else {
            // Fully arrived - no more path
            (new_pos, None)
        }
    } else {
        // Still moving toward current destination
        let direction = diff.normalize();
        let new_local = current + direction * step;
        
        let new_pos = Position {
            local: new_local,
            room: pos.room,
            room_id: pos.room_id,
        };
        
        (new_pos, Some(movement.clone()))
    }
}

/// Calculate path between rooms (simple A* on room graph)
pub fn find_path(
    world: &World,
    from_room_id: u32,
    to_room_id: u32,
) -> Option<Vec<u32>> {
    use crate::components::RoomConnections;
    use std::collections::{BinaryHeap, HashMap};
    use std::cmp::Reverse;

    if from_room_id == to_room_id {
        return Some(vec![to_room_id]);
    }

    // Build room graph from world
    let mut connections: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut room_id_counter = 0u32;
    
    for (_, conn) in world.query::<&RoomConnections>().iter() {
        connections.insert(room_id_counter, conn.connected_to.clone());
        room_id_counter += 1;
    }

    // A* search (using BFS heuristic since we don't have positions)
    let mut frontier = BinaryHeap::new();
    let mut came_from: HashMap<u32, u32> = HashMap::new();
    let mut cost_so_far: HashMap<u32, u32> = HashMap::new();

    frontier.push(Reverse((0, from_room_id)));
    cost_so_far.insert(from_room_id, 0);

    while let Some(Reverse((_, current))) = frontier.pop() {
        if current == to_room_id {
            // Reconstruct path
            let mut path = vec![current];
            let mut node = current;
            while let Some(&prev) = came_from.get(&node) {
                path.push(prev);
                node = prev;
            }
            path.reverse();
            return Some(path);
        }

        if let Some(neighbors) = connections.get(&current) {
            for &next in neighbors {
                let new_cost = cost_so_far.get(&current).unwrap_or(&u32::MAX).saturating_add(1);
                if !cost_so_far.contains_key(&next) || new_cost < *cost_so_far.get(&next).unwrap() {
                    cost_so_far.insert(next, new_cost);
                    came_from.insert(next, current);
                    frontier.push(Reverse((new_cost, next)));
                }
            }
        }
    }

    None // No path found
}

/// Start movement for an entity to a destination room
pub fn start_movement_to_room(
    world: &mut World,
    entity: hecs::Entity,
    target_room_id: u32,
    destination_in_room: Vec3,
    speed: f32,
    room_entities: &[hecs::Entity],
) -> bool {
    let current_room_id = match world.get::<&Position>(entity) {
        Ok(pos) => pos.room_id,
        Err(_) => return false,
    };
    
    if let Some(path) = find_path(world, current_room_id, target_room_id) {
        // Calculate door positions for each room in path
        let mut entry_door_positions = Vec::new();
        let mut exit_door_positions = Vec::new();
        
        for &room_id in &path {
            if (room_id as usize) < room_entities.len() {
                if let Ok(room) = world.get::<&Room>(room_entities[room_id as usize]) {
                    entry_door_positions.push(room.door_position());
                    exit_door_positions.push(room.door_position());
                } else {
                    entry_door_positions.push(Vec3::new(0.0, 0.0, 0.0));
                    exit_door_positions.push(Vec3::new(5.0, 5.0, 0.0));
                }
            }
        }
        
        // First destination: door of current room (to exit)
        let first_destination = if !exit_door_positions.is_empty() {
            exit_door_positions[0]
        } else {
            destination_in_room
        };
        
        let next_door = entry_door_positions.get(1).copied();
        
        let movement = Movement {
            destination: first_destination,
            final_destination: destination_in_room,
            speed,
            path,
            path_index: 0,
            next_door_position: next_door,
            entry_door_positions,
            exit_door_positions,
        };
        
        let _ = world.insert_one(entity, movement);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movement_arrives() {
        let mut world = World::new();
        
        let entity = world.spawn((
            Position::new(0.0, 0.0, 0),
            Movement::new(Vec3::new(1.0, 0.0, 0.0), 2.0),
        ));

        // Move for 1 second at speed 2 - should arrive (distance is 1)
        movement_system(&mut world, 1.0);
        
        // Should have arrived and Movement removed
        assert!(world.get::<&Movement>(entity).is_err());
        
        let pos = world.get::<&Position>(entity).unwrap();
        assert!((pos.local.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_movement_partial() {
        let mut world = World::new();
        
        let entity = world.spawn((
            Position::new(0.0, 0.0, 0),
            Movement::new(Vec3::new(10.0, 0.0, 0.0), 2.0),
        ));

        // Move for 1 second at speed 2 - should move 2 units
        movement_system(&mut world, 1.0);
        
        // Should still have Movement (not arrived)
        assert!(world.get::<&Movement>(entity).is_ok());
        
        let pos = world.get::<&Position>(entity).unwrap();
        assert!((pos.local.x - 2.0).abs() < 0.01);
    }
    
    #[test]
    fn test_multi_room_path() {
        let mut world = World::new();
        
        // Create path: room 0 -> room 1 -> room 2
        let path = vec![0, 1, 2];
        let destination = Vec3::new(3.0, 0.0, 0.0);
        let movement = Movement {
            destination,
            final_destination: destination,
            speed: 100.0, // Fast speed to arrive quickly
            path: path.clone(),
            path_index: 0,
            next_door_position: Some(Vec3::new(0.0, 0.0, 0.0)),
            entry_door_positions: vec![Vec3::new(0.0, 0.0, 0.0); 3],
            exit_door_positions: vec![Vec3::new(5.0, 0.0, 0.0); 3],
        };
        
        let entity = world.spawn((
            Position::new(0.0, 0.0, 0),
            movement,
        ));

        // First move - should arrive at room 0 exit
        movement_system(&mut world, 1.0);
        
        // Should now be in room 1
        let pos = world.get::<&Position>(entity).unwrap();
        assert_eq!(pos.room_id, 1);
    }
}
