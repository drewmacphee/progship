//! Need decay system - hunger, fatigue, social, comfort, hygiene.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

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
