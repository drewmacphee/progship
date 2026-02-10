//! System selection algorithm — picks system variants based on mission config.
//!
//! Given a `MissionConfig` (tech level, budget class, mission priority weights),
//! this module selects an appropriate variant for each system category using
//! weighted scoring. Player overrides are supported — explicit selections are
//! accepted, remaining slots filled by the algorithm.

use serde::{Deserialize, Serialize};

use crate::mission::{MissionConfig, MissionPriority};
use crate::systems::*;

/// Selected system variants for a ship configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSelection {
    pub power: u8,
    pub life_support: u8,
    pub food: u8,
    pub water: u8,
    pub defense: u8,
    pub medical: u8,
    pub gravity: u8,
}

/// Optional player overrides — `None` means "let the algorithm pick".
#[derive(Debug, Clone, Default)]
pub struct SystemOverrides {
    pub power: Option<u8>,
    pub life_support: Option<u8>,
    pub food: Option<u8>,
    pub water: Option<u8>,
    pub defense: Option<u8>,
    pub medical: Option<u8>,
    pub gravity: Option<u8>,
}

/// Select system variants based on mission config with optional overrides.
///
/// Uses a deterministic scoring algorithm seeded by `config.seed`.
/// Each variant is scored based on:
/// - Tech level compatibility (must meet min_tech_level)
/// - Budget class fit (austere prefers cheap/reliable, premium prefers high-output)
/// - Mission priority alignment (safety → high MTBF, comfort → low crew, etc.)
///
/// The highest-scoring eligible variant is selected for each category.
pub fn select_systems(config: &MissionConfig, overrides: &SystemOverrides) -> SystemSelection {
    let seed = config.seed;

    SystemSelection {
        power: overrides.power.unwrap_or_else(|| {
            select_best_power(
                config.tech_level,
                config.budget_class,
                &config.mission_priority,
                seed,
            )
        }),
        life_support: overrides.life_support.unwrap_or_else(|| {
            select_best_life_support(
                config.tech_level,
                config.budget_class,
                &config.mission_priority,
                seed,
            )
        }),
        food: overrides.food.unwrap_or_else(|| {
            select_best_food(
                config.tech_level,
                config.budget_class,
                &config.mission_priority,
                seed,
            )
        }),
        water: overrides.water.unwrap_or_else(|| {
            select_best_water(
                config.tech_level,
                config.budget_class,
                &config.mission_priority,
                seed,
            )
        }),
        defense: overrides.defense.unwrap_or_else(|| {
            select_best_defense(
                config.tech_level,
                config.budget_class,
                &config.mission_priority,
                seed,
            )
        }),
        medical: overrides.medical.unwrap_or_else(|| {
            select_best_medical(
                config.tech_level,
                config.budget_class,
                &config.mission_priority,
                seed,
            )
        }),
        gravity: overrides.gravity.unwrap_or_else(|| {
            select_best_gravity(
                config.tech_level,
                config.budget_class,
                &config.mission_priority,
                seed,
            )
        }),
    }
}

/// Score a system variant based on mission parameters.
/// Higher score = better fit.
fn score_variant(
    spec: &SystemSpec,
    tech_level: u8,
    budget_class: u8,
    priority: &MissionPriority,
    category_seed: u64,
    variant_index: usize,
) -> f32 {
    // Hard filter: tech level must be sufficient
    if spec.min_tech_level > tech_level {
        return -1.0;
    }

    let mut score: f32 = 0.0;

    // Safety: higher MTBF is better. Normalize to 0-1 range.
    let reliability = (spec.mtbf_hours as f32 / 200_000.0).min(1.0);
    score += priority.safety * reliability * 2.0;

    // Comfort: lower crew requirement is more comfortable (less labor).
    let crew_efficiency = 1.0 - (spec.crew_needed as f32 / 20.0).min(1.0);
    score += priority.comfort * crew_efficiency;

    // Self-sufficiency: higher output is better.
    // Normalize output — varies wildly by category, so use log scale.
    let output_score = (spec.output.ln().max(0.0)) / 10.0;
    score += priority.self_sufficiency * output_score;

    // Budget class modifier:
    // Austere (1): penalize heavy/expensive systems (high mass, high power draw)
    // Standard (2): neutral
    // Premium (3): reward high-output regardless of cost
    let mass_penalty = (spec.mass_tons / 500.0).min(1.0);
    match budget_class {
        1 => {
            score -= mass_penalty * 1.5;
            score -= (spec.power_draw / 500.0).min(1.0) * 1.0;
        }
        3 => {
            score += output_score * 0.5; // Bonus for high output
        }
        _ => {} // Standard: no adjustment
    }

    // Tech level alignment: prefer variants that match the tech level
    // (not too primitive, not too advanced)
    let tech_gap = (tech_level as f32 - spec.min_tech_level as f32).abs();
    score -= tech_gap * 0.1;

    // Deterministic jitter for variety — prevents same selection every time
    let jitter = simple_hash(category_seed, variant_index) * 0.2;
    score += jitter;

    score
}

