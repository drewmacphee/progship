//! Ship graph construction for spatial and infrastructure connectivity.
//!
//! Creates GraphNode entries for all rooms and GraphEdge entries for crew paths,
//! power distribution, water/HVAC/data networks.

use super::facilities::{get_facility_manifest, FacilitySpec};
use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

/// Scale room counts based on actual population.
/// The manifest counts are baselined for ~5000 people.
fn scale_room_count(spec: &FacilitySpec, total_pop: u32, crew_count: u32, passenger_count: u32) -> u32 {
    let base_pop = 5000.0f32;
    let pop_ratio = total_pop as f32 / base_pop;

    match spec.room_type {
        // Cabins scale to give every person a bed
        // Single cabin = 1 person, double = 2, family suite = 4, crew quarters = 4
        room_types::CABIN_SINGLE => {
            // ~40% of passengers in singles
            ((passenger_count as f32 * 0.4) / 1.0).ceil() as u32
        }
        room_types::CABIN_DOUBLE => {
            // ~30% of passengers in doubles
            ((passenger_count as f32 * 0.3) / 2.0).ceil() as u32
        }
        room_types::FAMILY_SUITE => {
            // ~15% of passengers in family suites
            ((passenger_count as f32 * 0.15) / 4.0).ceil() as u32
        }
        room_types::VIP_SUITE => {
            // ~5% of passengers in VIP
            ((passenger_count as f32 * 0.05) / 2.0).ceil().max(1.0) as u32
        }
        room_types::QUARTERS_PASSENGER => {
            // ~10% of passengers in shared quarters
            ((passenger_count as f32 * 0.1) / 8.0).ceil().max(1.0) as u32
        }
        room_types::QUARTERS_CREW => {
            // Every crew member needs a berth (4 per quarters)
            (crew_count as f32 / 4.0).ceil() as u32
        }
        room_types::QUARTERS_OFFICER => {
            // ~10% of crew are officers (2 per quarters)
            ((crew_count as f32 * 0.1) / 2.0).ceil().max(1.0) as u32
        }
        // Food/dining scales with population
        room_types::MESS_HALL | room_types::GALLEY => {
            (spec.count as f32 * pop_ratio).ceil().max(1.0) as u32
        }
        // Shared facilities scale with population
        room_types::SHARED_BATHROOM | room_types::SHARED_LAUNDRY => {
            (spec.count as f32 * pop_ratio).ceil().max(2.0) as u32
        }
        // Medical scales with population
        room_types::HOSPITAL_WARD | room_types::MEDBAY => {
            (spec.count as f32 * pop_ratio).ceil().max(1.0) as u32
        }
        // Recreation scales with population
        room_types::CAFE | room_types::BAR | room_types::GAME_ROOM => {
            (spec.count as f32 * pop_ratio).ceil().max(1.0) as u32
        }
        // Infrastructure rooms: use manifest count (don't scale)
        _ => spec.count,
    }
}

