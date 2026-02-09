//! Duty system - crew shift management.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

use super::activities::should_be_on_duty;

/// Update crew on/off duty status based on shift and time.
pub fn tick_duty(ctx: &ReducerContext, sim_time: f64) {
    let hour = (sim_time % 24.0) as f32;

    for crew in ctx.db.crew().iter() {
        let should_work = should_be_on_duty(crew.shift, hour);
        if crew.on_duty != should_work {
            let mut c = crew;
            c.on_duty = should_work;
            ctx.db.crew().person_id().update(c);
        }
    }
}
