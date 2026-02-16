//! Ring-and-spur ship layout generation (Wave 14).
//!
//! Pipeline: hull sizing → perimeter ring corridor → spine + cross-corridors as spurs →
//! shafts at intersections → segment identification → BSP room packing →
//! wavefront BFS gap fill → filler backfill → room-to-room doors.
//!
//! The ring corridor wraps the entire deck perimeter as a first-class public
//! walkway (same width as spine). Rooms fill rectangular segments between
//! corridors — every room touches at least one corridor by construction.

use super::doors::should_have_room_door;
use super::hull::{hull_length, hull_width};
use super::treemap::RoomRequest;
use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

// Grid cell type markers
const CELL_EMPTY: u8 = 0;
const CELL_MAIN_CORRIDOR: u8 = 1;
const CELL_SHAFT: u8 = 3;
const CELL_HULL: u8 = 4;
const CELL_ROOM_BASE: u8 = 10;

// Corridor geometry
const SPINE_WIDTH: usize = 3;
const CROSS_CORRIDOR_WIDTH: usize = 3;
const RING_WIDTH: usize = 3; // perimeter ring — same as spine
const SPUR_WIDTH: usize = 2; // spur corridors — narrower than spine
const MIN_ROOM_DIM: usize = 4;
const SPUR_THRESHOLD: usize = 12; // add spurs when segment wider than this

/// Filler room pool: used to backfill empty deck space after zone rooms are placed.
const FILLER_POOL: &[(u8, &str, f32, u32)] = &[
    (room_types::STORAGE, "Storage", 60.0, 0),
    (room_types::MAINTENANCE_BAY, "Maintenance Bay", 40.0, 4),
    (room_types::PARTS_STORAGE, "Parts Storage", 30.0, 0),
    (room_types::WORKSHOP, "Workshop", 35.0, 6),
    (room_types::UTILITY, "Utility Room", 20.0, 2),
    (room_types::EMERGENCY_SUPPLY, "Emergency Supply", 25.0, 0),
];

/// Returns true if room type is habitation (cabins, quarters, suites).
fn is_habitation(rt: u8) -> bool {
    matches!(
        rt,
        room_types::CABIN_SINGLE
            | room_types::CABIN_DOUBLE
            | room_types::FAMILY_SUITE
            | room_types::VIP_SUITE
            | room_types::QUARTERS_CREW
            | room_types::QUARTERS_OFFICER
            | room_types::QUARTERS_PASSENGER
            | room_types::SHARED_BATHROOM
            | room_types::SHARED_LAUNDRY
    )
}

/// Shaft definition for placement at corridor intersections.
struct ShaftPlacement {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    shaft_type: u8,
    name: &'static str,
    is_main: bool,
}

/// A rectangular segment between corridors where rooms can be placed.
/// Every room in a segment touches at least one corridor edge.
struct Segment {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    /// Which corridor the rooms should get their door to (spine/cross/ring)
    corridor_id: u32,
    /// Which wall side of the corridor the segment is on
    wall_side: u8,
}

