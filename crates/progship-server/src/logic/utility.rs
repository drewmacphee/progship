//! Pure utility AI logic — personality-driven activity selection with environmental factors.
//!
//! Replaces the hard-coded if/else cascade with a scored utility system where
//! each candidate activity gets a weighted score based on needs, personality,
//! time of day, and room environment.

/// Environmental context for a room.
#[derive(Debug, Clone, Default)]
pub struct RoomContext {
    pub room_type: u8,
    pub occupants: u32,
    pub capacity: u32,
}

/// All inputs needed to score activities — pure data, no DB access.
#[derive(Debug, Clone)]
pub struct UtilityInput {
    pub hunger: f32,
    pub fatigue: f32,
    pub social: f32,
    pub comfort: f32,
    pub hygiene: f32,
    pub health: f32,
    pub morale: f32,
    pub hour: f32,
    pub is_crew: bool,
    pub shift: Option<u8>,
    pub department: Option<u8>,
    pub extraversion: f32,
    pub neuroticism: f32,
    pub conscientiousness: f32,
    pub openness: f32,
    pub agreeableness: f32,
    pub current_room: Option<RoomContext>,
    pub fit_for_duty: bool,
    pub should_be_on_duty: bool,
}

/// A scored activity candidate.
#[derive(Debug, Clone)]
pub struct ScoredActivity {
    pub activity_type: u8,
    pub score: f32,
    pub duration: f32,
    pub room_type_hint: RoomTarget,
}

/// What kind of room the activity needs.
#[derive(Debug, Clone)]
pub enum RoomTarget {
    /// No specific room needed.
    None,
    /// A specific room type constant.
    Exact(u8),
    /// Any room matching a predicate category.
    Category(RoomCategory),
    /// Department duty station.
    DutyStation(u8),
}

/// Room category for predicate-based room finding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoomCategory {
    Quarters,
    Recreation,
    Medical,
    Dining,
}

use crate::tables::{activity_types, room_types};

/// Compute the overcrowding stress factor for a room.
/// Returns 0.0 (empty) to 1.0+ (severely overcrowded).
pub fn overcrowding_factor(occupants: u32, capacity: u32) -> f32 {
    if capacity == 0 {
        return 1.0;
    }
    let ratio = occupants as f32 / capacity as f32;
    if ratio <= 0.7 {
        0.0 // Comfortable
    } else if ratio <= 1.0 {
        (ratio - 0.7) / 0.3 // Linear ramp 0..1
    } else {
        1.0 + (ratio - 1.0) // Beyond capacity
    }
}

/// Noise level of a room type (0.0 = silent, 1.0 = very loud).
pub fn noise_level(room_type: u8) -> f32 {
    if room_types::is_corridor(room_type) {
        return 0.3;
    }
    match room_type {
        // Loud
        room_types::ENGINEERING
        | room_types::REACTOR
        | room_types::ENGINE_ROOM
        | room_types::MACHINE_SHOP => 0.9,
        room_types::ATMOSPHERE_PROCESSING
        | room_types::WASTE_PROCESSING
        | room_types::COOLING_PLANT => 0.7,
        // Moderate
        room_types::MESS_HALL | room_types::BAR | room_types::GYM | room_types::POOL => 0.5,
        room_types::THEATRE | room_types::GAME_ROOM | room_types::MUSIC_ROOM => 0.5,
        // Quiet
        room_types::LIBRARY | room_types::CHAPEL | room_types::OBSERVATION_LOUNGE => 0.1,
        room_types::ARBORETUM | room_types::HOLODECK => 0.2,
        // Quarters
        _ if room_types::is_quarters(room_type) => 0.1,
        // Medical
        _ if room_types::is_medical(room_type) => 0.2,
        // Default
        _ => 0.3,
    }
}

/// Comfort bonus of a room type (0.0 = uncomfortable, 1.0 = very comfortable).
pub fn room_comfort(room_type: u8) -> f32 {
    match room_type {
        room_types::VIP_SUITE | room_types::OBSERVATION_LOUNGE => 1.0,
        room_types::FAMILY_SUITE | room_types::QUARTERS_OFFICER | room_types::ARBORETUM => 0.9,
        room_types::LOUNGE | room_types::LIBRARY | room_types::CHAPEL => 0.8,
        room_types::HOLODECK | room_types::THEATRE | room_types::BAR => 0.7,
        _ if room_types::is_quarters(room_type) => 0.6,
        _ if room_types::is_recreation(room_type) => 0.6,
        _ if room_types::is_medical(room_type) => 0.3,
        room_types::MESS_HALL | room_types::WARDROOM | room_types::CAFE => 0.5,
        _ if room_types::is_corridor(room_type) => 0.1,
        _ => 0.3,
    }
}

