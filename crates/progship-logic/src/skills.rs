//! Skill checks, experience gain, and progression logic.
//!
//! Skills (engineering, medical, piloting, science, social, combat) are
//! set at generation but evolve over time through practice and training.
//!
//! # Skill Checks
//!
//! A skill check compares an agent's skill level against a difficulty
//! threshold and returns a [`SkillCheckResult`] with outcome quality.
//!
//! ```
//! use progship_logic::skills::{skill_check, SkillCheckResult};
//!
//! let result = skill_check(0.6, 0.5, Some(42));
//! assert!(matches!(result.outcome, progship_logic::skills::CheckOutcome::Success));
//! ```
//!
//! # Experience & Progression
//!
//! Performing activities grants experience that slowly raises the
//! corresponding skill. Training rooms provide a multiplier. Skills
//! decay when unused but never below their initial floor.

use serde::{Deserialize, Serialize};

/// All skill categories an agent can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillCategory {
    Engineering,
    Medical,
    Piloting,
    Science,
    Social,
    Combat,
}

impl SkillCategory {
    /// All skill categories in order.
    pub const ALL: [SkillCategory; 6] = [
        SkillCategory::Engineering,
        SkillCategory::Medical,
        SkillCategory::Piloting,
        SkillCategory::Science,
        SkillCategory::Social,
        SkillCategory::Combat,
    ];
}

/// Mutable skill profile for an agent, tracking current level and floor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProfile {
    /// Current skill levels (0.0–1.0).
    pub engineering: f32,
    pub medical: f32,
    pub piloting: f32,
    pub science: f32,
    pub social: f32,
    pub combat: f32,
    /// Initial (floor) values — skills never decay below these.
    pub floor_engineering: f32,
    pub floor_medical: f32,
    pub floor_piloting: f32,
    pub floor_science: f32,
    pub floor_social: f32,
    pub floor_combat: f32,
    /// Hours of practice accumulated per skill (for diminishing returns).
    pub hours_engineering: f32,
    pub hours_medical: f32,
    pub hours_piloting: f32,
    pub hours_science: f32,
    pub hours_social: f32,
    pub hours_combat: f32,
}

impl SkillProfile {
    /// Create a new profile with the given initial skills as both current and floor.
    pub fn new(
        engineering: f32,
        medical: f32,
        piloting: f32,
        science: f32,
        social: f32,
        combat: f32,
    ) -> Self {
        Self {
            engineering,
            medical,
            piloting,
            science,
            social,
            combat,
            floor_engineering: engineering,
            floor_medical: medical,
            floor_piloting: piloting,
            floor_science: science,
            floor_social: social,
            floor_combat: combat,
            hours_engineering: 0.0,
            hours_medical: 0.0,
            hours_piloting: 0.0,
            hours_science: 0.0,
            hours_social: 0.0,
            hours_combat: 0.0,
        }
    }

    /// Get the current level for a skill category.
    pub fn get(&self, cat: SkillCategory) -> f32 {
        match cat {
            SkillCategory::Engineering => self.engineering,
            SkillCategory::Medical => self.medical,
            SkillCategory::Piloting => self.piloting,
            SkillCategory::Science => self.science,
            SkillCategory::Social => self.social,
            SkillCategory::Combat => self.combat,
        }
    }

    /// Get the floor (minimum) value for a skill category.
    pub fn floor(&self, cat: SkillCategory) -> f32 {
        match cat {
            SkillCategory::Engineering => self.floor_engineering,
            SkillCategory::Medical => self.floor_medical,
            SkillCategory::Piloting => self.floor_piloting,
            SkillCategory::Science => self.floor_science,
            SkillCategory::Social => self.floor_social,
            SkillCategory::Combat => self.floor_combat,
        }
    }

    /// Get accumulated practice hours for a skill.
    pub fn hours(&self, cat: SkillCategory) -> f32 {
        match cat {
            SkillCategory::Engineering => self.hours_engineering,
            SkillCategory::Medical => self.hours_medical,
            SkillCategory::Piloting => self.hours_piloting,
            SkillCategory::Science => self.hours_science,
            SkillCategory::Social => self.hours_social,
            SkillCategory::Combat => self.hours_combat,
        }
    }