pub(super) fn layout_ship(ctx: &ReducerContext, deck_count: u32, total_pop: u32) {
    let nodes: Vec<GraphNode> = ctx.db.graph_node().iter().collect();

    // ---- Compute shaft requirements from population ----
    let shaft_templates = compute_shaft_templates(total_pop);

    // ---- Hull sizing from total room area ----
    let total_area: f32 = nodes.iter().map(|n| n.required_area).sum();
    let max_room_area: f32 = nodes.iter().map(|n| n.required_area).fold(0.0f32, f32::max);
    let shaft_area_per_deck: f32 = shaft_templates
        .iter()
        .map(|(_, _, _, w, h)| (*w * *h) as f32)
        .sum();

    let deck_count = if deck_count == 0 {
        compute_optimal_deck_count(
            total_area,
            shaft_area_per_deck,
            &shaft_templates,
            max_room_area,
        )
    } else {
        deck_count
    };

    let room_area_per_deck = total_area / deck_count as f32;
    let (ship_beam, ship_length) =
        compute_hull_dimensions(room_area_per_deck, shaft_area_per_deck, max_room_area);
    log::info!(
        "Hull sizing: {:.0}m² total room area, {}×{} hull ({} decks, {} shafts, {:.0}m² shaft overhead/deck)",
        total_area, ship_beam, ship_length, deck_count, shaft_templates.len(), shaft_area_per_deck
    );

    // ---- Build per-zone room request lists ----
    let mut zone_requests: Vec<Vec<RoomRequest>> = vec![Vec::new(); 7];
    for node in &nodes {
        let zone = (node.deck_preference as u8).min(6);
        zone_requests[zone as usize].push(RoomRequest {
            node_id: node.id,
            name: node.name.clone(),
            room_type: node.function,
            target_area: node.required_area,
            capacity: node.capacity,
            group: node.group,
        });
    }
    for zr in zone_requests.iter_mut() {
        zr.sort_by(|a, b| {
            b.target_area
                .partial_cmp(&a.target_area)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
    }

    // ---- Demand-driven zone-to-deck assignment (outside-in) ----
    let est_strip_area = {
        let uw = ship_beam.saturating_sub(SPINE_WIDTH + 2 * RING_WIDTH);
        let nc = ((ship_length as f32 / 35.0).round() as usize).max(1);
        let ul = ship_length.saturating_sub(2 * RING_WIDTH + nc * CROSS_CORRIDOR_WIDTH);
        uw as f32 * ul as f32 * 0.8 - shaft_area_per_deck
    };
    let est_strip_area = est_strip_area.max(100.0);

    let zone_areas: Vec<f32> = zone_requests
        .iter()
        .map(|zr| zr.iter().map(|r| r.target_area).sum())
        .collect();
    let zone_decks_needed: Vec<u32> = zone_areas
        .iter()
        .map(|&a| {
            if a > 0.0 {
                (a / est_strip_area).ceil().max(1.0) as u32
            } else {
                0
            }
        })
        .collect();

    let mut deck_zone_map: Vec<u8> = vec![1; deck_count as usize];
    let dc = deck_count as usize;

    // Top: COMMAND
    let cmd_decks = zone_decks_needed[0].min(deck_count) as usize;
    for d in 0..cmd_decks.min(dc) {
        deck_zone_map[d] = 0;
    }
    // Bottom: ENGINEERING
    let eng_decks = zone_decks_needed[6].min(deck_count) as usize;
    for d in 0..eng_decks.min(dc) {
        let idx = dc - 1 - d;
        if deck_zone_map[idx] == 1 {
            deck_zone_map[idx] = 6;
        }
    }
    // Above engineering: CARGO
    let cargo_start = dc.saturating_sub(eng_decks);
    let cargo_decks = zone_decks_needed[5].min(deck_count) as usize;
    for d in 0..cargo_decks {
        let idx = cargo_start.saturating_sub(1 + d);
        if idx < dc && deck_zone_map[idx] == 1 {
            deck_zone_map[idx] = 5;
        }
    }
    // Below command: LIFE_SUPPORT
    let life_start = cmd_decks;
    let life_decks = zone_decks_needed[4].min(deck_count) as usize;
    for d in 0..life_decks {
        let idx = life_start + d;
        if idx < dc && deck_zone_map[idx] == 1 {
            deck_zone_map[idx] = 4;
        }
    }
    // REC and SVC in middle of remaining HAB slots
    let hab_slots: Vec<usize> = (0..dc).filter(|&d| deck_zone_map[d] == 1).collect();
    if !hab_slots.is_empty() {
        let rec_decks = zone_decks_needed[3].min(hab_slots.len() as u32) as usize;
        for d in 0..rec_decks {
            let idx = hab_slots[hab_slots.len() / 2 + d.min(hab_slots.len() - 1)];
            deck_zone_map[idx] = 3;
        }
        let svc_decks = zone_decks_needed[2].min(hab_slots.len() as u32) as usize;
        let hab_slots2: Vec<usize> = (0..dc).filter(|&d| deck_zone_map[d] == 1).collect();
        for d in 0..svc_decks {
            if let Some(&idx) = hab_slots2.get(hab_slots2.len() / 2 + d) {
                deck_zone_map[idx] = 2;
            }
        }
    }

    let zone_names = ["CMD", "HAB", "SVC", "REC", "LIFE", "CARGO", "ENG"];
    for (d, &z) in deck_zone_map.iter().enumerate() {
        log::info!("Deck {} → Zone {} ({})", d, z, zone_names[z as usize]);
    }

    let mut zone_cursors: Vec<usize> = vec![0; 7];

    // Count how many decks each zone has, for proportional distribution
    let mut zone_deck_counts: Vec<u32> = vec![0; 7];
    for &z in &deck_zone_map {
        zone_deck_counts[z as usize] += 1;
    }
    let mut zone_deck_seen: Vec<u32> = vec![0; 7];

    let mut room_id_counter: u32 = 0;
    let mut next_id = || {
        let id = room_id_counter;
        room_id_counter += 1;
        id
    };

    // Track shaft positions across decks for VerticalShaft entries
    struct ShaftInfo {
        name: &'static str,
        shaft_type: u8,
        is_main: bool,
        deck_room_ids: Vec<Option<u32>>,
        ref_x: f32,
        ref_y: f32,
        ref_w: f32,
        ref_h: f32,
    }

    let mut shaft_infos: Vec<ShaftInfo> = shaft_templates
        .iter()
        .map(|(name, st, is_main, w, h)| ShaftInfo {
            name,
            shaft_type: *st,
            is_main: *is_main,
            deck_room_ids: vec![None; deck_count as usize],
            ref_x: 0.0,
            ref_y: 0.0,
            ref_w: *w as f32,
            ref_h: *h as f32,
        })
        .collect();

    // ---- Compute global shaft positions from midship deck ----
    let mid_deck = deck_count / 2;
    let mid_hw = hull_width(mid_deck, deck_count, ship_beam);
    let mid_hl = hull_length(mid_deck, deck_count, ship_length);
    let mid_spine_left = mid_hw / 2 - SPINE_WIDTH / 2;
    let mid_spine_right = mid_spine_left + SPINE_WIDTH;
    let mid_num_cross = ((mid_hl as f32 / 35.0).round() as usize).max(1);
    let mid_cross_spacing = mid_hl / (mid_num_cross + 1);
    let mut mid_cross_ys: Vec<usize> = Vec::new();
    for i in 1..=mid_num_cross {
        let cy = i * mid_cross_spacing;
        if cy + CROSS_CORRIDOR_WIDTH <= mid_hl {
            mid_cross_ys.push(cy);
        }
    }
    let global_shaft_placements = compute_shaft_placements(
        &shaft_templates,
        mid_spine_right,
        mid_spine_left,
        &mid_cross_ys,
        mid_hw,
        mid_hl,
    );

    // ---- Per-deck generation ----
    let spine_left = mid_spine_left;
    let spine_right = mid_spine_right;

    for deck in 0..deck_count as i32 {
        let deck_hw = hull_width(deck as u32, deck_count, ship_beam);
        let deck_hl = hull_length(deck as u32, deck_count, ship_length);

        if deck_hw < 12 || deck_hl < 30 {
            log::warn!(
                "Deck {} too small ({}×{}), skipping",
                deck + 1,
                deck_hw,
                deck_hl
            );
            continue;
        }

        let hw = mid_hw;
        let hl = mid_hl;
        let mut grid: Vec<Vec<u8>> = vec![vec![CELL_EMPTY; hl]; hw];

        // Mask cells outside tapered hull
        let x_margin = (mid_hw - deck_hw) / 2;
        let y_margin = (mid_hl - deck_hl) / 2;
        for x in 0..hw {
            for y in 0..hl {
                if x < x_margin || x >= hw - x_margin || y < y_margin || y >= hl - y_margin {
                    grid[x][y] = CELL_HULL;
                }
            }
        }

        // ---- Phase 1: Ring corridor (perimeter) ----
        let ring_x0 = x_margin;
        let ring_x1 = hw - x_margin;
        let ring_y0 = y_margin;
        let ring_y1 = hl - y_margin;
        let inner_x0 = ring_x0 + RING_WIDTH;
        let inner_x1 = ring_x1.saturating_sub(RING_WIDTH);
        let inner_y0 = ring_y0 + RING_WIDTH;
        let inner_y1 = ring_y1.saturating_sub(RING_WIDTH);

        // Stamp ring cells
        for x in ring_x0..ring_x1 {
            for y in ring_y0..ring_y1 {
                if grid[x][y] == CELL_HULL {
                    continue;
                }
                let in_ring = x < inner_x0 || x >= inner_x1 || y < inner_y0 || y >= inner_y1;
                if in_ring {
                    grid[x][y] = CELL_MAIN_CORRIDOR;
                }
            }
        }

        // Ring Room entries (4 segments: N, S, W, E)
        let ring_n_id = next_id();
        let ring_n_w = (ring_x1 - ring_x0) as f32;
        ctx.db.room().insert(Room {
            id: ring_n_id,
            node_id: 0,
            name: format!("Ring North D{}", deck + 1),
            room_type: room_types::CORRIDOR,
            deck,
            x: ring_x0 as f32 + ring_n_w / 2.0,
            y: ring_y0 as f32 + RING_WIDTH as f32 / 2.0,
            width: ring_n_w,
            height: RING_WIDTH as f32,
            capacity: 0,
        });
        let ring_s_id = next_id();
        ctx.db.room().insert(Room {
            id: ring_s_id,
            node_id: 0,
            name: format!("Ring South D{}", deck + 1),
            room_type: room_types::CORRIDOR,
            deck,
            x: ring_x0 as f32 + ring_n_w / 2.0,
            y: (ring_y1 - RING_WIDTH) as f32 + RING_WIDTH as f32 / 2.0,
            width: ring_n_w,
            height: RING_WIDTH as f32,
            capacity: 0,
        });
        let ring_w_id = next_id();
        let ring_side_h = (inner_y1 - inner_y0) as f32;
        ctx.db.room().insert(Room {
            id: ring_w_id,
            node_id: 0,
            name: format!("Ring West D{}", deck + 1),
            room_type: room_types::CORRIDOR,
            deck,
            x: ring_x0 as f32 + RING_WIDTH as f32 / 2.0,
            y: inner_y0 as f32 + ring_side_h / 2.0,
            width: RING_WIDTH as f32,
            height: ring_side_h,
            capacity: 0,
        });
        let ring_e_id = next_id();
        ctx.db.room().insert(Room {
            id: ring_e_id,
            node_id: 0,
            name: format!("Ring East D{}", deck + 1),
            room_type: room_types::CORRIDOR,
            deck,
            x: (ring_x1 - RING_WIDTH) as f32 + RING_WIDTH as f32 / 2.0,
            y: inner_y0 as f32 + ring_side_h / 2.0,
            width: RING_WIDTH as f32,
            height: ring_side_h,
            capacity: 0,
        });

        // Ring corner doors (N↔W, N↔E, S↔W, S↔E) — use find_shared_edge for correct walls
        // Ring grid bounds (top-left corner, width, height):
        let ring_n_grid = (ring_x0, ring_y0, ring_x1 - ring_x0, RING_WIDTH);
        let ring_s_grid = (ring_x0, inner_y1, ring_x1 - ring_x0, RING_WIDTH);
        let ring_w_grid = (ring_x0, inner_y0, RING_WIDTH, inner_y1 - inner_y0);
        let ring_e_grid = (
            ring_x1 - RING_WIDTH,
            inner_y0,
            RING_WIDTH,
            inner_y1 - inner_y0,
        );
        for &(a_id, a_grid, b_id, b_grid) in &[
            (ring_n_id, ring_n_grid, ring_w_id, ring_w_grid),
            (ring_n_id, ring_n_grid, ring_e_id, ring_e_grid),
            (ring_s_id, ring_s_grid, ring_w_id, ring_w_grid),
            (ring_s_id, ring_s_grid, ring_e_id, ring_e_grid),
        ] {
            if let Some((dx, dy, wa, wb)) = find_shared_edge(
                a_grid.0, a_grid.1, a_grid.2, a_grid.3, b_grid.0, b_grid.1, b_grid.2, b_grid.3,
            ) {
                ctx.db.door().insert(Door {
                    id: 0,
                    room_a: a_id,
                    room_b: b_id,
                    wall_a: wa,
                    wall_b: wb,
                    position_along_wall: 0.5,
                    width: RING_WIDTH as f32,
                    access_level: access_levels::PUBLIC,
                    door_x: dx,
                    door_y: dy,
                });
            }
        }

        // Ring Corridor table entries
        ctx.db.corridor().insert(Corridor {
            id: 0,
            deck,
            corridor_type: corridor_types::MAIN,
            x: ring_x0 as f32,
            y: ring_y0 as f32,
            width: (ring_x1 - ring_x0) as f32,
            length: RING_WIDTH as f32,
            orientation: 0,
            carries: carries_flags::CREW_PATH,
        });

        // ---- Phase 2: Spine corridor (center spur) ----
        // Spine runs from ring-north inner edge to ring-south inner edge
        for x in spine_left..spine_right.min(hw) {
            for y in inner_y0..inner_y1 {
                if grid[x][y] == CELL_EMPTY {
                    grid[x][y] = CELL_MAIN_CORRIDOR;
                }
            }
        }

        // Spine segment Room entries (between cross-corridors)
        let cross_ys = &mid_cross_ys;
        let mut spine_segments: Vec<(u32, usize, usize)> = Vec::new();
        {
            let mut seg_boundaries: Vec<usize> = vec![inner_y0];
            for &cy in cross_ys.iter() {
                if cy >= inner_y0 && cy + CROSS_CORRIDOR_WIDTH <= inner_y1 {
                    seg_boundaries.push(cy);
                    seg_boundaries.push(cy + CROSS_CORRIDOR_WIDTH);
                }
            }
            seg_boundaries.push(inner_y1);

            for chunk in seg_boundaries.chunks(2) {
                if chunk.len() < 2 || chunk[0] >= chunk[1] {
                    continue;
                }
                let y0 = chunk[0];
                let y1 = chunk[1];
                let seg_id = next_id();
                ctx.db.room().insert(Room {
                    id: seg_id,
                    node_id: 0,
                    name: format!("Spine D{} Y{}-{}", deck + 1, y0, y1),
                    room_type: room_types::CORRIDOR,
                    deck,
                    x: spine_left as f32 + SPINE_WIDTH as f32 / 2.0,
                    y: y0 as f32 + (y1 - y0) as f32 / 2.0,
                    width: SPINE_WIDTH as f32,
                    height: (y1 - y0) as f32,
                    capacity: 0,
                });
                spine_segments.push((seg_id, y0, y1));
            }
        }

        ctx.db.corridor().insert(Corridor {
            id: 0,
            deck,
            corridor_type: corridor_types::MAIN,
            x: spine_left as f32,
            y: inner_y0 as f32,
            width: SPINE_WIDTH as f32,
            length: (inner_y1 - inner_y0) as f32,
            orientation: 1,
            carries: carries_flags::CREW_PATH | carries_flags::POWER | carries_flags::DATA,
        });

        // Cross-corridor Room entries (from ring-west to ring-east)
        let mut cross_rooms: Vec<(u32, usize)> = Vec::new();
        for &cy in cross_ys.iter() {
            if cy < inner_y0 || cy + CROSS_CORRIDOR_WIDTH > inner_y1 {
                continue;
            }
            let cc_x0 = inner_x0;
            let cc_x1 = inner_x1;
            let cc_w = cc_x1.saturating_sub(cc_x0);
            if cc_w < MIN_ROOM_DIM {
                continue;
            }
            for x in cc_x0..cc_x1 {
                for y in cy..cy + CROSS_CORRIDOR_WIDTH {
                    if y < hl && grid[x][y] == CELL_EMPTY {
                        grid[x][y] = CELL_MAIN_CORRIDOR;
                    }
                }
            }
            let cc_id = next_id();
            ctx.db.room().insert(Room {
                id: cc_id,
                node_id: 0,
                name: format!("Cross-Corridor D{} Y{}", deck + 1, cy),
                room_type: room_types::CROSS_CORRIDOR,
                deck,
                x: cc_x0 as f32 + cc_w as f32 / 2.0,
                y: cy as f32 + CROSS_CORRIDOR_WIDTH as f32 / 2.0,
                width: cc_w as f32,
                height: CROSS_CORRIDOR_WIDTH as f32,
                capacity: 0,
            });
            ctx.db.corridor().insert(Corridor {
                id: 0,
                deck,
                corridor_type: corridor_types::BRANCH,
                x: cc_x0 as f32,
                y: cy as f32,
                width: cc_w as f32,
                length: CROSS_CORRIDOR_WIDTH as f32,
                orientation: 0,
                carries: carries_flags::CREW_PATH,
            });
            cross_rooms.push((cc_id, cy));
        }

        // ---- Phase 2.5: Spur corridors into wide segments ----
        // When segments are too wide for rooms to reach from spine/ring,
        // add perpendicular spur corridors from the spine into each segment.
        let mut spur_rooms: Vec<(u32, usize, usize, usize, usize)> = Vec::new(); // (id, x, y, w, h)
        {
            // Build Y boundaries for segments (same logic as find_segments)
            let mut y_bounds: Vec<usize> = vec![inner_y0];
            for &cy in cross_ys.iter() {
                if cy >= inner_y0 && cy + CROSS_CORRIDOR_WIDTH <= inner_y1 {
                    y_bounds.push(cy);
                    y_bounds.push(cy + CROSS_CORRIDOR_WIDTH);
                }
            }
            y_bounds.push(inner_y1);

            for chunk in y_bounds.chunks(2) {
                if chunk.len() < 2 || chunk[0] >= chunk[1] {
                    continue;
                }
                let seg_y0 = chunk[0];
                let seg_y1 = chunk[1];
                let seg_h = seg_y1 - seg_y0;
                if seg_h < SPUR_WIDTH + 2 * MIN_ROOM_DIM {
                    continue; // too short for a spur + rooms on both sides
                }

                // Spur Y position: centered in segment
                let spur_y = seg_y0 + (seg_h - SPUR_WIDTH) / 2;

                // Port spur: from spine_left toward ring-west
                let port_w = spine_left.saturating_sub(inner_x0);
                if port_w > SPUR_THRESHOLD {
                    let spur_x = inner_x0;
                    let spur_len = port_w;
                    // Stamp grid
                    for x in spur_x..spur_x + spur_len {
                        for y in spur_y..spur_y + SPUR_WIDTH {
                            if x < hw && y < hl && grid[x][y] == CELL_EMPTY {
                                grid[x][y] = CELL_MAIN_CORRIDOR;
                            }
                        }
                    }
                    let spur_id = next_id();
                    ctx.db.room().insert(Room {
                        id: spur_id,
                        node_id: 0,
                        name: format!("Spur Port D{} Y{}", deck + 1, spur_y),
                        room_type: room_types::CORRIDOR,
                        deck,
                        x: spur_x as f32 + spur_len as f32 / 2.0,
                        y: spur_y as f32 + SPUR_WIDTH as f32 / 2.0,
                        width: spur_len as f32,
                        height: SPUR_WIDTH as f32,
                        capacity: 0,
                    });
                    spur_rooms.push((spur_id, spur_x, spur_y, spur_len, SPUR_WIDTH));
                }

                // Starboard spur: from spine_right toward ring-east
                let starb_w = inner_x1.saturating_sub(spine_right);
                if starb_w > SPUR_THRESHOLD {
                    let spur_x = spine_right;
                    let spur_len = starb_w;
                    for x in spur_x..spur_x + spur_len {
                        for y in spur_y..spur_y + SPUR_WIDTH {
                            if x < hw && y < hl && grid[x][y] == CELL_EMPTY {
                                grid[x][y] = CELL_MAIN_CORRIDOR;
                            }
                        }
                    }
                    let spur_id = next_id();
                    ctx.db.room().insert(Room {
                        id: spur_id,
                        node_id: 0,
                        name: format!("Spur Starb D{} Y{}", deck + 1, spur_y),
                        room_type: room_types::CORRIDOR,
                        deck,
                        x: spur_x as f32 + spur_len as f32 / 2.0,
                        y: spur_y as f32 + SPUR_WIDTH as f32 / 2.0,
                        width: spur_len as f32,
                        height: SPUR_WIDTH as f32,
                        capacity: 0,
                    });
                    spur_rooms.push((spur_id, spur_x, spur_y, spur_len, SPUR_WIDTH));
                }
            }

            // Create doors: spur ↔ spine, spur ↔ ring
            for &(spur_id, sx, sy, sw, sh) in &spur_rooms {
                // Spur ↔ spine
                for &(seg_id, seg_y0, seg_y1) in &spine_segments {
                    if let Some((dx, dy, wa, wb)) = find_shared_edge(
                        sx,
                        sy,
                        sw,
                        sh,
                        spine_left,
                        seg_y0,
                        SPINE_WIDTH,
                        seg_y1 - seg_y0,
                    ) {
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: spur_id,
                            room_b: seg_id,
                            wall_a: wa,
                            wall_b: wb,
                            position_along_wall: 0.5,
                            width: SPUR_WIDTH as f32,
                            access_level: access_levels::PUBLIC,
                            door_x: dx,
                            door_y: dy,
                        });
                        break;
                    }
                }
                // Spur ↔ ring west
                if let Some((dx, dy, wa, wb)) = find_shared_edge(
                    sx,
                    sy,
                    sw,
                    sh,
                    ring_x0,
                    ring_y0,
                    RING_WIDTH,
                    ring_y1 - ring_y0,
                ) {
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: spur_id,
                        room_b: ring_w_id,
                        wall_a: wa,
                        wall_b: wb,
                        position_along_wall: 0.5,
                        width: SPUR_WIDTH as f32,
                        access_level: access_levels::PUBLIC,
                        door_x: dx,
                        door_y: dy,
                    });
                }
                // Spur ↔ ring east
                if let Some((dx, dy, wa, wb)) = find_shared_edge(
                    sx,
                    sy,
                    sw,
                    sh,
                    ring_x1 - RING_WIDTH,
                    ring_y0,
                    RING_WIDTH,
                    ring_y1 - ring_y0,
                ) {
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: spur_id,
                        room_b: ring_e_id,
                        wall_a: wa,
                        wall_b: wb,
                        position_along_wall: 0.5,
                        width: SPUR_WIDTH as f32,
                        access_level: access_levels::PUBLIC,
                        door_x: dx,
                        door_y: dy,
                    });
                }
            }
        }

        // ---- Corridor-to-corridor doors ----

        // Spine ↔ cross-corridors
        for &(cc_id, cy) in &cross_rooms {
            for &(seg_id, seg_y0, seg_y1) in &spine_segments {
                if seg_y1 == cy {
                    let dx = spine_left as f32 + SPINE_WIDTH as f32 / 2.0;
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: seg_id,
                        room_b: cc_id,
                        wall_a: wall_sides::SOUTH,
                        wall_b: wall_sides::NORTH,
                        position_along_wall: 0.5,
                        width: SPINE_WIDTH as f32,
                        access_level: access_levels::PUBLIC,
                        door_x: dx,
                        door_y: cy as f32,
                    });
                }
                if seg_y0 == cy + CROSS_CORRIDOR_WIDTH {
                    let dx = spine_left as f32 + SPINE_WIDTH as f32 / 2.0;
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: cc_id,
                        room_b: seg_id,
                        wall_a: wall_sides::SOUTH,
                        wall_b: wall_sides::NORTH,
                        position_along_wall: 0.5,
                        width: SPINE_WIDTH as f32,
                        access_level: access_levels::PUBLIC,
                        door_x: dx,
                        door_y: (cy + CROSS_CORRIDOR_WIDTH) as f32,
                    });
                }
            }
        }

        // Consecutive spine segments
        for i in 0..spine_segments.len().saturating_sub(1) {
            let (seg_a, _, seg_a_end) = spine_segments[i];
            let (seg_b, seg_b_start, _) = spine_segments[i + 1];
            if seg_a_end == seg_b_start {
                ctx.db.door().insert(Door {
                    id: 0,
                    room_a: seg_a,
                    room_b: seg_b,
                    wall_a: wall_sides::SOUTH,
                    wall_b: wall_sides::NORTH,
                    position_along_wall: 0.5,
                    width: SPINE_WIDTH as f32,
                    access_level: access_levels::PUBLIC,
                    door_x: spine_left as f32 + SPINE_WIDTH as f32 / 2.0,
                    door_y: seg_a_end as f32,
                });
            }
        }

        // Spine ↔ ring (north and south ends)
        if let Some(&(first_seg, _, _)) = spine_segments.first() {
            ctx.db.door().insert(Door {
                id: 0,
                room_a: ring_n_id,
                room_b: first_seg,
                wall_a: wall_sides::SOUTH,
                wall_b: wall_sides::NORTH,
                position_along_wall: 0.5,
                width: SPINE_WIDTH as f32,
                access_level: access_levels::PUBLIC,
                door_x: spine_left as f32 + SPINE_WIDTH as f32 / 2.0,
                door_y: inner_y0 as f32,
            });
        }
        if let Some(&(last_seg, _, _)) = spine_segments.last() {
            ctx.db.door().insert(Door {
                id: 0,
                room_a: last_seg,
                room_b: ring_s_id,
                wall_a: wall_sides::SOUTH,
                wall_b: wall_sides::NORTH,
                position_along_wall: 0.5,
                width: SPINE_WIDTH as f32,
                access_level: access_levels::PUBLIC,
                door_x: spine_left as f32 + SPINE_WIDTH as f32 / 2.0,
                door_y: inner_y1 as f32,
            });
        }

        // Cross-corridors ↔ ring (west and east ends)
        for &(cc_id, cy) in &cross_rooms {
            let cc_mid_y = cy as f32 + CROSS_CORRIDOR_WIDTH as f32 / 2.0;
            // West end
            ctx.db.door().insert(Door {
                id: 0,
                room_a: ring_w_id,
                room_b: cc_id,
                wall_a: wall_sides::EAST,
                wall_b: wall_sides::WEST,
                position_along_wall: 0.5,
                width: CROSS_CORRIDOR_WIDTH as f32,
                access_level: access_levels::PUBLIC,
                door_x: inner_x0 as f32,
                door_y: cc_mid_y,
            });
            // East end
            ctx.db.door().insert(Door {
                id: 0,
                room_a: cc_id,
                room_b: ring_e_id,
                wall_a: wall_sides::EAST,
                wall_b: wall_sides::WEST,
                position_along_wall: 0.5,
                width: CROSS_CORRIDOR_WIDTH as f32,
                access_level: access_levels::PUBLIC,
                door_x: inner_x1 as f32,
                door_y: cc_mid_y,
            });
        }

        // ---- Phase 3: Stamp shafts ----
        for sp in &global_shaft_placements {
            if sp.x >= x_margin
                && sp.x + sp.w <= hw - x_margin
                && sp.y >= y_margin
                && sp.y + sp.h <= hl - y_margin
            {
                for sx in sp.x..((sp.x + sp.w).min(hw)) {
                    for sy in sp.y..((sp.y + sp.h).min(hl)) {
                        grid[sx][sy] = CELL_SHAFT;
                    }
                }
            }
        }

        // Shaft Room entries + doors to corridors
        for (global_idx, sp) in global_shaft_placements.iter().enumerate() {
            if sp.x < x_margin
                || sp.x + sp.w > hw - x_margin
                || sp.y < y_margin
                || sp.y + sp.h > hl - y_margin
            {
                continue;
            }
            let shaft_room_id = next_id();
            let srt = if sp.shaft_type == shaft_types::ELEVATOR
                || sp.shaft_type == shaft_types::SERVICE_ELEVATOR
            {
                room_types::ELEVATOR_SHAFT
            } else {
                room_types::LADDER_SHAFT
            };
            ctx.db.room().insert(Room {
                id: shaft_room_id,
                node_id: 0,
                name: format!("{} D{}", sp.name, deck + 1),
                room_type: srt,
                deck,
                x: sp.x as f32 + sp.w as f32 / 2.0,
                y: sp.y as f32 + sp.h as f32 / 2.0,
                width: sp.w as f32,
                height: sp.h as f32,
                capacity: 0,
            });

            if global_idx < shaft_infos.len() {
                shaft_infos[global_idx].deck_room_ids[deck as usize] = Some(shaft_room_id);
                shaft_infos[global_idx].ref_x = sp.x as f32 + sp.w as f32 / 2.0;
                shaft_infos[global_idx].ref_y = sp.y as f32 + sp.h as f32 / 2.0;
                shaft_infos[global_idx].ref_w = sp.w as f32;
                shaft_infos[global_idx].ref_h = sp.h as f32;
            }

            let access = if sp.is_main {
                access_levels::PUBLIC
            } else {
                access_levels::CREW_ONLY
            };
            connect_shaft_to_corridor(
                ctx,
                shaft_room_id,
                sp,
                &spine_segments,
                &cross_rooms,
                &spur_rooms,
                [ring_n_id, ring_s_id, ring_w_id, ring_e_id],
                [ring_n_grid, ring_s_grid, ring_w_grid, ring_e_grid],
                spine_left,
                inner_x0,
                inner_x1,
                access,
            );
        }

        // ---- Phase 4: Identify rectangular segments between corridors ----
        let segments = find_segments(
            &grid,
            hw,
            hl,
            spine_left,
            spine_right,
            inner_x0,
            inner_x1,
            inner_y0,
            inner_y1,
            cross_ys,
            &spine_segments,
            &cross_rooms,
            ring_w_id,
            ring_e_id,
            ring_n_id,
            ring_s_id,
        );

        // ---- Phase 5: Collect room requests for this deck (proportional fill) ----
        let primary_zone = deck_zone_map[deck as usize] as usize;
        let mut deck_requests: Vec<RoomRequest> = Vec::new();
        let total_seg_area: usize = segments.iter().map(|s| s.w * s.h).sum();
        let mut filled_area = 0.0f32;
        let area_budget = total_seg_area as f32 * 0.95;

        // Proportional distribution: divide remaining rooms across remaining decks for this zone
        let zone_decks_total = zone_deck_counts[primary_zone];
        let zone_deck_num = zone_deck_seen[primary_zone]; // 0-indexed
        zone_deck_seen[primary_zone] += 1;
        let remaining_zone_decks = zone_decks_total - zone_deck_num;

        let cursor = &mut zone_cursors[primary_zone];
        let remaining_rooms = zone_requests[primary_zone].len() - *cursor;
        // Each deck gets at most ceil(remaining / remaining_decks) rooms
        let fair_share = if remaining_zone_decks > 0 {
            remaining_rooms.div_ceil(remaining_zone_decks as usize)
        } else {
            remaining_rooms
        };
        let mut taken = 0usize;
        while *cursor < zone_requests[primary_zone].len()
            && filled_area < area_budget
            && taken < fair_share
        {
            let req = zone_requests[primary_zone][*cursor].clone();
            filled_area += req.target_area;
            deck_requests.push(req);
            *cursor += 1;
            taken += 1;
        }

        if filled_area < area_budget * 0.7 {
            let overflow_order = [1u8, 2, 3, 4, 5, 0, 6];
            for &oz in &overflow_order {
                if oz as usize == primary_zone {
                    continue;
                }
                let oc = &mut zone_cursors[oz as usize];
                while *oc < zone_requests[oz as usize].len() && filled_area < area_budget {
                    let req = zone_requests[oz as usize][*oc].clone();
                    filled_area += req.target_area;
                    deck_requests.push(req);
                    *oc += 1;
                }
                if filled_area >= area_budget {
                    break;
                }
            }
        }

        deck_requests.sort_by(|a, b| {
            b.target_area
                .partial_cmp(&a.target_area)
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        // ---- Phase 6: BSP room placement into segments ----
        let mut placed_rooms: Vec<(u32, usize, usize, usize, usize, u8)> = Vec::new();
        let mut request_idx = 0;
        let total_request_area: f32 = deck_requests.iter().map(|r| r.target_area).sum();

        // Sort segments largest-first so big rooms get big segments
        let mut seg_order: Vec<usize> = (0..segments.len()).collect();
        seg_order.sort_by(|&a, &b| {
            let area_a = segments[a].w * segments[a].h;
            let area_b = segments[b].w * segments[b].h;
            area_b.cmp(&area_a)
        });

        for &si in &seg_order {
            if request_idx >= deck_requests.len() {
                break;
            }
            let seg = &segments[si];
            let mut sub_rects: Vec<(usize, usize, usize, usize)> = Vec::new();
            bsp_subdivide(
                seg.x,
                seg.y,
                seg.w,
                seg.h,
                &deck_requests[request_idx..],
                &mut sub_rects,
            );

            for (rx, ry, rw, rh) in &sub_rects {
                if request_idx >= deck_requests.len() {
                    break;
                }
                // Check for conflicts
                let mut has_conflict = false;
                for gx in *rx..(*rx + *rw).min(hw) {
                    for gy in *ry..(*ry + *rh).min(hl) {
                        if grid[gx][gy] != CELL_EMPTY {
                            has_conflict = true;
                            break;
                        }
                    }
                    if has_conflict {
                        break;
                    }
                }
                if has_conflict {
                    continue;
                }

                // Only place if room touches a corridor
                if !touches_any_corridor(
                    *rx,
                    *ry,
                    *rw,
                    *rh,
                    spine_left,
                    &spine_segments,
                    &cross_rooms,
                    &spur_rooms,
                    inner_x0,
                    inner_x1,
                    inner_y1,
                    ring_x0,
                    ring_x1,
                    ring_y0,
                    ring_y1,
                ) {
                    continue;
                }

                let req = &deck_requests[request_idx];
                let room_id = next_id();

                for gx in *rx..(*rx + *rw).min(hw) {
                    for gy in *ry..(*ry + *rh).min(hl) {
                        if grid[gx][gy] == CELL_EMPTY {
                            grid[gx][gy] = CELL_ROOM_BASE + (room_id as u8 % 246);
                        }
                    }
                }

                ctx.db.room().insert(Room {
                    id: room_id,
                    node_id: req.node_id,
                    name: req.name.clone(),
                    room_type: req.room_type,
                    deck,
                    x: *rx as f32 + *rw as f32 / 2.0,
                    y: *ry as f32 + *rh as f32 / 2.0,
                    width: *rw as f32,
                    height: *rh as f32,
                    capacity: req.capacity,
                });

                create_corridor_door(
                    ctx,
                    room_id,
                    *rx,
                    *ry,
                    *rw,
                    *rh,
                    spine_left,
                    spine_right,
                    &spine_segments,
                    &cross_rooms,
                    &spur_rooms,
                    inner_x0,
                    inner_x1,
                    inner_y0,
                    inner_y1,
                    ring_x0,
                    ring_x1,
                    ring_y0,
                    ring_y1,
                    ring_n_id,
                    ring_s_id,
                    ring_w_id,
                    ring_e_id,
                );

                placed_rooms.push((room_id, *rx, *ry, *rw, *rh, req.room_type));
                request_idx += 1;
            }
        }

        // ---- Phase 7: Wavefront BFS gap fill ----
        // Grow corridor into remaining empty cells, place rooms from remaining requests
        {
            let remaining_requests: Vec<RoomRequest> = if request_idx < deck_requests.len() {
                deck_requests[request_idx..].to_vec()
            } else {
                Vec::new()
            };
            let mut req_cursor = 0usize;

            // BFS frontier: all corridor cells adjacent to empty cells
            let mut frontier: Vec<(usize, usize)> = Vec::new();
            for x in inner_x0..inner_x1 {
                for y in inner_y0..inner_y1 {
                    if grid[x][y] == CELL_MAIN_CORRIDOR {
                        // Check if any neighbor is empty
                        for &(dx, dy) in &[(0isize, 1isize), (0, -1), (1, 0), (-1, 0)] {
                            let nx = (x as isize + dx) as usize;
                            let ny = (y as isize + dy) as usize;
                            if nx < hw && ny < hl && grid[nx][ny] == CELL_EMPTY {
                                frontier.push((x, y));
                                break;
                            }
                        }
                    }
                }
            }

            // Expand from each frontier position: find largest empty rect touching corridor
            let mut wave_placed = 0u32;
            let mut visited: Vec<Vec<bool>> = vec![vec![false; hl]; hw];

            for &(fx, fy) in &frontier {
                if req_cursor >= remaining_requests.len() {
                    break;
                }
                // For each direction from this corridor cell, try to find an empty rectangle
                for &(dx, dy) in &[(0isize, 1isize), (0, -1), (1, 0), (-1, 0)] {
                    let start_x = (fx as isize + dx) as usize;
                    let start_y = (fy as isize + dy) as usize;
                    if start_x >= hw
                        || start_y >= hl
                        || grid[start_x][start_y] != CELL_EMPTY
                        || visited[start_x][start_y]
                    {
                        continue;
                    }

                    // Expand to largest empty rectangle from this cell
                    let mut max_w = 0usize;
                    for ddx in 0..(inner_x1 - start_x) {
                        if start_x + ddx >= hw || grid[start_x + ddx][start_y] != CELL_EMPTY {
                            break;
                        }
                        max_w = ddx + 1;
                    }
                    let mut max_h = inner_y1 - start_y;
                    for ddy in 0..max_h {
                        if start_y + ddy >= hl {
                            max_h = ddy;
                            break;
                        }
                        let row_clear = (0..max_w).all(|ddx| {
                            start_x + ddx < hw && grid[start_x + ddx][start_y + ddy] == CELL_EMPTY
                        });
                        if !row_clear {
                            max_h = ddy;
                            break;
                        }
                    }

                    if max_w >= MIN_ROOM_DIM
                        && max_h >= MIN_ROOM_DIM
                        && req_cursor < remaining_requests.len()
                    {
                        let req = &remaining_requests[req_cursor];
                        let target_side = (req.target_area.sqrt() as usize).max(MIN_ROOM_DIM);
                        let rw = max_w.min(target_side.max(MIN_ROOM_DIM));
                        let rh = max_h.min(
                            ((req.target_area / rw as f32).ceil() as usize)
                                .max(MIN_ROOM_DIM)
                                .min(max_h),
                        );

                        if rw >= MIN_ROOM_DIM
                            && rh >= MIN_ROOM_DIM
                            && touches_any_corridor(
                                start_x,
                                start_y,
                                rw,
                                rh,
                                spine_left,
                                &spine_segments,
                                &cross_rooms,
                                &spur_rooms,
                                inner_x0,
                                inner_x1,
                                inner_y1,
                                ring_x0,
                                ring_x1,
                                ring_y0,
                                ring_y1,
                            )
                        {
                            let room_id = next_id();
                            for gx in start_x..(start_x + rw).min(hw) {
                                for gy in start_y..(start_y + rh).min(hl) {
                                    grid[gx][gy] = CELL_ROOM_BASE + (room_id as u8 % 246);
                                    visited[gx][gy] = true;
                                }
                            }
                            ctx.db.room().insert(Room {
                                id: room_id,
                                node_id: req.node_id,
                                name: req.name.clone(),
                                room_type: req.room_type,
                                deck,
                                x: start_x as f32 + rw as f32 / 2.0,
                                y: start_y as f32 + rh as f32 / 2.0,
                                width: rw as f32,
                                height: rh as f32,
                                capacity: req.capacity,
                            });

                            create_corridor_door(
                                ctx,
                                room_id,
                                start_x,
                                start_y,
                                rw,
                                rh,
                                spine_left,
                                spine_right,
                                &spine_segments,
                                &cross_rooms,
                                &spur_rooms,
                                inner_x0,
                                inner_x1,
                                inner_y0,
                                inner_y1,
                                ring_x0,
                                ring_x1,
                                ring_y0,
                                ring_y1,
                                ring_n_id,
                                ring_s_id,
                                ring_w_id,
                                ring_e_id,
                            );

                            placed_rooms.push((room_id, start_x, start_y, rw, rh, req.room_type));
                            req_cursor += 1;
                            wave_placed += 1;
                        }
                    }
                }
            }
            if wave_placed > 0 {
                log::info!("Deck {}: wavefront placed {} rooms", deck + 1, wave_placed);
            }
        }

        // ---- Phase 8: Filler backfill ----
        {
            let mut filler_idx = 0usize;
            let mut filler_count = 0u32;

            let mut y = inner_y0;
            while y < inner_y1 {
                let mut x = inner_x0;
                while x < inner_x1 {
                    if grid[x][y] != CELL_EMPTY {
                        x += 1;
                        continue;
                    }
                    // Expand to largest empty rectangle
                    let mut max_w = 0;
                    for dx in 0..(inner_x1 - x) {
                        if grid[x + dx][y] != CELL_EMPTY {
                            break;
                        }
                        max_w = dx + 1;
                    }
                    let mut max_h = inner_y1 - y;
                    for dy in 0..max_h {
                        let row_clear = (0..max_w).all(|dx| grid[x + dx][y + dy] == CELL_EMPTY);
                        if !row_clear {
                            max_h = dy;
                            break;
                        }
                    }

                    if max_w >= MIN_ROOM_DIM && max_h >= MIN_ROOM_DIM {
                        let (frt, fname, ftarget, fcap) =
                            FILLER_POOL[filler_idx % FILLER_POOL.len()];
                        filler_idx += 1;
                        let target_side = (ftarget.sqrt() as usize).max(MIN_ROOM_DIM);
                        let rw = max_w.min(target_side.max(MIN_ROOM_DIM));
                        let rh = max_h.min(
                            ((ftarget / rw as f32).ceil() as usize)
                                .max(MIN_ROOM_DIM)
                                .min(max_h),
                        );

                        if rw >= MIN_ROOM_DIM
                            && rh >= MIN_ROOM_DIM
                            && touches_any_corridor(
                                x,
                                y,
                                rw,
                                rh,
                                spine_left,
                                &spine_segments,
                                &cross_rooms,
                                &spur_rooms,
                                inner_x0,
                                inner_x1,
                                inner_y1,
                                ring_x0,
                                ring_x1,
                                ring_y0,
                                ring_y1,
                            )
                        {
                            filler_count += 1;
                            let room_id = next_id();
                            for gx in x..(x + rw) {
                                for gy in y..(y + rh) {
                                    grid[gx][gy] = CELL_ROOM_BASE + (room_id as u8 % 246);
                                }
                            }
                            ctx.db.room().insert(Room {
                                id: room_id,
                                node_id: 0,
                                name: format!("{} {}", fname, filler_count),
                                room_type: frt,
                                deck,
                                x: x as f32 + rw as f32 / 2.0,
                                y: y as f32 + rh as f32 / 2.0,
                                width: rw as f32,
                                height: rh as f32,
                                capacity: fcap,
                            });
                            placed_rooms.push((room_id, x, y, rw, rh, frt));

                            create_corridor_door(
                                ctx,
                                room_id,
                                x,
                                y,
                                rw,
                                rh,
                                spine_left,
                                spine_right,
                                &spine_segments,
                                &cross_rooms,
                                &spur_rooms,
                                inner_x0,
                                inner_x1,
                                inner_y0,
                                inner_y1,
                                ring_x0,
                                ring_x1,
                                ring_y0,
                                ring_y1,
                                ring_n_id,
                                ring_s_id,
                                ring_w_id,
                                ring_e_id,
                            );
                        }
                    }
                    x += max_w.max(1);
                }
                y += 1;
            }
            if filler_count > 0 {
                log::info!("Deck {}: placed {} filler rooms", deck + 1, filler_count);
            }
        }

        // ---- Phase 8.5: Room expansion into empty space ----
        // Try to grow each placed room in all 4 directions into adjacent empty cells.
        // This reduces gaps without adding new rooms.
        {
            let mut expanded = 0u32;
            for i in 0..placed_rooms.len() {
                let (room_id, mut rx, mut ry, mut rw, mut rh, _rt) = placed_rooms[i];
                let cell_tag = CELL_ROOM_BASE + (room_id as u8 % 246);
                let mut changed = true;
                while changed {
                    changed = false;
                    // Try expand east (+x)
                    let new_x1 = rx + rw;
                    if new_x1 < inner_x1 {
                        let col_clear = (ry..ry + rh).all(|y| grid[new_x1][y] == CELL_EMPTY);
                        if col_clear {
                            for y in ry..ry + rh {
                                grid[new_x1][y] = cell_tag;
                            }
                            rw += 1;
                            changed = true;
                        }
                    }
                    // Try expand west (-x)
                    if rx > inner_x0 {
                        let col_clear = (ry..ry + rh).all(|y| grid[rx - 1][y] == CELL_EMPTY);
                        if col_clear {
                            rx -= 1;
                            for y in ry..ry + rh {
                                grid[rx][y] = cell_tag;
                            }
                            rw += 1;
                            changed = true;
                        }
                    }
                    // Try expand south (+y)
                    let new_y1 = ry + rh;
                    if new_y1 < inner_y1 {
                        let row_clear = (rx..rx + rw).all(|x| grid[x][new_y1] == CELL_EMPTY);
                        if row_clear {
                            for x in rx..rx + rw {
                                grid[x][new_y1] = cell_tag;
                            }
                            rh += 1;
                            changed = true;
                        }
                    }
                    // Try expand north (-y)
                    if ry > inner_y0 {
                        let row_clear = (rx..rx + rw).all(|x| grid[x][ry - 1] == CELL_EMPTY);
                        if row_clear {
                            ry -= 1;
                            for x in rx..rx + rw {
                                grid[x][ry] = cell_tag;
                            }
                            rh += 1;
                            changed = true;
                        }
                    }
                }
                if (rx, ry, rw, rh)
                    != (
                        placed_rooms[i].1,
                        placed_rooms[i].2,
                        placed_rooms[i].3,
                        placed_rooms[i].4,
                    )
                {
                    expanded += 1;
                    placed_rooms[i].1 = rx;
                    placed_rooms[i].2 = ry;
                    placed_rooms[i].3 = rw;
                    placed_rooms[i].4 = rh;
                    // Update Room entry in DB
                    if let Some(mut room) = ctx.db.room().id().find(room_id) {
                        room.x = rx as f32 + rw as f32 / 2.0;
                        room.y = ry as f32 + rh as f32 / 2.0;
                        room.width = rw as f32;
                        room.height = rh as f32;
                        ctx.db.room().id().update(room);
                    }
                }
            }
            if expanded > 0 {
                log::info!(
                    "Deck {}: expanded {} rooms into empty space",
                    deck + 1,
                    expanded
                );
            }
        }

        // ---- Phase 9: Room-to-room doors (adjacent logical pairs) ----
        for i in 0..placed_rooms.len() {
            for j in (i + 1)..placed_rooms.len() {
                let (id_a, ax, ay, aw, ah, rt_a) = placed_rooms[i];
                let (id_b, bx, by, bw, bh, rt_b) = placed_rooms[j];
                if !should_have_room_door(rt_a, rt_b) {
                    continue;
                }
                if let Some((dx, dy, wa, wb)) = find_shared_edge(ax, ay, aw, ah, bx, by, bw, bh) {
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: id_a,
                        room_b: id_b,
                        wall_a: wa,
                        wall_b: wb,
                        position_along_wall: 0.5,
                        width: 3.0,
                        access_level: access_levels::PUBLIC,
                        door_x: dx,
                        door_y: dy,
                    });
                }
            }
        }

        // ---- Grid dump (debug) ----
        let max_rows = 60;
        let mut dump = format!(
            "Deck {} grid ({}x{}, {} rooms, {} segments, {} cross-corridors):\n",
            deck + 1,
            hw,
            hl,
            placed_rooms.len(),
            segments.len(),
            cross_ys.len(),
        );
        for y in 0..hl.min(max_rows) {
            for x in 0..hw {
                let ch = match grid[x][y] {
                    CELL_EMPTY => '.',
                    CELL_MAIN_CORRIDOR => '=',
                    CELL_SHAFT => '#',
                    CELL_HULL => ' ',
                    v if v >= CELL_ROOM_BASE => {
                        let idx = (v - CELL_ROOM_BASE) as usize;
                        (b'A' + (idx % 26) as u8) as char
                    }
                    _ => '?',
                };
                dump.push(ch);
            }
            dump.push('\n');
        }
        if hl > max_rows {
            dump.push_str(&format!("... ({} more rows)\n", hl - max_rows));
        }
        log::info!("{}", dump);
        log::info!(
            "Deck {}: placed {}/{} rooms ({:.0}/{:.0}m² area, {} segment area available)",
            deck + 1,
            request_idx.min(deck_requests.len()),
            deck_requests.len(),
            deck_requests
                .iter()
                .take(request_idx)
                .map(|r| r.target_area)
                .sum::<f32>(),
            total_request_area,
            total_seg_area,
        );
    } // end per-deck loop

    // ---- VerticalShaft entries + cross-deck doors ----
    for si in &shaft_infos {
        let placed_decks: Vec<String> = si
            .deck_room_ids
            .iter()
            .enumerate()
            .filter_map(|(d, opt)| opt.map(|_| d.to_string()))
            .collect();
        if placed_decks.is_empty() {
            continue;
        }
        ctx.db.vertical_shaft().insert(VerticalShaft {
            id: 0,
            shaft_type: si.shaft_type,
            name: si.name.to_string(),
            x: si.ref_x,
            y: si.ref_y,
            decks_served: placed_decks.join(","),
            width: si.ref_w,
            height: si.ref_h,
        });

        let access = if si.is_main {
            access_levels::PUBLIC
        } else {
            access_levels::CREW_ONLY
        };
        for d in 0..deck_count.saturating_sub(1) {
            if let (Some(room_a), Some(room_b)) = (
                si.deck_room_ids[d as usize],
                si.deck_room_ids[(d + 1) as usize],
            ) {
                if let (Some(ra), Some(rb)) = (
                    ctx.db.room().id().find(room_a),
                    ctx.db.room().id().find(room_b),
                ) {
                    let mid_x = (ra.x + ra.width / 2.0 + rb.x + rb.width / 2.0) / 2.0;
                    let mid_y = (ra.y + ra.height / 2.0 + rb.y + rb.height / 2.0) / 2.0;
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a,
                        room_b,
                        wall_a: wall_sides::SOUTH,
                        wall_b: wall_sides::NORTH,
                        position_along_wall: 0.5,
                        width: 3.0,
                        access_level: access,
                        door_x: mid_x,
                        door_y: mid_y,
                    });
                }
            }
        }
    }

    let total_rooms: usize = ctx.db.room().iter().count();
    let total_doors: usize = ctx.db.door().iter().count();
    log::info!(
        "Layout complete: {} rooms, {} doors across {} decks",
        total_rooms,
        total_doors,
        deck_count
    );
}

