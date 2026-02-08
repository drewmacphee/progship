//! Client-facing reducers for game interaction and simulation ticking.

use spacetimedb::{reducer, ReducerContext, Table};
use crate::tables::*;
use crate::simulation;

// ============================================================================
// PLAYER REDUCERS
// ============================================================================

/// Called when a client connects
#[reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) {
    log::info!("Client connected: {:?}", ctx.sender);
    ctx.db.connected_player().insert(ConnectedPlayer {
        identity: ctx.sender,
        person_id: None,
        connected_at: ctx.timestamp,
    });
}

/// Called when a client disconnects
#[reducer(client_disconnected)]
pub fn client_disconnected(ctx: &ReducerContext) {
    log::info!("Client disconnected: {:?}", ctx.sender);
    if let Some(player) = ctx.db.connected_player().identity().find(ctx.sender) {
        ctx.db.connected_player().identity().delete(player.identity);
    }
}

/// Player joins the game and creates their character
#[reducer]
pub fn player_join(ctx: &ReducerContext, given_name: String, family_name: String, is_crew: bool) {
    log::info!("Player joining: {} {} (crew: {})", given_name, family_name, is_crew);

    if let Some(player) = ctx.db.connected_player().identity().find(ctx.sender) {
        if player.person_id.is_some() {
            log::warn!("Player already has a character!");
            return;
        }
    }

    let person_id = ctx.db.person().insert(Person {
        id: 0,
        given_name,
        family_name,
        is_crew,
        is_player: true,
        owner_identity: Some(ctx.sender),
    }).id;

    // Start in the first corridor on deck 0
    let start_room = ctx.db.room().iter()
        .find(|r| r.deck == 0 && r.room_type == crate::tables::room_types::CORRIDOR);
    let (start_room_id, start_x, start_y) = if let Some(r) = &start_room {
        (r.id, r.x, r.y)
    } else {
        // Fallback: just use the first room on deck 0
        let fallback = ctx.db.room().iter().find(|r| r.deck == 0);
        if let Some(r) = fallback {
            (r.id, r.x, r.y)
        } else {
            (0, 0.0, 0.0)
        }
    };
    ctx.db.position().insert(Position {
        person_id,
        room_id: start_room_id,
        x: start_x, y: start_y, z: 0.0,
    });

    ctx.db.needs().insert(Needs {
        person_id,
        hunger: 0.0, fatigue: 0.0, social: 0.0, comfort: 0.0, hygiene: 0.0,
        health: 1.0, morale: 0.8,
    });

    ctx.db.personality().insert(Personality {
        person_id,
        openness: 0.5, conscientiousness: 0.5, extraversion: 0.5,
        agreeableness: 0.5, neuroticism: 0.3,
    });

    ctx.db.skills().insert(Skills {
        person_id,
        engineering: 0.3, medical: 0.2, piloting: 0.2,
        science: 0.2, social: 0.3, combat: 0.2,
    });

    ctx.db.activity().insert(Activity {
        person_id,
        activity_type: activity_types::IDLE,
        started_at: 0.0,
        duration: 0.5,
        target_room_id: None,
    });

    if is_crew {
        ctx.db.crew().insert(Crew {
            person_id,
            department: departments::OPERATIONS,
            rank: ranks::CREWMAN,
            shift: shifts::ALPHA,
            duty_station_id: 0,
            on_duty: false,
        });
    } else {
        ctx.db.passenger().insert(Passenger {
            person_id,
            cabin_class: cabin_classes::STANDARD,
            destination: "Kepler-442b".to_string(),
            profession: "Colonist".to_string(),
        });
    }

    // Link player to connection
    if let Some(mut player) = ctx.db.connected_player().identity().find(ctx.sender) {
        player.person_id = Some(person_id);
        ctx.db.connected_player().identity().update(player);
    }

    log::info!("Player character created with id {}", person_id);
}

