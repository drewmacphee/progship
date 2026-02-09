//! Activity selection system - NPCs choose activities based on needs and time.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

use super::movement::start_movement_to;

/// Select new activities when current ones complete, and handle activity effects.
pub fn tick_activities(ctx: &ReducerContext, sim_time: f64) {
    for activity in ctx.db.activity().iter() {
        // Skip player-controlled and dead characters
        if let Some(person) = ctx.db.person().id().find(activity.person_id) {
            if person.is_player || !person.is_alive {
                continue;
            }
        }
        let elapsed = sim_time - activity.started_at;
        if elapsed < activity.duration as f64 {
            continue; // Still doing current activity
        }

        // Activity complete - select new one based on needs
        let Some(needs) = ctx.db.needs().person_id().find(activity.person_id) else {
            continue;
        };

        let is_crew = ctx.db.crew().person_id().find(activity.person_id).is_some();
        let current_hour = (sim_time % 24.0) as f32;

        let (new_type, duration, target_room) =
            select_activity(&needs, current_hour, is_crew, activity.person_id, ctx);

        let mut a = activity;
        let person_id = a.person_id;
        a.activity_type = new_type;
        a.started_at = sim_time;
        a.duration = duration;
        a.target_room_id = target_room;
        ctx.db.activity().person_id().update(a);

        // If activity requires a different room, start movement
        if let Some(target) = target_room {
            let Some(pos) = ctx.db.position().person_id().find(person_id) else {
                continue;
            };
            if pos.room_id != target {
                start_movement_to(ctx, person_id, target);
            }
        }
    }
}

/// Select the best activity based on needs and time of day
fn select_activity(
    needs: &Needs,
    hour: f32,
    is_crew: bool,
    person_id: u64,
    ctx: &ReducerContext,
) -> (u8, f32, Option<u32>) {
    // Check if crew member should be on duty
    if is_crew {
        if let Some(crew) = ctx.db.crew().person_id().find(person_id) {
            if should_be_on_duty(crew.shift, hour) {
                let room = find_room_for_activity(ctx, activity_types::ON_DUTY, crew.department);
                return (activity_types::ON_DUTY, 2.0, room);
            }
        }
    }

    // Priority: critical needs first
    if needs.fatigue > 0.85 {
        let room = find_room_of_type_pred(ctx, room_types::is_quarters);
        return (activity_types::SLEEPING, 8.0, room);
    }
    if needs.hunger > 0.75 {
        let room = find_room_of_type(ctx, room_types::MESS_HALL);
        return (activity_types::EATING, 0.5, room);
    }
    if needs.hygiene > 0.8 {
        let room = find_room_of_type(ctx, room_types::SHARED_BATHROOM);
        return (activity_types::HYGIENE, 0.3, room);
    }

    // Moderate needs
    if needs.social > 0.6 {
        let room = find_room_of_type_pred(ctx, room_types::is_recreation);
        return (activity_types::SOCIALIZING, 1.0, room);
    }
    if needs.comfort > 0.6 {
        let room = find_room_of_type_pred(ctx, room_types::is_recreation);
        return (activity_types::RELAXING, 1.0, room);
    }
    if needs.hunger > 0.5 && is_meal_time(hour) {
        let room = find_room_of_type(ctx, room_types::MESS_HALL);
        return (activity_types::EATING, 0.5, room);
    }

    // Sleep schedule
    if needs.fatigue > 0.5 && is_sleep_time(hour, is_crew) {
        let room = find_room_of_type_pred(ctx, room_types::is_quarters);
        return (activity_types::SLEEPING, 8.0, room);
    }

    // Default: idle/wander
    (activity_types::IDLE, 0.02, None)
}

pub fn should_be_on_duty(shift: u8, hour: f32) -> bool {
    match shift {
        shifts::ALPHA => (6.0..14.0).contains(&hour),
        shifts::BETA => (14.0..22.0).contains(&hour),
        shifts::GAMMA => !(6.0..22.0).contains(&hour),
        _ => false,
    }
}

pub fn is_meal_time(hour: f32) -> bool {
    (7.0..8.0).contains(&hour) ||   // Breakfast
    (12.0..13.0).contains(&hour) ||  // Lunch
    (18.0..19.0).contains(&hour) // Dinner
}

pub fn is_sleep_time(hour: f32, is_crew: bool) -> bool {
    if !is_crew {
        // Passengers sleep outside 6:00-22:00
        !(6.0..22.0).contains(&hour)
    } else {
        // Crew sleeps based on shift (not general sleep time)
        false
    }
}

pub fn department_to_room_type(department: u8) -> u8 {
    match department {
        departments::COMMAND => room_types::BRIDGE,
        departments::ENGINEERING => room_types::ENGINEERING,
        departments::MEDICAL => room_types::HOSPITAL_WARD,
        departments::SCIENCE => room_types::LABORATORY,
        departments::SECURITY => room_types::CIC,
        departments::OPERATIONS => room_types::ENGINEERING,
        _ => room_types::CORRIDOR,
    }
}