/// Score all candidate activities and return them sorted best-first.
pub fn score_activities(input: &UtilityInput) -> Vec<ScoredActivity> {
    let mut candidates = Vec::with_capacity(10);

    // Environmental stress from current room
    let crowd_stress = input
        .current_room
        .as_ref()
        .map(|r| overcrowding_factor(r.occupants, r.capacity))
        .unwrap_or(0.0);
    let room_noise = input
        .current_room
        .as_ref()
        .map(|r| noise_level(r.room_type))
        .unwrap_or(0.0);

    // --- Medical urgency (overrides everything) ---
    if crate::logic::health::should_seek_medical(input.health) {
        candidates.push(ScoredActivity {
            activity_type: activity_types::IDLE,
            score: 100.0,
            duration: 1.0,
            room_type_hint: RoomTarget::Category(RoomCategory::Medical),
        });
        return candidates;
    }

    // --- Duty ---
    if input.should_be_on_duty && input.fit_for_duty {
        // Conscientiousness increases duty motivation
        let duty_score = 8.0 + input.conscientiousness * 3.0;
        let dept = input.department.unwrap_or(0);
        candidates.push(ScoredActivity {
            activity_type: activity_types::ON_DUTY,
            score: duty_score,
            duration: 2.0,
            room_type_hint: RoomTarget::DutyStation(dept),
        });
    }

    // --- Sleep ---
    {
        // Exponential urgency curve for fatigue
        let fatigue_urgency = input.fatigue * input.fatigue * 15.0;
        // Boost during sleep window
        let schedule_bonus = if input.is_crew {
            if let Some(shift) = input.shift {
                if crate::logic::duty::is_crew_sleep_time(shift, input.hour) {
                    3.0
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else if crate::logic::duty::is_passenger_sleep_time(input.hour) {
            3.0
        } else {
            0.0
        };
        let sleep_score = fatigue_urgency + schedule_bonus;
        candidates.push(ScoredActivity {
            activity_type: activity_types::SLEEPING,
            score: sleep_score,
            duration: 8.0,
            room_type_hint: RoomTarget::Category(RoomCategory::Quarters),
        });
    }

    // --- Eating ---
    {
        let hunger_urgency = input.hunger * input.hunger * 12.0;
        let meal_bonus = if is_meal_time(input.hour) { 2.0 } else { 0.0 };
        let eat_score = hunger_urgency + meal_bonus;
        candidates.push(ScoredActivity {
            activity_type: activity_types::EATING,
            score: eat_score,
            duration: 0.5,
            room_type_hint: RoomTarget::Exact(room_types::MESS_HALL),
        });
    }

    // --- Hygiene ---
    {
        let hygiene_urgency = input.hygiene * input.hygiene * 10.0;
        candidates.push(ScoredActivity {
            activity_type: activity_types::HYGIENE,
            score: hygiene_urgency,
            duration: 0.3,
            room_type_hint: RoomTarget::Exact(room_types::SHARED_BATHROOM),
        });
    }

    // --- Socializing ---
    {
        let social_urgency = input.social * input.social * 8.0;
        // Extraverts get a strong bonus, introverts a penalty
        let personality_mod = (input.extraversion - 0.5) * 4.0;
        // Overcrowding penalty — introverts hate crowds more
        let crowd_penalty = crowd_stress * (1.0 + (1.0 - input.extraversion));
        let social_score = (social_urgency + personality_mod - crowd_penalty).max(0.0);
        candidates.push(ScoredActivity {
            activity_type: activity_types::SOCIALIZING,
            score: social_score,
            duration: 1.0,
            room_type_hint: RoomTarget::Category(RoomCategory::Recreation),
        });
    }

    // --- Relaxing ---
    {
        let comfort_urgency = input.comfort * input.comfort * 6.0;
        // Noise stress: neurotic people are more affected
        let noise_stress = room_noise * input.neuroticism * 3.0;
        // Overcrowding stress
        let crowd_comfort_penalty = crowd_stress * 2.0;
        let relax_score = comfort_urgency + noise_stress + crowd_comfort_penalty;
        candidates.push(ScoredActivity {
            activity_type: activity_types::RELAXING,
            score: relax_score,
            duration: 1.0,
            room_type_hint: RoomTarget::Category(RoomCategory::Recreation),
        });
    }

    // --- Exercising ---
    {
        // Open/conscientious people exercise more; fatigue dampens desire
        let exercise_base = 1.5 + input.openness * 1.5 + input.conscientiousness;
        let fatigue_dampen = input.fatigue * 3.0;
        let exercise_score = (exercise_base - fatigue_dampen).max(0.0);
        candidates.push(ScoredActivity {
            activity_type: activity_types::EXERCISING,
            score: exercise_score,
            duration: 1.0,
            room_type_hint: RoomTarget::Exact(room_types::GYM),
        });
    }

    // Sort descending by score
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates
}

/// Pick the best activity from scored candidates.
/// Returns (activity_type, duration, room_target).
pub fn pick_best(input: &UtilityInput) -> (u8, f32, RoomTarget) {
    let scored = score_activities(input);
    if let Some(best) = scored.into_iter().next() {
        (best.activity_type, best.duration, best.room_type_hint)
    } else {
        (activity_types::IDLE, 0.02, RoomTarget::None)
    }
}

fn is_meal_time(hour: f32) -> bool {
    (7.0..8.0).contains(&hour) || (12.0..13.0).contains(&hour) || (18.0..19.0).contains(&hour)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_input() -> UtilityInput {
        UtilityInput {
            hunger: 0.3,
            fatigue: 0.3,
            social: 0.3,
            comfort: 0.3,
            hygiene: 0.3,
            health: 0.9,
            morale: 0.7,
            hour: 10.0,
            is_crew: false,
            shift: None,
            department: None,
            extraversion: 0.5,
            neuroticism: 0.5,
            conscientiousness: 0.5,
            openness: 0.5,
            agreeableness: 0.5,
            current_room: None,
            fit_for_duty: false,
            should_be_on_duty: false,
        }
    }

    #[test]
    fn test_overcrowding_factor() {
        assert!((overcrowding_factor(0, 10) - 0.0).abs() < 0.01);
        assert!((overcrowding_factor(5, 10) - 0.0).abs() < 0.01); // 50%
        assert!((overcrowding_factor(7, 10) - 0.0).abs() < 0.01); // 70%
        assert!(overcrowding_factor(8, 10) > 0.0); // 80%
        assert!((overcrowding_factor(10, 10) - 1.0).abs() < 0.01); // 100%
        assert!(overcrowding_factor(15, 10) > 1.0); // 150%
        assert!((overcrowding_factor(0, 0) - 1.0).abs() < 0.01); // Zero capacity
    }

    #[test]
    fn test_noise_levels() {
        assert!(noise_level(room_types::ENGINEERING) > 0.8);
        assert!(noise_level(room_types::LIBRARY) < 0.2);
        assert!(noise_level(room_types::MESS_HALL) > 0.3);
        assert!(noise_level(room_types::CABIN_SINGLE) < 0.2);
        assert!(noise_level(room_types::CORRIDOR) > 0.2);
    }

    #[test]
    fn test_room_comfort() {
        assert!(room_comfort(room_types::VIP_SUITE) > 0.9);
        assert!(room_comfort(room_types::CORRIDOR) < 0.2);
        assert!(room_comfort(room_types::LIBRARY) > 0.7);
        assert!(room_comfort(room_types::ENGINEERING) < 0.5);
    }

    #[test]
    fn test_medical_urgency_overrides_all() {
        let mut input = default_input();
        input.health = 0.3; // should_seek_medical threshold
        let (act, _, _) = pick_best(&input);
        assert_eq!(act, activity_types::IDLE); // Going to medical
    }

    #[test]
    fn test_very_hungry_picks_eating() {
        let mut input = default_input();
        input.hunger = 0.95;
        input.fatigue = 0.1;
        let (act, _, _) = pick_best(&input);
        assert_eq!(act, activity_types::EATING);
    }

    #[test]
    fn test_very_tired_picks_sleeping() {
        let mut input = default_input();
        input.fatigue = 0.95;
        input.hunger = 0.1;
        let (act, _, _) = pick_best(&input);
        assert_eq!(act, activity_types::SLEEPING);
    }

    #[test]
    fn test_dirty_picks_hygiene() {
        let mut input = default_input();
        input.hygiene = 0.95;
        input.fatigue = 0.1;
        input.hunger = 0.1;
        let (act, _, _) = pick_best(&input);
        assert_eq!(act, activity_types::HYGIENE);
    }

    #[test]
    fn test_duty_high_conscientiousness() {
        let mut input = default_input();
        input.is_crew = true;
        input.shift = Some(0);
        input.department = Some(0);
        input.should_be_on_duty = true;
        input.fit_for_duty = true;
        input.conscientiousness = 1.0;
        // Low needs — duty should win
        let (act, _, _) = pick_best(&input);
        assert_eq!(act, activity_types::ON_DUTY);
    }

    #[test]
    fn test_extravert_prefers_social() {
        let mut input = default_input();
        input.social = 0.7;
        input.comfort = 0.7;
        input.extraversion = 1.0;
        let scored = score_activities(&input);
        let social_score = scored
            .iter()
            .find(|s| s.activity_type == activity_types::SOCIALIZING)
            .unwrap()
            .score;
        let relax_score = scored
            .iter()
            .find(|s| s.activity_type == activity_types::RELAXING)
            .unwrap()
            .score;
        assert!(
            social_score > relax_score,
            "Extravert social={social_score} should beat relax={relax_score}"
        );
    }

    #[test]
    fn test_introvert_avoids_social() {
        let mut input = default_input();
        input.social = 0.5;
        input.comfort = 0.5;
        input.extraversion = 0.0;
        let scored = score_activities(&input);
        let social_score = scored
            .iter()
            .find(|s| s.activity_type == activity_types::SOCIALIZING)
            .unwrap()
            .score;
        let relax_score = scored
            .iter()
            .find(|s| s.activity_type == activity_types::RELAXING)
            .unwrap()
            .score;
        assert!(
            relax_score > social_score,
            "Introvert relax={relax_score} should beat social={social_score}"
        );
    }

    #[test]
    fn test_neurotic_in_noisy_room_relaxes() {
        let mut input = default_input();
        input.neuroticism = 1.0;
        input.comfort = 0.4;
        input.current_room = Some(RoomContext {
            room_type: room_types::ENGINEERING,
            occupants: 3,
            capacity: 10,
        });
        let scored = score_activities(&input);
        let relax = scored
            .iter()
            .find(|s| s.activity_type == activity_types::RELAXING)
            .unwrap();
        // Noise stress should boost relax score significantly
        assert!(relax.score > 3.0, "Neurotic noise stress: {}", relax.score);
    }

    #[test]
    fn test_overcrowded_room_deters_social() {
        let mut input = default_input();
        input.social = 0.6;
        input.extraversion = 0.3; // Mild introvert
        input.current_room = Some(RoomContext {
            room_type: room_types::MESS_HALL,
            occupants: 20,
            capacity: 10,
        });
        let scored = score_activities(&input);
        let social = scored
            .iter()
            .find(|s| s.activity_type == activity_types::SOCIALIZING)
            .unwrap();
        // Overcrowding should reduce social appeal
        let mut input2 = input.clone();
        input2.current_room = Some(RoomContext {
            room_type: room_types::MESS_HALL,
            occupants: 2,
            capacity: 10,
        });
        let scored2 = score_activities(&input2);
        let social2 = scored2
            .iter()
            .find(|s| s.activity_type == activity_types::SOCIALIZING)
            .unwrap();
        assert!(
            social2.score > social.score,
            "Empty room social={} should > crowded={}",
            social2.score,
            social.score
        );
    }

    #[test]
    fn test_meal_time_boosts_eating() {
        let mut input = default_input();
        input.hunger = 0.5;
        input.hour = 12.5; // Lunch
        let scored = score_activities(&input);
        let eat = scored
            .iter()
            .find(|s| s.activity_type == activity_types::EATING)
            .unwrap();

        let mut input2 = input.clone();
        input2.hour = 10.0; // Not meal time
        let scored2 = score_activities(&input2);
        let eat2 = scored2
            .iter()
            .find(|s| s.activity_type == activity_types::EATING)
            .unwrap();
        assert!(
            eat.score > eat2.score,
            "Meal time eat={} > non-meal={}",
            eat.score,
            eat2.score
        );
    }

    #[test]
    fn test_unfit_crew_doesnt_duty() {
        let mut input = default_input();
        input.is_crew = true;
        input.should_be_on_duty = true;
        input.fit_for_duty = false; // Unfit
        input.conscientiousness = 1.0;
        let scored = score_activities(&input);
        let duty = scored
            .iter()
            .find(|s| s.activity_type == activity_types::ON_DUTY);
        assert!(duty.is_none(), "Unfit crew should not have duty candidate");
    }
}
