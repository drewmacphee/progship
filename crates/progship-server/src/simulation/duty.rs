//! Duty system - crew shift management.

use crate::logic::duty as duty_logic;
use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

/// Update crew on/off duty status based on shift, time, and fitness.
pub fn tick_duty(ctx: &ReducerContext, sim_time: f64) {
    let hour = (sim_time % 24.0) as f32;

    for crew in ctx.db.crew().iter() {
        // Skip dead crew
        if let Some(person) = ctx.db.person().id().find(crew.person_id) {
            if !person.is_alive {
                if crew.on_duty {
                    let mut c = crew;
                    c.on_duty = false;
                    ctx.db.crew().person_id().update(c);
                }
                continue;
            }
        }

        // Check fitness: injured/exhausted crew can't work
        let fit = ctx
            .db
            .needs()
            .person_id()
            .find(crew.person_id)
            .map(|n| duty_logic::is_fit_for_duty(n.hunger, n.fatigue, n.health))
            .unwrap_or(false);

        let should_work = duty_logic::should_be_on_duty(crew.shift, hour) && fit;
        if crew.on_duty != should_work {
            let mut c = crew;
            c.on_duty = should_work;
            ctx.db.crew().person_id().update(c);
        }
    }
}
