//! Pure action logic — room type validation and needs effects.

use crate::tables::{activity_types, room_types};

/// The needs deltas and resulting activity from performing an action.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionEffect {
    pub activity_type: u8,
    pub duration: f32,
    pub hunger_delta: f32,
    pub fatigue_delta: f32,
    pub social_delta: f32,
    pub comfort_delta: f32,
    pub hygiene_delta: f32,
    pub morale_delta: f32,
    pub health_delta: f32,
}

/// Validate and compute the effect of a player action in a given room type.
///
/// Returns `None` if the action is invalid for the room type.
pub fn compute_action_effect(action: u8, room_type: u8) -> Option<ActionEffect> {
    match action {
        // Eat (must be in mess/galley/cafe/bakery)
        2 if room_type == room_types::MESS_HALL
            || room_type == room_types::GALLEY
            || room_type == room_types::CAFE
            || room_type == room_types::BAKERY =>
        {
            Some(ActionEffect {
                activity_type: activity_types::EATING,
                duration: 0.5,
                hunger_delta: -0.3,
                fatigue_delta: 0.0,
                social_delta: 0.0,
                comfort_delta: -0.05,
                hygiene_delta: 0.0,
                morale_delta: 0.0,
                health_delta: 0.0,
            })
        }
        // Sleep (must be in quarters)
        3 if room_types::is_quarters(room_type) => Some(ActionEffect {
            activity_type: activity_types::SLEEPING,
            duration: 2.0,
            hunger_delta: 0.0,
            fatigue_delta: -0.4,
            social_delta: 0.0,
            comfort_delta: -0.1,
            hygiene_delta: 0.0,
            morale_delta: 0.0,
            health_delta: 0.0,
        }),
        // Exercise (must be in gym/recreation)
        12 if room_type == room_types::GYM || room_types::is_recreation(room_type) => {
            Some(ActionEffect {
                activity_type: activity_types::EXERCISING,
                duration: 0.5,
                hunger_delta: 0.0,
                fatigue_delta: 0.1,
                social_delta: 0.0,
                comfort_delta: -0.15,
                hygiene_delta: 0.0,
                morale_delta: 0.05,
                health_delta: 0.0,
            })
        }
        // Hygiene (must be in quarters or shared bathroom)
        6 if room_types::is_quarters(room_type) || room_type == room_types::SHARED_BATHROOM => {
            Some(ActionEffect {
                activity_type: activity_types::HYGIENE,
                duration: 0.2,
                hunger_delta: 0.0,
                fatigue_delta: 0.0,
                social_delta: 0.0,
                comfort_delta: 0.0,
                hygiene_delta: -0.5,
                morale_delta: 0.0,
                health_delta: 0.0,
            })
        }
        _ => None,
    }
}

/// Snapshot of needs values for pure computation.
#[derive(Debug, Clone, Copy)]
pub struct NeedsValues {
    pub hunger: f32,
    pub fatigue: f32,
    pub social: f32,
    pub comfort: f32,
    pub hygiene: f32,
    pub morale: f32,
    pub health: f32,
}

/// Apply an action effect to needs values, clamping to [0.0, 1.0].
pub fn apply_needs_deltas(needs: &NeedsValues, effect: &ActionEffect) -> NeedsValues {
    NeedsValues {
        hunger: (needs.hunger + effect.hunger_delta).clamp(0.0, 1.0),
        fatigue: (needs.fatigue + effect.fatigue_delta).clamp(0.0, 1.0),
        social: (needs.social + effect.social_delta).clamp(0.0, 1.0),
        comfort: (needs.comfort + effect.comfort_delta).clamp(0.0, 1.0),
        hygiene: (needs.hygiene + effect.hygiene_delta).clamp(0.0, 1.0),
        morale: (needs.morale + effect.morale_delta).clamp(0.0, 1.0),
        health: (needs.health + effect.health_delta).clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eat_in_mess_hall() {
        let effect = compute_action_effect(2, room_types::MESS_HALL);
        assert!(effect.is_some());
        let e = effect.unwrap();
        assert_eq!(e.activity_type, activity_types::EATING);
        assert!(e.hunger_delta < 0.0); // reduces hunger
    }

    #[test]
    fn test_eat_in_cafe() {
        assert!(compute_action_effect(2, room_types::CAFE).is_some());
    }

    #[test]
    fn test_eat_in_wrong_room() {
        assert!(compute_action_effect(2, room_types::BRIDGE).is_none());
        assert!(compute_action_effect(2, room_types::GYM).is_none());
    }

    #[test]
    fn test_sleep_in_quarters() {
        let effect = compute_action_effect(3, room_types::CABIN_SINGLE);
        assert!(effect.is_some());
        let e = effect.unwrap();
        assert_eq!(e.activity_type, activity_types::SLEEPING);
        assert!(e.fatigue_delta < 0.0); // reduces fatigue
    }

    #[test]
    fn test_sleep_in_wrong_room() {
        assert!(compute_action_effect(3, room_types::BRIDGE).is_none());
    }

    #[test]
    fn test_exercise_in_gym() {
        let effect = compute_action_effect(12, room_types::GYM);
        assert!(effect.is_some());
        let e = effect.unwrap();
        assert_eq!(e.activity_type, activity_types::EXERCISING);
        assert!(e.fatigue_delta > 0.0); // increases fatigue
        assert!(e.morale_delta > 0.0); // boosts morale
    }

    #[test]
    fn test_hygiene_in_bathroom() {
        let effect = compute_action_effect(6, room_types::SHARED_BATHROOM);
        assert!(effect.is_some());
        let e = effect.unwrap();
        assert_eq!(e.activity_type, activity_types::HYGIENE);
        assert!(e.hygiene_delta < 0.0); // reduces hygiene need
    }

    #[test]
    fn test_invalid_action() {
        assert!(compute_action_effect(99, room_types::BRIDGE).is_none());
    }

    #[test]
    fn test_apply_needs_clamps() {
        let effect = ActionEffect {
            activity_type: 0,
            duration: 0.0,
            hunger_delta: -1.0,
            fatigue_delta: 1.0,
            social_delta: 0.0,
            comfort_delta: 0.0,
            hygiene_delta: 0.0,
            morale_delta: 0.0,
            health_delta: 0.0,
        };
        let needs = NeedsValues {
            hunger: 0.2,
            fatigue: 0.9,
            social: 0.5,
            comfort: 0.5,
            hygiene: 0.5,
            morale: 0.5,
            health: 1.0,
        };
        let result = apply_needs_deltas(&needs, &effect);
        assert!((result.hunger - 0.0).abs() < 0.001); // clamped to 0
        assert!((result.fatigue - 1.0).abs() < 0.001); // clamped to 1
    }
}