fn find_room_of_type(ctx: &ReducerContext, room_type: u8) -> Option<u32> {
    ctx.db
        .room()
        .iter()
        .find(|r| r.room_type == room_type)
        .map(|r| r.id)
}

fn find_room_of_type_pred(ctx: &ReducerContext, pred: fn(u8) -> bool) -> Option<u32> {
    ctx.db
        .room()
        .iter()
        .find(|r| pred(r.room_type))
        .map(|r| r.id)
}

fn find_room_for_activity(ctx: &ReducerContext, activity: u8, department: u8) -> Option<u32> {
    match activity {
        activity_types::ON_DUTY => {
            let room_type = department_to_room_type(department);
            find_room_of_type(ctx, room_type)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_be_on_duty_alpha_shift() {
        // Alpha: 6:00 - 14:00
        assert!(should_be_on_duty(shifts::ALPHA, 6.0));
        assert!(should_be_on_duty(shifts::ALPHA, 10.0));
        assert!(should_be_on_duty(shifts::ALPHA, 13.9));
        assert!(!should_be_on_duty(shifts::ALPHA, 5.9));
        assert!(!should_be_on_duty(shifts::ALPHA, 14.0));
        assert!(!should_be_on_duty(shifts::ALPHA, 20.0));
    }

    #[test]
    fn test_should_be_on_duty_beta_shift() {
        // Beta: 14:00 - 22:00
        assert!(should_be_on_duty(shifts::BETA, 14.0));
        assert!(should_be_on_duty(shifts::BETA, 18.0));
        assert!(should_be_on_duty(shifts::BETA, 21.9));
        assert!(!should_be_on_duty(shifts::BETA, 13.9));
        assert!(!should_be_on_duty(shifts::BETA, 22.0));
        assert!(!should_be_on_duty(shifts::BETA, 6.0));
    }

    #[test]
    fn test_should_be_on_duty_gamma_shift() {
        // Gamma: 22:00 - 6:00 (overnight)
        assert!(should_be_on_duty(shifts::GAMMA, 22.0));
        assert!(should_be_on_duty(shifts::GAMMA, 0.0));
        assert!(should_be_on_duty(shifts::GAMMA, 3.0));
        assert!(should_be_on_duty(shifts::GAMMA, 5.9));
        assert!(!should_be_on_duty(shifts::GAMMA, 6.0));
        assert!(!should_be_on_duty(shifts::GAMMA, 12.0));
        assert!(!should_be_on_duty(shifts::GAMMA, 21.9));
    }

    #[test]
    fn test_should_be_on_duty_invalid_shift() {
        assert!(!should_be_on_duty(99, 10.0));
    }

    #[test]
    fn test_is_meal_time() {
        // Breakfast: 7-8
        assert!(is_meal_time(7.0));
        assert!(is_meal_time(7.5));
        assert!(!is_meal_time(8.0));
        assert!(!is_meal_time(6.9));

        // Lunch: 12-13
        assert!(is_meal_time(12.0));
        assert!(is_meal_time(12.5));
        assert!(!is_meal_time(13.0));

        // Dinner: 18-19
        assert!(is_meal_time(18.0));
        assert!(is_meal_time(18.5));
        assert!(!is_meal_time(19.0));

        // Not meal time
        assert!(!is_meal_time(10.0));
        assert!(!is_meal_time(15.0));
        assert!(!is_meal_time(20.0));
    }

    #[test]
    fn test_is_sleep_time_crew() {
        // Crew never sleeps during sleep time (they sleep based on shifts)
        assert!(!is_sleep_time(23.0, true));
        assert!(!is_sleep_time(3.0, true));
        assert!(!is_sleep_time(12.0, true));
    }

    #[test]
    fn test_is_sleep_time_passenger() {
        // Passengers sleep outside 6:00-22:00
        assert!(is_sleep_time(23.0, false));
        assert!(is_sleep_time(0.0, false));
        assert!(is_sleep_time(3.0, false));
        assert!(is_sleep_time(5.9, false));
        assert!(!is_sleep_time(6.0, false));
        assert!(!is_sleep_time(12.0, false));
        assert!(!is_sleep_time(21.9, false));
        assert!(is_sleep_time(22.0, false));
    }

    #[test]
    fn test_department_to_room_type() {
        assert_eq!(
            department_to_room_type(departments::COMMAND),
            room_types::BRIDGE
        );
        assert_eq!(
            department_to_room_type(departments::ENGINEERING),
            room_types::ENGINEERING
        );
        assert_eq!(
            department_to_room_type(departments::MEDICAL),
            room_types::HOSPITAL_WARD
        );
        assert_eq!(
            department_to_room_type(departments::SCIENCE),
            room_types::LABORATORY
        );
        assert_eq!(
            department_to_room_type(departments::SECURITY),
            room_types::CIC
        );
        assert_eq!(
            department_to_room_type(departments::OPERATIONS),
            room_types::ENGINEERING
        );
        assert_eq!(department_to_room_type(99), room_types::CORRIDOR);
    }
}