    /// Set a skill level (clamped to 0.0–1.0).
    fn set(&mut self, cat: SkillCategory, value: f32) {
        let v = value.clamp(0.0, 1.0);
        match cat {
            SkillCategory::Engineering => self.engineering = v,
            SkillCategory::Medical => self.medical = v,
            SkillCategory::Piloting => self.piloting = v,
            SkillCategory::Science => self.science = v,
            SkillCategory::Social => self.social = v,
            SkillCategory::Combat => self.combat = v,
        }
    }

    /// Add practice hours to a skill.
    fn add_hours(&mut self, cat: SkillCategory, hours: f32) {
        match cat {
            SkillCategory::Engineering => self.hours_engineering += hours,
            SkillCategory::Medical => self.hours_medical += hours,
            SkillCategory::Piloting => self.hours_piloting += hours,
            SkillCategory::Science => self.hours_science += hours,
            SkillCategory::Social => self.hours_social += hours,
            SkillCategory::Combat => self.hours_combat += hours,
        }
    }
}

/// Outcome of a skill check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckOutcome {
    /// Skill significantly exceeds difficulty — bonus effects.
    CriticalSuccess,
    /// Skill meets or exceeds difficulty.
    Success,
    /// Skill slightly below difficulty — partial or degraded result.
    PartialSuccess,
    /// Skill well below difficulty.
    Failure,
}

/// Result of a skill check including outcome quality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCheckResult {
    pub outcome: CheckOutcome,
    /// How far above/below the threshold (positive = above).
    pub margin: f32,
    /// Efficiency multiplier (0.5–1.5) based on skill vs difficulty.
    pub efficiency: f32,
}

/// Perform a skill check.
///
/// # Arguments
///
/// * `skill_level` — agent's skill (0.0–1.0)
/// * `difficulty` — task difficulty (0.0–1.0)
/// * `rng_seed` — optional seed for slight randomness (±0.05 noise)
///
/// The check adds up to ±0.05 random noise to the skill, then compares:
/// * margin ≥ 0.15: CriticalSuccess, efficiency 1.5
/// * margin ≥ 0.0: Success, efficiency 1.0 + margin
/// * margin ≥ −0.15: PartialSuccess, efficiency 0.5 + (margin + 0.15) * 2.0
/// * else: Failure, efficiency 0.5
pub fn skill_check(skill_level: f32, difficulty: f32, rng_seed: Option<u32>) -> SkillCheckResult {
    let noise = match rng_seed {
        Some(seed) => {
            // Simple hash for deterministic noise
            let hash = seed.wrapping_mul(2654435761);
            (hash % 101) as f32 / 1000.0 - 0.05 // -0.05 to +0.05
        }
        None => 0.0,
    };

    let effective_skill = (skill_level + noise).clamp(0.0, 1.0);
    let margin = effective_skill - difficulty;

    let (outcome, efficiency) = if margin >= 0.15 {
        (CheckOutcome::CriticalSuccess, 1.5)
    } else if margin >= 0.0 {
        (CheckOutcome::Success, 1.0 + margin)
    } else if margin >= -0.15 {
        (
            CheckOutcome::PartialSuccess,
            (0.5 + (margin + 0.15) * 2.0).max(0.5),
        )
    } else {
        (CheckOutcome::Failure, 0.5)
    };

    SkillCheckResult {
        outcome,
        margin,
        efficiency,
    }
}

/// Configuration for skill progression rates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProgressionConfig {
    /// Base XP gain per hour of practice.
    pub base_gain_per_hour: f32,
    /// Training room multiplier (e.g., 3.0 = 3x faster in a classroom).
    pub training_multiplier: f32,
    /// Skill level above which diminishing returns kick in.
    pub diminishing_threshold: f32,
    /// Decay rate per day of non-use (subtracted from skill).
    pub decay_per_day: f32,
    /// Maximum skill level achievable through practice.
    pub skill_cap: f32,
}

impl Default for SkillProgressionConfig {
    fn default() -> Self {
        Self {
            base_gain_per_hour: 0.001,
            training_multiplier: 3.0,
            diminishing_threshold: 0.8,
            decay_per_day: 0.0005,
            skill_cap: 1.0,
        }
    }
}