/// Player movement input — bounded to room, can move through doors
#[reducer]
pub fn player_move(ctx: &ReducerContext, dx: f32, dy: f32) {
    let Some(player) = ctx.db.connected_player().identity().find(ctx.sender) else { return };
    let Some(person_id) = player.person_id else { return };

    let player_radius = 0.5; // collision padding for player size

    if let Some(mut pos) = ctx.db.position().person_id().find(person_id) {
        let new_x = pos.x + dx;
        let new_y = pos.y + dy;

        // Check if new position is inside current room (with player radius padding)
        if let Some(room) = ctx.db.room().id().find(pos.room_id) {
            let half_w = room.width / 2.0 - player_radius;
            let half_h = room.height / 2.0 - player_radius;

            if new_x >= room.x - half_w && new_x <= room.x + half_w &&
               new_y >= room.y - half_h && new_y <= room.y + half_h {
                pos.x = new_x;
                pos.y = new_y;
                ctx.db.position().person_id().update(pos);
                return;
            }
        }

        // Trying to leave current room — check if passing through a door
        let current_room_id = pos.room_id;
        let mut passed_door = false;
        for door in ctx.db.door().iter() {
            let (other_room_id, is_connected) = if door.room_a == current_room_id {
                (door.room_b, true)
            } else if door.room_b == current_room_id {
                (door.room_a, true)
            } else {
                (0, false)
            };
            if !is_connected { continue; }

            let Some(cur_room) = ctx.db.room().id().find(current_room_id) else { continue };
            let door_x = door.door_x;
            let door_y = door.door_y;

            // Distance from player to door center
            let dist_to_door = ((pos.x - door_x).powi(2) + (pos.y - door_y).powi(2)).sqrt();

            // Door interaction radius: half the door width + some slack
            let door_radius = (door.width / 2.0 + 1.0).max(2.0);

            if dist_to_door > door_radius {
                continue;
            }

            // Determine which wall this door is on for the current room
            let room_left = cur_room.x - cur_room.width / 2.0;
            let room_right = cur_room.x + cur_room.width / 2.0;
            let room_north = cur_room.y - cur_room.height / 2.0;
            let room_south = cur_room.y + cur_room.height / 2.0;

            let wall_tol = 2.0;
            let on_east = (door_x - room_right).abs() < wall_tol;
            let on_west = (door_x - room_left).abs() < wall_tol;
            let on_north = (door_y - room_north).abs() < wall_tol;
            let on_south = (door_y - room_south).abs() < wall_tol;

            // Check player is moving toward the door's wall
            let moving_toward = if on_east { dx > 0.0 }
                else if on_west { dx < 0.0 }
                else if on_north { dy < 0.0 }
                else if on_south { dy > 0.0 }
                else { true }; // embedded/interior doors — always passable

            if !moving_toward { continue; }

            if let Some(other_room) = ctx.db.room().id().find(other_room_id) {
                let half_w = other_room.width / 2.0 - player_radius;
                let half_h = other_room.height / 2.0 - player_radius;

                // Place player at door position, offset slightly into destination room
                let offset = 0.5;
                let entry_x;
                let entry_y;
                if dx.abs() > dy.abs() {
                    entry_x = if dx > 0.0 { door_x + offset } else { door_x - offset };
                    entry_y = door_y;
                } else {
                    entry_x = door_x;
                    entry_y = if dy > 0.0 { door_y + offset } else { door_y - offset };
                }
                // Clamp to destination room bounds
                pos.x = entry_x.clamp(other_room.x - half_w, other_room.x + half_w);
                pos.y = entry_y.clamp(other_room.y - half_h, other_room.y + half_h);
                pos.room_id = other_room_id;
                ctx.db.position().person_id().update(pos);
                passed_door = true;
                break;
            }
        }

        // Can't move through a door — slide along current room walls
        if !passed_door {
            if let Some(room) = ctx.db.room().id().find(current_room_id) {
                if let Some(mut pos2) = ctx.db.position().person_id().find(person_id) {
                    let half_w = room.width / 2.0 - player_radius;
                    let half_h = room.height / 2.0 - player_radius;
                    pos2.x = new_x.clamp(room.x - half_w, room.x + half_w);
                    pos2.y = new_y.clamp(room.y - half_h, room.y + half_h);
                    ctx.db.position().person_id().update(pos2);
                }
            }
        }
    }
}

