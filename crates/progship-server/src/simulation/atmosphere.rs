//! Atmosphere simulation - per-deck O2, CO2, temperature, humidity.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

/// Calculate life support efficiency from subsystems
pub fn calculate_life_support_efficiency(subsystems: &[Subsystem]) -> f32 {
    if subsystems.is_empty() {
        return 0.0;
    }
    let total: f32 = subsystems
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
        .sum();
    total / subsystems.len() as f32
}

/// Calculate metabolic impact on atmosphere
pub fn calculate_metabolic_impact(
    population: f32,
    exercising: f32,
    delta_hours: f32,
) -> (f32, f32, f32, f32) {
    let o2_consumption = (population * 0.035 + exercising * 0.07) * delta_hours;
    let co2_production = (population * 0.043 + exercising * 0.09) * delta_hours;
    let humidity_add = (population * 0.005 + exercising * 0.015) * delta_hours;
    let heat_add = (population * 0.1 + exercising * 0.3) * delta_hours;
    (o2_consumption, co2_production, humidity_add, heat_add)
}

/// Apply atmosphere changes with life support counteraction
pub fn apply_atmosphere_changes(
    oxygen: f32,
    co2: f32,
    humidity: f32,
    temperature: f32,
    metabolic: (f32, f32, f32, f32),
    ls_efficiency: f32,
) -> (f32, f32, f32, f32) {
    let (o2_consumption, co2_production, humidity_add, heat_add) = metabolic;

    let mut o2 = oxygen - o2_consumption;
    let mut co2_val = co2 + co2_production;
    let mut hum = humidity + humidity_add;
    let mut temp = temperature + heat_add;

    // Life support counteraction
    o2 += o2_consumption * ls_efficiency;
    co2_val -= co2_production * ls_efficiency * 0.95;
    hum -= humidity_add * ls_efficiency * 0.8;
    temp -= heat_add * ls_efficiency * 0.9;

    // Clamp values
    o2 = o2.clamp(0.0, 0.25);
    co2_val = co2_val.clamp(0.0, 0.1);
    hum = hum.clamp(0.0, 1.0);
    temp = temp.clamp(-10.0, 50.0);

    (o2, co2_val, hum, temp)
}

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
    let ls_efficiency = calculate_life_support_efficiency(&ls_subsystems);

    for atmo in ctx.db.deck_atmosphere().iter() {
        let pop = *deck_population.get(&atmo.deck).unwrap_or(&0) as f32;
        let exercising = *deck_exercising.get(&atmo.deck).unwrap_or(&0) as f32;

        let mut a = atmo;

        let metabolic = calculate_metabolic_impact(pop, exercising, delta_hours);
        (a.oxygen, a.co2, a.humidity, a.temperature) = apply_atmosphere_changes(
            a.oxygen,
            a.co2,
            a.humidity,
            a.temperature,
            metabolic,
            ls_efficiency,
        );

        ctx.db.deck_atmosphere().deck().update(a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_life_support_efficiency_empty() {
        let subsystems: Vec<Subsystem> = vec![];
        assert_eq!(calculate_life_support_efficiency(&subsystems), 0.0);
    }

    #[test]
    fn test_calculate_life_support_efficiency_all_nominal() {
        let subsystems = vec![
            Subsystem {
                id: 1,
                system_id: 1,
                name: String::from("O2 Gen"),
                subsystem_type: subsystem_types::O2_GENERATION,
                health: 1.0,
                status: system_statuses::NOMINAL,
                node_id: 0,
                power_draw: 10.0,
                crew_required: 0,
            },
            Subsystem {
                id: 2,
                system_id: 1,
                name: String::from("CO2 Scrubber"),
                subsystem_type: subsystem_types::CO2_SCRUBBING,
                health: 1.0,
                status: system_statuses::NOMINAL,
                node_id: 0,
                power_draw: 8.0,
                crew_required: 0,
            },
        ];
        assert_eq!(calculate_life_support_efficiency(&subsystems), 1.0);
    }

    #[test]
    fn test_calculate_life_support_efficiency_degraded() {
        let subsystems = vec![Subsystem {
            id: 1,
            system_id: 1,
            name: String::from("O2 Gen"),
            subsystem_type: subsystem_types::O2_GENERATION,
            health: 1.0,
            status: system_statuses::DEGRADED,
            node_id: 0,
            power_draw: 10.0,
            crew_required: 0,
        }];
        assert_eq!(calculate_life_support_efficiency(&subsystems), 0.5);
    }

    #[test]
    fn test_calculate_life_support_efficiency_offline() {
        let subsystems = vec![Subsystem {
            id: 1,
            system_id: 1,
            name: String::from("O2 Gen"),
            subsystem_type: subsystem_types::O2_GENERATION,
            health: 1.0,
            status: system_statuses::OFFLINE,
            node_id: 0,
            power_draw: 10.0,
            crew_required: 0,
        }];
        assert_eq!(calculate_life_support_efficiency(&subsystems), 0.0);
    }

    #[test]
    fn test_calculate_life_support_efficiency_mixed() {
        let subsystems = vec![
            Subsystem {
                id: 1,
                system_id: 1,
                name: String::from("O2 Gen"),
                subsystem_type: subsystem_types::O2_GENERATION,
                health: 1.0,
                status: system_statuses::NOMINAL,
                node_id: 0,
                power_draw: 10.0,
                crew_required: 0,
            },
            Subsystem {
                id: 2,
                system_id: 1,
                name: String::from("CO2 Scrubber"),
                subsystem_type: subsystem_types::CO2_SCRUBBING,
                health: 0.5,
                status: system_statuses::DEGRADED,
                node_id: 0,
                power_draw: 8.0,
                crew_required: 0,
            },
        ];
        // (1.0 + 0.5 * 0.5) / 2 = 1.25 / 2 = 0.625
        assert_eq!(calculate_life_support_efficiency(&subsystems), 0.625);
    }

    #[test]
    fn test_calculate_metabolic_impact_no_population() {
        let (o2, co2, hum, heat) = calculate_metabolic_impact(0.0, 0.0, 1.0);
        assert_eq!(o2, 0.0);
        assert_eq!(co2, 0.0);
        assert_eq!(hum, 0.0);
        assert_eq!(heat, 0.0);
    }

    #[test]
    fn test_calculate_metabolic_impact_population_only() {
        let (o2, co2, hum, heat) = calculate_metabolic_impact(10.0, 0.0, 1.0);
        assert_eq!(o2, 10.0 * 0.035);
        assert_eq!(co2, 10.0 * 0.043);
        assert_eq!(hum, 10.0 * 0.005);
        assert_eq!(heat, 10.0 * 0.1);
    }

    #[test]
    fn test_calculate_metabolic_impact_with_exercising() {
        let (o2, co2, hum, heat) = calculate_metabolic_impact(10.0, 2.0, 1.0);
        assert_eq!(o2, 10.0 * 0.035 + 2.0 * 0.07);
        assert_eq!(co2, 10.0 * 0.043 + 2.0 * 0.09);
        assert_eq!(hum, 10.0 * 0.005 + 2.0 * 0.015);
        assert_eq!(heat, 10.0 * 0.1 + 2.0 * 0.3);
    }

    #[test]
    fn test_apply_atmosphere_changes_no_life_support() {
        let (o2, co2, hum, temp) =
            apply_atmosphere_changes(0.21, 0.02, 0.4, 20.0, (0.05, 0.06, 0.02, 0.5), 0.0);
        // No life support, only metabolic impact
        assert!((o2 - 0.16).abs() < 0.001); // 0.21 - 0.05
        assert!((co2 - 0.08).abs() < 0.001); // 0.02 + 0.06
        assert!((hum - 0.42).abs() < 0.001); // 0.4 + 0.02
        assert!((temp - 20.5).abs() < 0.001); // 20.0 + 0.5
    }

    #[test]
    fn test_apply_atmosphere_changes_full_life_support() {
        let (o2, co2, hum, temp) =
            apply_atmosphere_changes(0.21, 0.02, 0.4, 20.0, (0.05, 0.06, 0.02, 0.5), 1.0);
        // Full life support (100% efficiency)
        assert!((o2 - 0.21).abs() < 0.001); // 0.21 - 0.05 + 0.05 * 1.0
        assert!((co2 - (0.02 + 0.06 - 0.06 * 0.95)).abs() < 0.001); // 0.02 + 0.06 - 0.057
        assert!((hum - (0.4 + 0.02 - 0.02 * 0.8)).abs() < 0.001); // 0.4 + 0.02 - 0.016
        assert!((temp - (20.0 + 0.5 - 0.5 * 0.9)).abs() < 0.001); // 20.0 + 0.5 - 0.45
    }

    #[test]
    fn test_apply_atmosphere_changes_clamping() {
        // Test oxygen clamp at 0.25
        let (o2, _, _, _) =
            apply_atmosphere_changes(0.25, 0.02, 0.4, 20.0, (-0.1, 0.0, 0.0, 0.0), 1.0);
        assert_eq!(o2, 0.25);

        // Test CO2 clamp at 0.1
        let (_, co2, _, _) =
            apply_atmosphere_changes(0.21, 0.09, 0.4, 20.0, (0.0, 0.02, 0.0, 0.0), 0.0);
        assert_eq!(co2, 0.1);

        // Test humidity clamp at 1.0
        let (_, _, hum, _) =
            apply_atmosphere_changes(0.21, 0.02, 0.95, 20.0, (0.0, 0.0, 0.1, 0.0), 0.0);
        assert_eq!(hum, 1.0);

        // Test temp clamp at 50.0
        let (_, _, _, temp) =
            apply_atmosphere_changes(0.21, 0.02, 0.4, 48.0, (0.0, 0.0, 0.0, 5.0), 0.0);
        assert_eq!(temp, 50.0);
    }
}