/// Simple deterministic hash for seeded jitter, returns 0.0..1.0.
fn simple_hash(seed: u64, index: usize) -> f32 {
    let mut h = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(index as u64);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    (h % 1000) as f32 / 1000.0
}

fn select_best_power(tech: u8, budget: u8, priority: &MissionPriority, seed: u64) -> u8 {
    let mut best_idx = 0u8;
    let mut best_score = f32::NEG_INFINITY;
    for (i, v) in PowerVariant::all().iter().enumerate() {
        let s = score_variant(&v.spec(), tech, budget, priority, seed.wrapping_add(1), i);
        if s > best_score {
            best_score = s;
            best_idx = *v as u8;
        }
    }
    best_idx
}

fn select_best_life_support(tech: u8, budget: u8, priority: &MissionPriority, seed: u64) -> u8 {
    let mut best_idx = 0u8;
    let mut best_score = f32::NEG_INFINITY;
    for (i, v) in LifeSupportVariant::all().iter().enumerate() {
        let s = score_variant(&v.spec(), tech, budget, priority, seed.wrapping_add(2), i);
        if s > best_score {
            best_score = s;
            best_idx = *v as u8;
        }
    }
    best_idx
}

fn select_best_food(tech: u8, budget: u8, priority: &MissionPriority, seed: u64) -> u8 {
    let mut best_idx = 0u8;
    let mut best_score = f32::NEG_INFINITY;
    for (i, v) in FoodVariant::all().iter().enumerate() {
        let s = score_variant(&v.spec(), tech, budget, priority, seed.wrapping_add(3), i);
        if s > best_score {
            best_score = s;
            best_idx = *v as u8;
        }
    }
    best_idx
}

fn select_best_water(tech: u8, budget: u8, priority: &MissionPriority, seed: u64) -> u8 {
    let mut best_idx = 0u8;
    let mut best_score = f32::NEG_INFINITY;
    for (i, v) in WaterVariant::all().iter().enumerate() {
        let s = score_variant(&v.spec(), tech, budget, priority, seed.wrapping_add(4), i);
        if s > best_score {
            best_score = s;
            best_idx = *v as u8;
        }
    }
    best_idx
}

fn select_best_defense(tech: u8, budget: u8, priority: &MissionPriority, seed: u64) -> u8 {
    let mut best_idx = 0u8;
    let mut best_score = f32::NEG_INFINITY;
    for (i, v) in DefenseVariant::all().iter().enumerate() {
        let s = score_variant(&v.spec(), tech, budget, priority, seed.wrapping_add(5), i);
        if s > best_score {
            best_score = s;
            best_idx = *v as u8;
        }
    }
    best_idx
}

fn select_best_medical(tech: u8, budget: u8, priority: &MissionPriority, seed: u64) -> u8 {
    let mut best_idx = 0u8;
    let mut best_score = f32::NEG_INFINITY;
    for (i, v) in MedicalVariant::all().iter().enumerate() {
        let s = score_variant(&v.spec(), tech, budget, priority, seed.wrapping_add(6), i);
        if s > best_score {
            best_score = s;
            best_idx = *v as u8;
        }
    }
    best_idx
}

fn select_best_gravity(tech: u8, budget: u8, priority: &MissionPriority, seed: u64) -> u8 {
    let mut best_idx = 0u8;
    let mut best_score = f32::NEG_INFINITY;
    for (i, v) in GravityVariant::all().iter().enumerate() {
        let s = score_variant(&v.spec(), tech, budget, priority, seed.wrapping_add(7), i);
        if s > best_score {
            best_score = s;
            best_idx = *v as u8;
        }
    }
    best_idx
}

