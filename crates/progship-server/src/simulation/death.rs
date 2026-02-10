//! Death system - checks for and processes NPC deaths.

use progship_logic::health;
use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

/// Check all living people for death conditions and process deaths.
/// Should run after `tick_needs` so health values are current.
pub fn tick_death(ctx: &ReducerContext, sim_time: f64) {
    // Collect dead people first to avoid mutation during iteration
    let dead_people: Vec<u64> = ctx
        .db
        .needs()
        .iter()
        .filter(|n| health::is_dead(n.health))
        .map(|n| n.person_id)
        .collect();

    for person_id in dead_people {
        // Skip already-dead people
        let Some(person) = ctx.db.person().id().find(person_id) else {
            continue;
        };
        if !person.is_alive {
            continue;
        }

        // Mark as dead
        let mut p = person;
        p.is_alive = false;
        ctx.db.person().id().update(p);

        // Get room for event placement
        let room_id = ctx
            .db
            .position()
            .person_id()
            .find(person_id)
            .map(|pos| pos.room_id)
            .unwrap_or(0);

        // Create death event
        ctx.db.event().insert(Event {
            id: 0,
            event_type: event_types::DEATH,
            room_id,
            started_at: sim_time,
            duration: 1.0,
            state: event_states::ACTIVE,
            responders_needed: 1,
            responders_assigned: 0,
            severity: 0.8,
        });

        // Apply morale impact to people in the same room (witnesses)
        let (witness_delta, shipwide_delta) = health::death_morale_impact();
        for mut needs in ctx.db.needs().iter() {
            if needs.person_id == person_id {
                continue;
            }
            let is_witness = ctx
                .db
                .position()
                .person_id()
                .find(needs.person_id)
                .map(|pos| pos.room_id == room_id)
                .unwrap_or(false);

            let delta = if is_witness {
                witness_delta
            } else {
                shipwide_delta
            };
            needs.morale = (needs.morale + delta).clamp(0.0, 1.0);
            ctx.db.needs().person_id().update(needs);
        }

        // Cancel any active movement
        if ctx.db.movement().person_id().find(person_id).is_some() {
            ctx.db.movement().person_id().delete(person_id);
        }

        // Cancel any conversation
        if let Some(ic) = ctx.db.in_conversation().person_id().find(person_id) {
            ctx.db.in_conversation().person_id().delete(person_id);
            // End the conversation
            if let Some(mut conv) = ctx.db.conversation().id().find(ic.conversation_id) {
                conv.state = conversation_states::ENDED;
                ctx.db.conversation().id().update(conv);
            }
        }

        // Update death count
        if let Some(mut config) = ctx.db.ship_config().id().find(0) {
            config.death_count += 1;
            ctx.db.ship_config().id().update(config);
        }

        log::info!(
            "Person {} has died (room {}). Total deaths: {}",
            person_id,
            room_id,
            ctx.db
                .ship_config()
                .id()
                .find(0)
                .map(|c| c.death_count)
                .unwrap_or(0)
        );
    }
}
