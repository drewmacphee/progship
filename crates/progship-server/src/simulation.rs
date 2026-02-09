//! Simulation tick systems - all game logic that runs each tick.
//!
//! Systems are called by the `tick` reducer at appropriate frequencies.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

// ============================================================================
// NEEDS SYSTEM
// ============================================================================

/// Decay needs over time, with rates modified by current activity.
/// Also applies atmosphere effects on health.
pub fn tick_needs(ctx: &ReducerContext, delta_hours: f32) {
    // Pre-collect atmosphere data for lookups
    let atmospheres: Vec<DeckAtmosphere> = ctx.db.deck_atmosphere().iter().collect();

    for needs in ctx.db.needs().iter() {
        let mut n = needs;

        // Look up activity for modified decay rates
        let activity = ctx.db.activity().person_id().find(n.person_id);
        let (hunger_rate, fatigue_rate, social_rate, comfort_rate, hygiene_rate) =
            activity_decay_rates(activity.as_ref());

        n.hunger = (n.hunger + delta_hours * hunger_rate).min(1.0);
        n.fatigue = (n.fatigue + delta_hours * fatigue_rate).min(1.0);
        n.social = (n.social + delta_hours * social_rate).min(1.0);
        n.comfort = (n.comfort + delta_hours * comfort_rate).min(1.0);
        n.hygiene = (n.hygiene + delta_hours * hygiene_rate).min(1.0);

        // Natural health recovery (slow)
        if n.health < 1.0 && n.hunger < 0.5 && n.fatigue < 0.5 {
            n.health = (n.health + 0.01 * delta_hours).min(1.0);
        }

        // Starvation damage
        if n.hunger >= 1.0 {
            n.health -= 0.05 * delta_hours;
        }

        // Exhaustion damage
        if n.fatigue >= 1.0 {
            n.health -= 0.02 * delta_hours;
        }

        // Morale affected by needs satisfaction
        let avg_needs = (n.hunger + n.fatigue + n.social + n.comfort + n.hygiene) / 5.0;
        if avg_needs > 0.7 {
            n.morale = (n.morale - 0.03 * delta_hours).max(0.0);
        } else if avg_needs < 0.3 {
            n.morale = (n.morale + 0.01 * delta_hours).min(1.0);
        }

        // Atmosphere effects on health
        if let Some(pos) = ctx.db.position().person_id().find(n.person_id) {
            if let Some(room) = ctx.db.room().id().find(pos.room_id) {
                if let Some(atmo) = atmospheres.iter().find(|a| a.deck == room.deck) {
                    // Low oxygen → health damage
                    if atmo.oxygen < 0.16 {
                        let o2_damage = (0.16 - atmo.oxygen) * 0.5 * delta_hours;
                        n.health -= o2_damage;
                        n.fatigue = (n.fatigue + 0.1 * delta_hours).min(1.0);
                    }
                    // High CO2 → fatigue and health damage
                    if atmo.co2 > 0.04 {
                        n.fatigue = (n.fatigue + (atmo.co2 - 0.04) * 2.0 * delta_hours).min(1.0);
                        if atmo.co2 > 0.06 {
                            n.health -= (atmo.co2 - 0.06) * 0.3 * delta_hours;
                        }
                    }
                    // Temperature extremes → comfort
                    if atmo.temperature < 15.0 || atmo.temperature > 30.0 {
                        n.comfort = (n.comfort + 0.1 * delta_hours).min(1.0);
                    }
                    // Extreme temperature → health damage
                    if atmo.temperature < 5.0 || atmo.temperature > 40.0 {
                        n.health -= 0.05 * delta_hours;
                    }
                    // Low pressure → rapid health damage
                    if atmo.pressure < 80.0 {
                        n.health -= (80.0 - atmo.pressure) * 0.01 * delta_hours;
                    }
                }
            }
        }

        n.health = n.health.clamp(0.0, 1.0);
        ctx.db.needs().person_id().update(n);
    }
}

/// Returns (hunger, fatigue, social, comfort, hygiene) decay rates per hour
fn activity_decay_rates(activity: Option<&Activity>) -> (f32, f32, f32, f32, f32) {
    match activity.map(|a| a.activity_type) {
        Some(activity_types::SLEEPING) => (0.02, -0.15, 0.01, -0.02, 0.01),
        Some(activity_types::EATING) => (-0.3, 0.01, -0.05, -0.02, 0.02),
        Some(activity_types::EXERCISING) => (0.08, 0.1, 0.0, 0.03, 0.06),
        Some(activity_types::SOCIALIZING) => (0.03, 0.02, -0.15, -0.01, 0.02),
        Some(activity_types::HYGIENE) => (0.02, 0.01, 0.0, -0.03, -0.3),
        Some(activity_types::RELAXING) => (0.02, -0.03, 0.01, -0.05, 0.01),
        Some(activity_types::WORKING) | Some(activity_types::ON_DUTY) => {
            (0.05, 0.06, 0.02, 0.03, 0.03)
        }
        Some(activity_types::MAINTENANCE) => (0.06, 0.08, 0.01, 0.04, 0.05),
        _ => (0.04, 0.03, 0.02, 0.02, 0.02),
    }
}

// ============================================================================
// ACTIVITY SYSTEM
// ============================================================================

/// Select new activities when current ones complete, and handle activity effects.
pub fn tick_activities(ctx: &ReducerContext, sim_time: f64) {
    for activity in ctx.db.activity().iter() {
        // Skip player-controlled characters
        if let Some(person) = ctx.db.person().id().find(activity.person_id) {
            if person.is_player {
                continue;
            }
        }
        let elapsed = sim_time - activity.started_at;
        if elapsed < activity.duration as f64 {
            continue; // Still doing current activity
        }

        // Activity complete - select new one based on needs
        let Some(needs) = ctx.db.needs().person_id().find(activity.person_id) else {
            continue;
        };

        let is_crew = ctx.db.crew().person_id().find(activity.person_id).is_some();
        let current_hour = (sim_time % 24.0) as f32;

        let (new_type, duration, target_room) =
            select_activity(&needs, current_hour, is_crew, activity.person_id, ctx);

        let mut a = activity;
        let person_id = a.person_id;
        a.activity_type = new_type;
        a.started_at = sim_time;
        a.duration = duration;
        a.target_room_id = target_room;
        ctx.db.activity().person_id().update(a);

        // If activity requires a different room, start movement
        if let Some(target) = target_room {
            let Some(pos) = ctx.db.position().person_id().find(person_id) else {
                continue;
            };
            if pos.room_id != target {
                start_movement_to(ctx, person_id, target);
            }
        }
    }
}

/// Select the best activity based on needs and time of day
fn select_activity(
    needs: &Needs,
    hour: f32,
    is_crew: bool,
    person_id: u64,
    ctx: &ReducerContext,
) -> (u8, f32, Option<u32>) {
    // Check if crew member should be on duty
    if is_crew {
        if let Some(crew) = ctx.db.crew().person_id().find(person_id) {
            if should_be_on_duty(crew.shift, hour) {
                let room = find_room_for_activity(ctx, activity_types::ON_DUTY, crew.department);
                return (activity_types::ON_DUTY, 2.0, room);
            }
        }
    }

    // Priority: critical needs first
    if needs.fatigue > 0.85 {
        let room = find_room_of_type_pred(ctx, room_types::is_quarters);
        return (activity_types::SLEEPING, 8.0, room);
    }
    if needs.hunger > 0.75 {
        let room = find_room_of_type(ctx, room_types::MESS_HALL);
        return (activity_types::EATING, 0.5, room);
    }
    if needs.hygiene > 0.8 {
        let room = find_room_of_type(ctx, room_types::SHARED_BATHROOM);
        return (activity_types::HYGIENE, 0.3, room);
    }

    // Moderate needs
    if needs.social > 0.6 {
        let room = find_room_of_type_pred(ctx, room_types::is_recreation);
        return (activity_types::SOCIALIZING, 1.0, room);
    }
    if needs.comfort > 0.6 {
        let room = find_room_of_type_pred(ctx, room_types::is_recreation);
        return (activity_types::RELAXING, 1.0, room);
    }
    if needs.hunger > 0.5 && is_meal_time(hour) {
        let room = find_room_of_type(ctx, room_types::MESS_HALL);
        return (activity_types::EATING, 0.5, room);
    }

    // Sleep schedule
    if needs.fatigue > 0.5 && is_sleep_time(hour, is_crew) {
        let room = find_room_of_type_pred(ctx, room_types::is_quarters);
        return (activity_types::SLEEPING, 8.0, room);
    }

    // Default: idle/wander
    (activity_types::IDLE, 0.02, None)
}