pub(super) fn build_ship_graph(ctx: &ReducerContext, _deck_count: u32, crew_count: u32, passenger_count: u32) {
    let facility_manifest = get_facility_manifest();
    let total_pop = crew_count + passenger_count;

    // Expand manifest: one GraphNode per individual room instance.
    // Scale habitation room counts based on actual population.
    let mut node_ids: Vec<u64> = Vec::new();
    let mut node_groups: Vec<u8> = Vec::new();
    let mut node_functions: Vec<u8> = Vec::new();
    let mut node_zones: Vec<u8> = Vec::new();

    for spec in &facility_manifest {
        // Scale room counts for population-dependent room types
        let count = scale_room_count(spec, total_pop, crew_count, passenger_count);
        let area = spec.target_area;

        for i in 0..count {
            let name = if count == 1 {
                spec.name.to_string()
            } else {
                format!("{} {}", spec.name, i + 1)
            };
            let node = ctx.db.graph_node().insert(GraphNode {
                id: 0,
                node_type: node_types::ROOM,
                name,
                function: spec.room_type,
                capacity: spec.capacity,
                required_area: area,
                deck_preference: spec.deck_zone as i32,
                group: spec.group,
            });
            node_ids.push(node.id);
            node_groups.push(spec.group);
            node_functions.push(spec.room_type);
            node_zones.push(spec.deck_zone);
        }
    }

    // Intra-zone crew_path edges (connect rooms in same zone, sample to keep edge count manageable)
    for zone in 0..7u8 {
        let zone_ids: Vec<u64> = node_ids
            .iter()
            .zip(node_zones.iter())
            .filter(|(_, z)| **z == zone)
            .map(|(id, _)| *id)
            .collect();
        // Fully connect small groups; for large groups connect each to a few neighbors
        let threshold = 30;
        if zone_ids.len() <= threshold {
            for i in 0..zone_ids.len() {
                for j in (i + 1)..zone_ids.len() {
                    ctx.db.graph_edge().insert(GraphEdge {
                        id: 0,
                        from_node: zone_ids[i],
                        to_node: zone_ids[j],
                        edge_type: edge_types::CREW_PATH,
                        weight: 1.0,
                        bidirectional: true,
                    });
                }
            }
        } else {
            // Ring + short-range links
            for i in 0..zone_ids.len() {
                let next = (i + 1) % zone_ids.len();
                ctx.db.graph_edge().insert(GraphEdge {
                    id: 0,
                    from_node: zone_ids[i],
                    to_node: zone_ids[next],
                    edge_type: edge_types::CREW_PATH,
                    weight: 1.0,
                    bidirectional: true,
                });
                // Skip-3 link for connectivity
                let skip = (i + 3) % zone_ids.len();
                if skip != next && skip != i {
                    ctx.db.graph_edge().insert(GraphEdge {
                        id: 0,
                        from_node: zone_ids[i],
                        to_node: zone_ids[skip],
                        edge_type: edge_types::CREW_PATH,
                        weight: 1.0,
                        bidirectional: true,
                    });
                }
            }
        }
    }

    // Cross-zone crew paths: connect adjacent zones
    for z in 0..6u8 {
        let z_ids: Vec<u64> = node_ids
            .iter()
            .zip(node_zones.iter())
            .filter(|(_, zz)| **zz == z)
            .map(|(id, _)| *id)
            .collect();
        let z1_ids: Vec<u64> = node_ids
            .iter()
            .zip(node_zones.iter())
            .filter(|(_, zz)| **zz == z + 1)
            .map(|(id, _)| *id)
            .collect();
        if let (Some(&a), Some(&b)) = (z_ids.first(), z1_ids.first()) {
            ctx.db.graph_edge().insert(GraphEdge {
                id: 0,
                from_node: a,
                to_node: b,
                edge_type: edge_types::CREW_PATH,
                weight: 2.0,
                bidirectional: true,
            });
        }
        if let (Some(&a), Some(&b)) = (z_ids.last(), z1_ids.last()) {
            ctx.db.graph_edge().insert(GraphEdge {
                id: 0,
                from_node: a,
                to_node: b,
                edge_type: edge_types::CREW_PATH,
                weight: 2.0,
                bidirectional: true,
            });
        }
    }

    // Infrastructure edges
    let find_by_func = |func: u8| -> Option<u64> {
        node_ids
            .iter()
            .zip(node_functions.iter())
            .find(|(_, f)| **f == func)
            .map(|(id, _)| *id)
    };

    let reactor_node = find_by_func(room_types::REACTOR);
    let eng_node = find_by_func(room_types::ENGINEERING);
    let water_node = find_by_func(room_types::WATER_RECYCLING);
    let hvac_node = find_by_func(room_types::HVAC_CONTROL);
    let comms_node = find_by_func(room_types::COMMS_ROOM);
    let bridge_node = find_by_func(room_types::BRIDGE);
    let cic_node = find_by_func(room_types::CIC);

    // POWER: Reactor -> Engineering -> every other room
    if let (Some(reactor), Some(eng)) = (reactor_node, eng_node) {
        ctx.db.graph_edge().insert(GraphEdge {
            id: 0,
            from_node: reactor,
            to_node: eng,
            edge_type: edge_types::POWER,
            weight: 100.0,
            bidirectional: false,
        });
        for &nid in &node_ids {
            if nid != reactor && nid != eng {
                ctx.db.graph_edge().insert(GraphEdge {
                    id: 0,
                    from_node: eng,
                    to_node: nid,
                    edge_type: edge_types::POWER,
                    weight: 10.0,
                    bidirectional: false,
                });
            }
        }
    }

    // WATER: Water Recycling -> habitable rooms (sample to keep edge count sane)
    if let Some(water) = water_node {
        for &nid in &node_ids {
            if nid != water {
                let func = node_functions[node_ids.iter().position(|&x| x == nid).unwrap_or(0)];
                if room_types::is_quarters(func)
                    || room_types::is_dining(func)
                    || func == room_types::HYDROPONICS
                    || func == room_types::HOSPITAL_WARD
                {
                    ctx.db.graph_edge().insert(GraphEdge {
                        id: 0,
                        from_node: water,
                        to_node: nid,
                        edge_type: edge_types::WATER,
                        weight: 5.0,
                        bidirectional: false,
                    });
                }
            }
        }
    }

    // HVAC: HVAC Control -> all rooms (sample: only first 200 to keep manageable)
    if let Some(hvac) = hvac_node {
        let mut hvac_count = 0u32;
        for &nid in &node_ids {
            if nid != hvac && hvac_count < 200 {
                ctx.db.graph_edge().insert(GraphEdge {
                    id: 0,
                    from_node: hvac,
                    to_node: nid,
                    edge_type: edge_types::HVAC,
                    weight: 1.0,
                    bidirectional: false,
                });
                hvac_count += 1;
            }
        }
    }

    // DATA: Comms -> Bridge, CIC, Engineering
    if let Some(comms) = comms_node {
        let data_targets: Vec<u64> = [bridge_node, cic_node, eng_node]
            .iter()
            .filter_map(|n| *n)
            .collect();
        for &t in &data_targets {
            ctx.db.graph_edge().insert(GraphEdge {
                id: 0,
                from_node: comms,
                to_node: t,
                edge_type: edge_types::DATA,
                weight: 1.0,
                bidirectional: false,
            });
        }
    }
}