/// Player interacts with a nearby person (start conversation)
#[reducer]
pub fn player_interact(ctx: &ReducerContext, target_person_id: u64) {
    let Some(player) = ctx.db.connected_player().identity().find(ctx.sender) else { return };
    let Some(person_id) = player.person_id else { return };

    // Check they're in the same room
    let Some(my_pos) = ctx.db.position().person_id().find(person_id) else { return };
    let Some(their_pos) = ctx.db.position().person_id().find(target_person_id) else { return };

    if my_pos.room_id != their_pos.room_id {
        log::warn!("Can't interact - not in same room");
        return;
    }

    // Check neither is in a conversation
    if ctx.db.in_conversation().person_id().find(person_id).is_some() ||
       ctx.db.in_conversation().person_id().find(target_person_id).is_some() {
        log::warn!("Can't interact - someone is already in a conversation");
        return;
    }

    let sim_time = ctx.db.ship_config().id().find(0).map(|c| c.sim_time).unwrap_or(0.0);

    let conv_id = ctx.db.conversation().insert(Conversation {
        id: 0,
        topic: conversation_topics::GREETING,
        state: conversation_states::ACTIVE,
        started_at: sim_time,
        participant_a: person_id,
        participant_b: target_person_id,
    }).id;

    ctx.db.in_conversation().insert(InConversation { person_id, conversation_id: conv_id });
    ctx.db.in_conversation().insert(InConversation { person_id: target_person_id, conversation_id: conv_id });
}

/// Player performs an action at their current location
#[reducer]
pub fn player_action(ctx: &ReducerContext, action: u8) {
    let Some(player) = ctx.db.connected_player().identity().find(ctx.sender) else { return };
    let Some(person_id) = player.person_id else { return };
    let Some(pos) = ctx.db.position().person_id().find(person_id) else { return };
    let Some(room) = ctx.db.room().id().find(pos.room_id) else { return };
    let Some(mut needs) = ctx.db.needs().person_id().find(person_id) else { return };
    let sim_time = ctx.db.ship_config().id().find(0).map(|c| c.sim_time).unwrap_or(0.0);

    match action {
        // Eat (must be in mess/galley)
        2 if room.room_type == room_types::MESS_HALL || room.room_type == room_types::GALLEY => {
            needs.hunger = (needs.hunger - 0.3).max(0.0);
            needs.comfort = (needs.comfort - 0.05).max(0.0);
            ctx.db.needs().person_id().update(needs);
            if let Some(mut act) = ctx.db.activity().person_id().find(person_id) {
                act.activity_type = activity_types::EATING;
                act.started_at = sim_time;
                act.duration = 0.5;
                ctx.db.activity().person_id().update(act);
            }
        }
        // Sleep (must be in quarters)
        3 if room_types::is_quarters(room.room_type) => {
            needs.fatigue = (needs.fatigue - 0.4).max(0.0);
            needs.comfort = (needs.comfort - 0.1).max(0.0);
            ctx.db.needs().person_id().update(needs);
            if let Some(mut act) = ctx.db.activity().person_id().find(person_id) {
                act.activity_type = activity_types::SLEEPING;
                act.started_at = sim_time;
                act.duration = 2.0;
                ctx.db.activity().person_id().update(act);
            }
        }
        // Repair (must be near a damaged subsystem in this room)
        8 => {
            let mut repaired = false;
            let room_node_id = ctx.db.room().id().find(pos.room_id).map(|r| r.node_id);
            if let Some(node_id) = room_node_id {
                for mut sub in ctx.db.subsystem().iter() {
                    if sub.node_id == node_id && sub.health < 0.9 {
                    sub.health = (sub.health + 0.2).min(1.0);
                    if sub.health > 0.8 {
                        sub.status = system_statuses::NOMINAL;
                    } else if sub.health > 0.5 {
                        sub.status = system_statuses::DEGRADED;
                    }
                    ctx.db.subsystem().id().update(sub);
                    repaired = true;
                    break;
                }
                }
            }
            if repaired {
                if let Some(mut act) = ctx.db.activity().person_id().find(person_id) {
                    act.activity_type = activity_types::MAINTENANCE;
                    act.started_at = sim_time;
                    act.duration = 0.25;
                    ctx.db.activity().person_id().update(act);
                }
            }
        }
        // Exercise (must be in gym/recreation)
        12 if room.room_type == room_types::GYM || room_types::is_recreation(room.room_type) => {
            needs.comfort = (needs.comfort - 0.15).max(0.0);
            needs.fatigue = (needs.fatigue + 0.1).min(1.0);
            needs.morale = (needs.morale + 0.05).min(1.0);
            ctx.db.needs().person_id().update(needs);
            if let Some(mut act) = ctx.db.activity().person_id().find(person_id) {
                act.activity_type = activity_types::EXERCISING;
                act.started_at = sim_time;
                act.duration = 0.5;
                ctx.db.activity().person_id().update(act);
            }
        }
        // Hygiene (must be in quarters)
        6 if room_types::is_quarters(room.room_type) || room.room_type == room_types::SHARED_BATHROOM => {
            needs.hygiene = (needs.hygiene - 0.5).max(0.0);
            ctx.db.needs().person_id().update(needs);
            if let Some(mut act) = ctx.db.activity().person_id().find(person_id) {
                act.activity_type = activity_types::HYGIENE;
                act.started_at = sim_time;
                act.duration = 0.2;
                ctx.db.activity().person_id().update(act);
            }
        }
        _ => {
            log::warn!("Invalid action {} for room type {}", action, room.room_type);
        }
    }
}

