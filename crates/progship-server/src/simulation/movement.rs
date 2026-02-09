//! Movement and pathfinding system - moves people through rooms via doors.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

/// Move people toward their destinations, following door waypoints.
pub fn tick_movement(ctx: &ReducerContext, delta_seconds: f32) {
    let movements: Vec<Movement> = ctx.db.movement().iter().collect();

    for mov in movements {
        let Some(mut pos) = ctx.db.position().person_id().find(mov.person_id) else {
            ctx.db.movement().person_id().delete(mov.person_id);
            continue;
        };

        // Determine current waypoint target
        let (wp_x, wp_y, wp_room_id, is_final) = get_current_waypoint(&mov);

        let dx = wp_x - pos.x;
        let dy = wp_y - pos.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 1.5 {
            // Reached current waypoint
            pos.x = wp_x;
            pos.y = wp_y;
            pos.room_id = wp_room_id;
            ctx.db.position().person_id().update(pos);

            if is_final {
                // Arrived at final destination
                ctx.db.movement().person_id().delete(mov.person_id);
            } else {
                // Advance to next waypoint
                let mut updated = mov.clone();
                updated.path_index += 1;
                ctx.db.movement().person_id().update(updated);
            }
        } else {
            // Move toward current waypoint
            let move_dist = mov.speed * delta_seconds;
            let ratio = (move_dist / dist).min(1.0);
            pos.x += dx * ratio;
            pos.y += dy * ratio;
            ctx.db.position().person_id().update(pos);
        }
    }
}

/// Parse the path string and return (x, y, room_id, is_final_waypoint) for the current step
fn get_current_waypoint(mov: &Movement) -> (f32, f32, u32, bool) {
    if mov.path.is_empty() {
        return (mov.target_x, mov.target_y, mov.target_room_id, true);
    }

    // Path format: "door_x,door_y,room_id;door_x,door_y,room_id;...;final_x,final_y,room_id"
    let waypoints: Vec<&str> = mov.path.split(';').collect();
    let idx = mov.path_index as usize;

    if idx >= waypoints.len() {
        return (mov.target_x, mov.target_y, mov.target_room_id, true);
    }

    let parts: Vec<&str> = waypoints[idx].split(',').collect();
    if parts.len() >= 3 {
        let x = parts[0].parse::<f32>().unwrap_or(mov.target_x);
        let y = parts[1].parse::<f32>().unwrap_or(mov.target_y);
        let room_id = parts[2].parse::<u32>().unwrap_or(mov.target_room_id);
        let is_final = idx == waypoints.len() - 1;
        (x, y, room_id, is_final)
    } else {
        (mov.target_x, mov.target_y, mov.target_room_id, true)
    }
}

/// Compute the world position of a door.
/// Uses the stored absolute door_x/door_y coordinates.
pub fn door_world_position(door: &Door, _rooms: &[Room]) -> (f32, f32) {
    (door.door_x, door.door_y)
}

/// BFS pathfinding through doors, returns list of (door_x, door_y, next_room_id)
fn find_path(ctx: &ReducerContext, from_room: u32, to_room: u32) -> Vec<(f32, f32, u32)> {
    if from_room == to_room {
        return vec![];
    }

    // Build adjacency list from doors using absolute door coordinates
    let doors: Vec<Door> = ctx.db.door().iter().collect();
    let mut adj: std::collections::HashMap<u32, Vec<(u32, f32, f32)>> =
        std::collections::HashMap::new();
    for door in &doors {
        adj.entry(door.room_a)
            .or_default()
            .push((door.room_b, door.door_x, door.door_y));
        adj.entry(door.room_b)
            .or_default()
            .push((door.room_a, door.door_x, door.door_y));
    }

    // BFS
    let mut visited: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut queue: std::collections::VecDeque<(u32, Vec<(f32, f32, u32)>)> =
        std::collections::VecDeque::new();
    visited.insert(from_room);
    queue.push_back((from_room, vec![]));

    while let Some((current, path)) = queue.pop_front() {
        if let Some(neighbors) = adj.get(&current) {
            for &(next_room, door_x, door_y) in neighbors {
                if next_room == to_room {
                    let mut result = path.clone();
                    result.push((door_x, door_y, next_room));
                    return result;
                }
                if visited.insert(next_room) {
                    let mut new_path = path.clone();
                    new_path.push((door_x, door_y, next_room));
                    queue.push_back((next_room, new_path));
                }
            }
        }
    }

    // No path found — direct move as fallback
    vec![]
}

/// Start movement for a person to a target room, using pathfinding
pub fn start_movement_to(ctx: &ReducerContext, person_id: u64, target_room_id: u32) {
    if ctx.db.movement().person_id().find(person_id).is_some() {
        return;
    }

    let Some(pos) = ctx.db.position().person_id().find(person_id) else {
        return;
    };
    let Some(target_room) = ctx.db.room().id().find(target_room_id) else {
        return;
    };

    // Find path through doors
    let waypoints = find_path(ctx, pos.room_id, target_room_id);

    // Build path string: each waypoint is "x,y,room_id" separated by ";"
    // Final waypoint is the target position inside the destination room
    let mut path_parts: Vec<String> = waypoints
        .iter()
        .map(|(dx, dy, rid)| format!("{},{},{}", dx, dy, rid))
        .collect();
    // Add final destination (center of target room)
    path_parts.push(format!(
        "{},{},{}",
        target_room.x, target_room.y, target_room_id
    ));

    let path = path_parts.join(";");

    ctx.db.movement().insert(Movement {
        person_id,
        target_room_id,
        target_x: target_room.x,
        target_y: target_room.y,
        target_z: 0.0,
        speed: 5.0,
        path,
        path_index: 0,
    });
}
