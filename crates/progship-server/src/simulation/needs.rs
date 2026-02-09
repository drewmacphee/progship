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
        let rates = activity_decay_rates(activity.as_ref());

        // Apply need decay
        (n.hunger, n.fatigue, n.social, n.comfort, n.hygiene) = apply_need_decay(
            n.hunger,
            n.fatigue,
            n.social,
            n.comfort,
            n.hygiene,
            delta_hours,
            rates,
        );

        // Health changes
        n.health = health_recovery(n.health, n.hunger, n.fatigue, delta_hours);
        n.health = starvation_damage(n.health, n.hunger, delta_hours);
        n.health = exhaustion_damage(n.health, n.fatigue, delta_hours);

        // Morale affected by needs satisfaction
        let avg_needs = (n.hunger + n.fatigue + n.social + n.comfort + n.hygiene) / 5.0;
        n.morale = morale_change(n.morale, avg_needs, delta_hours);

        // Atmosphere effects on health
        if let Some(pos) = ctx.db.position().person_id().find(n.person_id) {
            if let Some(room) = ctx.db.room().id().find(pos.room_id) {
                if let Some(atmo) = atmospheres.iter().find(|a| a.deck == room.deck) {
                    (n.health, n.fatigue, n.comfort) = atmosphere_effects(
                        n.health,
                        n.fatigue,
                        n.comfort,
                        atmo.oxygen,
                        atmo.co2,
                        atmo.temperature,
                        atmo.pressure,
                        delta_hours,
                    );
                }
            }
        }

        n.health = n.health.clamp(0.0, 1.0);
        ctx.db.needs().person_id().update(n);
    }
}

