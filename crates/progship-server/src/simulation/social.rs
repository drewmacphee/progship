//! Social system - conversations and relationships between people.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

/// Start and end conversations between nearby people.
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
