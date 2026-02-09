//! Pure health, medical, and death logic.
//!
//! Injury severity tiers, sickbay healing rates, natural recovery,
//! and death determination — all as pure functions.

use crate::constants::room_types;

/// Injury severity tiers based on health value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjurySeverity {
    /// Health >= 0.7 — recovers naturally without medical attention.
    Healthy,
    /// Health 0.4..0.7 — should seek medical help, slow natural recovery.
    Light,
    /// Health 0.2..0.4 — needs sickbay, no natural recovery.
    Moderate,
    /// Health < 0.2 — critical, requires sickbay or will die.
    Critical,
}

impl InjurySeverity {
    pub fn from_health(health: f32) -> Self {
        if health >= 0.7 {
            Self::Healthy
        } else if health >= 0.4 {
            Self::Light
        } else if health >= 0.2 {
            Self::Moderate
        } else {
            Self::Critical
        }
    }

    /// Whether this person should seek a medical room.
    pub fn needs_medical(self) -> bool {
        matches!(self, Self::Light | Self::Moderate | Self::Critical)
    }

    /// Whether natural recovery (outside sickbay) is possible.
    pub fn can_recover_naturally(self) -> bool {
        matches!(self, Self::Healthy | Self::Light)
    }
}

/// Base natural recovery rate per hour when needs are satisfied.
const NATURAL_RECOVERY_RATE: f32 = 0.01;

/// Sickbay base healing rate per hour.
const SICKBAY_BASE_RATE: f32 = 0.05;

/// Maximum medical skill bonus multiplier for sickbay healing.
const MEDICAL_SKILL_BONUS: f32 = 1.0;

/// Compute health recovery for a person this tick.
///
/// - `health`: current health [0.0, 1.0]
/// - `hunger`, `fatigue`: need levels (low = satisfied)
/// - `in_medical_room`: whether person is in a sickbay/hospital room
/// - `medical_skill_nearby`: highest medical skill of staff in the room (0.0 if none)
/// - `delta_hours`: time step
///
/// Returns new health value.
pub fn compute_health_recovery(
    health: f32,
    hunger: f32,
    fatigue: f32,
    in_medical_room: bool,
    medical_skill_nearby: f32,
    delta_hours: f32,
) -> f32 {
    if health >= 1.0 {
        return 1.0;
    }

    let severity = InjurySeverity::from_health(health);

    let recovery = if in_medical_room {
        // Sickbay healing: base rate + bonus from medical staff skill
        let skill_bonus = medical_skill_nearby * MEDICAL_SKILL_BONUS;
        (SICKBAY_BASE_RATE + skill_bonus) * delta_hours
    } else if severity.can_recover_naturally() && hunger < 0.5 && fatigue < 0.5 {
        // Natural recovery only for healthy/light injuries with satisfied needs
        NATURAL_RECOVERY_RATE * delta_hours
    } else {
        0.0
    };

    (health + recovery).min(1.0)
}

/// Check if a person should be considered dead.
pub fn is_dead(health: f32) -> bool {
    health <= 0.0
}

/// Determine if an NPC should seek medical attention based on current health.
pub fn should_seek_medical(health: f32) -> bool {
    InjurySeverity::from_health(health).needs_medical()
}

/// Check if a room type is a medical facility suitable for healing.
pub fn is_healing_room(room_type: u8) -> bool {
    room_types::is_medical(room_type)
}

