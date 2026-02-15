//! Event system - random ship events with real consequences.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

/// Generate random events and progress existing ones with real consequences.
pub fn tick_events(ctx: &ReducerContext, sim_time: f64, delta_hours: f32) {
    // Progress existing events and apply consequences
    let events: Vec<Event> = ctx.db.event().iter().collect();
    let mut active_count = 0u32;
    for event in events {
        if event.state == event_states::RESOLVED {
            // Clean up resolved events immediately
            ctx.db.event().id().delete(event.id);
            continue;
        }

        active_count += 1;
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

    // Cap active events to prevent runaway accumulation
    if active_count >= 10 {
        return;
    }

    // Generate new events - use high-precision time bits for pseudo-randomness
    let time_bits = (sim_time * 100000.0) as u64;
    let hash = time_bits
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let event_chance = (hash >> 32) % 1000; // Use upper bits for better distribution

    if event_chance < 3 {
        // ~0.3% chance per tick (reduced from 0.5%)
        let hash2 = hash.wrapping_mul(2862933555777941757);
        let event_type = (hash2 % 8) as u8;
        let severity = 0.3 + ((hash2 / 8 % 50) as f32 * 0.01);

        // Pick a random room (only content rooms, not corridors)
        let rooms: Vec<Room> = ctx.db.room().iter().filter(|r| r.room_type < 100).collect();
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