// ---- Helper functions ----

/// Identify rectangular segments between corridors where rooms can be placed.
/// Each segment is bounded by the ring, spine, and/or cross-corridors.
#[allow(clippy::too_many_arguments)]
fn find_segments(
    grid: &[Vec<u8>],
    _hw: usize,
    _hl: usize,
    spine_left: usize,
    spine_right: usize,
    inner_x0: usize,
    inner_x1: usize,
    inner_y0: usize,
    inner_y1: usize,
    cross_ys: &[usize],
    spine_segments: &[(u32, usize, usize)],
    cross_rooms: &[(u32, usize)],
    ring_w_id: u32,
    ring_e_id: u32,
    _ring_n_id: u32,
    _ring_s_id: u32,
) -> Vec<Segment> {
    let mut segments = Vec::new();

    // Y boundaries: inner_y0, cross-corridor edges, inner_y1
    let mut y_bounds: Vec<usize> = vec![inner_y0];
    for &cy in cross_ys {
        if cy >= inner_y0 && cy + CROSS_CORRIDOR_WIDTH <= inner_y1 {
            y_bounds.push(cy);
            y_bounds.push(cy + CROSS_CORRIDOR_WIDTH);
        }
    }
    y_bounds.push(inner_y1);

    for chunk in y_bounds.chunks(2) {
        if chunk.len() < 2 || chunk[0] >= chunk[1] {
            continue;
        }
        let seg_y0 = chunk[0];
        let seg_y1 = chunk[1];
        let seg_h = seg_y1 - seg_y0;
        if seg_h < MIN_ROOM_DIM {
            continue;
        }

        // Port side: ring-west inner edge to spine-left
        let port_x0 = inner_x0;
        let port_x1 = spine_left;
        if port_x1 > port_x0 {
            let port_w = port_x1 - port_x0;
            if port_w >= MIN_ROOM_DIM {
                // Find the corridor this segment touches
                let corridor_id =
                    find_corridor_for_y(seg_y0, seg_y1, spine_segments, cross_rooms, ring_w_id);
                // Split around shafts
                let sub_rects = find_clear_rects_in_region(grid, port_x0, port_x1, seg_y0, seg_y1);
                for (rx, ry, rw, rh) in sub_rects {
                    if rw >= MIN_ROOM_DIM && rh >= MIN_ROOM_DIM {
                        // Determine which corridor edge this sub-rect touches
                        let (cid, ws) = if rx + rw == spine_left {
                            // Touches spine — door on east wall
                            let sid = find_spine_segment(seg_y0, seg_y1, spine_segments);
                            (sid, wall_sides::EAST)
                        } else if rx == inner_x0 {
                            // Touches ring west — door on west wall
                            (ring_w_id, wall_sides::WEST)
                        } else {
                            (corridor_id, wall_sides::EAST)
                        };
                        segments.push(Segment {
                            x: rx,
                            y: ry,
                            w: rw,
                            h: rh,
                            corridor_id: cid,
                            wall_side: ws,
                        });
                    }
                }
            }
        }

        // Starboard side: spine-right to ring-east inner edge
        let stbd_x0 = spine_right;
        let stbd_x1 = inner_x1;
        if stbd_x1 > stbd_x0 {
            let stbd_w = stbd_x1 - stbd_x0;
            if stbd_w >= MIN_ROOM_DIM {
                let corridor_id =
                    find_corridor_for_y(seg_y0, seg_y1, spine_segments, cross_rooms, ring_e_id);
                let sub_rects = find_clear_rects_in_region(grid, stbd_x0, stbd_x1, seg_y0, seg_y1);
                for (rx, ry, rw, rh) in sub_rects {
                    if rw >= MIN_ROOM_DIM && rh >= MIN_ROOM_DIM {
                        let (cid, ws) = if rx == spine_right {
                            let sid = find_spine_segment(seg_y0, seg_y1, spine_segments);
                            (sid, wall_sides::WEST)
                        } else if rx + rw == inner_x1 {
                            (ring_e_id, wall_sides::EAST)
                        } else {
                            (corridor_id, wall_sides::WEST)
                        };
                        segments.push(Segment {
                            x: rx,
                            y: ry,
                            w: rw,
                            h: rh,
                            corridor_id: cid,
                            wall_side: ws,
                        });
                    }
                }
            }
        }
    }

    // Sort: spine-touching segments first, then by area descending
    segments.sort_by(|a, b| {
        let a_spine = if a.x + a.w == spine_left || a.x == spine_right {
            0
        } else {
            1
        };
        let b_spine = if b.x + b.w == spine_left || b.x == spine_right {
            0
        } else {
            1
        };
        let spine_cmp = a_spine.cmp(&b_spine);
        if spine_cmp != core::cmp::Ordering::Equal {
            return spine_cmp;
        }
        (b.w * b.h).cmp(&(a.w * a.h))
    });

    segments
}