/// Compute morale impact from a death event.
/// Returns (witness_morale_delta, shipwide_morale_delta).
pub fn death_morale_impact() -> (f32, f32) {
    (-0.3, -0.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_injury_severity_tiers() {
        assert_eq!(InjurySeverity::from_health(1.0), InjurySeverity::Healthy);
        assert_eq!(InjurySeverity::from_health(0.7), InjurySeverity::Healthy);
        assert_eq!(InjurySeverity::from_health(0.69), InjurySeverity::Light);
        assert_eq!(InjurySeverity::from_health(0.4), InjurySeverity::Light);
        assert_eq!(InjurySeverity::from_health(0.39), InjurySeverity::Moderate);
        assert_eq!(InjurySeverity::from_health(0.2), InjurySeverity::Moderate);
        assert_eq!(InjurySeverity::from_health(0.19), InjurySeverity::Critical);
        assert_eq!(InjurySeverity::from_health(0.0), InjurySeverity::Critical);
    }

    #[test]
    fn test_needs_medical() {
        assert!(!InjurySeverity::Healthy.needs_medical());
        assert!(InjurySeverity::Light.needs_medical());
        assert!(InjurySeverity::Moderate.needs_medical());
        assert!(InjurySeverity::Critical.needs_medical());
    }

    #[test]
    fn test_can_recover_naturally() {
        assert!(InjurySeverity::Healthy.can_recover_naturally());
        assert!(InjurySeverity::Light.can_recover_naturally());
        assert!(!InjurySeverity::Moderate.can_recover_naturally());
        assert!(!InjurySeverity::Critical.can_recover_naturally());
    }

    #[test]
    fn test_natural_recovery_satisfied_needs() {
        // Light injury, needs satisfied → should recover slowly
        let h = compute_health_recovery(0.6, 0.3, 0.3, false, 0.0, 1.0);
        assert!(h > 0.6);
        assert!((h - 0.61).abs() < 0.001); // 0.01/hour
    }

    #[test]
    fn test_no_recovery_hungry() {
        // Light injury but hungry → no recovery
        let h = compute_health_recovery(0.6, 0.6, 0.3, false, 0.0, 1.0);
        assert!((h - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_no_natural_recovery_moderate() {
        // Moderate injury, even with satisfied needs → no natural recovery
        let h = compute_health_recovery(0.3, 0.2, 0.2, false, 0.0, 1.0);
        assert!((h - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_sickbay_recovery_no_staff() {
        // In sickbay, no medical staff → base sickbay rate
        let h = compute_health_recovery(0.3, 0.8, 0.8, true, 0.0, 1.0);
        assert!((h - 0.35).abs() < 0.001); // 0.05/hour base
    }

    #[test]
    fn test_sickbay_recovery_with_staff() {
        // In sickbay with skilled doctor → base + skill bonus
        let h = compute_health_recovery(0.3, 0.8, 0.8, true, 0.8, 1.0);
        // (0.05 + 0.8*1.0) * 1.0 = 0.85/hour → 0.3 + 0.85 = 1.15 → clamped to 1.0
        assert!((h - 1.0).abs() < f32::EPSILON);
        // Smaller delta: (0.05 + 0.8*1.0) * 0.1 = 0.085 → 0.3 + 0.085 = 0.385
        let h2 = compute_health_recovery(0.3, 0.8, 0.8, true, 0.8, 0.1);
        assert!(h2 > 0.3);
        assert!(h2 < 0.4);
    }

    #[test]
    fn test_sickbay_heals_critical() {
        // Critical in sickbay → heals (even moderate/critical can heal in sickbay)
        let h = compute_health_recovery(0.1, 1.0, 1.0, true, 0.5, 1.0);
        assert!(h > 0.1); // should heal
    }

    #[test]
    fn test_recovery_capped_at_one() {
        let h = compute_health_recovery(0.99, 0.1, 0.1, true, 0.8, 10.0);
        assert!((h - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_already_full_health() {
        let h = compute_health_recovery(1.0, 0.1, 0.1, true, 0.8, 1.0);
        assert!((h - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_is_dead() {
        assert!(is_dead(0.0));
        assert!(is_dead(-0.1));
        assert!(!is_dead(0.01));
        assert!(!is_dead(1.0));
    }

    #[test]
    fn test_should_seek_medical() {
        assert!(!should_seek_medical(1.0));
        assert!(!should_seek_medical(0.7));
        assert!(should_seek_medical(0.69));
        assert!(should_seek_medical(0.1));
    }

    #[test]
    fn test_death_morale_impact() {
        let (witness, shipwide) = death_morale_impact();
        assert!(witness < 0.0);
        assert!(shipwide < 0.0);
        assert!(witness < shipwide); // witnesses affected more
    }

    #[test]
    fn test_is_healing_room() {
        assert!(is_healing_room(room_types::HOSPITAL_WARD));
        assert!(is_healing_room(room_types::SURGERY));
        assert!(!is_healing_room(room_types::BRIDGE));
        assert!(!is_healing_room(room_types::GYM));
    }
}
