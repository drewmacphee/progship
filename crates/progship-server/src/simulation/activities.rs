//! Activity selection system - NPCs choose activities based on needs and time.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

use super::movement::start_movement_to;

/// Select new activities when current ones complete, and handle activity effects.
pub fn tick_activities(ctx: &ReducerContext, sim_time: f64) {
    for activity in ctx.db.activity().iter() {
        // Skip player-controlled characters
        if let Some(person) = ctx.db.person().id().find(activity.person_id) {
            if person.is_player {
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

fn is_meal_time(hour: f32) -> bool {
    (7.0..8.0).contains(&hour) ||   // Breakfast
    (12.0..13.0).contains(&hour) ||  // Lunch
    (18.0..19.0).contains(&hour) // Dinner
}

fn is_sleep_time(hour: f32, is_crew: bool) -> bool {
    if is_crew {
        false
    }
    // Crew sleeps based on shift
    else {
        !(6.0..22.0).contains(&hour)
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
            let room_type = match department {
                departments::COMMAND => room_types::BRIDGE,
                departments::ENGINEERING => room_types::ENGINEERING,
                departments::MEDICAL => room_types::HOSPITAL_WARD,
                departments::SCIENCE => room_types::LABORATORY,
                departments::SECURITY => room_types::CIC,
                departments::OPERATIONS => room_types::ENGINEERING,
                _ => room_types::CORRIDOR,
            };
            find_room_of_type(ctx, room_type)
        }
        _ => None,
    }
}
