//! Ship systems simulation - resource flow, subsystem degradation, economy loop.

use progship_logic::economy;
use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

// Resource consumption rates (per person per hour)
const FOOD_RATE: f32 = 2.0 / 24.0;
const WATER_RATE: f32 = 3.0 / 24.0;
const OXYGEN_RATE: f32 = 0.84 / 24.0;

fn resource_values(r: &ShipResources) -> economy::ResourceValues {
    economy::ResourceValues {
        food: r.food,
        food_cap: r.food_cap,
        water: r.water,
        water_cap: r.water_cap,
        oxygen: r.oxygen,
        oxygen_cap: r.oxygen_cap,
        power: r.power,
        power_cap: r.power_cap,
        fuel: r.fuel,
        fuel_cap: r.fuel_cap,
        spare_parts: r.spare_parts,
        spare_parts_cap: r.spare_parts_cap,
    }
}

/// Calculate resource consumption for a population
pub fn calculate_resource_consumption(person_count: f32, delta_hours: f32) -> (f32, f32, f32) {
    (
        person_count * FOOD_RATE * delta_hours,
        person_count * WATER_RATE * delta_hours,
        person_count * OXYGEN_RATE * delta_hours,
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

/// Update ship systems: resource production, consumption, degradation, economy.
pub fn tick_ship_systems(ctx: &ReducerContext, delta_hours: f32) {
    let Some(mut resources) = ctx.db.ship_resources().id().find(0) else {
        return;
    };

    let alive_count = ctx.db.person().iter().filter(|p| p.is_alive).count() as f32;

    // Compute current rationing level
    let levels = economy::compute_levels(&resource_values(&resources));
    let rationing = economy::compute_rationing(&levels);
    let consumption_factor = economy::rationing_consumption_factor(rationing);

    // Base consumption adjusted by rationing
    let (food_consumed, water_consumed, oxygen_consumed) =
        calculate_resource_consumption(alive_count, delta_hours);

    resources.food = (resources.food - food_consumed * consumption_factor).max(0.0);
    resources.water = (resources.water - water_consumed * consumption_factor).max(0.0);
    resources.oxygen = (resources.oxygen - oxygen_consumed).max(0.0); // O2 can't be rationed

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
                let o2_produced = alive_count * OXYGEN_RATE * efficiency * delta_hours;
                resources.oxygen = (resources.oxygen + o2_produced).min(resources.oxygen_cap);
            }
            subsystem_types::WATER_FILTRATION | subsystem_types::WATER_DISTILLATION => {
                let recycled = alive_count * WATER_RATE * 0.45 * efficiency * delta_hours;
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

    // --- Economy effects: scarcity, rationing, morale, health ---

    // Recompute levels after production/consumption
    let res = ctx.db.ship_resources().id().find(0).unwrap();
    let updated_levels = economy::compute_levels(&resource_values(&res));
    let new_rationing = economy::compute_rationing(&updated_levels);

    // Update rationing level on ShipConfig
    if let Some(config) = ctx.db.ship_config().id().find(0) {
        let old_rationing = economy::u8_to_rationing(config.rationing_level);
        if new_rationing != old_rationing {
            let mut c = config;
            c.rationing_level = economy::rationing_to_u8(new_rationing);
            ctx.db.ship_config().id().update(c);
        }
    }

    // Generate RESOURCE_SHORTAGE events for critical shortages
    let shortages = economy::detect_shortages(&updated_levels);
    let sim_time = ctx
        .db
        .ship_config()
        .id()
        .find(0)
        .map(|c| c.sim_time)
        .unwrap_or(0.0);
    for (resource_name, level) in &shortages {
        // Only create event if no active shortage event for this resource
        let already_active = ctx.db.event().iter().any(|e| {
            e.event_type == event_types::RESOURCE_SHORTAGE && e.state == event_states::ACTIVE
        });
        if !already_active {
            let severity = if *level < 0.05 { 0.9 } else { 0.6 };
            ctx.db.event().insert(Event {
                id: 0,
                event_type: event_types::RESOURCE_SHORTAGE,
                room_id: 0, // Ship-wide
                started_at: sim_time,
                duration: 1.0,
                state: event_states::ACTIVE,
                responders_needed: 0,
                responders_assigned: 0,
                severity,
            });
            log::warn!(
                "Resource shortage: {} at {:.0}%",
                resource_name,
                level * 100.0
            );
            break; // One event per tick is enough
        }
    }

    // Morale and health effects from rationing/depletion
    let morale_penalty = economy::rationing_morale_penalty(new_rationing) * delta_hours;
    let health_damage = economy::resource_health_damage(&updated_levels) * delta_hours;

    if morale_penalty > 0.0 || health_damage > 0.0 {
        let needs_list: Vec<Needs> = ctx.db.needs().iter().collect();
        for needs in needs_list {
            // Skip dead
            if let Some(person) = ctx.db.person().id().find(needs.person_id) {
                if !person.is_alive {
                    continue;
                }
            }
            let mut n = needs;
            if morale_penalty > 0.0 {
                n.morale = (n.morale - morale_penalty).max(0.0);
            }
            if health_damage > 0.0 {
                n.health = (n.health - health_damage).max(0.0);
            }
            ctx.db.needs().person_id().update(n);
        }
    }
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