/// Use an elevator to travel to a different deck
#[reducer]
pub fn player_use_elevator(ctx: &ReducerContext, target_deck: i32) {
    let Some(cp) = ctx.db.connected_player().identity().find(ctx.sender) else { return };
    let Some(person_id) = cp.person_id else { return };
    let Some(pos) = ctx.db.position().person_id().find(person_id) else { return };
    let Some(current_room) = ctx.db.room().id().find(pos.room_id) else { return };

    // Must be in an elevator shaft
    if current_room.room_type != room_types::ELEVATOR_SHAFT {
        log::warn!("Not in an elevator shaft");
        return;
    }

    // Service elevators require crew status
    if current_room.name.contains("Service") {
        let is_crew = ctx.db.person().id().find(person_id).map(|p| p.is_crew).unwrap_or(false);
        if !is_crew {
            log::warn!("Service elevator restricted to crew");
            return;
        }
    }

    if target_deck == current_room.deck {
        return; // Already on this deck
    }

    // Find the connected elevator on the target deck by traversing connections
    let target_elevator = find_elevator_on_deck(ctx, pos.room_id, target_deck);
    if let Some(target_room_id) = target_elevator {
        if let Some(target_room) = ctx.db.room().id().find(target_room_id) {
            let mut p = pos;
            p.room_id = target_room_id;
            p.x = target_room.x;
            p.y = target_room.y;
            ctx.db.position().person_id().update(p);
            log::info!("Player took elevator to deck {}", target_deck + 1);
        }
    } else {
        log::warn!("No elevator connection to deck {}", target_deck + 1);
    }
}

