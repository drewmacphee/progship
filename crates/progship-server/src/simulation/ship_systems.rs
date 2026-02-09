//! Ship systems simulation - resource flow, subsystem degradation, power generation.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

/// Update ship systems: resource production, consumption, degradation.
pub fn tick_ship_systems(ctx: &ReducerContext, delta_hours: f32) {
    let Some(mut resources) = ctx.db.ship_resources().id().find(0) else {
        return;
    };

    let person_count = ctx.db.person().iter().count() as f32;

    // Base consumption rates (per person per hour)
    let food_rate = 2.0 / 24.0;
    let water_rate = 3.0 / 24.0;
    let oxygen_rate = 0.84 / 24.0;

    resources.food = (resources.food - person_count * food_rate * delta_hours).max(0.0);
    resources.water = (resources.water - person_count * water_rate * delta_hours).max(0.0);
    resources.oxygen = (resources.oxygen - person_count * oxygen_rate * delta_hours).max(0.0);

    // Subsystem-level production/consumption and degradation
    let subsystems: Vec<Subsystem> = ctx.db.subsystem().iter().collect();
    for sub in &subsystems {
        if sub.status == system_statuses::OFFLINE || sub.status == system_statuses::DESTROYED {
            continue;
        }
        let efficiency = sub.health
            * if sub.status == system_statuses::DEGRADED {
                0.5
            } else {
                1.0
            };

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
                let o2_produced = person_count * oxygen_rate * efficiency * delta_hours;
                resources.oxygen = (resources.oxygen + o2_produced).min(resources.oxygen_cap);
            }
            subsystem_types::WATER_FILTRATION | subsystem_types::WATER_DISTILLATION => {
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
        s.status = if s.health > 0.7 {
            system_statuses::NOMINAL
        } else if s.health > 0.3 {
            system_statuses::DEGRADED
        } else if s.health > 0.0 {
            system_statuses::CRITICAL
        } else {
            system_statuses::OFFLINE
        };
        ctx.db.subsystem().id().update(s);
    }

    // Degrade components slowly
    let components: Vec<SystemComponent> = ctx.db.system_component().iter().collect();
    for comp in components {
        let mut c = comp;
        c.health = (c.health - 0.00005 * delta_hours).max(0.0);
        c.status = if c.health > 0.7 {
            system_statuses::NOMINAL
        } else if c.health > 0.3 {
            system_statuses::DEGRADED
        } else if c.health > 0.0 {
            system_statuses::CRITICAL
        } else {
            system_statuses::OFFLINE
        };
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
