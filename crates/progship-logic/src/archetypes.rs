//! Behavioral archetypes — personality-derived behavioral tendencies.
//!
//! Archetypes provide gameplay-visible modifiers derived from Big Five personality
//! traits. Each NPC gets exactly one archetype at generation based on their dominant
//! trait combination. Archetypes affect duty compliance, social behavior, and
//! activity preferences through numerical modifiers that feed into the utility AI.

use serde::{Deserialize, Serialize};

/// Behavioral archetype assigned at NPC generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Archetype {
    /// High conscientiousness + high agreeableness.
    /// +15% duty compliance, follows protocols strictly.
    Compliant,
    /// Low conscientiousness + high openness.
    /// -15% duty compliance, +10% creative problem-solving.
    Independent,
    /// High extraversion + high agreeableness.
    /// +20% conversation initiation rate, seeks crowded rooms.
    SocialButterfly,
    /// Low extraversion + low agreeableness.
    /// Slower social need decay, prefers empty rooms.
    Loner,
    /// High neuroticism + low openness.
    /// Faster comfort decay, seeks familiar/safe rooms.
    Anxious,
    /// Low neuroticism + high openness.
    /// Slower comfort decay, tolerates emergencies better.
    Stoic,
    /// No dominant trait pattern — balanced personality.
    Balanced,
}

/// Numerical modifiers applied by an archetype to simulation behavior.
#[derive(Debug, Clone, Copy)]
pub struct ArchetypeModifiers {
    /// Multiplier on duty compliance score (1.0 = neutral).
    pub duty_compliance: f32,
    /// Multiplier on social activity initiation (1.0 = neutral).
    pub social_initiation: f32,
    /// Multiplier on social need decay rate (>1.0 = decays faster, needs social more).
    pub social_decay_rate: f32,
    /// Additive bonus to comfort in empty/quiet rooms.
    pub solitude_comfort: f32,
    /// Additive bonus to comfort in crowded/social rooms.
    pub crowd_comfort: f32,
    /// Multiplier on comfort decay rate (>1.0 = gets uncomfortable faster).
    pub comfort_decay_rate: f32,
    /// Multiplier on morale loss during emergencies (<1.0 = more resilient).
    pub emergency_resilience: f32,
}

impl Archetype {
    /// Get the gameplay modifiers for this archetype.
    pub fn modifiers(self) -> ArchetypeModifiers {
        match self {
            Archetype::Compliant => ArchetypeModifiers {
                duty_compliance: 1.15,
                social_initiation: 1.0,
                social_decay_rate: 1.0,
                solitude_comfort: 0.0,
                crowd_comfort: 0.0,
                comfort_decay_rate: 1.0,
                emergency_resilience: 1.0,
            },
            Archetype::Independent => ArchetypeModifiers {
                duty_compliance: 0.85,
                social_initiation: 1.0,
                social_decay_rate: 1.0,
                solitude_comfort: 0.05,
                crowd_comfort: -0.05,
                comfort_decay_rate: 1.0,
                emergency_resilience: 0.9,
            },
            Archetype::SocialButterfly => ArchetypeModifiers {
                duty_compliance: 1.0,
                social_initiation: 1.20,
                social_decay_rate: 1.2,
                solitude_comfort: -0.1,
                crowd_comfort: 0.1,
                comfort_decay_rate: 1.0,
                emergency_resilience: 1.0,
            },
            Archetype::Loner => ArchetypeModifiers {
                duty_compliance: 1.0,
                social_initiation: 0.7,
                social_decay_rate: 0.7,
                solitude_comfort: 0.15,
                crowd_comfort: -0.15,
                comfort_decay_rate: 1.0,
                emergency_resilience: 1.0,
            },
            Archetype::Anxious => ArchetypeModifiers {
                duty_compliance: 1.05,
                social_initiation: 0.9,
                social_decay_rate: 1.0,
                solitude_comfort: 0.05,
                crowd_comfort: -0.05,
                comfort_decay_rate: 1.3,
                emergency_resilience: 1.3,
            },
            Archetype::Stoic => ArchetypeModifiers {
                duty_compliance: 1.0,
                social_initiation: 0.9,
                social_decay_rate: 1.0,
                solitude_comfort: 0.0,
                crowd_comfort: 0.0,
                comfort_decay_rate: 0.8,
                emergency_resilience: 0.7,
            },
            Archetype::Balanced => ArchetypeModifiers {
                duty_compliance: 1.0,
                social_initiation: 1.0,
                social_decay_rate: 1.0,
                solitude_comfort: 0.0,
                crowd_comfort: 0.0,
                comfort_decay_rate: 1.0,
                emergency_resilience: 1.0,
            },
        }
    }

    /// All archetype variants for iteration.
    pub fn all() -> &'static [Archetype] {
        &[
            Archetype::Compliant,
            Archetype::Independent,
            Archetype::SocialButterfly,
            Archetype::Loner,
            Archetype::Anxious,
            Archetype::Stoic,
            Archetype::Balanced,
        ]
    }
}