/// Use a ladder shaft to move one deck up or down
#[reducer]
pub fn player_use_ladder(ctx: &ReducerContext, direction: i32) {
    let Some(cp) = ctx.db.connected_player().identity().find(ctx.sender) else { return };
    let Some(person_id) = cp.person_id else { return };
    let Some(pos) = ctx.db.position().person_id().find(person_id) else { return };
    let Some(current_room) = ctx.db.room().id().find(pos.room_id) else { return };

    if current_room.room_type != room_types::LADDER_SHAFT {
        log::warn!("Not in a ladder shaft");
        return;
    }

    let target_deck = current_room.deck + direction.signum();

    // Find connected ladder on target deck
    for door in ctx.db.door().iter() {
        let other_id = if door.room_a == pos.room_id { door.room_b }
                       else if door.room_b == pos.room_id { door.room_a }
                       else { continue };
        if let Some(other_room) = ctx.db.room().id().find(other_id) {
            if other_room.room_type == room_types::LADDER_SHAFT && other_room.deck == target_deck {
                let mut p = pos;
                p.room_id = other_id;
                p.x = other_room.x;
                p.y = other_room.y;
                ctx.db.position().person_id().update(p);
                log::info!("Player climbed ladder to deck {}", target_deck + 1);
                return;
            }
        }
    }
    log::warn!("No ladder connection in that direction");
}

/// Find an elevator room on target_deck connected (possibly through chain) to start_room
fn find_elevator_on_deck(ctx: &ReducerContext, start_room: u32, target_deck: i32) -> Option<u32> {
    // BFS through elevator connections
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(start_room);
    visited.insert(start_room);

    while let Some(current) = queue.pop_front() {
        if let Some(room) = ctx.db.room().id().find(current) {
            if room.deck == target_deck && room.room_type == room_types::ELEVATOR_SHAFT {
                return Some(current);
            }
        }
        for door in ctx.db.door().iter() {
            let other = if door.room_a == current { door.room_b }
                       else if door.room_b == current { door.room_a }
                       else { continue };
            if visited.contains(&other) { continue; }
            // Only follow elevator shaft connections
            if let Some(other_room) = ctx.db.room().id().find(other) {
                if other_room.room_type == room_types::ELEVATOR_SHAFT {
                    visited.insert(other);
                    queue.push_back(other);
                }
            }
        }
    }
    None
}

// ============================================================================
// SIMULATION CONTROL REDUCERS
// ============================================================================

/// Pause/unpause the simulation
#[reducer]
pub fn set_paused(ctx: &ReducerContext, paused: bool) {
    if let Some(mut config) = ctx.db.ship_config().id().find(0) {
        config.paused = paused;
        ctx.db.ship_config().id().update(config);
        log::info!("Simulation {}", if paused { "paused" } else { "resumed" });
    }
}

/// Set simulation time scale
#[reducer]
pub fn set_time_scale(ctx: &ReducerContext, scale: f32) {
    if let Some(mut config) = ctx.db.ship_config().id().find(0) {
        config.time_scale = scale.clamp(0.0, 100.0);
        ctx.db.ship_config().id().update(config);
        log::info!("Time scale set to {}", scale);
    }
}

// ============================================================================
// SIMULATION TICK
// ============================================================================

/// Main simulation tick - called by client or scheduled reducer
#[reducer]
pub fn tick(ctx: &ReducerContext, delta_seconds: f32) {
    let Some(mut config) = ctx.db.ship_config().id().find(0) else { return };
    if config.paused { return; }

    let scaled_delta = delta_seconds * config.time_scale;
    let delta_hours = scaled_delta as f64 / 3600.0;

    config.sim_time += delta_hours;
    ctx.db.ship_config().id().update(config.clone());

    let sim_time = config.sim_time;

    // T0: Movement (every tick)
    simulation::tick_movement(ctx, scaled_delta);

    // T1: Activities & wandering (every tick, internally throttled)
    simulation::tick_activities(ctx, sim_time);
    simulation::tick_wandering(ctx, sim_time);

    // T2: Slower systems (needs, social, duty)
    simulation::tick_needs(ctx, delta_hours as f32);
    simulation::tick_social(ctx, sim_time);
    simulation::tick_duty(ctx, sim_time);

    // T3: Ship systems (resources, atmosphere, events, maintenance)
    simulation::tick_ship_systems(ctx, delta_hours as f32);
    simulation::tick_atmosphere(ctx, delta_hours as f32);
    simulation::tick_events(ctx, sim_time, delta_hours as f32);
    simulation::tick_maintenance(ctx, sim_time, delta_hours as f32);
}
