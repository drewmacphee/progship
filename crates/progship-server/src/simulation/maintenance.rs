//! Maintenance system - task creation, crew assignment, repair progress.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

/// Check subsystems/components for maintenance needs, assign crew, progress repairs.
pub fn tick_maintenance(ctx: &ReducerContext, sim_time: f64, delta_hours: f32) {
    // Generate tasks for degraded subsystems
    for sub in ctx.db.subsystem().iter() {
        if sub.health < 0.7 {
            let has_task = ctx
                .db
                .maintenance_task()
                .iter()
                .any(|t| t.subsystem_id == sub.id && t.progress < 1.0);
            if has_task {
                continue;
            }

            // Find the parent system type to determine required skill
            let skill = ctx
                .db
                .ship_system()
                .id()
                .find(sub.system_id)
                .map(|sys| match sys.system_type {
                    system_types::MEDICAL => skill_types::MEDICAL,
                    system_types::NAVIGATION => skill_types::PILOTING,
                    _ => skill_types::ENGINEERING,
                })
                .unwrap_or(skill_types::ENGINEERING);

            // Find a degraded component within this subsystem to target
            let target_comp = ctx
                .db
                .system_component()
                .iter()
                .find(|c| c.subsystem_id == sub.id && c.health < 0.7);
            let comp_id = target_comp.map(|c| c.id).unwrap_or(0);

            ctx.db.maintenance_task().insert(MaintenanceTask {
                id: 0,
                component_id: comp_id,
                subsystem_id: sub.id,
                assigned_crew_id: None,
                priority: 1.0 - sub.health,
                progress: 0.0,
                created_at: sim_time,
                required_skill: skill,
                duration_hours: 2.0 + (1.0 - sub.health) * 4.0,
            });
        }
    }

    // Assign unassigned tasks to available crew
    let tasks: Vec<MaintenanceTask> = ctx
        .db
        .maintenance_task()
        .iter()
        .filter(|t| t.assigned_crew_id.is_none() && t.progress < 1.0)
        .collect();

    for task in tasks {
        let assigned = ctx
            .db
            .crew()
            .iter()
            .find(|c| !c.on_duty)
            .map(|c| c.person_id);

        if let Some(crew_id) = assigned {
            let duration_hours = task.duration_hours;
            let mut t = task;
            t.assigned_crew_id = Some(crew_id);
            ctx.db.maintenance_task().id().update(t);

            if let Some(mut act) = ctx.db.activity().person_id().find(crew_id) {
                act.activity_type = activity_types::MAINTENANCE;
                act.started_at = sim_time;
                act.duration = duration_hours;
                ctx.db.activity().person_id().update(act);
            }
        }
    }

    // Progress active repairs
    let active_tasks: Vec<MaintenanceTask> = ctx
        .db
        .maintenance_task()
        .iter()
        .filter(|t| t.assigned_crew_id.is_some() && t.progress < 1.0)
        .collect();

    for task in active_tasks {
        let mut t = task;
        t.progress = (t.progress + delta_hours / t.duration_hours).min(1.0);

        if t.progress >= 1.0 {
            // Repair complete - restore component and subsystem health
            if t.component_id > 0 {
                if let Some(mut comp) = ctx.db.system_component().id().find(t.component_id) {
                    comp.health = (comp.health + 0.3).min(1.0);
                    comp.status = if comp.health > 0.7 {
                        system_statuses::NOMINAL
                    } else {
                        system_statuses::DEGRADED
                    };
                    comp.last_maintenance = ctx
                        .db
                        .ship_config()
                        .id()
                        .find(0)
                        .map(|c| c.sim_time)
                        .unwrap_or(0.0);
                    ctx.db.system_component().id().update(comp);
                }
            }
            if let Some(mut sub) = ctx.db.subsystem().id().find(t.subsystem_id) {
                sub.health = (sub.health + 0.3).min(1.0);
                sub.status = if sub.health > 0.7 {
                    system_statuses::NOMINAL
                } else {
                    system_statuses::DEGRADED
                };
                ctx.db.subsystem().id().update(sub);
            }
        }

        ctx.db.maintenance_task().id().update(t);
    }
}
