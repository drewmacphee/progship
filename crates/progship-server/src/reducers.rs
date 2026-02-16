//! Client-facing reducers for game interaction and simulation ticking.

use crate::simulation;
use crate::tables::*;
use progship_logic::actions::{apply_needs_deltas, compute_action_effect, NeedsValues};
use progship_logic::movement::{compute_move, DoorInfo, MoveInput, MoveResult, RoomBounds};
use spacetimedb::{reducer, ReducerContext, Table};

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
    log::info!(
        "Player joining: {} {} (crew: {})",
        given_name,
        family_name,
        is_crew
    );

    if let Some(player) = ctx.db.connected_player().identity().find(ctx.sender) {
        if player.person_id.is_some() {
            log::warn!("Player already has a character!");
            return;
        }
    }

    let person_id = ctx
        .db
        .person()
        .insert(Person {
            id: 0,
            given_name,
            family_name,
            is_crew,
            is_player: true,
            is_alive: true,
            owner_identity: Some(ctx.sender),
        })
        .id;

    // Start on the lowest deck that has an elevator shaft, so the player
    // can immediately use elevators to reach other decks.
    let spawn_deck = ctx
        .db
        .room()
        .iter()
        .filter(|r| {
            r.room_type == room_types::ELEVATOR_SHAFT
                || r.room_type == room_types::SERVICE_ELEVATOR_SHAFT
        })
        .map(|r| r.deck)
        .min()
        .unwrap_or(0);
    let start_room = ctx
        .db
        .room()
        .iter()
        .find(|r| r.deck == spawn_deck && r.room_type == room_types::CORRIDOR);
    let (start_room_id, start_x, start_y) = if let Some(r) = &start_room {
        (r.id, r.x, r.y)
    } else {
        let fallback = ctx.db.room().iter().find(|r| r.deck == spawn_deck);
        if let Some(r) = fallback {
            (r.id, r.x, r.y)
        } else {
            (0, 0.0, 0.0)
        }
    };
    ctx.db.position().insert(Position {
        person_id,
        room_id: start_room_id,
        x: start_x,
        y: start_y,
        z: 0.0,
    });

    ctx.db.needs().insert(Needs {
        person_id,
        hunger: 0.0,
        fatigue: 0.0,
        social: 0.0,
        comfort: 0.0,
        hygiene: 0.0,
        health: 1.0,
        morale: 0.8,
    });

    ctx.db.personality().insert(Personality {
        person_id,
        openness: 0.5,
        conscientiousness: 0.5,
        extraversion: 0.5,
        agreeableness: 0.5,
        neuroticism: 0.3,
    });

    ctx.db.skills().insert(Skills {
        person_id,
        engineering: 0.3,
        medical: 0.2,
        piloting: 0.2,
        science: 0.2,
        social: 0.3,
        combat: 0.2,
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
    let Some(player) = ctx.db.connected_player().identity().find(ctx.sender) else {
        return;
    };
    let Some(person_id) = player.person_id else {
        return;
    };

    let player_radius = 0.4;

    if let Some(mut pos) = ctx.db.position().person_id().find(person_id) {
        let Some(room) = ctx.db.room().id().find(pos.room_id) else {
            return;
        };
        let current = RoomBounds::new(room.id, room.x, room.y, room.width, room.height);

        // Collect doors connected to the current room (same-deck only;
        // cross-deck doors are used via elevator/ladder reducers)
        let doors: Vec<DoorInfo> = ctx
            .db
            .door()
            .iter()
            .filter(|d| d.room_a == pos.room_id || d.room_b == pos.room_id)
            .filter(|d| {
                let other_id = if d.room_a == pos.room_id {
                    d.room_b
                } else {
                    d.room_a
                };
                ctx.db
                    .room()
                    .id()
                    .find(other_id)
                    .is_some_and(|r| r.deck == room.deck)
            })
            .map(|d| DoorInfo {
                room_a: d.room_a,
                room_b: d.room_b,
                door_x: d.door_x,
                door_y: d.door_y,
                width: d.width,
            })
            .collect();

        let room_lookup = |id: u32| -> Option<RoomBounds> {
            ctx.db
                .room()
                .id()
                .find(id)
                .map(|r| RoomBounds::new(r.id, r.x, r.y, r.width, r.height))
        };

        let result = compute_move(
            &MoveInput {
                px: pos.x,
                py: pos.y,
                dx,
                dy,
                player_radius,
            },
            &current,
            &doors,
            &room_lookup,
        );

        let (mut final_x, mut final_y, new_room) = match result {
            MoveResult::InRoom { x, y } | MoveResult::WallSlide { x, y } => (x, y, pos.room_id),
            MoveResult::DoorTraversal { room_id, x, y } => (x, y, room_id),
        };

        // Push away from NPCs — only when fully inside a room (not in a door zone)
        let in_same_room = new_room == pos.room_id;
        let inside_bounds = if in_same_room {
            current.contains(final_x, final_y, player_radius)
        } else {
            let dest = room_lookup(new_room);
            dest.is_some_and(|d| d.contains(final_x, final_y, player_radius))
        };

        if inside_bounds {
            let npc_radius = 0.3;
            let min_dist = player_radius + npc_radius;
            for other_pos in ctx.db.position().iter() {
                if other_pos.person_id == person_id || other_pos.room_id != new_room {
                    continue;
                }
                let dx_npc = final_x - other_pos.x;
                let dy_npc = final_y - other_pos.y;
                let dist_sq = dx_npc * dx_npc + dy_npc * dy_npc;
                if dist_sq < min_dist * min_dist && dist_sq > 0.001 {
                    let dist = dist_sq.sqrt();
                    let push = (min_dist - dist) * 0.5;
                    final_x += (dx_npc / dist) * push;
                    final_y += (dy_npc / dist) * push;
                }
            }

            // Reclamp to room bounds after NPC push
            let room_bounds = if in_same_room {
                current
            } else {
                room_lookup(new_room).unwrap_or(current)
            };
            let (cx, cy) = room_bounds.clamp(final_x, final_y, player_radius);
            final_x = cx;
            final_y = cy;
        }

        pos.room_id = new_room;
        pos.x = final_x;
        pos.y = final_y;
        ctx.db.position().person_id().update(pos);
    }
}

/// Player interacts with a nearby person (start conversation)
#[reducer]
pub fn player_interact(ctx: &ReducerContext, target_person_id: u64) {
    let Some(player) = ctx.db.connected_player().identity().find(ctx.sender) else {
        return;
    };
    let Some(person_id) = player.person_id else {
        return;
    };

    // Check they're in the same room
    let Some(my_pos) = ctx.db.position().person_id().find(person_id) else {
        return;
    };
    let Some(their_pos) = ctx.db.position().person_id().find(target_person_id) else {
        return;
    };

    if my_pos.room_id != their_pos.room_id {
        log::warn!("Can't interact - not in same room");
        return;
    }

    // Check neither is in a conversation
    if ctx
        .db
        .in_conversation()
        .person_id()
        .find(person_id)
        .is_some()
        || ctx
            .db
            .in_conversation()
            .person_id()
            .find(target_person_id)
            .is_some()
    {
        log::warn!("Can't interact - someone is already in a conversation");
        return;
    }

    let sim_time = ctx
        .db
        .ship_config()
        .id()
        .find(0)
        .map(|c| c.sim_time)
        .unwrap_or(0.0);

    let conv_id = ctx
        .db
        .conversation()
        .insert(Conversation {
            id: 0,
            topic: conversation_topics::GREETING,
            state: conversation_states::ACTIVE,
            started_at: sim_time,
            participant_a: person_id,
            participant_b: target_person_id,
        })
        .id;

    ctx.db.in_conversation().insert(InConversation {
        person_id,
        conversation_id: conv_id,
    });
    ctx.db.in_conversation().insert(InConversation {
        person_id: target_person_id,
        conversation_id: conv_id,
    });
}

/// Player performs an action at their current location
#[reducer]
pub fn player_action(ctx: &ReducerContext, action: u8) {
    let Some(player) = ctx.db.connected_player().identity().find(ctx.sender) else {
        return;
    };
    let Some(person_id) = player.person_id else {
        return;
    };
    let Some(pos) = ctx.db.position().person_id().find(person_id) else {
        return;
    };
    let Some(room) = ctx.db.room().id().find(pos.room_id) else {
        return;
    };
    let Some(mut needs) = ctx.db.needs().person_id().find(person_id) else {
        return;
    };
    let sim_time = ctx
        .db
        .ship_config()
        .id()
        .find(0)
        .map(|c| c.sim_time)
        .unwrap_or(0.0);

    // Try repair action separately (requires DB queries for subsystems)
    if action == 8 {
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
        return;
    }

    // All other actions use the extracted pure logic
    match compute_action_effect(action, room.room_type) {
        Some(effect) => {
            let result = apply_needs_deltas(
                &NeedsValues {
                    hunger: needs.hunger,
                    fatigue: needs.fatigue,
                    social: needs.social,
                    comfort: needs.comfort,
                    hygiene: needs.hygiene,
                    morale: needs.morale,
                    health: needs.health,
                },
                &effect,
            );
            needs.hunger = result.hunger;
            needs.fatigue = result.fatigue;
            needs.social = result.social;
            needs.comfort = result.comfort;
            needs.hygiene = result.hygiene;
            needs.morale = result.morale;
            needs.health = result.health;
            ctx.db.needs().person_id().update(needs);

            if let Some(mut act) = ctx.db.activity().person_id().find(person_id) {
                act.activity_type = effect.activity_type;
                act.started_at = sim_time;
                act.duration = effect.duration;
                ctx.db.activity().person_id().update(act);
            }
        }
        None => {
            log::warn!("Invalid action {} for room type {}", action, room.room_type);
        }
    }
}

/// Use an elevator to travel to a different deck
#[reducer]
pub fn player_use_elevator(ctx: &ReducerContext, target_deck: i32) {
    let Some(cp) = ctx.db.connected_player().identity().find(ctx.sender) else {
        return;
    };
    let Some(person_id) = cp.person_id else {
        return;
    };
    let Some(pos) = ctx.db.position().person_id().find(person_id) else {
        return;
    };
    let Some(current_room) = ctx.db.room().id().find(pos.room_id) else {
        return;
    };

    // Must be in an elevator shaft
    if current_room.room_type != room_types::ELEVATOR_SHAFT
        && current_room.room_type != room_types::SERVICE_ELEVATOR_SHAFT
    {
        log::warn!("Not in an elevator shaft");
        return;
    }

    // Service elevators require crew status
    if current_room.name.contains("Service") {
        let is_crew = ctx
            .db
            .person()
            .id()
            .find(person_id)
            .map(|p| p.is_crew)
            .unwrap_or(false);
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
    let Some(cp) = ctx.db.connected_player().identity().find(ctx.sender) else {
        return;
    };
    let Some(person_id) = cp.person_id else {
        return;
    };
    let Some(pos) = ctx.db.position().person_id().find(person_id) else {
        return;
    };
    let Some(current_room) = ctx.db.room().id().find(pos.room_id) else {
        return;
    };

    if current_room.room_type != room_types::LADDER_SHAFT {
        log::warn!("Not in a ladder shaft");
        return;
    }

    let target_deck = current_room.deck + direction.signum();

    // Find connected ladder on target deck
    for door in ctx.db.door().iter() {
        let other_id = if door.room_a == pos.room_id {
            door.room_b
        } else if door.room_b == pos.room_id {
            door.room_a
        } else {
            continue;
        };
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
            if room.deck == target_deck
                && (room.room_type == room_types::ELEVATOR_SHAFT
                    || room.room_type == room_types::SERVICE_ELEVATOR_SHAFT)
            {
                return Some(current);
            }
        }
        for door in ctx.db.door().iter() {
            let other = if door.room_a == current {
                door.room_b
            } else if door.room_b == current {
                door.room_a
            } else {
                continue;
            };
            if visited.contains(&other) {
                continue;
            }
            // Only follow elevator shaft connections
            if let Some(other_room) = ctx.db.room().id().find(other) {
                if other_room.room_type == room_types::ELEVATOR_SHAFT
                    || other_room.room_type == room_types::SERVICE_ELEVATOR_SHAFT
                {
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
    let Some(mut config) = ctx.db.ship_config().id().find(0) else {
        return;
    };
    if config.paused {
        return;
    }

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

    // T2: Slower systems (needs, social, duty, death)
    simulation::tick_needs(ctx, delta_hours as f32);
    simulation::tick_death(ctx, sim_time);
    simulation::tick_social(ctx, sim_time);
    simulation::tick_duty(ctx, sim_time);

    // T3: Ship systems (resources, atmosphere, events, maintenance)
    simulation::tick_ship_systems(ctx, delta_hours as f32);
    simulation::tick_atmosphere(ctx, delta_hours as f32);
    simulation::tick_events(ctx, sim_time, delta_hours as f32);
    simulation::tick_maintenance(ctx, sim_time, delta_hours as f32);
}
