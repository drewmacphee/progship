//! Ship systems simulation - resource flow, subsystem degradation, power generation.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

/// Calculate resource consumption for a population
pub fn calculate_resource_consumption(person_count: f32, delta_hours: f32) -> (f32, f32, f32) {
    let food_rate = 2.0 / 24.0;
    let water_rate = 3.0 / 24.0;
    let oxygen_rate = 0.84 / 24.0;
    (
        person_count * food_rate * delta_hours,
        person_count * water_rate * delta_hours,
        person_count * oxygen_rate * delta_hours,
    )
}

/// Calculate subsystem efficiency based on health and status
pub fn calculate_subsystem_efficiency(health: f32, status: u8) -> f32 {
    if status == system_statuses::OFFLINE || status == system_statuses::DESTROYED {
        0.0
    } else {
        health
            * if status == system_statuses::DEGRADED {
                0.5
            } else {
                1.0
            }
    }
}

/// Determine system status from health
pub fn health_to_status(health: f32) -> u8 {
    if health > 0.7 {
        system_statuses::NOMINAL
    } else if health > 0.3 {
        system_statuses::DEGRADED
    } else if health > 0.0 {
        system_statuses::CRITICAL
    } else {
        system_statuses::OFFLINE
    }
}

/// Update ship systems: resource production, consumption, degradation.
pub fn tick_ship_systems(ctx: &ReducerContext, delta_hours: f32) {
    let Some(mut resources) = ctx.db.ship_resources().id().find(0) else {
        return;
    };

    let person_count = ctx.db.person().iter().count() as f32;

    // Base consumption rates
    let (food_consumed, water_consumed, oxygen_consumed) =
        calculate_resource_consumption(person_count, delta_hours);

    resources.food = (resources.food - food_consumed).max(0.0);
    resources.water = (resources.water - water_consumed).max(0.0);
    resources.oxygen = (resources.oxygen - oxygen_consumed).max(0.0);

    // Subsystem-level production/consumption and degradation
    let subsystems: Vec<Subsystem> = ctx.db.subsystem().iter().collect();
    for sub in &subsystems {
        let efficiency = calculate_subsystem_efficiency(sub.health, sub.status);
        if efficiency == 0.0 {
            continue;
        }

        // Production based on subsystem type
        match sub.subsystem_type {
            subsystem_types::REACTOR_CORE => {
                resources.power =
                    (resources.power + 100.0 * efficiency * delta_hours).min(resources.power_cap);
            }
            subsystem_types::EMERGENCY_GENERATOR => {
                // Only produces if main reactor is down
                let reactor_down = subsystems.iter().any(|s| {
                    s.subsystem_type == subsystem_types::REACTOR_CORE
                        && (s.status == system_statuses::OFFLINE
                            || s.status == system_statuses::DESTROYED)
                });
                if reactor_down {
                    resources.power = (resources.power + 30.0 * efficiency * delta_hours)
                        .min(resources.power_cap);
                }
            }
            subsystem_types::O2_GENERATION => {
                let oxygen_rate = 0.84 / 24.0;
                let o2_produced = person_count * oxygen_rate * efficiency * delta_hours;
                resources.oxygen = (resources.oxygen + o2_produced).min(resources.oxygen_cap);
            }
            subsystem_types::WATER_FILTRATION | subsystem_types::WATER_DISTILLATION => {
                let water_rate = 3.0 / 24.0;
                let recycled = person_count * water_rate * 0.45 * efficiency * delta_hours;
                resources.water = (resources.water + recycled).min(resources.water_cap);
            }
            subsystem_types::GROWTH_CHAMBER => {
                resources.food =
                    (resources.food + 5.0 * efficiency * delta_hours).min(resources.food_cap);
            }
            _ => {}
        }

        // Power consumption from subsystem power_draw
        if sub.power_draw > 0.0 {
            resources.power = (resources.power - sub.power_draw * delta_hours).max(0.0);
        }
    }

    // Degrade subsystems slowly, update their status
    let subsystems_for_update: Vec<Subsystem> = ctx.db.subsystem().iter().collect();
    for sub in subsystems_for_update {
        let mut s = sub;
        s.health = (s.health - 0.0001 * delta_hours).max(0.0);
        s.status = health_to_status(s.health);
        ctx.db.subsystem().id().update(s);
    }

    // Degrade components slowly
    let components: Vec<SystemComponent> = ctx.db.system_component().iter().collect();
    for comp in components {
        let mut c = comp;
        c.health = (c.health - 0.00005 * delta_hours).max(0.0);
        c.status = health_to_status(c.health);
        ctx.db.system_component().id().update(c);
    }

    // Recompute parent ShipSystem overall_health/status from subsystems
    let all_subsystems: Vec<Subsystem> = ctx.db.subsystem().iter().collect();
    let systems: Vec<ShipSystem> = ctx.db.ship_system().iter().collect();
    for sys in systems {
        let children: Vec<&Subsystem> = all_subsystems
            .iter()
            .filter(|s| s.system_id == sys.id)
            .collect();
        if children.is_empty() {
            continue;
        }
        let avg_health = children.iter().map(|s| s.health).sum::<f32>() / children.len() as f32;
        let worst_status = children.iter().map(|s| s.status).max().unwrap_or(0);
        let mut s = sys;
        s.overall_health = avg_health;
        s.overall_status = worst_status;
        ctx.db.ship_system().id().update(s);
    }

    // InfraEdge degradation (very slow)
    let infra_edges: Vec<InfraEdge> = ctx.db.infra_edge().iter().collect();
    for edge in infra_edges {
        let mut e = edge;
        e.health = (e.health - 0.00002 * delta_hours).max(0.0);
        ctx.db.infra_edge().id().update(e);
    }

    // Update infra_edge flow based on health
    let all_infra_edges: Vec<InfraEdge> = ctx.db.infra_edge().iter().collect();
    let graph_edges: Vec<GraphEdge> = ctx.db.graph_edge().iter().collect();
    for ge in graph_edges {
        // Skip crew paths — only infrastructure edges
        if ge.edge_type == edge_types::CREW_PATH {
            continue;
        }
        let infra_health = all_infra_edges
            .iter()
            .filter(|ie| ie.graph_edge_id == ge.id)
            .map(|ie| ie.health)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(1.0);
        // Update flow on each infra_edge for this graph_edge
        for ie in all_infra_edges
            .iter()
            .filter(|ie| ie.graph_edge_id == ge.id)
        {
            let mut e = ie.clone();
            e.current_flow = e.capacity * infra_health;
            ctx.db.infra_edge().id().update(e);
        }
    }

    ctx.db.ship_resources().id().update(resources);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_resource_consumption() {
        let (food, water, oxygen) = calculate_resource_consumption(100.0, 1.0);
        // food_rate = 2.0 / 24.0, water_rate = 3.0 / 24.0, oxygen_rate = 0.84 / 24.0
        assert!((food - 100.0 * 2.0 / 24.0).abs() < 0.001);
        assert!((water - 100.0 * 3.0 / 24.0).abs() < 0.001);
        assert!((oxygen - 100.0 * 0.84 / 24.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_resource_consumption_zero_population() {
        let (food, water, oxygen) = calculate_resource_consumption(0.0, 1.0);
        assert_eq!(food, 0.0);
        assert_eq!(water, 0.0);
        assert_eq!(oxygen, 0.0);
    }

    #[test]
    fn test_calculate_subsystem_efficiency_nominal() {
        let eff = calculate_subsystem_efficiency(1.0, system_statuses::NOMINAL);
        assert_eq!(eff, 1.0);

        let eff = calculate_subsystem_efficiency(0.8, system_statuses::NOMINAL);
        assert_eq!(eff, 0.8);
    }

    #[test]
    fn test_calculate_subsystem_efficiency_degraded() {
        let eff = calculate_subsystem_efficiency(1.0, system_statuses::DEGRADED);
        assert_eq!(eff, 0.5);

        let eff = calculate_subsystem_efficiency(0.8, system_statuses::DEGRADED);
        assert_eq!(eff, 0.4);
    }

    #[test]
    fn test_calculate_subsystem_efficiency_offline() {
        let eff = calculate_subsystem_efficiency(1.0, system_statuses::OFFLINE);
        assert_eq!(eff, 0.0);
    }

    #[test]
    fn test_calculate_subsystem_efficiency_destroyed() {
        let eff = calculate_subsystem_efficiency(1.0, system_statuses::DESTROYED);
        assert_eq!(eff, 0.0);
    }

    #[test]
    fn test_health_to_status_nominal() {
        assert_eq!(health_to_status(1.0), system_statuses::NOMINAL);
        assert_eq!(health_to_status(0.71), system_statuses::NOMINAL);
    }

    #[test]
    fn test_health_to_status_degraded() {
        assert_eq!(health_to_status(0.7), system_statuses::DEGRADED);
        assert_eq!(health_to_status(0.5), system_statuses::DEGRADED);
        assert_eq!(health_to_status(0.31), system_statuses::DEGRADED);
    }

    #[test]
    fn test_health_to_status_critical() {
        assert_eq!(health_to_status(0.3), system_statuses::CRITICAL);
        assert_eq!(health_to_status(0.1), system_statuses::CRITICAL);
        assert_eq!(health_to_status(0.01), system_statuses::CRITICAL);
    }

    #[test]
    fn test_health_to_status_offline() {
        assert_eq!(health_to_status(0.0), system_statuses::OFFLINE);
    }
}