/// Find the spine segment whose Y range overlaps [y0, y1).
fn find_spine_segment(y0: usize, y1: usize, spine_segments: &[(u32, usize, usize)]) -> u32 {
    let mid_y = (y0 + y1) / 2;
    for &(seg_id, seg_y0, seg_y1) in spine_segments {
        if mid_y >= seg_y0 && mid_y < seg_y1 {
            return seg_id;
        }
    }
    spine_segments.first().map(|s| s.0).unwrap_or(0)
}

/// Find a corridor ID for a Y range (spine segment or cross-corridor).
fn find_corridor_for_y(
    y0: usize,
    y1: usize,
    spine_segments: &[(u32, usize, usize)],
    cross_rooms: &[(u32, usize)],
    ring_fallback: u32,
) -> u32 {
    // Try spine
    let mid_y = (y0 + y1) / 2;
    for &(seg_id, seg_y0, seg_y1) in spine_segments {
        if mid_y >= seg_y0 && mid_y < seg_y1 {
            return seg_id;
        }
    }
    // Try cross-corridor
    for &(cc_id, cy) in cross_rooms {
        if y0 <= cy + CROSS_CORRIDOR_WIDTH && y1 >= cy {
            return cc_id;
        }
    }
    ring_fallback
}

/// Find clear rectangles in a region, splitting around shaft obstacles.
fn find_clear_rects_in_region(
    grid: &[Vec<u8>],
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
) -> Vec<(usize, usize, usize, usize)> {
    let mut results = Vec::new();
    if x1 <= x0 || y1 <= y0 {
        return results;
    }

    // Scan columns for clear runs, splitting around obstacles
    let mut col_start = x0;
    let mut x = x0;
    while x <= x1 {
        let col_has_obstacle = if x < x1 {
            (y0..y1).any(|y| y >= grid[x].len() || grid[x][y] != CELL_EMPTY)
        } else {
            true
        };

        if col_has_obstacle {
            let run_w = x - col_start;
            if run_w >= MIN_ROOM_DIM {
                results.push((col_start, y0, run_w, y1 - y0));
            }

            if x < x1 {
                let obs_col_start = x;
                let mut obs_end = x + 1;
                while obs_end < x1 {
                    let has_obs = (y0..y1)
                        .any(|y| y >= grid[obs_end].len() || grid[obs_end][y] != CELL_EMPTY);
                    if !has_obs {
                        break;
                    }
                    obs_end += 1;
                }
                let obs_w = obs_end - obs_col_start;

                // Find clear Y sub-ranges in obstacle columns
                let mut row_start = y0;
                let mut ry = y0;
                while ry <= y1 {
                    let row_clear = if ry < y1 {
                        (obs_col_start..obs_end)
                            .all(|cx| ry < grid[cx].len() && grid[cx][ry] == CELL_EMPTY)
                    } else {
                        false
                    };
                    if !row_clear {
                        let run_h = ry - row_start;
                        if run_h >= MIN_ROOM_DIM && obs_w >= MIN_ROOM_DIM {
                            results.push((obs_col_start, row_start, obs_w, run_h));
                        }
                        row_start = ry + 1;
                    }
                    ry += 1;
                }

                x = obs_end;
                col_start = obs_end;
                continue;
            }
            col_start = x + 1;
        }
        x += 1;
    }

    results
}