/// Returns (hunger, fatigue, social, comfort, hygiene) decay rates per hour
pub fn activity_decay_rates(activity: Option<&Activity>) -> (f32, f32, f32, f32, f32) {
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

/// Apply need decay with rates, clamping result to [0.0, 1.0]
pub fn apply_need_decay(
    hunger: f32,
    fatigue: f32,
    social: f32,
    comfort: f32,
    hygiene: f32,
    delta_hours: f32,
    rates: (f32, f32, f32, f32, f32),
) -> (f32, f32, f32, f32, f32) {
    (
        (hunger + delta_hours * rates.0).clamp(0.0, 1.0),
        (fatigue + delta_hours * rates.1).clamp(0.0, 1.0),
        (social + delta_hours * rates.2).clamp(0.0, 1.0),
        (comfort + delta_hours * rates.3).clamp(0.0, 1.0),
        (hygiene + delta_hours * rates.4).clamp(0.0, 1.0),
    )
}

/// Calculate health recovery rate (slow, requires low hunger and fatigue)
pub fn health_recovery(health: f32, hunger: f32, fatigue: f32, delta_hours: f32) -> f32 {
    if health < 1.0 && hunger < 0.5 && fatigue < 0.5 {
        (health + 0.01 * delta_hours).min(1.0)
    } else {
        health
    }
}

/// Calculate health damage from starvation
pub fn starvation_damage(health: f32, hunger: f32, delta_hours: f32) -> f32 {
    if hunger >= 1.0 {
        health - 0.05 * delta_hours
    } else {
        health
    }
}

/// Calculate health damage from exhaustion
pub fn exhaustion_damage(health: f32, fatigue: f32, delta_hours: f32) -> f32 {
    if fatigue >= 1.0 {
        health - 0.02 * delta_hours
    } else {
        health
    }
}

/// Calculate morale change based on average needs
pub fn morale_change(morale: f32, avg_needs: f32, delta_hours: f32) -> f32 {
    if avg_needs > 0.7 {
        (morale - 0.03 * delta_hours).max(0.0)
    } else if avg_needs < 0.3 {
        (morale + 0.01 * delta_hours).min(1.0)
    } else {
        morale
    }
}

/// Calculate atmosphere effects on health, fatigue, and comfort
#[allow(clippy::too_many_arguments)]
pub fn atmosphere_effects(
    health: f32,
    fatigue: f32,
    comfort: f32,
    oxygen: f32,
    co2: f32,
    temperature: f32,
    pressure: f32,
    delta_hours: f32,
) -> (f32, f32, f32) {
    let mut h = health;
    let mut f = fatigue;
    let mut c = comfort;

    // Low oxygen → health damage and fatigue
    if oxygen < 0.16 {
        let o2_damage = (0.16 - oxygen) * 0.5 * delta_hours;
        h -= o2_damage;
        f = (f + 0.1 * delta_hours).min(1.0);
    }

    // High CO2 → fatigue and health damage
    if co2 > 0.04 {
        f = (f + (co2 - 0.04) * 2.0 * delta_hours).min(1.0);
        if co2 > 0.06 {
            h -= (co2 - 0.06) * 0.3 * delta_hours;
        }
    }

    // Temperature extremes → comfort
    if !(15.0..=30.0).contains(&temperature) {
        c = (c + 0.1 * delta_hours).min(1.0);
    }

    // Extreme temperature → health damage
    if !(5.0..=40.0).contains(&temperature) {
        h -= 0.05 * delta_hours;
    }

    // Low pressure → rapid health damage
    if pressure < 80.0 {
        h -= (80.0 - pressure) * 0.01 * delta_hours;
    }

    (h, f, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_decay_rates_sleeping() {
        let rates = activity_decay_rates(None);
        // Sleeping should decrease fatigue (negative rate) and comfort
        let sleeping_activity = Activity {
            person_id: 1,
            activity_type: activity_types::SLEEPING,
            started_at: 0.0,
            duration: 8.0,
            target_room_id: None,
        };
        let sleeping_rates = activity_decay_rates(Some(&sleeping_activity));
        assert_eq!(sleeping_rates.1, -0.15); // Fatigue decreases
        assert!(sleeping_rates.0 < rates.0); // Hunger increases slower
    }

    #[test]
    fn test_activity_decay_rates_eating() {
        let eating_activity = Activity {
            person_id: 1,
            activity_type: activity_types::EATING,
            started_at: 0.0,
            duration: 0.5,
            target_room_id: None,
        };
        let rates = activity_decay_rates(Some(&eating_activity));
        assert_eq!(rates.0, -0.3); // Hunger decreases
        assert!(rates.1 > 0.0); // Fatigue still increases slightly
    }

    #[test]
    fn test_activity_decay_rates_exercising() {
        let exercising_activity = Activity {
            person_id: 1,
            activity_type: activity_types::EXERCISING,
            started_at: 0.0,
            duration: 1.0,
            target_room_id: None,
        };
        let rates = activity_decay_rates(Some(&exercising_activity));
        assert!(rates.0 > 0.05); // Hunger increases more
        assert!(rates.1 > 0.05); // Fatigue increases more
        assert!(rates.4 > 0.03); // Hygiene increases more
    }

    #[test]
    fn test_apply_need_decay_clamps_at_one() {
        let result = apply_need_decay(0.9, 0.8, 0.7, 0.6, 0.5, 10.0, (0.1, 0.1, 0.1, 0.1, 0.1));
        assert_eq!(result.0, 1.0); // Clamped at 1.0
        assert_eq!(result.1, 1.0);
        assert_eq!(result.2, 1.0);
        assert_eq!(result.3, 1.0);
        assert_eq!(result.4, 1.0);
    }

    #[test]
    fn test_apply_need_decay_clamps_at_zero() {
        let result = apply_need_decay(
            0.1,
            0.1,
            0.1,
            0.1,
            0.1,
            10.0,
            (-0.1, -0.1, -0.1, -0.1, -0.1),
        );
        assert_eq!(result.0, 0.0); // Clamped at 0.0
        assert_eq!(result.1, 0.0);
        assert_eq!(result.2, 0.0);
        assert_eq!(result.3, 0.0);
        assert_eq!(result.4, 0.0);
    }

    #[test]
    fn test_apply_need_decay_normal() {
        let result = apply_need_decay(0.0, 0.0, 0.0, 0.0, 0.0, 1.0, (0.04, 0.03, 0.02, 0.02, 0.02));
        assert_eq!(result.0, 0.04);
        assert_eq!(result.1, 0.03);
        assert_eq!(result.2, 0.02);
        assert_eq!(result.3, 0.02);
        assert_eq!(result.4, 0.02);
    }

    #[test]
    fn test_health_recovery_requires_low_hunger_and_fatigue() {
        // Should recover
        let health = health_recovery(0.5, 0.3, 0.3, 1.0);
        assert!(health > 0.5);
        assert_eq!(health, 0.51); // 0.5 + 0.01 * 1.0

        // Should not recover - hunger too high
        let health = health_recovery(0.5, 0.6, 0.3, 1.0);
        assert_eq!(health, 0.5);

        // Should not recover - fatigue too high
        let health = health_recovery(0.5, 0.3, 0.6, 1.0);
        assert_eq!(health, 0.5);

        // Should not recover - already at max
        let health = health_recovery(1.0, 0.3, 0.3, 1.0);
        assert_eq!(health, 1.0);
    }

    #[test]
    fn test_starvation_damage() {
        // At 100% hunger, should take damage
        let health = starvation_damage(1.0, 1.0, 1.0);
        assert_eq!(health, 0.95); // 1.0 - 0.05 * 1.0

        // Below 100% hunger, no damage
        let health = starvation_damage(1.0, 0.99, 1.0);
        assert_eq!(health, 1.0);
    }

    #[test]
    fn test_exhaustion_damage() {
        // At 100% fatigue, should take damage
        let health = exhaustion_damage(1.0, 1.0, 1.0);
        assert_eq!(health, 0.98); // 1.0 - 0.02 * 1.0

        // Below 100% fatigue, no damage
        let health = exhaustion_damage(1.0, 0.99, 1.0);
        assert_eq!(health, 1.0);
    }

    #[test]
    fn test_morale_change_high_needs() {
        // High average needs (0.7+) should decrease morale
        let morale = morale_change(1.0, 0.8, 1.0);
        assert_eq!(morale, 0.97); // 1.0 - 0.03 * 1.0

        // Morale clamped at 0
        let morale = morale_change(0.01, 0.8, 1.0);
        assert_eq!(morale, 0.0);
    }

    #[test]
    fn test_morale_change_low_needs() {
        // Low average needs (<0.3) should increase morale
        let morale = morale_change(0.0, 0.2, 1.0);
        assert_eq!(morale, 0.01); // 0.0 + 0.01 * 1.0

        // Morale clamped at 1
        let morale = morale_change(0.99, 0.2, 1.0);
        assert_eq!(morale, 1.0);
    }

    #[test]
    fn test_morale_change_moderate_needs() {
        // Moderate needs (0.3-0.7) should not change morale
        let morale = morale_change(0.5, 0.5, 1.0);
        assert_eq!(morale, 0.5);
    }

    #[test]
    fn test_atmosphere_effects_low_oxygen() {
        let (h, f, c) = atmosphere_effects(1.0, 0.0, 0.0, 0.10, 0.02, 20.0, 100.0, 1.0);
        assert!(h < 1.0); // Health damage
        assert!(f > 0.0); // Fatigue increase
        assert_eq!(c, 0.0); // Comfort unchanged
    }

    #[test]
    fn test_atmosphere_effects_high_co2() {
        let (h, f, c) = atmosphere_effects(1.0, 0.0, 0.0, 0.21, 0.08, 20.0, 100.0, 1.0);
        assert!(h < 1.0); // Health damage (CO2 > 0.06)
        assert!(f > 0.0); // Fatigue increase
    }

    #[test]
    fn test_atmosphere_effects_temperature_extremes() {
        // Cold
        let (h, _f, c) = atmosphere_effects(1.0, 0.0, 0.0, 0.21, 0.02, 10.0, 100.0, 1.0);
        assert_eq!(h, 1.0); // No health damage (> 5.0)
        assert!(c > 0.0); // Comfort decreased

        // Hot
        let (h, _f, c) = atmosphere_effects(1.0, 0.0, 0.0, 0.21, 0.02, 35.0, 100.0, 1.0);
        assert_eq!(h, 1.0); // No health damage (< 40.0)
        assert!(c > 0.0); // Comfort decreased

        // Extreme cold
        let (h, _f, c) = atmosphere_effects(1.0, 0.0, 0.0, 0.21, 0.02, 0.0, 100.0, 1.0);
        assert!(h < 1.0); // Health damage
        assert!(c > 0.0); // Comfort decreased

        // Extreme heat
        let (h, _f, c) = atmosphere_effects(1.0, 0.0, 0.0, 0.21, 0.02, 45.0, 100.0, 1.0);
        assert!(h < 1.0); // Health damage
        assert!(c > 0.0); // Comfort decreased
    }

    #[test]
    fn test_atmosphere_effects_low_pressure() {
        let (h, _f, _c) = atmosphere_effects(1.0, 0.0, 0.0, 0.21, 0.02, 20.0, 50.0, 1.0);
        assert!(h < 1.0); // Health damage
                          // Damage should be: (80 - 50) * 0.01 * 1.0 = 0.3
        assert!((h - 0.7).abs() < 0.001); // Allow small floating point error
    }

    #[test]
    fn test_atmosphere_effects_normal_conditions() {
        let (h, f, c) = atmosphere_effects(1.0, 0.0, 0.0, 0.21, 0.02, 20.0, 100.0, 1.0);
        assert_eq!(h, 1.0); // No health change
        assert_eq!(f, 0.0); // No fatigue change
        assert_eq!(c, 0.0); // No comfort change
    }
}
