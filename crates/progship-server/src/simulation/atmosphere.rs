//! Atmosphere simulation - per-deck O2, CO2, temperature, humidity.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

/// Update per-deck atmosphere based on occupancy and life support.
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