/// Check if a rectangle touches any corridor (spine, cross-corridor, or ring).
/// Pure geometry check — no side effects.
#[allow(clippy::too_many_arguments)]
fn touches_any_corridor(
    rx: usize,
    ry: usize,
    rw: usize,
    rh: usize,
    spine_left: usize,
    spine_segments: &[(u32, usize, usize)],
    cross_rooms: &[(u32, usize)],
    spur_rooms: &[(u32, usize, usize, usize, usize)],
    inner_x0: usize,
    inner_x1: usize,
    inner_y1: usize,
    ring_x0: usize,
    ring_x1: usize,
    ring_y0: usize,
    ring_y1: usize,
) -> bool {
    for &(_, seg_y0, seg_y1) in spine_segments {
        if find_shared_edge(
            rx,
            ry,
            rw,
            rh,
            spine_left,
            seg_y0,
            SPINE_WIDTH,
            seg_y1 - seg_y0,
        )
        .is_some()
        {
            return true;
        }
    }
    for &(_, cy) in cross_rooms {
        if find_shared_edge(
            rx,
            ry,
            rw,
            rh,
            inner_x0,
            cy,
            inner_x1 - inner_x0,
            CROSS_CORRIDOR_WIDTH,
        )
        .is_some()
        {
            return true;
        }
    }
    for &(_, sx, sy, sw, sh) in spur_rooms {
        if find_shared_edge(rx, ry, rw, rh, sx, sy, sw, sh).is_some() {
            return true;
        }
    }
    let ring_checks: [(usize, usize, usize, usize); 4] = [
        (ring_x0, ring_y0, RING_WIDTH, ring_y1 - ring_y0),
        (ring_x1 - RING_WIDTH, ring_y0, RING_WIDTH, ring_y1 - ring_y0),
        (ring_x0, ring_y0, ring_x1 - ring_x0, RING_WIDTH),
        (ring_x0, inner_y1, ring_x1 - ring_x0, RING_WIDTH),
    ];
    for (rcx, rcy, rcw, rch) in ring_checks {
        if find_shared_edge(rx, ry, rw, rh, rcx, rcy, rcw, rch).is_some() {
            return true;
        }
    }
    false
}