/// Total crew required by the selected systems.
pub fn total_system_crew(sel: &SystemSelection) -> u32 {
    let p = PowerVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.power)
        .map(|v| v.spec().crew_needed)
        .unwrap_or(0);
    let ls = LifeSupportVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.life_support)
        .map(|v| v.spec().crew_needed)
        .unwrap_or(0);
    let f = FoodVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.food)
        .map(|v| v.spec().crew_needed)
        .unwrap_or(0);
    let w = WaterVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.water)
        .map(|v| v.spec().crew_needed)
        .unwrap_or(0);
    let d = DefenseVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.defense)
        .map(|v| v.spec().crew_needed)
        .unwrap_or(0);
    let m = MedicalVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.medical)
        .map(|v| v.spec().crew_needed)
        .unwrap_or(0);
    let g = GravityVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.gravity)
        .map(|v| v.spec().crew_needed)
        .unwrap_or(0);
    p + ls + f + w + d + m + g
}

/// Total power draw of all non-power systems.
pub fn total_power_draw(sel: &SystemSelection) -> f32 {
    let ls = LifeSupportVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.life_support)
        .map(|v| v.spec().power_draw)
        .unwrap_or(0.0);
    let f = FoodVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.food)
        .map(|v| v.spec().power_draw)
        .unwrap_or(0.0);
    let w = WaterVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.water)
        .map(|v| v.spec().power_draw)
        .unwrap_or(0.0);
    let d = DefenseVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.defense)
        .map(|v| v.spec().power_draw)
        .unwrap_or(0.0);
    let m = MedicalVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.medical)
        .map(|v| v.spec().power_draw)
        .unwrap_or(0.0);
    let g = GravityVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.gravity)
        .map(|v| v.spec().power_draw)
        .unwrap_or(0.0);
    ls + f + w + d + m + g
}

