//! Activity system - manages what people are doing

use hecs::World;
use crate::components::{Person, Activity, Needs, NeedType, ActivityType};

/// Update activities - check for completion, apply effects
pub fn activity_system(world: &mut World, sim_time: f64, _delta_hours: f32) {
    let mut completed = Vec::new();

    // Find completed activities
    for (entity, (_, activity)) in world.query::<(&Person, &Activity)>().iter() {
        if activity.is_complete(sim_time) {
            completed.push(entity);
        }
    }

    // Process completions
    for entity in completed {
        if let Ok(activity) = world.get::<&Activity>(entity) {
            let activity_type = activity.activity_type;
            
            // Apply need satisfaction
            if let Some(need_type) = activity_type.satisfies() {
                if let Ok(mut needs) = world.get::<&mut Needs>(entity) {
                    // Amount satisfied depends on activity duration
                    needs.satisfy(need_type, 0.8);
                }
            }
        }
        
        // Remove completed activity
        let _ = world.remove_one::<Activity>(entity);
    }
}

/// Select an appropriate activity based on needs
pub fn select_activity(needs: &Needs, current_hour: f32) -> ActivityType {
    // Priority 1: Urgent needs
    if let Some(urgent_need) = needs.most_urgent(0.8) {
        return match urgent_need {
            NeedType::Hunger => ActivityType::Eating,
            NeedType::Fatigue => ActivityType::Sleeping,
            NeedType::Social => ActivityType::Socializing,
            NeedType::Comfort => ActivityType::Relaxing,
            NeedType::Hygiene => ActivityType::Hygiene,
        };
    }

    // Priority 2: Moderate needs
    if let Some(moderate_need) = needs.most_urgent(0.5) {
        return match moderate_need {
            NeedType::Hunger => ActivityType::Eating,
            NeedType::Fatigue => ActivityType::Relaxing,
            NeedType::Social => ActivityType::Socializing,
            NeedType::Comfort => ActivityType::Relaxing,
            NeedType::Hygiene => ActivityType::Hygiene,
        };
    }

    // Priority 3: Time-based defaults
    let hour = current_hour % 24.0;
    if hour >= 22.0 || hour < 6.0 {
        ActivityType::Sleeping
    } else if (7.0..8.0).contains(&hour) || (12.0..13.0).contains(&hour) || (18.0..19.0).contains(&hour) {
        ActivityType::Eating
    } else {
        ActivityType::Idle
    }
}

/// Get the typical duration for an activity type (in hours)
pub fn activity_duration(activity_type: ActivityType) -> f32 {
    match activity_type {
        ActivityType::Idle => 0.25,
        ActivityType::Working => 4.0,
        ActivityType::Eating => 0.5,
        ActivityType::Sleeping => 8.0,
        ActivityType::Socializing => 1.0,
        ActivityType::Relaxing => 1.0,
        ActivityType::Hygiene => 0.25,
        ActivityType::Traveling => 0.1,
        ActivityType::Maintenance => 2.0,
        ActivityType::OnDuty => 8.0,
        ActivityType::OffDuty => 8.0,
        ActivityType::Emergency => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_activity_hungry() {
        let mut needs = Needs::default();
        needs.hunger = 0.9;
        
        let activity = select_activity(&needs, 12.0);
        assert_eq!(activity, ActivityType::Eating);
    }

    #[test]
    fn test_select_activity_night() {
        let needs = Needs::default();
        let activity = select_activity(&needs, 23.0);
        assert_eq!(activity, ActivityType::Sleeping);
    }

    #[test]
    fn test_activity_completion() {
        let mut world = World::new();
        
        let entity = world.spawn((
            Person,
            Needs { hunger: 0.8, ..Default::default() },
            Activity::new(ActivityType::Eating, 0.0, 0.5),
        ));

        // After activity completes
        activity_system(&mut world, 1.0, 0.5);
        
        // Activity should be removed
        assert!(world.get::<&Activity>(entity).is_err());
        
        // Hunger should be satisfied
        let needs = world.get::<&Needs>(entity).unwrap();
        assert!(needs.hunger < 0.8);
    }
}