/// Try to create a door from a room to the nearest corridor it actually touches.
/// Returns true if a door was created.
#[allow(clippy::too_many_arguments)]
fn create_corridor_door(
    ctx: &ReducerContext,
    room_id: u32,
    rx: usize,
    ry: usize,
    rw: usize,
    rh: usize,
    spine_left: usize,
    _spine_right: usize,
    spine_segments: &[(u32, usize, usize)],
    cross_rooms: &[(u32, usize)],
    spur_rooms: &[(u32, usize, usize, usize, usize)],
    inner_x0: usize,
    inner_x1: usize,
    _inner_y0: usize,
    inner_y1: usize,
    ring_x0: usize,
    ring_x1: usize,
    ring_y0: usize,
    ring_y1: usize,
    ring_n_id: u32,
    ring_s_id: u32,
    ring_w_id: u32,
    ring_e_id: u32,
) -> bool {
    // Try spine segments
    for &(seg_id, seg_y0, seg_y1) in spine_segments {
        if let Some((dx, dy, wa, wb)) = find_shared_edge(
            rx,
            ry,
            rw,
            rh,
            spine_left,
            seg_y0,
            SPINE_WIDTH,
            seg_y1 - seg_y0,
        ) {
            ctx.db.door().insert(Door {
                id: 0,
                room_a: room_id,
                room_b: seg_id,
                wall_a: wa,
                wall_b: wb,
                position_along_wall: 0.5,
                width: 2.0_f32.min(rw as f32).min(rh as f32),
                access_level: access_levels::PUBLIC,
                door_x: dx,
                door_y: dy,
            });
            return true;
        }
    }
    // Try cross-corridors
    for &(cr_id, cy) in cross_rooms {
        if let Some((dx, dy, wa, wb)) = find_shared_edge(
            rx,
            ry,
            rw,
            rh,
            inner_x0,
            cy,
            inner_x1 - inner_x0,
            CROSS_CORRIDOR_WIDTH,
        ) {
            ctx.db.door().insert(Door {
                id: 0,
                room_a: room_id,
                room_b: cr_id,
                wall_a: wa,
                wall_b: wb,
                position_along_wall: 0.5,
                width: 2.0_f32.min(rw as f32).min(rh as f32),
                access_level: access_levels::PUBLIC,
                door_x: dx,
                door_y: dy,
            });
            return true;
        }
    }
    // Try spur corridors
    for &(spur_id, sx, sy, sw, sh) in spur_rooms {
        if let Some((dx, dy, wa, wb)) = find_shared_edge(rx, ry, rw, rh, sx, sy, sw, sh) {
            ctx.db.door().insert(Door {
                id: 0,
                room_a: room_id,
                room_b: spur_id,
                wall_a: wa,
                wall_b: wb,
                position_along_wall: 0.5,
                width: 2.0_f32.min(rw as f32).min(rh as f32),
                access_level: access_levels::PUBLIC,
                door_x: dx,
                door_y: dy,
            });
            return true;
        }
    }
    // Try ring corridors
    let ring_checks: [(u32, usize, usize, usize, usize); 4] = [
        (ring_w_id, ring_x0, ring_y0, RING_WIDTH, ring_y1 - ring_y0),
        (
            ring_e_id,
            ring_x1 - RING_WIDTH,
            ring_y0,
            RING_WIDTH,
            ring_y1 - ring_y0,
        ),
        (ring_n_id, ring_x0, ring_y0, ring_x1 - ring_x0, RING_WIDTH),
        (ring_s_id, ring_x0, inner_y1, ring_x1 - ring_x0, RING_WIDTH),
    ];
    for (rid, rcx, rcy, rcw, rch) in ring_checks {
        if let Some((dx, dy, wa, wb)) = find_shared_edge(rx, ry, rw, rh, rcx, rcy, rcw, rch) {
            ctx.db.door().insert(Door {
                id: 0,
                room_a: room_id,
                room_b: rid,
                wall_a: wa,
                wall_b: wb,
                position_along_wall: 0.5,
                width: 2.0_f32.min(rw as f32).min(rh as f32),
                access_level: access_levels::PUBLIC,
                door_x: dx,
                door_y: dy,
            });
            return true;
        }
    }
    false
}