/// Apply experience gain from performing an activity for `hours`.
///
/// Returns the new skill level after gain.
///
/// Gain formula: `base_gain × hours × training_mult × diminishing_factor`
/// where diminishing_factor = 1.0 below threshold, linearly decreasing above.
pub fn apply_experience(
    profile: &mut SkillProfile,
    skill: SkillCategory,
    hours: f32,
    in_training_room: bool,
    config: &SkillProgressionConfig,
) -> f32 {
    let current = profile.get(skill);
    if current >= config.skill_cap {
        return current;
    }

    let training_mult = if in_training_room {
        config.training_multiplier
    } else {
        1.0
    };

    // Diminishing returns above threshold
    let diminishing = if current > config.diminishing_threshold {
        let excess = current - config.diminishing_threshold;
        let range = config.skill_cap - config.diminishing_threshold;
        if range > 0.0 {
            1.0 - (excess / range)
        } else {
            0.0
        }
    } else {
        1.0
    };

    let gain = config.base_gain_per_hour * hours * training_mult * diminishing.max(0.01);
    let new_level = (current + gain).min(config.skill_cap);

    profile.set(skill, new_level);
    profile.add_hours(skill, hours);

    new_level
}

/// Apply skill decay for time without practice.
///
/// Decays `days_without_use` worth of decay, but never below the
/// skill's floor (initial value at generation).
pub fn apply_decay(
    profile: &mut SkillProfile,
    skill: SkillCategory,
    days_without_use: f32,
    config: &SkillProgressionConfig,
) -> f32 {
    let current = profile.get(skill);
    let floor = profile.floor(skill);
    let decay = config.decay_per_day * days_without_use;
    let new_level = (current - decay).max(floor);
    profile.set(skill, new_level);
    new_level
}

/// Calculate repair speed multiplier based on engineering skill.
///
/// Returns a multiplier (0.5–2.0) applied to base repair time.
/// Unskilled (0.0) = 0.5x speed (takes twice as long).
/// Average (0.5) = 1.0x speed.
/// Expert (1.0) = 2.0x speed.
pub fn repair_speed_multiplier(engineering_skill: f32) -> f32 {
    0.5 + engineering_skill * 1.5
}

/// Calculate healing rate multiplier based on medical skill.
///
/// Returns a multiplier (0.5–2.0) applied to base healing time.
pub fn healing_rate_multiplier(medical_skill: f32) -> f32 {
    0.5 + medical_skill * 1.5
}

/// Task difficulty mapping for common ship activities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDifficulty {
    pub category: SkillCategory,
    pub difficulty: f32,
    pub description: &'static str,
}

