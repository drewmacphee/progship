//! Maintenance system - task creation, crew assignment, repair progress.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

/// Calculate task priority based on subsystem health (1.0 = max priority)
pub fn calculate_task_priority(subsystem_health: f32) -> f32 {
    1.0 - subsystem_health
}

/// Calculate task duration based on subsystem health
pub fn calculate_task_duration(subsystem_health: f32) -> f32 {
    2.0 + (1.0 - subsystem_health) * 4.0
}

/// Calculate repair progress increase
pub fn calculate_repair_progress(
    current_progress: f32,
    delta_hours: f32,
    duration_hours: f32,
) -> f32 {
    (current_progress + delta_hours / duration_hours).min(1.0)
}

/// Apply repair to component/subsystem health
pub fn apply_repair(health: f32) -> f32 {
    (health + 0.3).min(1.0)
}

/// Determine required skill for system type
pub fn system_type_to_skill(system_type: u8) -> u8 {
    match system_type {
        system_types::MEDICAL => skill_types::MEDICAL,
        system_types::NAVIGATION => skill_types::PILOTING,
        _ => skill_types::ENGINEERING,
    }
}

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
                .map(|sys| system_type_to_skill(sys.system_type))
                .unwrap_or(skill_types::ENGINEERING);

            // Find a degraded component within this subsystem to target
            let target_comp = ctx
                .db
                .system_component()
                .iter()
                .find(|c| c.subsystem_id == sub.id && c.health < 0.7);
            let comp_id = target_comp.map(|c| c.id).unwrap_or(0);

            let priority = calculate_task_priority(sub.health);
            let duration = calculate_task_duration(sub.health);

            ctx.db.maintenance_task().insert(MaintenanceTask {
                id: 0,
                component_id: comp_id,
                subsystem_id: sub.id,
                assigned_crew_id: None,
                priority,
                progress: 0.0,
                created_at: sim_time,
                required_skill: skill,
                duration_hours: duration,
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
        t.progress = calculate_repair_progress(t.progress, delta_hours, t.duration_hours);

        if t.progress >= 1.0 {
            // Repair complete - restore component and subsystem health
            if t.component_id > 0 {
                if let Some(mut comp) = ctx.db.system_component().id().find(t.component_id) {
                    comp.health = apply_repair(comp.health);
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
                sub.health = apply_repair(sub.health);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_task_priority_critical() {
        let priority = calculate_task_priority(0.1);
        assert_eq!(priority, 0.9);
    }

    #[test]
    fn test_calculate_task_priority_degraded() {
        let priority = calculate_task_priority(0.5);
        assert_eq!(priority, 0.5);
    }

    #[test]
    fn test_calculate_task_priority_nominal() {
        let priority = calculate_task_priority(0.9);
        assert!((priority - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_calculate_task_duration_critical() {
        let duration = calculate_task_duration(0.0);
        assert_eq!(duration, 6.0); // 2.0 + 1.0 * 4.0
    }

    #[test]
    fn test_calculate_task_duration_degraded() {
        let duration = calculate_task_duration(0.5);
        assert_eq!(duration, 4.0); // 2.0 + 0.5 * 4.0
    }

    #[test]
    fn test_calculate_task_duration_nominal() {
        let duration = calculate_task_duration(0.9);
        assert!((duration - 2.4).abs() < 0.001); // 2.0 + 0.1 * 4.0
    }

    #[test]
    fn test_calculate_repair_progress_partial() {
        let progress = calculate_repair_progress(0.0, 1.0, 4.0);
        assert_eq!(progress, 0.25);

        let progress = calculate_repair_progress(0.5, 1.0, 4.0);
        assert_eq!(progress, 0.75);
    }

    #[test]
    fn test_calculate_repair_progress_complete() {
        let progress = calculate_repair_progress(0.8, 2.0, 4.0);
        assert_eq!(progress, 1.0); // Clamped at 1.0
    }

    #[test]
    fn test_apply_repair() {
        assert_eq!(apply_repair(0.5), 0.8);
        assert_eq!(apply_repair(0.2), 0.5);
        assert_eq!(apply_repair(0.0), 0.3);
    }

    #[test]
    fn test_apply_repair_at_max() {
        assert_eq!(apply_repair(0.8), 1.0); // Clamped at 1.0
        assert_eq!(apply_repair(1.0), 1.0);
    }

    #[test]
    fn test_system_type_to_skill() {
        assert_eq!(
            system_type_to_skill(system_types::MEDICAL),
            skill_types::MEDICAL
        );
        assert_eq!(
            system_type_to_skill(system_types::NAVIGATION),
            skill_types::PILOTING
        );
        assert_eq!(
            system_type_to_skill(system_types::LIFE_SUPPORT),
            skill_types::ENGINEERING
        );
        assert_eq!(
            system_type_to_skill(system_types::PROPULSION),
            skill_types::ENGINEERING
        );
        assert_eq!(system_type_to_skill(99), skill_types::ENGINEERING);
    }
}
