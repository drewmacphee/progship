//! Activity selection system - NPCs choose activities based on utility scoring.

use progship_logic::duty as duty_logic;
use progship_logic::utility::{self, RoomCategory, RoomTarget, UtilityInput};
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

        // Activity complete - select new one based on utility scoring
        let Some(needs) = ctx.db.needs().person_id().find(activity.person_id) else {
            continue;
        };

        let crew_opt = ctx.db.crew().person_id().find(activity.person_id);
        let is_crew = crew_opt.is_some();
        let current_hour = (sim_time % 24.0) as f32;

        // Build room context for current position
        let current_room = ctx
            .db
            .position()
            .person_id()
            .find(activity.person_id)
            .and_then(|pos| {
                ctx.db.room().id().find(pos.room_id).map(|room| {
                    let occupants = ctx
                        .db
                        .position()
                        .iter()
                        .filter(|p| p.room_id == pos.room_id)
                        .count() as u32;
                    utility::RoomContext {
                        room_type: room.room_type,
                        occupants,
                        capacity: room.capacity,
                    }
                })
            });

        // Get personality (default to neutral 0.5 if missing)
        let personality = ctx.db.personality().person_id().find(activity.person_id);
        let (ext, neu, con, opn, agr) = personality
            .as_ref()
            .map(|p| {
                (
                    p.extraversion,
                    p.neuroticism,
                    p.conscientiousness,
                    p.openness,
                    p.agreeableness,
                )
            })
            .unwrap_or((0.5, 0.5, 0.5, 0.5, 0.5));

        let (shift, department) = crew_opt
            .as_ref()
            .map(|c| (Some(c.shift), Some(c.department)))
            .unwrap_or((None, None));

        let fit = duty_logic::is_fit_for_duty(needs.hunger, needs.fatigue, needs.health);
        let on_duty = shift
            .map(|s| duty_logic::should_be_on_duty(s, current_hour))
            .unwrap_or(false);

        let input = UtilityInput {
            hunger: needs.hunger,
            fatigue: needs.fatigue,
            social: needs.social,
            comfort: needs.comfort,
            hygiene: needs.hygiene,
            health: needs.health,
            morale: needs.morale,
            hour: current_hour,
            is_crew,
            shift,
            department,
            extraversion: ext,
            neuroticism: neu,
            conscientiousness: con,
            openness: opn,
            agreeableness: agr,
            current_room,
            fit_for_duty: fit,
            should_be_on_duty: on_duty,
        };

        let (new_type, duration, room_target) = utility::pick_best(&input);
        let target_room = resolve_room_target(ctx, &room_target);

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

/// Resolve a RoomTarget to an actual room ID.
fn resolve_room_target(ctx: &ReducerContext, target: &RoomTarget) -> Option<u32> {
    match target {
        RoomTarget::None => None,
        RoomTarget::Exact(rt) => find_room_of_type(ctx, *rt),
        RoomTarget::Category(cat) => match cat {
            RoomCategory::Quarters => find_room_of_type_pred(ctx, room_types::is_quarters),
            RoomCategory::Recreation => find_room_of_type_pred(ctx, room_types::is_recreation),
            RoomCategory::Medical => find_room_of_type(ctx, room_types::HOSPITAL_WARD),
            RoomCategory::Dining => find_room_of_type_pred(ctx, room_types::is_dining),
        },
        RoomTarget::DutyStation(dept) => {
            let rt = department_to_room_type(*dept);
            find_room_of_type(ctx, rt)
        }
    }
}

pub fn should_be_on_duty(shift: u8, hour: f32) -> bool {
    duty_logic::should_be_on_duty(shift, hour)
}

pub fn is_meal_time(hour: f32) -> bool {
    (7.0..8.0).contains(&hour) || (12.0..13.0).contains(&hour) || (18.0..19.0).contains(&hour)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_be_on_duty_alpha_shift() {
        assert!(should_be_on_duty(shifts::ALPHA, 6.0));
        assert!(should_be_on_duty(shifts::ALPHA, 10.0));
        assert!(!should_be_on_duty(shifts::ALPHA, 5.9));
        assert!(!should_be_on_duty(shifts::ALPHA, 14.0));
    }

    #[test]
    fn test_should_be_on_duty_gamma_shift() {
        assert!(should_be_on_duty(shifts::GAMMA, 22.0));
        assert!(should_be_on_duty(shifts::GAMMA, 0.0));
        assert!(!should_be_on_duty(shifts::GAMMA, 6.0));
        assert!(!should_be_on_duty(shifts::GAMMA, 12.0));
    }

    #[test]
    fn test_is_meal_time() {
        assert!(is_meal_time(7.0));
        assert!(is_meal_time(12.5));
        assert!(is_meal_time(18.0));
        assert!(!is_meal_time(10.0));
        assert!(!is_meal_time(15.0));
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
        assert_eq!(department_to_room_type(99), room_types::CORRIDOR);
    }
}