/// Determine the archetype for an NPC based on Big Five personality traits.
///
/// Each trait is expected in \[0.0, 1.0\]. The archetype is chosen by finding
/// the strongest trait pattern. If no pattern is dominant (all traits near 0.5),
/// returns `Balanced`.
pub fn assign_archetype(
    extraversion: f32,
    agreeableness: f32,
    conscientiousness: f32,
    neuroticism: f32,
    openness: f32,
) -> Archetype {
    // Thresholds for "high" and "low" trait values
    const HIGH: f32 = 0.65;
    const LOW: f32 = 0.35;

    // Score each archetype by how well the traits match its pattern.
    // Higher score = better fit. Pick the best.
    let mut best = Archetype::Balanced;
    let mut best_score: f32 = 0.0;

    // Compliant: high C + high A
    let s = trait_score(conscientiousness, HIGH) + trait_score(agreeableness, HIGH);
    if s > best_score {
        best_score = s;
        best = Archetype::Compliant;
    }

    // Independent: low C + high O
    let s = trait_score_low(conscientiousness, LOW) + trait_score(openness, HIGH);
    if s > best_score {
        best_score = s;
        best = Archetype::Independent;
    }

    // SocialButterfly: high E + high A
    let s = trait_score(extraversion, HIGH) + trait_score(agreeableness, HIGH);
    if s > best_score {
        best_score = s;
        best = Archetype::SocialButterfly;
    }

    // Loner: low E + low A
    let s = trait_score_low(extraversion, LOW) + trait_score_low(agreeableness, LOW);
    if s > best_score {
        best_score = s;
        best = Archetype::Loner;
    }

    // Anxious: high N + low O
    let s = trait_score(neuroticism, HIGH) + trait_score_low(openness, LOW);
    if s > best_score {
        best_score = s;
        best = Archetype::Anxious;
    }

    // Stoic: low N + high O
    let s = trait_score_low(neuroticism, LOW) + trait_score(openness, HIGH);
    if s > best_score {
        best_score = s;
        best = Archetype::Stoic;
    }

    // Only assign non-Balanced if score is meaningfully above zero
    if best_score < 0.3 {
        return Archetype::Balanced;
    }

    best
}

/// How much a trait exceeds the "high" threshold (0 if below).
fn trait_score(value: f32, threshold: f32) -> f32 {
    (value - threshold).max(0.0)
}

/// How much a trait is below the "low" threshold (0 if above).
fn trait_score_low(value: f32, threshold: f32) -> f32 {
    (threshold - value).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliant_high_c_high_a() {
        let a = assign_archetype(0.5, 0.9, 0.9, 0.5, 0.5);
        assert_eq!(a, Archetype::Compliant);
    }

    #[test]
    fn test_independent_low_c_high_o() {
        let a = assign_archetype(0.5, 0.5, 0.1, 0.5, 0.9);
        assert_eq!(a, Archetype::Independent);
    }

    #[test]
    fn test_social_butterfly_high_e_high_a() {
        let a = assign_archetype(0.9, 0.9, 0.5, 0.5, 0.5);
        assert_eq!(a, Archetype::SocialButterfly);
    }

    #[test]
    fn test_loner_low_e_low_a() {
        let a = assign_archetype(0.1, 0.1, 0.5, 0.5, 0.5);
        assert_eq!(a, Archetype::Loner);
    }

    #[test]
    fn test_anxious_high_n_low_o() {
        let a = assign_archetype(0.5, 0.5, 0.5, 0.9, 0.1);
        assert_eq!(a, Archetype::Anxious);
    }

    #[test]
    fn test_stoic_low_n_high_o() {
        let a = assign_archetype(0.5, 0.5, 0.5, 0.1, 0.9);
        assert_eq!(a, Archetype::Stoic);
    }

    #[test]
    fn test_balanced_middle_traits() {
        let a = assign_archetype(0.5, 0.5, 0.5, 0.5, 0.5);
        assert_eq!(a, Archetype::Balanced);
    }

    #[test]
    fn test_all_archetypes_count() {
        assert_eq!(Archetype::all().len(), 7);
    }

    #[test]
    fn test_compliant_modifiers() {
        let m = Archetype::Compliant.modifiers();
        assert!(m.duty_compliance > 1.0, "compliant should boost duty");
    }

    #[test]
    fn test_loner_social_decay() {
        let m = Archetype::Loner.modifiers();
        assert!(
            m.social_decay_rate < 1.0,
            "loners need social interaction less often"
        );
        assert!(
            m.solitude_comfort > 0.0,
            "loners should get comfort from solitude"
        );
    }

    #[test]
    fn test_social_butterfly_initiation() {
        let m = Archetype::SocialButterfly.modifiers();
        assert!(m.social_initiation > 1.0);
        assert!(m.crowd_comfort > 0.0);
    }

    #[test]
    fn test_stoic_emergency_resilience() {
        let m = Archetype::Stoic.modifiers();
        assert!(
            m.emergency_resilience < 1.0,
            "stoics should lose less morale in emergencies"
        );
    }

    #[test]
    fn test_balanced_is_neutral() {
        let m = Archetype::Balanced.modifiers();
        assert!((m.duty_compliance - 1.0).abs() < f32::EPSILON);
        assert!((m.social_initiation - 1.0).abs() < f32::EPSILON);
        assert!((m.social_decay_rate - 1.0).abs() < f32::EPSILON);
        assert!((m.comfort_decay_rate - 1.0).abs() < f32::EPSILON);
        assert!((m.emergency_resilience - 1.0).abs() < f32::EPSILON);
    }
}