fn should_be_on_duty(shift: u8, hour: f32) -> bool {
    match shift {
        shifts::ALPHA => (6.0..14.0).contains(&hour),
        shifts::BETA => (14.0..22.0).contains(&hour),
        shifts::GAMMA => !(6.0..22.0).contains(&hour),
        _ => false,
    }
}

fn is_meal_time(hour: f32) -> bool {
    (7.0..8.0).contains(&hour) ||   // Breakfast
    (12.0..13.0).contains(&hour) ||  // Lunch
    (18.0..19.0).contains(&hour) // Dinner
}

fn is_sleep_time(hour: f32, is_crew: bool) -> bool {
    if is_crew {
        false
    }
    // Crew sleeps based on shift
    else {
        !(6.0..22.0).contains(&hour)
    }
}

fn find_room_of_type(ctx: &ReducerContext, room_type: u8) -> Option<u32> {
    ctx.db
        .room()
        .iter()
        .find(|r| r.room_type == room_type)
        .map(|r| r.id)
}

fn find_room_of_type_pred(ctx: &ReducerContext, pred: fn(u8) -> bool) -> Option<u32> {
    ctx.db
        .room()
        .iter()
        .find(|r| pred(r.room_type))
        .map(|r| r.id)
}

fn find_room_for_activity(ctx: &ReducerContext, activity: u8, department: u8) -> Option<u32> {
    match activity {
        activity_types::ON_DUTY => {
            let room_type = match department {
                departments::COMMAND => room_types::BRIDGE,
                departments::ENGINEERING => room_types::ENGINEERING,
                departments::MEDICAL => room_types::HOSPITAL_WARD,
                departments::SCIENCE => room_types::LABORATORY,
                departments::SECURITY => room_types::CIC,
                departments::OPERATIONS => room_types::ENGINEERING,
                _ => room_types::CORRIDOR,
            };
            find_room_of_type(ctx, room_type)
        }
        _ => None,
    }
}

// ============================================================================
// MOVEMENT SYSTEM
// ============================================================================

