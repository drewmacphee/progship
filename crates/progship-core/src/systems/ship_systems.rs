//! Ship systems simulation - resource flow, health, maintenance

use crate::components::{ResourceFlow, ResourceStorage, ResourceType, ShipSystem, SystemStatus};
use hecs::World;
use serde::{Deserialize, Serialize};

/// Ship-wide resource state (singleton)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShipResources {
    pub storage: ResourceStorage,
    pub capacity: ResourceStorage,
    pub production: ResourceStorage,
    pub consumption: ResourceStorage,
}

impl ShipResources {
    pub fn new() -> Self {
        Self {
            storage: ResourceStorage {
                power: 1000.0,
                water: 5000.0,
                oxygen: 10000.0,
                food: 10000.0,
                fuel: 50000.0,
                coolant: 1000.0,
                spare_parts: 500.0,
            },
            capacity: ResourceStorage {
                power: 2000.0,
                water: 10000.0,
                oxygen: 20000.0,
                food: 20000.0,
                fuel: 100000.0,
                coolant: 2000.0,
                spare_parts: 1000.0,
            },
            production: ResourceStorage::default(),
            consumption: ResourceStorage::default(),
        }
    }

    /// Get resource level as percentage (0-1)
    pub fn level(&self, resource: ResourceType) -> f32 {
        let current = self.storage.get(resource);
        let cap = self.capacity.get(resource);
        if cap > 0.0 {
            current / cap
        } else {
            0.0
        }
    }

    /// Check if resource is critically low (<10%)
    pub fn is_critical(&self, resource: ResourceType) -> bool {
        self.level(resource) < 0.1
    }

    /// Check if resource is low (<25%)
    pub fn is_low(&self, resource: ResourceType) -> bool {
        self.level(resource) < 0.25
    }
}

/// Update ship systems - calculate production/consumption, apply degradation
pub fn ship_systems_system(world: &mut World, resources: &mut ShipResources, delta_hours: f32) {
    // Reset production/consumption tracking
    resources.production = ResourceStorage::default();
    resources.consumption = ResourceStorage::default();

    // Calculate production and consumption from all systems
    for (_, (system, flow)) in world.query::<(&ShipSystem, &ResourceFlow)>().iter() {
        // Efficiency based on health and status
        let efficiency = match system.status {
            SystemStatus::Nominal => system.health,
            SystemStatus::Degraded => system.health * 0.75,
            SystemStatus::Critical => system.health * 0.25,
            SystemStatus::Offline | SystemStatus::Destroyed => 0.0,
        };

        // Add production
        for (resource, rate) in &flow.produces {
            let amount = rate * efficiency * delta_hours;
            *resources.production.get_mut(*resource) += amount;
        }

        // Add consumption (still consume even if degraded, just less efficient)
        for (resource, rate) in &flow.consumes {
            let amount = rate * delta_hours; // Full consumption regardless of efficiency
            *resources.consumption.get_mut(*resource) += amount;
        }
    }

    // Apply net resource changes
    apply_resource_changes(resources, delta_hours);

    // Degrade systems over time
    degrade_systems(world, delta_hours);
}

/// Apply production minus consumption to storage
fn apply_resource_changes(resources: &mut ShipResources, _delta_hours: f32) {
    // Power (instantaneous, doesn't accumulate)
    let net_power = resources.production.power - resources.consumption.power;
    resources.storage.power =
        (resources.storage.power + net_power).clamp(0.0, resources.capacity.power);

    // Water
    let net_water = resources.production.water - resources.consumption.water;
    resources.storage.water =
        (resources.storage.water + net_water).clamp(0.0, resources.capacity.water);

    // Oxygen
    let net_oxygen = resources.production.oxygen - resources.consumption.oxygen;
    resources.storage.oxygen =
        (resources.storage.oxygen + net_oxygen).clamp(0.0, resources.capacity.oxygen);

    // Food
    let net_food = resources.production.food - resources.consumption.food;
    resources.storage.food =
        (resources.storage.food + net_food).clamp(0.0, resources.capacity.food);

    // Fuel (consumed by propulsion)
    let net_fuel = resources.production.fuel - resources.consumption.fuel;
    resources.storage.fuel =
        (resources.storage.fuel + net_fuel).clamp(0.0, resources.capacity.fuel);
}

/// Degrade systems over time (slow wear)
fn degrade_systems(world: &mut World, delta_hours: f32) {
    // Base degradation rate: 0.001 per hour = ~0.7% per month
    let base_rate = 0.001;

    for (_, system) in world.query::<&mut ShipSystem>().iter() {
        // Only degrade active systems
        if system.status != SystemStatus::Offline && system.status != SystemStatus::Destroyed {
            system.degrade(delta_hours, base_rate);
        }
    }
}

/// Generate maintenance tasks for systems needing repair
pub fn check_maintenance_needs(world: &World) -> Vec<(hecs::Entity, String, f32)> {
    let mut tasks = Vec::new();

    for (entity, system) in world.query::<&ShipSystem>().iter() {
        if system.health < 0.7 {
            let priority = 1.0 - system.health; // Higher priority for lower health
            let task_name = format!("Repair {}", system.name);
            tasks.push((entity, task_name, priority));
        }
    }

    // Sort by priority (highest first)
    tasks.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    tasks
}

/// Perform maintenance on a system
pub fn perform_maintenance(
    world: &mut World,
    system_entity: hecs::Entity,
    repair_amount: f32,
) -> bool {
    if let Ok(mut system) = world.get::<&mut ShipSystem>(system_entity) {
        system.repair(repair_amount);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::SystemType;

    #[test]
    fn test_ship_resources_levels() {
        let resources = ShipResources::new();

        // Initial power is 1000/2000 = 50%
        assert!((resources.level(ResourceType::Power) - 0.5).abs() < 0.01);

        // Should not be critical or low at 50%
        assert!(!resources.is_critical(ResourceType::Power));
        assert!(!resources.is_low(ResourceType::Power));
    }

    #[test]
    fn test_system_degradation() {
        let mut world = World::new();

        let system = ShipSystem::new("Test Reactor", SystemType::Power);
        world.spawn((system,));

        let mut resources = ShipResources::new();

        // Run for 100 hours
        ship_systems_system(&mut world, &mut resources, 100.0);

        // System should have degraded
        for (_, sys) in world.query::<&ShipSystem>().iter() {
            assert!(sys.health < 1.0);
        }
    }

    #[test]
    fn test_maintenance_detection() {
        let mut world = World::new();

        let mut system = ShipSystem::new("Damaged System", SystemType::LifeSupport);
        system.health = 0.5;
        system.update_status();
        world.spawn((system,));

        let tasks = check_maintenance_needs(&world);
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].1.contains("Damaged System"));
    }
}
