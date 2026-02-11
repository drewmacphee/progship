//! Maintenance system - generates and assigns repair tasks

use crate::components::{
    Activity, ActivityType, Crew, Department, MaintenanceTask, Position, ShipSystem,
};
use hecs::World;
use serde::{Deserialize, Serialize};

/// Maintenance task queue (singleton-like, stored in engine)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaintenanceQueue {
    pub tasks: Vec<MaintenanceTask>,
}

impl MaintenanceQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a task if one doesn't already exist for this system
    pub fn add_task(&mut self, task: MaintenanceTask) {
        if !self
            .tasks
            .iter()
            .any(|t| t.system_entity_id == task.system_entity_id)
        {
            self.tasks.push(task);
        }
    }

    /// Get highest priority unassigned task
    pub fn get_unassigned(&self) -> Option<&MaintenanceTask> {
        self.tasks
            .iter()
            .filter(|t| t.assigned_crew_id.is_none())
            .max_by(|a, b| {
                a.priority
                    .partial_cmp(&b.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Remove completed tasks
    pub fn remove_completed(&mut self) {
        self.tasks.retain(|t| !t.is_complete());
    }
}

/// Generate maintenance tasks for damaged systems
pub fn generate_maintenance_tasks(world: &World, queue: &mut MaintenanceQueue, current_time: f64) {
    let mut system_id: u32 = 0;

    for (entity, system) in world.query::<&ShipSystem>().iter() {
        // Systems below 70% health need maintenance
        if system.health < 0.7 {
            let priority = 1.0 - system.health; // Lower health = higher priority
            let task = MaintenanceTask::new(system_id, priority, current_time);
            queue.add_task(task);
        }
        system_id += 1;
        let _ = entity; // Suppress unused warning
    }
}

/// Assign available engineering crew to maintenance tasks
pub fn assign_maintenance_crew(world: &World, queue: &mut MaintenanceQueue) {
    // Find available engineering crew (not already doing maintenance)
    let mut available_crew: Vec<(hecs::Entity, u32)> = Vec::new();
    let mut crew_idx: u32 = 0;

    for (entity, (crew, activity)) in world.query::<(&Crew, Option<&Activity>)>().iter() {
        // Only engineering department can do maintenance
        if crew.department != Department::Engineering {
            crew_idx += 1;
            continue;
        }

        // Check if already doing maintenance
        let is_busy = activity
            .map(|a| a.activity_type == ActivityType::Maintenance)
            .unwrap_or(false);
        if !is_busy {
            available_crew.push((entity, crew_idx));
        }
        crew_idx += 1;
    }

    // Assign crew to unassigned tasks
    for (_, crew_id) in available_crew {
        // Find unassigned task with highest priority
        if let Some(task_idx) = queue
            .tasks
            .iter()
            .position(|t| t.assigned_crew_id.is_none())
        {
            // Check this crew isn't already assigned elsewhere
            let already_assigned = queue
                .tasks
                .iter()
                .any(|t| t.assigned_crew_id == Some(crew_id));

            if !already_assigned {
                queue.tasks[task_idx].assign(crew_id);
            }
        }
    }
}

/// Progress maintenance tasks and apply repairs
pub fn progress_maintenance(world: &mut World, queue: &mut MaintenanceQueue, delta_hours: f32) {
    // Collect completed repairs to apply
    let mut repairs: Vec<(u32, f32)> = Vec::new();

    for task in &mut queue.tasks {
        if task.assigned_crew_id.is_some() {
            // Progress based on time (1 hour to complete a repair)
            task.progress += delta_hours;

            if task.is_complete() {
                // Repair restores 30% health
                repairs.push((task.system_entity_id, 0.3));
            }
        }
    }

    // Apply repairs to systems
    let mut system_idx: u32 = 0;
    for (_, system) in world.query::<&mut ShipSystem>().iter() {
        for (target_id, repair_amount) in &repairs {
            if system_idx == *target_id {
                system.repair(*repair_amount);
            }
        }
        system_idx += 1;
    }

    // Clean up completed tasks
    queue.remove_completed();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Rank, Shift, SystemType};

    #[test]
    fn test_maintenance_task_generation() {
        let mut world = World::new();
        let mut queue = MaintenanceQueue::new();

        // Create a damaged system
        let mut system = ShipSystem::new("Damaged Reactor", SystemType::Power);
        system.health = 0.5; // Below 70% threshold
        world.spawn((system,));

        // Generate tasks
        generate_maintenance_tasks(&world, &mut queue, 0.0);

        assert_eq!(queue.tasks.len(), 1);
        assert!((queue.tasks[0].priority - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_crew_assignment() {
        let mut world = World::new();
        let mut queue = MaintenanceQueue::new();

        // Create a damaged system
        let mut system = ShipSystem::new("Damaged", SystemType::Power);
        system.health = 0.5;
        world.spawn((system,));

        // Create engineering crew member
        let crew = Crew::new(Department::Engineering, Rank::Ensign, Shift::Alpha);
        world.spawn((crew,));

        // Generate and assign
        generate_maintenance_tasks(&world, &mut queue, 0.0);
        assign_maintenance_crew(&world, &mut queue);

        assert!(queue.tasks[0].assigned_crew_id.is_some());
    }

    #[test]
    fn test_maintenance_progress() {
        let mut world = World::new();
        let mut queue = MaintenanceQueue::new();

        // Create a damaged system
        let mut system = ShipSystem::new("Damaged", SystemType::Power);
        system.health = 0.5;
        world.spawn((system,));

        // Create engineering crew
        let crew = Crew::new(Department::Engineering, Rank::Ensign, Shift::Alpha);
        world.spawn((crew,));

        // Generate, assign, and progress
        generate_maintenance_tasks(&world, &mut queue, 0.0);
        assign_maintenance_crew(&world, &mut queue);

        // Run for 1.5 hours (should complete)
        progress_maintenance(&mut world, &mut queue, 1.5);

        // Task should be removed and system repaired
        assert!(queue.tasks.is_empty());

        // Check system health improved
        for (_, sys) in world.query::<&ShipSystem>().iter() {
            assert!(sys.health > 0.5);
        }
    }
}
