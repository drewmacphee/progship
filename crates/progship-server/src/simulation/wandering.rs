//! Wandering system - idle NPCs move to random nearby locations.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

use super::movement::start_movement_to;

/// Make idle NPCs wander to random nearby locations.
pub fn tick_wandering(ctx: &ReducerContext, sim_time: f64) {
    for activity in ctx.db.activity().iter() {
        if activity.activity_type != activity_types::IDLE {
            continue;
        }
        // Skip player-controlled characters
        if let Some(person) = ctx.db.person().id().find(activity.person_id) {
            if person.is_player {
                continue;
            }
        }
        // Only wander if not already moving
        if ctx
            .db
            .movement()
            .person_id()
            .find(activity.person_id)
            .is_some()
        {
            continue;
        }

        let Some(pos) = ctx.db.position().person_id().find(activity.person_id) else {
            continue;
        };

        // Deterministic "random" offset based on person_id and time
        let seed = (activity.person_id as f64 * 17.0 + sim_time * 3.7) % 100.0;

        // 30% chance to wander to an adjacent room
        if (seed * 13.0) % 10.0 < 3.0 {
            // Find a connected room to wander to
            let doors: Vec<Door> = ctx
                .db
                .door()
                .iter()
                .filter(|d| d.room_a == pos.room_id || d.room_b == pos.room_id)
                .collect();
            if !doors.is_empty() {
                let idx = ((seed * 19.0) as usize) % doors.len();
                let door = &doors[idx];
                let target_room_id = if door.room_a == pos.room_id {
                    door.room_b
                } else {
                    door.room_a
                };
                // Only move to rooms on same deck
                if let Some(target_room) = ctx.db.room().id().find(target_room_id) {
                    if let Some(cur_room) = ctx.db.room().id().find(pos.room_id) {
                        if target_room.deck == cur_room.deck {
                            start_movement_to(ctx, activity.person_id, target_room_id);
                            continue;
                        }
                    }
                }
            }
        }

        // Otherwise wander within current room
        let Some(room) = ctx.db.room().id().find(pos.room_id) else {
            continue;
        };
        let half_w = (room.width / 2.0 - 1.0).max(0.5);
        let half_h = (room.height / 2.0 - 1.0).max(0.5);
        let offset_x = ((seed * 7.3) % (half_w as f64 * 2.0)) as f32 - half_w;
        let offset_y = ((seed * 11.1) % (half_h as f64 * 2.0)) as f32 - half_h;

        ctx.db.movement().insert(Movement {
            person_id: activity.person_id,
            target_room_id: pos.room_id,
            target_x: room.x + offset_x,
            target_y: room.y + offset_y,
            target_z: 0.0,
            speed: 2.0,
            path: String::new(),
            path_index: 0,
        });
    }
}