/// Move people toward their destinations, following door waypoints
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
fn start_movement_to(ctx: &ReducerContext, person_id: u64, target_room_id: u32) {
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

// ============================================================================
// WANDERING SYSTEM
// ============================================================================

/// Make idle NPCs wander to random nearby locations
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

// ============================================================================
// SOCIAL SYSTEM
// ============================================================================

/// Start and end conversations between nearby people
pub fn tick_social(ctx: &ReducerContext, sim_time: f64) {
    // End expired conversations
    let conversations: Vec<Conversation> = ctx.db.conversation().iter().collect();
    for conv in conversations {
        if conv.state == conversation_states::ENDED {
            continue;
        }
        let elapsed = sim_time - conv.started_at;
        if elapsed > 0.5 {
            // 30 min max conversation
            end_conversation(ctx, conv.id, sim_time);
        }
    }

    // Find people in the same room who aren't in conversations
    let positions: Vec<Position> = ctx.db.position().iter().collect();
    let mut room_occupants: std::collections::HashMap<u32, Vec<u64>> =
        std::collections::HashMap::new();

    for pos in &positions {
        // Skip people already in conversations or moving
        if ctx
            .db
            .in_conversation()
            .person_id()
            .find(pos.person_id)
            .is_some()
        {
            continue;
        }
        if ctx.db.movement().person_id().find(pos.person_id).is_some() {
            continue;
        }
        room_occupants
            .entry(pos.room_id)
            .or_default()
            .push(pos.person_id);
    }

    // Start conversations between pairs in the same room
    for people in room_occupants.values() {
        if people.len() < 2 {
            continue;
        }

        // Deterministic pairing: first two available
        let a = people[0];
        let b = people[1];

        // Check social need - only start if someone is lonely enough
        let needs_a = ctx.db.needs().person_id().find(a);
        let needs_b = ctx.db.needs().person_id().find(b);

        let social_need = match (&needs_a, &needs_b) {
            (Some(na), Some(nb)) => na.social.max(nb.social),
            _ => 0.0,
        };

        if social_need < 0.3 {
            continue;
        }

        start_conversation(ctx, a, b, sim_time);
    }
}

fn start_conversation(ctx: &ReducerContext, person_a: u64, person_b: u64, sim_time: f64) {
    // Pick topic based on relationship, personality, and needs
    let topic = select_conversation_topic(ctx, person_a, person_b, sim_time);

    let conv_id = ctx
        .db
        .conversation()
        .insert(Conversation {
            id: 0, // auto_inc
            topic,
            state: conversation_states::ACTIVE,
            started_at: sim_time,
            participant_a: person_a,
            participant_b: person_b,
        })
        .id;

    ctx.db.in_conversation().insert(InConversation {
        person_id: person_a,
        conversation_id: conv_id,
    });
    ctx.db.in_conversation().insert(InConversation {
        person_id: person_b,
        conversation_id: conv_id,
    });

    // Update or create relationship
    update_relationship(ctx, person_a, person_b, sim_time, 0.02);
}

fn end_conversation(ctx: &ReducerContext, conv_id: u64, sim_time: f64) {
    if let Some(mut conv) = ctx.db.conversation().id().find(conv_id) {
        let participant_a = conv.participant_a;
        let participant_b = conv.participant_b;
        conv.state = conversation_states::ENDED;

        // Conversation effects depend on topic
        let (strength_delta, social_recovery) = match conv.topic {
            conversation_topics::GREETING => (0.01, 0.05),
            conversation_topics::WORK => (0.02, 0.03),
            conversation_topics::GOSSIP => (0.03, 0.08),
            conversation_topics::PERSONAL => (0.05, 0.1),
            conversation_topics::COMPLAINT => (-0.02, 0.04),
            conversation_topics::REQUEST => (0.01, 0.02),
            conversation_topics::FLIRTATION => (0.06, 0.12),
            conversation_topics::ARGUMENT => (-0.1, 0.02),
            conversation_topics::FAREWELL => (0.0, 0.01),
            _ => (0.01, 0.05),
        };

        // Apply social need recovery
        for pid in [participant_a, participant_b] {
            if let Some(mut needs) = ctx.db.needs().person_id().find(pid) {
                needs.social = (needs.social - social_recovery).max(0.0);
                if conv.topic == conversation_topics::ARGUMENT {
                    needs.morale = (needs.morale - 0.03).max(0.0);
                } else if conv.topic == conversation_topics::PERSONAL
                    || conv.topic == conversation_topics::FLIRTATION
                {
                    needs.morale = (needs.morale + 0.02).min(1.0);
                }
                ctx.db.needs().person_id().update(needs);
            }
        }

        // Update relationship
        update_relationship(ctx, participant_a, participant_b, sim_time, strength_delta);

        ctx.db.conversation().id().update(conv);

        // Remove InConversation markers
        if let Some(ic) = ctx.db.in_conversation().person_id().find(participant_a) {
            if ic.conversation_id == conv_id {
                ctx.db.in_conversation().person_id().delete(participant_a);
            }
        }
        if let Some(ic) = ctx.db.in_conversation().person_id().find(participant_b) {
            if ic.conversation_id == conv_id {
                ctx.db.in_conversation().person_id().delete(participant_b);
            }
        }
    }
}

/// Select conversation topic based on relationship, personality, and context
fn select_conversation_topic(
    ctx: &ReducerContext,
    person_a: u64,
    person_b: u64,
    sim_time: f64,
) -> u8 {
    // Check existing relationship
    let familiarity = ctx
        .db
        .relationship()
        .iter()
        .find(|r| {
            (r.person_a == person_a && r.person_b == person_b)
                || (r.person_a == person_b && r.person_b == person_a)
        })
        .map(|r| r.familiarity)
        .unwrap_or(0.0);

    // Check personality traits
    let extraversion_a = ctx
        .db
        .personality()
        .person_id()
        .find(person_a)
        .map(|p| p.extraversion)
        .unwrap_or(0.5);
    let agreeableness_b = ctx
        .db
        .personality()
        .person_id()
        .find(person_b)
        .map(|p| p.agreeableness)
        .unwrap_or(0.5);
    let neuroticism_a = ctx
        .db
        .personality()
        .person_id()
        .find(person_a)
        .map(|p| p.neuroticism)
        .unwrap_or(0.3);

    // Check morale
    let morale_a = ctx
        .db
        .needs()
        .person_id()
        .find(person_a)
        .map(|n| n.morale)
        .unwrap_or(0.5);

    // Deterministic seed for variety
    let seed = ((person_a as f64 * 7.3 + person_b as f64 * 11.1 + sim_time * 3.7) % 10.0) as f32;

    // Strangers greet first
    if familiarity < 0.05 {
        return conversation_topics::GREETING;
    }

    // Low morale + high neuroticism → complaints or arguments
    if morale_a < 0.3 && neuroticism_a > 0.6 {
        if agreeableness_b < 0.4 && seed < 3.0 {
            return conversation_topics::ARGUMENT;
        }
        return conversation_topics::COMPLAINT;
    }

    // Both crew → work talk likely
    let both_crew = ctx.db.crew().person_id().find(person_a).is_some()
        && ctx.db.crew().person_id().find(person_b).is_some();
    if both_crew && seed < 4.0 {
        return conversation_topics::WORK;
    }

    // High extraversion + high familiarity → personal or flirtation
    if extraversion_a > 0.7 && familiarity > 0.3 {
        if seed < 2.0 {
            return conversation_topics::FLIRTATION;
        }
        if seed < 5.0 {
            return conversation_topics::PERSONAL;
        }
    }

    // Medium familiarity → gossip
    if familiarity > 0.1 && seed < 3.0 {
        return conversation_topics::GOSSIP;
    }

    // Default: greeting or general chat
    if seed < 5.0 {
        conversation_topics::GREETING
    } else {
        conversation_topics::PERSONAL
    }
}

fn update_relationship(
    ctx: &ReducerContext,
    person_a: u64,
    person_b: u64,
    sim_time: f64,
    strength_delta: f32,
) {
    // Look for existing relationship
    for rel in ctx.db.relationship().iter() {
        if (rel.person_a == person_a && rel.person_b == person_b)
            || (rel.person_a == person_b && rel.person_b == person_a)
        {
            let mut r = rel;
            r.strength = (r.strength + strength_delta).clamp(-1.0, 1.0);
            r.familiarity = (r.familiarity + 0.01).min(1.0);
            r.last_interaction = sim_time;
            // Update relationship type based on strength
            r.relationship_type = classify_relationship(r.strength, r.familiarity);
            ctx.db.relationship().id().update(r);
            return;
        }
    }

    // Create new relationship
    ctx.db.relationship().insert(Relationship {
        id: 0,
        person_a,
        person_b,
        relationship_type: relationship_types::STRANGER,
        strength: strength_delta,
        familiarity: 0.01,
        last_interaction: sim_time,
    });
}

fn classify_relationship(strength: f32, familiarity: f32) -> u8 {
    if familiarity < 0.1 {
        return relationship_types::STRANGER;
    }
    if strength < -0.5 {
        return relationship_types::ENEMY;
    }
    if strength < -0.2 {
        return relationship_types::RIVAL;
    }
    if familiarity < 0.3 {
        return relationship_types::ACQUAINTANCE;
    }
    if strength > 0.7 {
        return relationship_types::CLOSE_FRIEND;
    }
    if strength > 0.3 {
        return relationship_types::FRIEND;
    }
    relationship_types::COLLEAGUE
}

// ============================================================================
// DUTY SYSTEM
// ============================================================================

/// Update crew on/off duty status based on shift and time
pub fn tick_duty(ctx: &ReducerContext, sim_time: f64) {
    let hour = (sim_time % 24.0) as f32;

    for crew in ctx.db.crew().iter() {
        let should_work = should_be_on_duty(crew.shift, hour);
        if crew.on_duty != should_work {
            let mut c = crew;
            c.on_duty = should_work;
            ctx.db.crew().person_id().update(c);
        }
    }
}

// ============================================================================
// SHIP SYSTEMS
// ============================================================================

/// Update ship systems: resource production, consumption, degradation
pub fn tick_ship_systems(ctx: &ReducerContext, delta_hours: f32) {
    let Some(mut resources) = ctx.db.ship_resources().id().find(0) else {
        return;
    };

    let person_count = ctx.db.person().iter().count() as f32;

    // Base consumption rates (per person per hour)
    let food_rate = 2.0 / 24.0;
    let water_rate = 3.0 / 24.0;
    let oxygen_rate = 0.84 / 24.0;

    resources.food = (resources.food - person_count * food_rate * delta_hours).max(0.0);
    resources.water = (resources.water - person_count * water_rate * delta_hours).max(0.0);
    resources.oxygen = (resources.oxygen - person_count * oxygen_rate * delta_hours).max(0.0);

    // Subsystem-level production/consumption and degradation
    let subsystems: Vec<Subsystem> = ctx.db.subsystem().iter().collect();
    for sub in &subsystems {
        if sub.status == system_statuses::OFFLINE || sub.status == system_statuses::DESTROYED {
            continue;
        }
        let efficiency = sub.health
            * if sub.status == system_statuses::DEGRADED {
                0.5
            } else {
                1.0
            };

        // Production based on subsystem type
        match sub.subsystem_type {
            subsystem_types::REACTOR_CORE => {
                resources.power =
                    (resources.power + 100.0 * efficiency * delta_hours).min(resources.power_cap);
            }
            subsystem_types::EMERGENCY_GENERATOR => {
                // Only produces if main reactor is down
                let reactor_down = subsystems.iter().any(|s| {
                    s.subsystem_type == subsystem_types::REACTOR_CORE
                        && (s.status == system_statuses::OFFLINE
                            || s.status == system_statuses::DESTROYED)
                });
                if reactor_down {
                    resources.power = (resources.power + 30.0 * efficiency * delta_hours)
                        .min(resources.power_cap);
                }
            }
            subsystem_types::O2_GENERATION => {
                let o2_produced = person_count * oxygen_rate * efficiency * delta_hours;
                resources.oxygen = (resources.oxygen + o2_produced).min(resources.oxygen_cap);
            }
            subsystem_types::WATER_FILTRATION | subsystem_types::WATER_DISTILLATION => {
                let recycled = person_count * water_rate * 0.45 * efficiency * delta_hours;
                resources.water = (resources.water + recycled).min(resources.water_cap);
            }
            subsystem_types::GROWTH_CHAMBER => {
                resources.food =
                    (resources.food + 5.0 * efficiency * delta_hours).min(resources.food_cap);
            }
            _ => {}
        }

        // Power consumption from subsystem power_draw
        if sub.power_draw > 0.0 {
            resources.power = (resources.power - sub.power_draw * delta_hours).max(0.0);
        }
    }

    // Degrade subsystems slowly, update their status
    let subsystems_for_update: Vec<Subsystem> = ctx.db.subsystem().iter().collect();
    for sub in subsystems_for_update {
        let mut s = sub;
        s.health = (s.health - 0.0001 * delta_hours).max(0.0);
        s.status = if s.health > 0.7 {
            system_statuses::NOMINAL
        } else if s.health > 0.3 {
            system_statuses::DEGRADED
        } else if s.health > 0.0 {
            system_statuses::CRITICAL
        } else {
            system_statuses::OFFLINE
        };
        ctx.db.subsystem().id().update(s);
    }

    // Degrade components slowly
    let components: Vec<SystemComponent> = ctx.db.system_component().iter().collect();
    for comp in components {
        let mut c = comp;
        c.health = (c.health - 0.00005 * delta_hours).max(0.0);
        c.status = if c.health > 0.7 {
            system_statuses::NOMINAL
        } else if c.health > 0.3 {
            system_statuses::DEGRADED
        } else if c.health > 0.0 {
            system_statuses::CRITICAL
        } else {
            system_statuses::OFFLINE
        };
        ctx.db.system_component().id().update(c);
    }

    // Recompute parent ShipSystem overall_health/status from subsystems
    let all_subsystems: Vec<Subsystem> = ctx.db.subsystem().iter().collect();
    let systems: Vec<ShipSystem> = ctx.db.ship_system().iter().collect();
    for sys in systems {
        let children: Vec<&Subsystem> = all_subsystems
            .iter()
            .filter(|s| s.system_id == sys.id)
            .collect();
        if children.is_empty() {
            continue;
        }
        let avg_health = children.iter().map(|s| s.health).sum::<f32>() / children.len() as f32;
        let worst_status = children.iter().map(|s| s.status).max().unwrap_or(0);
        let mut s = sys;
        s.overall_health = avg_health;
        s.overall_status = worst_status;
        ctx.db.ship_system().id().update(s);
    }

    // InfraEdge degradation (very slow)
    let infra_edges: Vec<InfraEdge> = ctx.db.infra_edge().iter().collect();
    for edge in infra_edges {
        let mut e = edge;
        e.health = (e.health - 0.00002 * delta_hours).max(0.0);
        ctx.db.infra_edge().id().update(e);
    }

    // Update infra_edge flow based on health
    let all_infra_edges: Vec<InfraEdge> = ctx.db.infra_edge().iter().collect();
    let graph_edges: Vec<GraphEdge> = ctx.db.graph_edge().iter().collect();
    for ge in graph_edges {
        // Skip crew paths — only infrastructure edges
        if ge.edge_type == edge_types::CREW_PATH {
            continue;
        }
        let infra_health = all_infra_edges
            .iter()
            .filter(|ie| ie.graph_edge_id == ge.id)
            .map(|ie| ie.health)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(1.0);
        // Update flow on each infra_edge for this graph_edge
        for ie in all_infra_edges
            .iter()
            .filter(|ie| ie.graph_edge_id == ge.id)
        {
            let mut e = ie.clone();
            e.current_flow = e.capacity * infra_health;
            ctx.db.infra_edge().id().update(e);
        }
    }

    ctx.db.ship_resources().id().update(resources);
}

// ============================================================================
// ATMOSPHERE SYSTEM
// ============================================================================

/// Update per-deck atmosphere based on occupancy and life support
pub fn tick_atmosphere(ctx: &ReducerContext, delta_hours: f32) {
    // Count people per deck
    let mut deck_population: std::collections::HashMap<i32, u32> = std::collections::HashMap::new();
    let mut deck_exercising: std::collections::HashMap<i32, u32> = std::collections::HashMap::new();

    for pos in ctx.db.position().iter() {
        if let Some(room) = ctx.db.room().id().find(pos.room_id) {
            *deck_population.entry(room.deck).or_insert(0) += 1;

            // Check if exercising (high metabolic output)
            if let Some(act) = ctx.db.activity().person_id().find(pos.person_id) {
                if act.activity_type == activity_types::EXERCISING {
                    *deck_exercising.entry(room.deck).or_insert(0) += 1;
                }
            }
        }
    }

    // Check life support efficiency from subsystems
    let ls_subsystems: Vec<Subsystem> = ctx
        .db
        .subsystem()
        .iter()
        .filter(|s| {
            s.subsystem_type == subsystem_types::O2_GENERATION
                || s.subsystem_type == subsystem_types::CO2_SCRUBBING
                || s.subsystem_type == subsystem_types::AIR_CIRCULATION
        })
        .collect();
    let ls_efficiency = if ls_subsystems.is_empty() {
        0.0
    } else {
        ls_subsystems
            .iter()
            .map(|s| {
                if s.status == system_statuses::OFFLINE {
                    0.0
                } else {
                    s.health
                        * if s.status == system_statuses::DEGRADED {
                            0.5
                        } else {
                            1.0
                        }
                }
            })
            .sum::<f32>()
            / ls_subsystems.len() as f32
    };

    for atmo in ctx.db.deck_atmosphere().iter() {
        let pop = *deck_population.get(&atmo.deck).unwrap_or(&0) as f32;
        let exercising = *deck_exercising.get(&atmo.deck).unwrap_or(&0) as f32;

        let mut a = atmo;

        // Metabolic impact (per person per hour)
        let o2_consumption = (pop * 0.035 + exercising * 0.07) * delta_hours; // fraction units
        let co2_production = (pop * 0.043 + exercising * 0.09) * delta_hours;
        let humidity_add = (pop * 0.005 + exercising * 0.015) * delta_hours;
        let heat_add = (pop * 0.1 + exercising * 0.3) * delta_hours;

        a.oxygen -= o2_consumption;
        a.co2 += co2_production;
        a.humidity += humidity_add;
        a.temperature += heat_add;

        // Life support counteraction
        a.oxygen += o2_consumption * ls_efficiency; // Regenerate O2
        a.co2 -= co2_production * ls_efficiency * 0.95; // Scrub CO2
        a.humidity -= humidity_add * ls_efficiency * 0.8; // Dehumidify
        a.temperature -= heat_add * ls_efficiency * 0.9; // Cool

        // Clamp values
        a.oxygen = a.oxygen.clamp(0.0, 0.25);
        a.co2 = a.co2.clamp(0.0, 0.1);
        a.humidity = a.humidity.clamp(0.0, 1.0);
        a.temperature = a.temperature.clamp(-10.0, 50.0);

        ctx.db.deck_atmosphere().deck().update(a);
    }
}

// ============================================================================
// EVENT SYSTEM
// ============================================================================

/// Generate random events and progress existing ones with real consequences
pub fn tick_events(ctx: &ReducerContext, sim_time: f64, delta_hours: f32) {
    // Progress existing events and apply consequences
    let events: Vec<Event> = ctx.db.event().iter().collect();
    for event in events {
        if event.state == event_states::RESOLVED {
            continue;
        }

        let elapsed = sim_time - event.started_at;
        let mut e = event.clone();

        // Apply ongoing event effects based on type
        apply_event_effects(ctx, &event, delta_hours);

        // Events resolve when handled long enough or expire
        if e.state == event_states::BEING_HANDLED && elapsed > e.duration as f64 * 0.5 {
            e.state = event_states::RESOLVED;
            log::info!("Event {} resolved (handled)", e.id);
        } else if elapsed > e.duration as f64 {
            // Unhandled events escalate then resolve with damage
            if e.state == event_states::ACTIVE {
                e.state = event_states::ESCALATED;
                e.severity = (e.severity * 1.5).min(1.0);
                apply_escalation_effects(ctx, &e);
                log::info!("Event {} escalated! severity={:.2}", e.id, e.severity);
            } else {
                e.state = event_states::RESOLVED;
                log::info!("Event {} resolved (expired with damage)", e.id);
            }
        }

        ctx.db.event().id().update(e);
    }

    // Generate new events - use high-precision time bits for pseudo-randomness
    let time_bits = (sim_time * 100000.0) as u64;
    let hash = time_bits
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let event_chance = (hash >> 32) % 1000; // Use upper bits for better distribution

    if event_chance < 5 {
        // ~0.5% chance per tick
        let hash2 = hash.wrapping_mul(2862933555777941757);
        let event_type = (hash2 % 8) as u8;
        let severity = 0.3 + ((hash2 / 8 % 50) as f32 * 0.01);

        // Pick a random room
        let rooms: Vec<Room> = ctx.db.room().iter().collect();
        if rooms.is_empty() {
            return;
        }
        let room_idx = (hash2 / 400) as usize % rooms.len();

        let responders_needed = match event_type {
            event_types::FIRE | event_types::HULL_BREACH => 3,
            event_types::SYSTEM_FAILURE | event_types::MEDICAL_EMERGENCY => 2,
            _ => 1,
        };

        ctx.db.event().insert(Event {
            id: 0,
            event_type,
            room_id: rooms[room_idx].id,
            started_at: sim_time,
            duration: 1.0 + severity * 2.0,
            state: event_states::ACTIVE,
            responders_needed,
            responders_assigned: 0,
            severity,
        });

        log::info!(
            "Event spawned: type={} room={} severity={:.2}",
            event_type,
            rooms[room_idx].name,
            severity
        );
    }
}

/// Apply ongoing effects of active events each tick
fn apply_event_effects(ctx: &ReducerContext, event: &Event, delta_hours: f32) {
    let severity = event.severity;
    let escalated = event.state == event_states::ESCALATED;
    let damage_mult = if escalated { 2.0 } else { 1.0 };

    match event.event_type {
        event_types::FIRE => {
            // Fire: damages people in room, consumes O2, produces CO2
            for pos in ctx.db.position().iter() {
                if pos.room_id == event.room_id {
                    if let Some(mut needs) = ctx.db.needs().person_id().find(pos.person_id) {
                        needs.health -= severity * 0.05 * damage_mult * delta_hours;
                        needs.comfort = (needs.comfort + 0.3 * delta_hours).min(1.0);
                        needs.health = needs.health.max(0.0);
                        ctx.db.needs().person_id().update(needs);
                    }
                }
            }
            // Fire affects deck atmosphere
            if let Some(room) = ctx.db.room().id().find(event.room_id) {
                if let Some(mut atmo) = ctx.db.deck_atmosphere().deck().find(room.deck) {
                    atmo.oxygen -= severity * 0.01 * damage_mult * delta_hours;
                    atmo.co2 += severity * 0.015 * damage_mult * delta_hours;
                    atmo.temperature += severity * 2.0 * damage_mult * delta_hours;
                    atmo.oxygen = atmo.oxygen.clamp(0.0, 0.25);
                    atmo.co2 = atmo.co2.clamp(0.0, 0.1);
                    atmo.temperature = atmo.temperature.clamp(-10.0, 50.0);
                    ctx.db.deck_atmosphere().deck().update(atmo);
                }
            }
        }
        event_types::HULL_BREACH => {
            // Hull breach: rapid pressure/O2 loss on deck, severe health damage
            if let Some(room) = ctx.db.room().id().find(event.room_id) {
                if let Some(mut atmo) = ctx.db.deck_atmosphere().deck().find(room.deck) {
                    atmo.pressure -= severity * 5.0 * damage_mult * delta_hours;
                    atmo.oxygen -= severity * 0.02 * damage_mult * delta_hours;
                    atmo.temperature -= severity * 3.0 * damage_mult * delta_hours;
                    atmo.pressure = atmo.pressure.max(0.0);
                    atmo.oxygen = atmo.oxygen.max(0.0);
                    atmo.temperature = atmo.temperature.max(-40.0);
                    ctx.db.deck_atmosphere().deck().update(atmo);
                }
                // Damage everyone on that deck
                for pos in ctx.db.position().iter() {
                    if let Some(r) = ctx.db.room().id().find(pos.room_id) {
                        if r.deck == room.deck {
                            if let Some(mut needs) = ctx.db.needs().person_id().find(pos.person_id)
                            {
                                needs.health -= severity * 0.1 * damage_mult * delta_hours;
                                needs.health = needs.health.max(0.0);
                                ctx.db.needs().person_id().update(needs);
                            }
                        }
                    }
                }
            }
        }
        event_types::MEDICAL_EMERGENCY => {
            // Medical emergency: one person's health declining
            // Find the person closest to the event room
            for pos in ctx.db.position().iter() {
                if pos.room_id == event.room_id {
                    if let Some(mut needs) = ctx.db.needs().person_id().find(pos.person_id) {
                        if needs.health < 0.9 {
                            continue;
                        } // already affected
                        needs.health -= severity * 0.15 * delta_hours;
                        needs.health = needs.health.max(0.0);
                        ctx.db.needs().person_id().update(needs);
                        break; // only one person affected
                    }
                }
            }
        }
        event_types::SYSTEM_FAILURE => {
            // System failure: damage a random subsystem, cascading effects
            let subsystems: Vec<Subsystem> = ctx.db.subsystem().iter().collect();
            if !subsystems.is_empty() {
                let idx = (event.started_at * 7.1) as usize % subsystems.len();
                let mut sub = subsystems[idx].clone();
                sub.health = (sub.health - severity * 0.1 * delta_hours).max(0.0);
                if sub.health < 0.3 {
                    sub.status = system_statuses::OFFLINE;
                } else if sub.health < 0.7 {
                    sub.status = system_statuses::DEGRADED;
                }
                ctx.db.subsystem().id().update(sub);
            }
        }
        event_types::RESOURCE_SHORTAGE => {
            // Resource shortage: reduce power, morale drop
            if let Some(mut resources) = ctx.db.ship_resources().id().find(0) {
                resources.power -= severity * 50.0 * delta_hours;
                resources.power = resources.power.max(0.0);
                ctx.db.ship_resources().id().update(resources);
            }
            // Morale drop for everyone
            for needs in ctx.db.needs().iter() {
                let mut n = needs;
                n.morale = (n.morale - 0.02 * severity * delta_hours).max(0.0);
                n.comfort = (n.comfort + 0.05 * delta_hours).min(1.0);
                ctx.db.needs().person_id().update(n);
            }
        }
        event_types::ALTERCATION => {
            // Altercation: morale drop for people in room
            for pos in ctx.db.position().iter() {
                if pos.room_id == event.room_id {
                    if let Some(mut needs) = ctx.db.needs().person_id().find(pos.person_id) {
                        needs.morale = (needs.morale - 0.05 * severity * delta_hours).max(0.0);
                        ctx.db.needs().person_id().update(needs);
                    }
                }
            }
        }
        event_types::DISCOVERY => {
            // Discovery: morale boost for people in room
            for pos in ctx.db.position().iter() {
                if pos.room_id == event.room_id {
                    if let Some(mut needs) = ctx.db.needs().person_id().find(pos.person_id) {
                        needs.morale = (needs.morale + 0.1 * delta_hours).min(1.0);
                        ctx.db.needs().person_id().update(needs);
                    }
                }
            }
        }
        event_types::CELEBRATION => {
            // Celebration: morale boost for everyone on deck
            if let Some(room) = ctx.db.room().id().find(event.room_id) {
                for pos in ctx.db.position().iter() {
                    if let Some(r) = ctx.db.room().id().find(pos.room_id) {
                        if r.deck == room.deck {
                            if let Some(mut needs) = ctx.db.needs().person_id().find(pos.person_id)
                            {
                                needs.morale = (needs.morale + 0.05 * delta_hours).min(1.0);
                                needs.social = (needs.social - 0.05 * delta_hours).max(0.0);
                                ctx.db.needs().person_id().update(needs);
                            }
                        }
                    }
                }
            }
        }
        _ => {} // Other events: no special effects yet
    }
}

/// Apply one-time effects when an event escalates
fn apply_escalation_effects(ctx: &ReducerContext, event: &Event) {
    match event.event_type {
        event_types::FIRE => {
            // Fire spreads: damage subsystems in the room
            // Find the node_id for the event's room
            let event_node_id = ctx.db.room().id().find(event.room_id).map(|r| r.node_id);
            if let Some(node_id) = event_node_id {
                let subsystems: Vec<Subsystem> = ctx
                    .db
                    .subsystem()
                    .iter()
                    .filter(|s| s.node_id == node_id)
                    .collect();
                for sub in subsystems {
                    let mut s = sub;
                    s.health = (s.health - event.severity * 0.3).max(0.0);
                    if s.health < 0.3 {
                        s.status = system_statuses::OFFLINE;
                    } else if s.health < 0.7 {
                        s.status = system_statuses::DEGRADED;
                    }
                    ctx.db.subsystem().id().update(s);
                }
            }
        }
        event_types::SYSTEM_FAILURE => {
            // Cascading: if reactor core is offline, degrade life support subsystems
            let reactor_down = ctx.db.subsystem().iter().any(|s| {
                s.subsystem_type == subsystem_types::REACTOR_CORE
                    && s.status == system_statuses::OFFLINE
            });
            if reactor_down {
                let ls_subs: Vec<Subsystem> = ctx
                    .db
                    .subsystem()
                    .iter()
                    .filter(|s| {
                        s.subsystem_type == subsystem_types::O2_GENERATION
                            || s.subsystem_type == subsystem_types::CO2_SCRUBBING
                            || s.subsystem_type == subsystem_types::AIR_CIRCULATION
                    })
                    .collect();
                for sub in ls_subs {
                    let mut s = sub;
                    s.health = (s.health - 0.2).max(0.0);
                    s.status = if s.health < 0.3 {
                        system_statuses::OFFLINE
                    } else {
                        system_statuses::DEGRADED
                    };
                    ctx.db.subsystem().id().update(s);
                }
            }
        }
        _ => {}
    }
}

// ============================================================================
// MAINTENANCE SYSTEM
// ============================================================================

/// Check subsystems/components for maintenance needs, assign crew, progress repairs
pub fn tick_maintenance(ctx: &ReducerContext, sim_time: f64, delta_hours: f32) {
    // Generate tasks for degraded subsystems
    for sub in ctx.db.subsystem().iter() {
        if sub.health < 0.7 {
            let has_task = ctx
                .db
                .maintenance_task()
                .iter()
                .any(|t| t.subsystem_id == sub.id && t.progress < 1.0);
            if has_task {
                continue;
            }

            // Find the parent system type to determine required skill
            let skill = ctx
                .db
                .ship_system()
                .id()
                .find(sub.system_id)
                .map(|sys| match sys.system_type {
                    system_types::MEDICAL => skill_types::MEDICAL,
                    system_types::NAVIGATION => skill_types::PILOTING,
                    _ => skill_types::ENGINEERING,
                })
                .unwrap_or(skill_types::ENGINEERING);

            // Find a degraded component within this subsystem to target
            let target_comp = ctx
                .db
                .system_component()
                .iter()
                .find(|c| c.subsystem_id == sub.id && c.health < 0.7);
            let comp_id = target_comp.map(|c| c.id).unwrap_or(0);

            ctx.db.maintenance_task().insert(MaintenanceTask {
                id: 0,
                component_id: comp_id,
                subsystem_id: sub.id,
                assigned_crew_id: None,
                priority: 1.0 - sub.health,
                progress: 0.0,
                created_at: sim_time,
                required_skill: skill,
                duration_hours: 2.0 + (1.0 - sub.health) * 4.0,
            });
        }
    }

    // Assign unassigned tasks to available crew
    let tasks: Vec<MaintenanceTask> = ctx
        .db
        .maintenance_task()
        .iter()
        .filter(|t| t.assigned_crew_id.is_none() && t.progress < 1.0)
        .collect();

    for task in tasks {
        let assigned = ctx
            .db
            .crew()
            .iter()
            .find(|c| !c.on_duty)
            .map(|c| c.person_id);

        if let Some(crew_id) = assigned {
            let duration_hours = task.duration_hours;
            let mut t = task;
            t.assigned_crew_id = Some(crew_id);
            ctx.db.maintenance_task().id().update(t);

            if let Some(mut act) = ctx.db.activity().person_id().find(crew_id) {
                act.activity_type = activity_types::MAINTENANCE;
                act.started_at = sim_time;
                act.duration = duration_hours;
                ctx.db.activity().person_id().update(act);
            }
        }
    }

    // Progress active repairs
    let active_tasks: Vec<MaintenanceTask> = ctx
        .db
        .maintenance_task()
        .iter()
        .filter(|t| t.assigned_crew_id.is_some() && t.progress < 1.0)
        .collect();

    for task in active_tasks {
        let mut t = task;
        t.progress = (t.progress + delta_hours / t.duration_hours).min(1.0);

        if t.progress >= 1.0 {
            // Repair complete - restore component and subsystem health
            if t.component_id > 0 {
                if let Some(mut comp) = ctx.db.system_component().id().find(t.component_id) {
                    comp.health = (comp.health + 0.3).min(1.0);
                    comp.status = if comp.health > 0.7 {
                        system_statuses::NOMINAL
                    } else {
                        system_statuses::DEGRADED
                    };
                    comp.last_maintenance = ctx
                        .db
                        .ship_config()
                        .id()
                        .find(0)
                        .map(|c| c.sim_time)
                        .unwrap_or(0.0);
                    ctx.db.system_component().id().update(comp);
                }
            }
            if let Some(mut sub) = ctx.db.subsystem().id().find(t.subsystem_id) {
                sub.health = (sub.health + 0.3).min(1.0);
                sub.status = if sub.health > 0.7 {
                    system_statuses::NOMINAL
                } else {
                    system_statuses::DEGRADED
                };
                ctx.db.subsystem().id().update(sub);
            }
        }

        ctx.db.maintenance_task().id().update(t);
    }
}

// ============================================================================
// PURE HELPER FUNCTIONS (extracted for testing)
// ============================================================================

/// Calculate need decay with clamping (0.0 - 1.0 range)
pub fn calculate_need_change(current: f32, rate: f32, delta_hours: f32) -> f32 {
    (current + rate * delta_hours).clamp(0.0, 1.0)
}

/// Calculate health recovery based on needs satisfaction
pub fn calculate_health_recovery(current_health: f32, hunger: f32, fatigue: f32, delta_hours: f32) -> f32 {
    if current_health < 1.0 && hunger < 0.5 && fatigue < 0.5 {
        (current_health + 0.01 * delta_hours).min(1.0)
    } else {
        current_health
    }
}

/// Calculate health damage from starvation
pub fn calculate_starvation_damage(current_health: f32, hunger: f32, delta_hours: f32) -> f32 {
    if hunger >= 1.0 {
        current_health - 0.05 * delta_hours
    } else {
        current_health
    }
}

/// Calculate health damage from exhaustion
pub fn calculate_exhaustion_damage(current_health: f32, fatigue: f32, delta_hours: f32) -> f32 {
    if fatigue >= 1.0 {
        current_health - 0.02 * delta_hours
    } else {
        current_health
    }
}

/// Calculate morale change based on average needs satisfaction
pub fn calculate_morale_change(current_morale: f32, avg_needs: f32, delta_hours: f32) -> f32 {
    if avg_needs > 0.7 {
        (current_morale - 0.03 * delta_hours).max(0.0)
    } else if avg_needs < 0.3 {
        (current_morale + 0.01 * delta_hours).min(1.0)
    } else {
        current_morale
    }
}

/// Calculate health damage from low oxygen
pub fn calculate_oxygen_damage(current_health: f32, oxygen: f32, delta_hours: f32) -> f32 {
    if oxygen < 0.16 {
        let o2_damage = (0.16 - oxygen) * 0.5 * delta_hours;
        current_health - o2_damage
    } else {
        current_health
    }
}

/// Calculate fatigue increase from high CO2
pub fn calculate_co2_fatigue(current_fatigue: f32, co2: f32, delta_hours: f32) -> f32 {
    if co2 > 0.04 {
        (current_fatigue + (co2 - 0.04) * 2.0 * delta_hours).min(1.0)
    } else {
        current_fatigue
    }
}

/// Calculate health damage from high CO2
pub fn calculate_co2_damage(current_health: f32, co2: f32, delta_hours: f32) -> f32 {
    if co2 > 0.06 {
        current_health - (co2 - 0.06) * 0.3 * delta_hours
    } else {
        current_health
    }
}

/// Calculate comfort impact from temperature extremes
pub fn calculate_temperature_discomfort(current_comfort: f32, temperature: f32, delta_hours: f32) -> f32 {
    if temperature < 15.0 || temperature > 30.0 {
        (current_comfort + 0.1 * delta_hours).min(1.0)
    } else {
        current_comfort
    }
}

/// Calculate health damage from extreme temperatures
pub fn calculate_temperature_damage(current_health: f32, temperature: f32, delta_hours: f32) -> f32 {
    if temperature < 5.0 || temperature > 40.0 {
        current_health - 0.05 * delta_hours
    } else {
        current_health
    }
}

/// Calculate health damage from low pressure
pub fn calculate_pressure_damage(current_health: f32, pressure: f32, delta_hours: f32) -> f32 {
    if pressure < 80.0 {
        current_health - (80.0 - pressure) * 0.01 * delta_hours
    } else {
        current_health
    }
}

/// Calculate O2 consumption for a given population (fraction units per hour)
pub fn calculate_o2_consumption(population: f32, exercising_count: f32, delta_hours: f32) -> f32 {
    (population * 0.035 + exercising_count * 0.07) * delta_hours
}

/// Calculate CO2 production for a given population (fraction units per hour)
pub fn calculate_co2_production(population: f32, exercising_count: f32, delta_hours: f32) -> f32 {
    (population * 0.043 + exercising_count * 0.09) * delta_hours
}

/// Calculate life support efficiency from multiple subsystems
pub fn calculate_life_support_efficiency(subsystem_healths: &[(f32, bool)]) -> f32 {
    if subsystem_healths.is_empty() {
        return 0.0;
    }
    let sum: f32 = subsystem_healths
        .iter()
        .map(|(health, degraded)| {
            if *degraded {
                health * 0.5
            } else {
                *health
            }
        })
        .sum();
    sum / subsystem_healths.len() as f32
}

/// Calculate resource consumption per person per hour
pub fn calculate_food_consumption(person_count: f32, delta_hours: f32) -> f32 {
    let food_rate = 2.0 / 24.0;
    person_count * food_rate * delta_hours
}

/// Calculate water consumption per person per hour
pub fn calculate_water_consumption(person_count: f32, delta_hours: f32) -> f32 {
    let water_rate = 3.0 / 24.0;
    person_count * water_rate * delta_hours
}

/// Calculate oxygen resource consumption per person per hour
pub fn calculate_oxygen_resource_consumption(person_count: f32, delta_hours: f32) -> f32 {
    let oxygen_rate = 0.84 / 24.0;
    person_count * oxygen_rate * delta_hours
}

/// Calculate maintenance task priority (0.0 - 1.0)
pub fn calculate_maintenance_priority(subsystem_health: f32) -> f32 {
    (1.0 - subsystem_health).clamp(0.0, 1.0)
}

/// Calculate maintenance task duration based on damage severity
pub fn calculate_maintenance_duration(subsystem_health: f32) -> f32 {
    2.0 + (1.0 - subsystem_health) * 4.0
}

/// Calculate maintenance progress increment
pub fn calculate_maintenance_progress(current_progress: f32, delta_hours: f32, duration_hours: f32) -> f32 {
    (current_progress + delta_hours / duration_hours).min(1.0)
}

/// Determine component/subsystem status after repair
pub fn calculate_repair_status(health: f32) -> u8 {
    if health > 0.7 {
        system_statuses::NOMINAL
    } else {
        system_statuses::DEGRADED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // NEEDS SYSTEM TESTS
    // ============================================================================

    #[test]
    fn need_increase_clamps_to_max() {
        let result = calculate_need_change(0.9, 0.5, 1.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn need_decrease_clamps_to_zero() {
        let result = calculate_need_change(0.1, -0.5, 1.0);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn need_change_normal_increase() {
        let result = calculate_need_change(0.3, 0.2, 1.0);
        assert_eq!(result, 0.5);
    }

    #[test]
    fn need_change_normal_decrease() {
        let result = calculate_need_change(0.5, -0.2, 1.0);
        assert_eq!(result, 0.3);
    }

    #[test]
    fn health_recovery_occurs_when_needs_satisfied() {
        let result = calculate_health_recovery(0.8, 0.3, 0.3, 1.0);
        assert_eq!(result, 0.81);
    }

    #[test]
    fn health_recovery_no_change_when_already_full() {
        let result = calculate_health_recovery(1.0, 0.3, 0.3, 1.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn health_recovery_blocked_by_high_hunger() {
        let result = calculate_health_recovery(0.8, 0.6, 0.3, 1.0);
        assert_eq!(result, 0.8);
    }

    #[test]
    fn health_recovery_blocked_by_high_fatigue() {
        let result = calculate_health_recovery(0.8, 0.3, 0.6, 1.0);
        assert_eq!(result, 0.8);
    }

    #[test]
    fn starvation_causes_damage() {
        let result = calculate_starvation_damage(1.0, 1.0, 1.0);
        assert_eq!(result, 0.95);
    }

    #[test]
    fn starvation_no_damage_when_fed() {
        let result = calculate_starvation_damage(1.0, 0.8, 1.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn exhaustion_causes_damage() {
        let result = calculate_exhaustion_damage(1.0, 1.0, 1.0);
        assert_eq!(result, 0.98);
    }

    #[test]
    fn exhaustion_no_damage_when_rested() {
        let result = calculate_exhaustion_damage(1.0, 0.8, 1.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn morale_decreases_when_needs_high() {
        let result = calculate_morale_change(1.0, 0.8, 1.0);
        assert_eq!(result, 0.97);
    }

    #[test]
    fn morale_increases_when_needs_low() {
        let result = calculate_morale_change(0.5, 0.2, 1.0);
        assert_eq!(result, 0.51);
    }

    #[test]
    fn morale_stable_when_needs_moderate() {
        let result = calculate_morale_change(0.7, 0.5, 1.0);
        assert_eq!(result, 0.7);
    }

    #[test]
    fn morale_clamps_to_zero() {
        let result = calculate_morale_change(0.01, 0.9, 1.0);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn morale_clamps_to_one() {
        let result = calculate_morale_change(0.99, 0.1, 1.0);
        assert_eq!(result, 1.0);
    }

    // ============================================================================
    // ATMOSPHERE SYSTEM TESTS
    // ============================================================================

    #[test]
    fn low_oxygen_causes_damage() {
        let result = calculate_oxygen_damage(1.0, 0.10, 1.0);
        assert_eq!(result, 0.97); // (0.16 - 0.10) * 0.5 = 0.03 damage
    }

    #[test]
    fn normal_oxygen_no_damage() {
        let result = calculate_oxygen_damage(1.0, 0.21, 1.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn high_co2_increases_fatigue() {
        let result = calculate_co2_fatigue(0.5, 0.08, 1.0);
        assert_eq!(result, 0.58); // (0.08 - 0.04) * 2.0 = 0.08
    }

    #[test]
    fn normal_co2_no_fatigue_increase() {
        let result = calculate_co2_fatigue(0.5, 0.03, 1.0);
        assert_eq!(result, 0.5);
    }

    #[test]
    fn very_high_co2_causes_damage() {
        let result = calculate_co2_damage(1.0, 0.10, 1.0);
        assert_eq!(result, 0.988); // (0.10 - 0.06) * 0.3 = 0.012 damage
    }

    #[test]
    fn moderate_co2_no_damage() {
        let result = calculate_co2_damage(1.0, 0.05, 1.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn cold_temperature_causes_discomfort() {
        let result = calculate_temperature_discomfort(0.3, 10.0, 1.0);
        assert_eq!(result, 0.4);
    }

    #[test]
    fn hot_temperature_causes_discomfort() {
        let result = calculate_temperature_discomfort(0.3, 35.0, 1.0);
        assert_eq!(result, 0.4);
    }

    #[test]
    fn comfortable_temperature_no_discomfort() {
        let result = calculate_temperature_discomfort(0.3, 22.0, 1.0);
        assert_eq!(result, 0.3);
    }

    #[test]
    fn extreme_cold_causes_damage() {
        let result = calculate_temperature_damage(1.0, 0.0, 1.0);
        assert_eq!(result, 0.95);
    }

    #[test]
    fn extreme_heat_causes_damage() {
        let result = calculate_temperature_damage(1.0, 45.0, 1.0);
        assert_eq!(result, 0.95);
    }

    #[test]
    fn moderate_temperature_no_damage() {
        let result = calculate_temperature_damage(1.0, 25.0, 1.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn low_pressure_causes_damage() {
        let result = calculate_pressure_damage(1.0, 70.0, 1.0);
        assert_eq!(result, 0.9); // (80.0 - 70.0) * 0.01 = 0.1 damage
    }

    #[test]
    fn normal_pressure_no_damage() {
        let result = calculate_pressure_damage(1.0, 101.0, 1.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn o2_consumption_scales_with_population() {
        let result = calculate_o2_consumption(100.0, 0.0, 1.0);
        assert_eq!(result, 3.5); // 100 * 0.035 * 1.0
    }

    #[test]
    fn o2_consumption_higher_when_exercising() {
        let result = calculate_o2_consumption(100.0, 10.0, 1.0);
        assert_eq!(result, 4.2); // 100 * 0.035 + 10 * 0.07 = 3.5 + 0.7
    }

    #[test]
    fn co2_production_scales_with_population() {
        let result = calculate_co2_production(100.0, 0.0, 1.0);
        assert_eq!(result, 4.3); // 100 * 0.043 * 1.0
    }

    #[test]
    fn co2_production_higher_when_exercising() {
        let result = calculate_co2_production(100.0, 10.0, 1.0);
        assert!((result - 5.2).abs() < 0.01); // 100 * 0.043 + 10 * 0.09 = 4.3 + 0.9
    }

    #[test]
    fn life_support_efficiency_zero_when_empty() {
        let result = calculate_life_support_efficiency(&[]);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn life_support_efficiency_full_when_healthy() {
        let result = calculate_life_support_efficiency(&[(1.0, false), (1.0, false), (1.0, false)]);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn life_support_efficiency_reduced_when_degraded() {
        let result = calculate_life_support_efficiency(&[(1.0, true), (1.0, false)]);
        assert_eq!(result, 0.75); // (1.0 * 0.5 + 1.0) / 2
    }

    #[test]
    fn life_support_efficiency_partial_when_damaged() {
        let result = calculate_life_support_efficiency(&[(0.6, false), (0.4, false)]);
        assert_eq!(result, 0.5); // (0.6 + 0.4) / 2
    }

    // ============================================================================
    // SHIP SYSTEMS TESTS
    // ============================================================================

    #[test]
    fn food_consumption_scales_with_population() {
        let result = calculate_food_consumption(100.0, 24.0);
        assert!((result - 200.0).abs() < 0.01); // 100 * (2.0/24.0) * 24 = 200
    }

    #[test]
    fn water_consumption_scales_with_population() {
        let result = calculate_water_consumption(100.0, 24.0);
        assert!((result - 300.0).abs() < 0.01); // 100 * (3.0/24.0) * 24 = 300
    }

    #[test]
    fn oxygen_resource_consumption_scales_with_population() {
        let result = calculate_oxygen_resource_consumption(100.0, 24.0);
        assert!((result - 84.0).abs() < 0.01); // 100 * (0.84/24.0) * 24 = 84
    }

    // ============================================================================
    // DUTY SYSTEM TESTS
    // ============================================================================

    #[test]
    fn alpha_shift_on_duty_during_day() {
        assert!(should_be_on_duty(shifts::ALPHA, 10.0));
    }

    #[test]
    fn alpha_shift_off_duty_at_night() {
        assert!(!should_be_on_duty(shifts::ALPHA, 20.0));
    }

    #[test]
    fn beta_shift_on_duty_during_evening() {
        assert!(should_be_on_duty(shifts::BETA, 18.0));
    }

    #[test]
    fn beta_shift_off_duty_at_night() {
        assert!(!should_be_on_duty(shifts::BETA, 23.0));
    }

    #[test]
    fn gamma_shift_on_duty_at_night() {
        assert!(should_be_on_duty(shifts::GAMMA, 23.0));
    }

    #[test]
    fn gamma_shift_off_duty_during_day() {
        assert!(!should_be_on_duty(shifts::GAMMA, 12.0));
    }

    #[test]
    fn breakfast_time_detected() {
        assert!(is_meal_time(7.5));
    }

    #[test]
    fn lunch_time_detected() {
        assert!(is_meal_time(12.5));
    }

    #[test]
    fn dinner_time_detected() {
        assert!(is_meal_time(18.5));
    }

    #[test]
    fn non_meal_time_detected() {
        assert!(!is_meal_time(10.0));
    }

    #[test]
    fn passenger_sleep_time_at_night() {
        assert!(is_sleep_time(23.0, false));
    }

    #[test]
    fn passenger_awake_time_during_day() {
        assert!(!is_sleep_time(10.0, false));
    }

    #[test]
    fn crew_never_sleep_time_based_on_simple_check() {
        // Crew uses shift-based sleep, so this function returns false for crew
        assert!(!is_sleep_time(23.0, true));
    }

    // ============================================================================
    // SOCIAL SYSTEM TESTS
    // ============================================================================

    #[test]
    fn strangers_have_low_familiarity() {
        let result = classify_relationship(0.0, 0.05);
        assert_eq!(result, relationship_types::STRANGER);
    }

    #[test]
    fn enemies_have_negative_strength() {
        let result = classify_relationship(-0.6, 0.2);
        assert_eq!(result, relationship_types::ENEMY);
    }

    #[test]
    fn rivals_have_moderate_negative_strength() {
        let result = classify_relationship(-0.3, 0.2);
        assert_eq!(result, relationship_types::RIVAL);
    }

    #[test]
    fn acquaintances_have_low_familiarity() {
        let result = classify_relationship(0.1, 0.2);
        assert_eq!(result, relationship_types::ACQUAINTANCE);
    }

    #[test]
    fn friends_have_positive_strength() {
        let result = classify_relationship(0.5, 0.4);
        assert_eq!(result, relationship_types::FRIEND);
    }

    #[test]
    fn close_friends_have_high_strength() {
        let result = classify_relationship(0.8, 0.5);
        assert_eq!(result, relationship_types::CLOSE_FRIEND);
    }

    #[test]
    fn colleagues_are_familiar_but_neutral() {
        let result = classify_relationship(0.1, 0.4);
        assert_eq!(result, relationship_types::COLLEAGUE);
    }

    // ============================================================================
    // MAINTENANCE SYSTEM TESTS
    // ============================================================================

    #[test]
    fn maintenance_priority_high_for_damaged_system() {
        let result = calculate_maintenance_priority(0.2);
        assert_eq!(result, 0.8);
    }

    #[test]
    fn maintenance_priority_low_for_healthy_system() {
        let result = calculate_maintenance_priority(0.9);
        assert!((result - 0.1).abs() < 0.01);
    }

    #[test]
    fn maintenance_priority_max_for_destroyed_system() {
        let result = calculate_maintenance_priority(0.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn maintenance_duration_short_for_minor_damage() {
        let result = calculate_maintenance_duration(0.9);
        assert_eq!(result, 2.4); // 2.0 + (1.0 - 0.9) * 4.0
    }

    #[test]
    fn maintenance_duration_long_for_severe_damage() {
        let result = calculate_maintenance_duration(0.2);
        assert_eq!(result, 5.2); // 2.0 + (1.0 - 0.2) * 4.0
    }

    #[test]
    fn maintenance_progress_increments() {
        let result = calculate_maintenance_progress(0.3, 1.0, 4.0);
        assert_eq!(result, 0.55); // 0.3 + 1.0/4.0
    }

    #[test]
    fn maintenance_progress_clamps_to_one() {
        let result = calculate_maintenance_progress(0.9, 1.0, 2.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn repair_status_nominal_when_healthy() {
        let result = calculate_repair_status(0.8);
        assert_eq!(result, system_statuses::NOMINAL);
    }

    #[test]
    fn repair_status_degraded_when_damaged() {
        let result = calculate_repair_status(0.5);
        assert_eq!(result, system_statuses::DEGRADED);
    }

    #[test]
    fn repair_status_degraded_at_threshold() {
        let result = calculate_repair_status(0.7);
        assert_eq!(result, system_statuses::DEGRADED);
    }
}