/// BSP subdivide a rectangle into sub-rectangles for room packing.
/// Splits in both X and Y directions — chooses the longer axis so rooms
/// don't become impossibly thin.
fn bsp_subdivide(
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    requests: &[RoomRequest],
    out: &mut Vec<(usize, usize, usize, usize)>,
) {
    if requests.is_empty() || w < MIN_ROOM_DIM || h < MIN_ROOM_DIM {
        return;
    }

    if requests.len() == 1 {
        let target = requests[0].target_area;
        let max_area = target * 1.5;
        let actual_area = (w * h) as f32;
        if actual_area <= max_area || (w <= MIN_ROOM_DIM + 1 && h <= MIN_ROOM_DIM + 1) {
            out.push((x, y, w, h));
        } else if w > h {
            let new_w = ((target / h as f32) as usize).max(MIN_ROOM_DIM).min(w);
            out.push((x, y, new_w, h));
        } else {
            let new_h = ((target / w as f32) as usize).max(MIN_ROOM_DIM).min(h);
            out.push((x, y, w, new_h));
        }
        return;
    }

    let split_at = requests.len() / 2;
    let area_ratio = requests[..split_at]
        .iter()
        .map(|r| r.target_area)
        .sum::<f32>()
        / requests.iter().map(|r| r.target_area).sum::<f32>();

    // Split along the longer axis for better packing
    if h >= w {
        let split_h = (h as f32 * area_ratio).round() as usize;
        let split_h = split_h
            .max(MIN_ROOM_DIM)
            .min(h.saturating_sub(MIN_ROOM_DIM));
        if split_h >= MIN_ROOM_DIM && h - split_h >= MIN_ROOM_DIM {
            bsp_subdivide(x, y, w, split_h, &requests[..split_at], out);
            bsp_subdivide(x, y + split_h, w, h - split_h, &requests[split_at..], out);
        } else {
            out.push((x, y, w, h));
        }
    } else {
        let split_w = (w as f32 * area_ratio).round() as usize;
        let split_w = split_w
            .max(MIN_ROOM_DIM)
            .min(w.saturating_sub(MIN_ROOM_DIM));
        if split_w >= MIN_ROOM_DIM && w - split_w >= MIN_ROOM_DIM {
            bsp_subdivide(x, y, split_w, h, &requests[..split_at], out);
            bsp_subdivide(x + split_w, y, w - split_w, h, &requests[split_at..], out);
        } else {
            out.push((x, y, w, h));
        }
    }
}

/// Connect a shaft room to the nearest corridor via a door.
#[allow(clippy::too_many_arguments)]
fn connect_shaft_to_corridor(
    ctx: &ReducerContext,
    shaft_room_id: u32,
    sp: &ShaftPlacement,
    spine_segments: &[(u32, usize, usize)],
    cross_rooms: &[(u32, usize)],
    spur_rooms: &[(u32, usize, usize, usize, usize)],
    ring_ids: [u32; 4],
    ring_grids: [(usize, usize, usize, usize); 4],
    spine_left: usize,
    inner_x0: usize,
    inner_x1: usize,
    access: u8,
) {
    let sx = sp.x;
    let sy = sp.y;
    let sw = sp.w;
    let sh = sp.h;

    // Try cross-corridors (full inner width)
    for &(cc_id, cy) in cross_rooms {
        let cc_w = inner_x1 - inner_x0;
        if let Some((dx, dy, wa, wb)) =
            find_shared_edge(sx, sy, sw, sh, inner_x0, cy, cc_w, CROSS_CORRIDOR_WIDTH)
        {
            ctx.db.door().insert(Door {
                id: 0,
                room_a: shaft_room_id,
                room_b: cc_id,
                wall_a: wa,
                wall_b: wb,
                position_along_wall: 0.5,
                width: sw.min(sh) as f32,
                access_level: access,
                door_x: dx,
                door_y: dy,
            });
            return;
        }
    }

    // Try spine segments
    for &(seg_id, seg_y0, seg_y1) in spine_segments {
        let seg_h = seg_y1 - seg_y0;
        if let Some((dx, dy, wa, wb)) =
            find_shared_edge(sx, sy, sw, sh, spine_left, seg_y0, SPINE_WIDTH, seg_h)
        {
            ctx.db.door().insert(Door {
                id: 0,
                room_a: shaft_room_id,
                room_b: seg_id,
                wall_a: wa,
                wall_b: wb,
                position_along_wall: 0.5,
                width: sw.min(sh) as f32,
                access_level: access,
                door_x: dx,
                door_y: dy,
            });
            return;
        }
    }

    // Try spur corridors
    for &(spur_id, spur_x, spur_y, spur_w, spur_h) in spur_rooms {
        if let Some((dx, dy, wa, wb)) =
            find_shared_edge(sx, sy, sw, sh, spur_x, spur_y, spur_w, spur_h)
        {
            ctx.db.door().insert(Door {
                id: 0,
                room_a: shaft_room_id,
                room_b: spur_id,
                wall_a: wa,
                wall_b: wb,
                position_along_wall: 0.5,
                width: sw.min(sh) as f32,
                access_level: access,
                door_x: dx,
                door_y: dy,
            });
            return;
        }
    }

    // Try ring corridors (N, S, W, E)
    for i in 0..4 {
        let (rx, ry, rw, rh) = ring_grids[i];
        if let Some((dx, dy, wa, wb)) = find_shared_edge(sx, sy, sw, sh, rx, ry, rw, rh) {
            ctx.db.door().insert(Door {
                id: 0,
                room_a: shaft_room_id,
                room_b: ring_ids[i],
                wall_a: wa,
                wall_b: wb,
                position_along_wall: 0.5,
                width: sw.min(sh) as f32,
                access_level: access,
                door_x: dx,
                door_y: dy,
            });
            return;
        }
    }

    log::warn!(
        "Shaft {} at ({},{}) {}x{} has no adjacent corridor",
        sp.name,
        sx,
        sy,
        sw,
        sh
    );
}