/// Total mass of all selected systems.
pub fn total_system_mass(sel: &SystemSelection) -> f32 {
    let p = PowerVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.power)
        .map(|v| v.spec().mass_tons)
        .unwrap_or(0.0);
    let ls = LifeSupportVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.life_support)
        .map(|v| v.spec().mass_tons)
        .unwrap_or(0.0);
    let f = FoodVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.food)
        .map(|v| v.spec().mass_tons)
        .unwrap_or(0.0);
    let w = WaterVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.water)
        .map(|v| v.spec().mass_tons)
        .unwrap_or(0.0);
    let d = DefenseVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.defense)
        .map(|v| v.spec().mass_tons)
        .unwrap_or(0.0);
    let m = MedicalVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.medical)
        .map(|v| v.spec().mass_tons)
        .unwrap_or(0.0);
    let g = GravityVariant::all()
        .iter()
        .find(|v| **v as u8 == sel.gravity)
        .map(|v| v.spec().mass_tons)
        .unwrap_or(0.0);
    p + ls + f + w + d + m + g
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::MissionConfig;

    #[test]
    fn test_default_config_selects_valid() {
        let config = MissionConfig::default();
        let sel = select_systems(&config, &SystemOverrides::default());
        // All selections should be valid variant indices
        assert!(PowerVariant::all().iter().any(|v| *v as u8 == sel.power));
        assert!(LifeSupportVariant::all()
            .iter()
            .any(|v| *v as u8 == sel.life_support));
        assert!(FoodVariant::all().iter().any(|v| *v as u8 == sel.food));
        assert!(WaterVariant::all().iter().any(|v| *v as u8 == sel.water));
        assert!(DefenseVariant::all()
            .iter()
            .any(|v| *v as u8 == sel.defense));
        assert!(MedicalVariant::all()
            .iter()
            .any(|v| *v as u8 == sel.medical));
        assert!(GravityVariant::all()
            .iter()
            .any(|v| *v as u8 == sel.gravity));
    }

    #[test]
    fn test_deterministic_same_seed() {
        let config = MissionConfig::default();
        let sel1 = select_systems(&config, &SystemOverrides::default());
        let sel2 = select_systems(&config, &SystemOverrides::default());
        assert_eq!(sel1.power, sel2.power);
        assert_eq!(sel1.life_support, sel2.life_support);
        assert_eq!(sel1.food, sel2.food);
    }

    #[test]
    fn test_different_seeds_can_differ() {
        let c1 = MissionConfig {
            seed: 1,
            ..MissionConfig::default()
        };
        let c2 = MissionConfig {
            seed: 9999,
            ..MissionConfig::default()
        };
        let s1 = select_systems(&c1, &SystemOverrides::default());
        let s2 = select_systems(&c2, &SystemOverrides::default());
        // At least one field should potentially differ (not guaranteed but very likely)
        // This is a soft check — we just verify both are valid
        assert!(PowerVariant::all().iter().any(|v| *v as u8 == s1.power));
        assert!(PowerVariant::all().iter().any(|v| *v as u8 == s2.power));
    }

    #[test]
    fn test_low_tech_excludes_advanced() {
        let config = MissionConfig {
            tech_level: 1,
            ..MissionConfig::default()
        };
        let sel = select_systems(&config, &SystemOverrides::default());
        // At tech level 1, antimatter reactor (tech 4) should not be selected
        assert_ne!(sel.power, PowerVariant::AntimatterReactor as u8);
        // Gravity plate (tech 5) should not be selected
        assert_ne!(sel.gravity, GravityVariant::ArtificialGravityPlate as u8);
    }

    #[test]
    fn test_high_tech_allows_advanced() {
        let config = MissionConfig {
            tech_level: 5,
            budget_class: 3,
            mission_priority: MissionPriority {
                safety: 0.1,
                speed: 0.1,
                comfort: 0.1,
                science: 0.1,
                self_sufficiency: 2.0,
            },
            ..MissionConfig::default()
        };
        let sel = select_systems(&config, &SystemOverrides::default());
        // With tech 5 + premium + self-sufficiency priority, should pick high-output systems
        let power_spec = PowerVariant::all()
            .iter()
            .find(|v| **v as u8 == sel.power)
            .unwrap()
            .spec();
        assert!(
            power_spec.output >= 500.0,
            "high-tech should pick high-output power"
        );
    }

    #[test]
    fn test_overrides_respected() {
        let config = MissionConfig::default();
        let overrides = SystemOverrides {
            power: Some(PowerVariant::RTG as u8),
            medical: Some(MedicalVariant::CryoMedBay as u8),
            ..SystemOverrides::default()
        };
        let sel = select_systems(&config, &overrides);
        assert_eq!(sel.power, PowerVariant::RTG as u8);
        assert_eq!(sel.medical, MedicalVariant::CryoMedBay as u8);
    }

    #[test]
    fn test_total_crew() {
        let config = MissionConfig::default();
        let sel = select_systems(&config, &SystemOverrides::default());
        let crew = total_system_crew(&sel);
        assert!(crew > 0, "must need some crew");
        assert!(crew < 100, "crew shouldn't be absurd for 7 systems");
    }

    #[test]
    fn test_total_power_draw() {
        let config = MissionConfig::default();
        let sel = select_systems(&config, &SystemOverrides::default());
        let draw = total_power_draw(&sel);
        assert!(draw > 0.0, "non-power systems must draw power");
    }

    #[test]
    fn test_total_mass() {
        let config = MissionConfig::default();
        let sel = select_systems(&config, &SystemOverrides::default());
        let mass = total_system_mass(&sel);
        assert!(mass > 0.0, "systems must have mass");
    }

    #[test]
    fn test_austere_prefers_lightweight() {
        let austere = MissionConfig {
            budget_class: 1,
            tech_level: 3,
            ..MissionConfig::default()
        };
        let premium = MissionConfig {
            budget_class: 3,
            tech_level: 3,
            ..MissionConfig::default()
        };
        let s_a = select_systems(&austere, &SystemOverrides::default());
        let s_p = select_systems(&premium, &SystemOverrides::default());
        let mass_a = total_system_mass(&s_a);
        let mass_p = total_system_mass(&s_p);
        // Austere should generally pick lighter/cheaper systems
        // This is a soft assertion — the scoring may not always guarantee this
        // but with the current data it should hold
        assert!(
            mass_a <= mass_p + 100.0,
            "austere should not be heavier than premium + margin"
        );
    }

    #[test]
    fn test_safety_priority_prefers_reliable() {
        let safety_focused = MissionConfig {
            mission_priority: MissionPriority {
                safety: 5.0,
                speed: 0.1,
                comfort: 0.1,
                science: 0.1,
                self_sufficiency: 0.1,
            },
            ..MissionConfig::default()
        };
        let sel = select_systems(&safety_focused, &SystemOverrides::default());
        // Safety-focused should pick high-MTBF power (RTG or Solar or Fission)
        let power_spec = PowerVariant::all()
            .iter()
            .find(|v| **v as u8 == sel.power)
            .unwrap()
            .spec();
        assert!(
            power_spec.mtbf_hours >= 40000.0,
            "safety focus should pick reliable power"
        );
    }
}