/// Standard task difficulties for common ship operations.
pub fn standard_task_difficulties() -> Vec<TaskDifficulty> {
    vec![
        // Engineering
        TaskDifficulty {
            category: SkillCategory::Engineering,
            difficulty: 0.2,
            description: "Routine maintenance",
        },
        TaskDifficulty {
            category: SkillCategory::Engineering,
            difficulty: 0.4,
            description: "System repair",
        },
        TaskDifficulty {
            category: SkillCategory::Engineering,
            difficulty: 0.6,
            description: "Critical system repair",
        },
        TaskDifficulty {
            category: SkillCategory::Engineering,
            difficulty: 0.8,
            description: "Emergency reactor repair",
        },
        // Medical
        TaskDifficulty {
            category: SkillCategory::Medical,
            difficulty: 0.2,
            description: "First aid",
        },
        TaskDifficulty {
            category: SkillCategory::Medical,
            difficulty: 0.4,
            description: "Injury treatment",
        },
        TaskDifficulty {
            category: SkillCategory::Medical,
            difficulty: 0.7,
            description: "Surgery",
        },
        TaskDifficulty {
            category: SkillCategory::Medical,
            difficulty: 0.9,
            description: "Emergency trauma surgery",
        },
        // Piloting
        TaskDifficulty {
            category: SkillCategory::Piloting,
            difficulty: 0.3,
            description: "Course correction",
        },
        TaskDifficulty {
            category: SkillCategory::Piloting,
            difficulty: 0.6,
            description: "Asteroid avoidance",
        },
        TaskDifficulty {
            category: SkillCategory::Piloting,
            difficulty: 0.9,
            description: "Emergency maneuver",
        },
        // Science
        TaskDifficulty {
            category: SkillCategory::Science,
            difficulty: 0.3,
            description: "Routine analysis",
        },
        TaskDifficulty {
            category: SkillCategory::Science,
            difficulty: 0.6,
            description: "Complex research",
        },
        TaskDifficulty {
            category: SkillCategory::Science,
            difficulty: 0.85,
            description: "Breakthrough research",
        },
        // Social
        TaskDifficulty {
            category: SkillCategory::Social,
            difficulty: 0.2,
            description: "Casual conversation",
        },
        TaskDifficulty {
            category: SkillCategory::Social,
            difficulty: 0.5,
            description: "Conflict mediation",
        },
        TaskDifficulty {
            category: SkillCategory::Social,
            difficulty: 0.8,
            description: "Crisis negotiation",
        },
        // Combat
        TaskDifficulty {
            category: SkillCategory::Combat,
            difficulty: 0.3,
            description: "Security patrol",
        },
        TaskDifficulty {
            category: SkillCategory::Combat,
            difficulty: 0.6,
            description: "Detain suspect",
        },
        TaskDifficulty {
            category: SkillCategory::Combat,
            difficulty: 0.85,
            description: "Armed confrontation",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_profile() -> SkillProfile {
        SkillProfile::new(0.5, 0.3, 0.2, 0.4, 0.6, 0.1)
    }

    #[test]
    fn profile_getters() {
        let p = test_profile();
        assert!((p.get(SkillCategory::Engineering) - 0.5).abs() < f32::EPSILON);
        assert!((p.get(SkillCategory::Medical) - 0.3).abs() < f32::EPSILON);
        assert!((p.floor(SkillCategory::Engineering) - 0.5).abs() < f32::EPSILON);
        assert!((p.hours(SkillCategory::Engineering)).abs() < f32::EPSILON);
    }

    #[test]
    fn skill_check_success() {
        let result = skill_check(0.7, 0.5, None);
        assert_eq!(result.outcome, CheckOutcome::CriticalSuccess);
        assert!(result.margin > 0.15);
        assert!((result.efficiency - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn skill_check_marginal_success() {
        let result = skill_check(0.5, 0.5, None);
        assert_eq!(result.outcome, CheckOutcome::Success);
        assert!(result.margin.abs() < f32::EPSILON);
    }

    #[test]
    fn skill_check_partial() {
        let result = skill_check(0.4, 0.5, None);
        assert_eq!(result.outcome, CheckOutcome::PartialSuccess);
        assert!(result.margin < 0.0);
        assert!(result.efficiency >= 0.5);
        assert!(result.efficiency < 1.0);
    }

    #[test]
    fn skill_check_failure() {
        let result = skill_check(0.1, 0.5, None);
        assert_eq!(result.outcome, CheckOutcome::Failure);
        assert!(result.margin < -0.15);
        assert!((result.efficiency - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn skill_check_deterministic_with_seed() {
        let r1 = skill_check(0.5, 0.5, Some(42));
        let r2 = skill_check(0.5, 0.5, Some(42));
        assert!((r1.margin - r2.margin).abs() < f32::EPSILON);
    }

    #[test]
    fn skill_check_noise_bounded() {
        // With any seed, noise should be ±0.05
        for seed in 0..100 {
            let result = skill_check(0.5, 0.5, Some(seed));
            assert!(result.margin >= -0.05 - f32::EPSILON);
            assert!(result.margin <= 0.05 + f32::EPSILON);
        }
    }

    #[test]
    fn experience_gain_basic() {
        let mut p = test_profile();
        let config = SkillProgressionConfig::default();
        let old = p.get(SkillCategory::Engineering);
        let new = apply_experience(&mut p, SkillCategory::Engineering, 10.0, false, &config);
        assert!(new > old);
        assert!((new - old - 0.01).abs() < 0.001); // 0.001 * 10 hours = 0.01
        assert!((p.hours(SkillCategory::Engineering) - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn training_room_multiplier() {
        let config = SkillProgressionConfig::default();
        let mut p1 = test_profile();
        let mut p2 = test_profile();
        let gain_normal =
            apply_experience(&mut p1, SkillCategory::Engineering, 10.0, false, &config) - 0.5;
        let gain_training =
            apply_experience(&mut p2, SkillCategory::Engineering, 10.0, true, &config) - 0.5;
        assert!((gain_training / gain_normal - 3.0).abs() < 0.01);
    }

    #[test]
    fn diminishing_returns_above_threshold() {
        let config = SkillProgressionConfig::default();
        let mut p = SkillProfile::new(0.9, 0.0, 0.0, 0.0, 0.0, 0.0);
        // 0.9 is above 0.8 threshold — gain should be reduced
        let gain_high =
            apply_experience(&mut p, SkillCategory::Engineering, 10.0, false, &config) - 0.9;

        let mut p2 = SkillProfile::new(0.5, 0.0, 0.0, 0.0, 0.0, 0.0);
        let gain_low =
            apply_experience(&mut p2, SkillCategory::Engineering, 10.0, false, &config) - 0.5;

        assert!(gain_high < gain_low);
    }

    #[test]
    fn skill_never_exceeds_cap() {
        let config = SkillProgressionConfig::default();
        let mut p = SkillProfile::new(0.99, 0.0, 0.0, 0.0, 0.0, 0.0);
        let result = apply_experience(&mut p, SkillCategory::Engineering, 1000.0, true, &config);
        assert!(result <= 1.0);
    }

    #[test]
    fn decay_reduces_skill() {
        let config = SkillProgressionConfig::default();
        // First gain some skill above the floor
        let mut p = SkillProfile::new(0.3, 0.0, 0.0, 0.0, 0.0, 0.0);
        apply_experience(&mut p, SkillCategory::Engineering, 100.0, false, &config);
        let old = p.get(SkillCategory::Engineering);
        assert!(old > 0.3, "should have gained skill");
        let new = apply_decay(&mut p, SkillCategory::Engineering, 10.0, &config);
        assert!(new < old);
    }

    #[test]
    fn decay_never_below_floor() {
        let config = SkillProgressionConfig::default();
        let mut p = test_profile();
        let floor = p.floor(SkillCategory::Combat);
        let result = apply_decay(&mut p, SkillCategory::Combat, 10000.0, &config);
        assert!((result - floor).abs() < f32::EPSILON);
    }

    #[test]
    fn repair_speed_scaling() {
        assert!((repair_speed_multiplier(0.0) - 0.5).abs() < f32::EPSILON);
        assert!((repair_speed_multiplier(0.5) - 1.25).abs() < f32::EPSILON);
        assert!((repair_speed_multiplier(1.0) - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn healing_rate_scaling() {
        assert!((healing_rate_multiplier(0.0) - 0.5).abs() < f32::EPSILON);
        assert!((healing_rate_multiplier(1.0) - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn hundred_hours_measurable_improvement() {
        // Acceptance: 100 hours of engineering work = measurably higher skill
        let config = SkillProgressionConfig::default();
        let mut p = SkillProfile::new(0.3, 0.0, 0.0, 0.0, 0.0, 0.0);
        let old = p.get(SkillCategory::Engineering);
        apply_experience(&mut p, SkillCategory::Engineering, 100.0, false, &config);
        let new = p.get(SkillCategory::Engineering);
        assert!(
            new - old >= 0.05,
            "100 hours should yield ≥0.05 skill gain, got {}",
            new - old
        );
    }

    #[test]
    fn skilled_engineer_repairs_faster() {
        // Acceptance: skilled engineer repairs faster than unskilled
        let slow = repair_speed_multiplier(0.2);
        let fast = repair_speed_multiplier(0.8);
        assert!(fast > slow);
        assert!(fast / slow > 1.5, "skilled should be 50%+ faster");
    }

    #[test]
    fn standard_tasks_valid() {
        let tasks = standard_task_difficulties();
        assert!(tasks.len() >= 15);
        for task in &tasks {
            assert!(task.difficulty >= 0.0 && task.difficulty <= 1.0);
            assert!(!task.description.is_empty());
        }
    }

    #[test]
    fn all_categories_enum() {
        assert_eq!(SkillCategory::ALL.len(), 6);
    }
}