/// Find shared edge between two axis-aligned rectangles.
#[allow(clippy::too_many_arguments)]
fn find_shared_edge(
    ax: usize,
    ay: usize,
    aw: usize,
    ah: usize,
    bx: usize,
    by: usize,
    bw: usize,
    bh: usize,
) -> Option<(f32, f32, u8, u8)> {
    if ax + aw == bx {
        let oy0 = ay.max(by);
        let oy1 = (ay + ah).min(by + bh);
        if oy1 > oy0 {
            return Some((
                (ax + aw) as f32,
                (oy0 + oy1) as f32 / 2.0,
                wall_sides::EAST,
                wall_sides::WEST,
            ));
        }
    }
    if bx + bw == ax {
        let oy0 = ay.max(by);
        let oy1 = (ay + ah).min(by + bh);
        if oy1 > oy0 {
            return Some((
                ax as f32,
                (oy0 + oy1) as f32 / 2.0,
                wall_sides::WEST,
                wall_sides::EAST,
            ));
        }
    }
    if ay + ah == by {
        let ox0 = ax.max(bx);
        let ox1 = (ax + aw).min(bx + bw);
        if ox1 > ox0 {
            return Some((
                (ox0 + ox1) as f32 / 2.0,
                (ay + ah) as f32,
                wall_sides::SOUTH,
                wall_sides::NORTH,
            ));
        }
    }
    if by + bh == ay {
        let ox0 = ax.max(bx);
        let ox1 = (ax + aw).min(bx + bw);
        if ox1 > ox0 {
            return Some((
                (ox0 + ox1) as f32 / 2.0,
                ay as f32,
                wall_sides::NORTH,
                wall_sides::SOUTH,
            ));
        }
    }
    None
}

/// Compute shaft templates scaled to population.
fn compute_shaft_templates(total_pop: u32) -> Vec<(&'static str, u8, bool, usize, usize)> {
    let main_count = (total_pop as f32 / 200.0).ceil().max(2.0) as usize;
    let svc_count = (total_pop as f32 / 500.0).ceil().max(1.0) as usize;
    let ladder_count = (total_pop as f32 / 500.0).ceil().max(2.0) as usize;

    const MAIN_NAMES: &[&str] = &[
        "Fore Elevator",
        "Aft Elevator",
        "Midship Elevator",
        "Elevator 4",
        "Elevator 5",
        "Elevator 6",
        "Elevator 7",
        "Elevator 8",
        "Elevator 9",
        "Elevator 10",
        "Elevator 11",
        "Elevator 12",
        "Elevator 13",
        "Elevator 14",
        "Elevator 15",
        "Elevator 16",
        "Elevator 17",
        "Elevator 18",
        "Elevator 19",
        "Elevator 20",
        "Elevator 21",
        "Elevator 22",
        "Elevator 23",
        "Elevator 24",
        "Elevator 25",
        "Elevator 26",
        "Elevator 27",
        "Elevator 28",
    ];
    const SVC_NAMES: &[&str] = &[
        "Service Elevator A",
        "Service Elevator B",
        "Service Elevator C",
        "Service Elevator D",
        "Service Elevator E",
        "Service Elevator F",
        "Service Elevator G",
        "Service Elevator H",
        "Service Elevator I",
        "Service Elevator J",
    ];
    const LADDER_NAMES: &[&str] = &[
        "Ladder A", "Ladder B", "Ladder C", "Ladder D", "Ladder E", "Ladder F", "Ladder G",
        "Ladder H", "Ladder I", "Ladder J",
    ];

    let mut templates = Vec::new();
    for i in 0..main_count.min(MAIN_NAMES.len()) {
        templates.push((MAIN_NAMES[i], shaft_types::ELEVATOR, true, 3, 3));
    }
    for i in 0..svc_count.min(SVC_NAMES.len()) {
        templates.push((SVC_NAMES[i], shaft_types::SERVICE_ELEVATOR, false, 2, 2));
    }
    for i in 0..ladder_count.min(LADDER_NAMES.len()) {
        templates.push((LADDER_NAMES[i], shaft_types::LADDER, false, 2, 2));
    }

    log::info!(
        "Shaft templates for {} people: {} main elevators, {} service, {} ladders",
        total_pop,
        main_count,
        svc_count,
        ladder_count
    );
    templates
}

/// Compute shaft placements by distributing templates across cross-corridor intersections.
fn compute_shaft_placements(
    templates: &[(&'static str, u8, bool, usize, usize)],
    spine_right: usize,
    spine_left: usize,
    cross_ys: &[usize],
    hw: usize,
    hl: usize,
) -> Vec<ShaftPlacement> {
    let mut placements = Vec::new();
    let cross_end_offset = CROSS_CORRIDOR_WIDTH;

    if cross_ys.is_empty() {
        if let Some((name, st, is_main, w, h)) = templates.first() {
            placements.push(ShaftPlacement {
                x: spine_right,
                y: hl / 4,
                w: *w,
                h: *h,
                shaft_type: *st,
                name,
                is_main: *is_main,
            });
        }
        return placements;
    }

    let main_elevators: Vec<_> = templates
        .iter()
        .filter(|(_, st, _, _, _)| *st == shaft_types::ELEVATOR)
        .collect();
    let service_elevators: Vec<_> = templates
        .iter()
        .filter(|(_, st, _, _, _)| *st == shaft_types::SERVICE_ELEVATOR)
        .collect();
    let ladders: Vec<_> = templates
        .iter()
        .filter(|(_, st, _, _, _)| *st == shaft_types::LADDER)
        .collect();

    let num_positions = cross_ys.len();

    // Main elevators: starboard of spine
    for (i, (name, st, is_main, w, h)) in main_elevators.iter().enumerate() {
        let cross_idx = if main_elevators.len() <= num_positions {
            i * num_positions / main_elevators.len()
        } else {
            i % num_positions
        };
        let cy = cross_ys[cross_idx.min(num_positions - 1)];
        let stack_offset = if main_elevators.len() > num_positions {
            (i / num_positions) * *h
        } else {
            0
        };
        placements.push(ShaftPlacement {
            x: spine_right,
            y: cy + cross_end_offset + stack_offset,
            w: *w,
            h: *h,
            shaft_type: *st,
            name,
            is_main: *is_main,
        });
    }

    // Service elevators: near cross-corridor intersections, offset from spine
    // (moved inward from hull edge since ring replaces service corridor)
    let svc_x = spine_right + 3 + 1; // just past main elevator
    if svc_x + 2 < hw {
        for (i, (name, st, is_main, w, h)) in service_elevators.iter().enumerate() {
            let cross_idx = if service_elevators.len() <= num_positions {
                (i * num_positions / service_elevators.len().max(1)).min(num_positions - 1)
            } else {
                i % num_positions
            };
            let cy = cross_ys[cross_idx];
            placements.push(ShaftPlacement {
                x: svc_x,
                y: cy + cross_end_offset,
                w: *w,
                h: *h,
                shaft_type: *st,
                name,
                is_main: *is_main,
            });
        }
    }

    // Ladders: port side of spine
    for (i, (name, st, is_main, w, h)) in ladders.iter().enumerate() {
        let cross_idx = if ladders.len() <= num_positions {
            (i * num_positions / ladders.len().max(1)).min(num_positions - 1)
        } else {
            i % num_positions
        };
        let cy = cross_ys[cross_idx];
        placements.push(ShaftPlacement {
            x: spine_left.saturating_sub(*w),
            y: cy + cross_end_offset,
            w: *w,
            h: *h,
            shaft_type: *st,
            name,
            is_main: *is_main,
        });
    }

    placements
}

/// Compute hull dimensions using iterative overhead calculation.
fn compute_hull_dimensions(
    room_area_per_deck: f32,
    shaft_area_per_deck: f32,
    max_room_area: f32,
) -> (usize, usize) {
    let aspect_ratio = 3.5f32;
    let mut mult = 1.4f32;

    // Minimum beam: largest room must fit in segment between ring and spine
    // segment_width = (beam - SPINE_WIDTH - 2*RING_WIDTH) / 2
    let min_strip_w = (max_room_area.sqrt()).max(MIN_ROOM_DIM as f32);
    let min_beam = (min_strip_w * 2.0 + SPINE_WIDTH as f32 + 2.0 * RING_WIDTH as f32).max(30.0);

    for _ in 0..5 {
        let apd = room_area_per_deck * mult;
        let b = (apd.sqrt() / aspect_ratio.sqrt()).max(min_beam);
        let l = (apd / b).max(100.0);

        let num_cross = (l / 35.0).round().max(1.0);
        // Ring (3-wide perimeter) + spine + cross-corridors
        let corridor_area = SPINE_WIDTH as f32 * l
            + num_cross * CROSS_CORRIDOR_WIDTH as f32 * b
            + 2.0 * (b + l) * RING_WIDTH as f32;

        let actual_need = room_area_per_deck + corridor_area + shaft_area_per_deck;
        let new_mult = actual_need / room_area_per_deck;

        if (new_mult - mult).abs() < 0.01 {
            break;
        }
        mult = new_mult;
    }

    let apd = room_area_per_deck * mult;
    let b = (apd.sqrt() / aspect_ratio.sqrt()).max(min_beam) as usize;
    let l = (apd / b as f32).max(100.0) as usize;
    (b, l)
}

/// Auto-compute optimal deck count.
fn compute_optimal_deck_count(
    total_room_area: f32,
    shaft_area_per_deck: f32,
    shaft_templates: &[(&'static str, u8, bool, usize, usize)],
    max_room_area: f32,
) -> u32 {
    let num_banks = shaft_templates
        .iter()
        .filter(|(_, st, _, _, _)| *st == shaft_types::ELEVATOR)
        .count()
        .max(1);

    let mut best = 7u32;
    for d in 7..=30u32 {
        let room_per_deck = total_room_area / d as f32;
        let (b, l) = compute_hull_dimensions(room_per_deck, shaft_area_per_deck, max_room_area);

        let num_cross = (l as f32 / 35.0).round().max(1.0) as usize;
        let usable_width = b.saturating_sub(SPINE_WIDTH + 2 * RING_WIDTH);
        let usable_length = l.saturating_sub(2 * RING_WIDTH) - num_cross * CROSS_CORRIDOR_WIDTH;
        let strip_area = usable_width as f32 * usable_length as f32 * 0.8 - shaft_area_per_deck;

        let fill = if strip_area > 0.0 {
            room_per_deck / strip_area
        } else {
            99.0
        };
        let max_walk = l as f32 / (2.0 * num_banks as f32) + b as f32 / 2.0;

        if fill <= 0.90 && max_walk <= 50.0 {
            best = d;
            log::info!(
                "Auto deck count: {} decks ({}x{}, fill {:.0}%, walk {:.0}m)",
                d,
                b,
                l,
                fill * 100.0,
                max_walk
            );
            break;
        }
        best = d;
    }
    best
}
