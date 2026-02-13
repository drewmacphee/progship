//! Corridor-first ship layout generation.
//!
//! Pipeline: hull sizing → corridor skeleton → shafts at intersections →
//! attachment strip scanning → BSP room packing → perimeter service corridor
//! → room-to-room doors.
//! Rooms are ONLY placed adjacent to corridors, guaranteeing connectivity
//! by construction (no BFS cleanup needed).
//! The service corridor wraps around the perimeter of all placed content
//! rather than occupying a fixed starboard strip.

use super::doors::should_have_room_door;
use super::hull::{hull_length, hull_width};
use super::treemap::RoomRequest;
use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

// Grid cell type markers
const CELL_EMPTY: u8 = 0;
const CELL_MAIN_CORRIDOR: u8 = 1;

/// Returns true if room type is habitation (cabins, quarters, suites).
/// These rooms should NOT have doors to the service corridor.
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
const CELL_SERVICE_CORRIDOR: u8 = 2;
const CELL_SHAFT: u8 = 3;
const CELL_HULL: u8 = 4; // Outside hull boundary (tapered decks)
const CELL_ROOM_BASE: u8 = 10;

// Corridor geometry
const SPINE_WIDTH: usize = 3;
const CROSS_CORRIDOR_WIDTH: usize = 3;
const SVC_CORRIDOR_WIDTH: usize = 2;
const MIN_ROOM_DIM: usize = 4;

/// Filler room pool: used to backfill empty deck space after zone rooms are placed.
/// (room_type, name_prefix, target_area, capacity)
const FILLER_POOL: &[(u8, &str, f32, u32)] = &[
    (room_types::STORAGE, "Storage", 60.0, 0),
    (room_types::MAINTENANCE_BAY, "Maintenance Bay", 40.0, 4),
    (room_types::PARTS_STORAGE, "Parts Storage", 30.0, 0),
    (room_types::WORKSHOP, "Workshop", 35.0, 6),
    (room_types::UTILITY, "Utility Room", 20.0, 2),
    (room_types::EMERGENCY_SUPPLY, "Emergency Supply", 25.0, 0),
];

/// An attachment strip: a rectangular area of empty cells directly adjacent to a corridor wall.
/// Rooms can only be placed within attachment strips, guaranteeing corridor adjacency.
struct AttachmentStrip {
    corridor_room_id: u32,
    /// Grid coordinates of the strip
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    /// Which wall side of the corridor this strip is on
    wall_side: u8,
    /// Where the door should go (corridor-adjacent edge)
    door_x: usize,
    door_y: usize,
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

    // Auto-compute deck count if 0, otherwise use provided value
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

    // Iterative hull sizing: compute actual overhead instead of fixed 1.4×
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
    // Sort: largest rooms first for better packing
    for zr in zone_requests.iter_mut() {
        zr.sort_by(|a, b| {
            b.target_area
                .partial_cmp(&a.target_area)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
    }

    // ---- Demand-driven zone-to-deck assignment (outside-in) ----
    // Compute usable strip area per deck for demand estimation
    let est_strip_area = {
        let uw = ship_beam.saturating_sub(SPINE_WIDTH + 2 * SVC_CORRIDOR_WIDTH);
        let nc = ((ship_length as f32 / 35.0).round() as usize).max(1);
        let ul = ship_length.saturating_sub(2 * SVC_CORRIDOR_WIDTH + nc * CROSS_CORRIDOR_WIDTH);
        uw as f32 * ul as f32 * 0.8 - shaft_area_per_deck
    };
    let est_strip_area = est_strip_area.max(100.0);

    // Zone areas and deck requirements
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

    // Outside-in ordering: CMD=0 (top), ENG=6 (bottom), CARGO=5 (above eng),
    // LIFE=4 (below cmd), REC=3 (above cargo), SVC=2 (middle), HAB=1 (fills remaining)
    // Assign from extremes inward:
    let mut deck_zone_map: Vec<u8> = vec![1; deck_count as usize]; // default: HAB
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

    // Find remaining HAB slots, assign REC and SVC to middle
    let hab_slots: Vec<usize> = (0..dc).filter(|&d| deck_zone_map[d] == 1).collect();
    if !hab_slots.is_empty() {
        // REC in the upper-middle of HAB region
        let rec_decks = zone_decks_needed[3].min(hab_slots.len() as u32) as usize;
        for d in 0..rec_decks {
            let idx = hab_slots[hab_slots.len() / 2 + d.min(hab_slots.len() - 1)];
            deck_zone_map[idx] = 3;
        }
        // SVC just below/above REC
        let svc_decks = zone_decks_needed[2].min(hab_slots.len() as u32) as usize;
        let hab_slots2: Vec<usize> = (0..dc).filter(|&d| deck_zone_map[d] == 1).collect();
        for d in 0..svc_decks {
            if let Some(&idx) = hab_slots2.get(hab_slots2.len() / 2 + d) {
                deck_zone_map[idx] = 2;
            }
        }
    }

    // Log assignment
    let zone_names = ["CMD", "HAB", "SVC", "REC", "LIFE", "CARGO", "ENG"];
    for (d, &z) in deck_zone_map.iter().enumerate() {
        log::info!("Deck {} → Zone {} ({})", d, z, zone_names[z as usize]);
    }

    // Build per-zone cursor for greedy fill
    let mut zone_cursors: Vec<usize> = vec![0; 7];

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
        // Per-deck room IDs for cross-deck doors
        deck_room_ids: Vec<Option<u32>>,
        // Reference position (from largest deck)
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
    // Shafts must be at identical positions across all decks for vertical alignment.
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
    // All decks use midship grid dimensions so corridors and shafts
    // are at identical absolute positions. Tapered decks mask cells
    // outside their hull boundary as CELL_HULL.
    let spine_left = mid_spine_left;
    let spine_right = mid_spine_right;

    for deck in 0..deck_count as i32 {
        let deck_hw: usize = hull_width(deck as u32, deck_count, ship_beam);
        let deck_hl: usize = hull_length(deck as u32, deck_count, ship_length);

        if deck_hw < 12 || deck_hl < 30 {
            log::warn!(
                "Deck {} too small ({}×{}), skipping",
                deck + 1,
                deck_hw,
                deck_hl
            );
            continue;
        }

        // Grid always uses full midship dimensions
        let hw = mid_hw;
        let hl = mid_hl;
        let mut grid: Vec<Vec<u8>> = vec![vec![CELL_EMPTY; hl]; hw];

        // Mask cells outside the tapered hull boundary.
        // Tapered decks are centered within the midship grid.
        let x_margin = (mid_hw - deck_hw) / 2;
        let y_margin = (mid_hl - deck_hl) / 2;
        for x in 0..hw {
            for y in 0..hl {
                if x < x_margin || x >= hw - x_margin || y < y_margin || y >= hl - y_margin {
                    grid[x][y] = CELL_HULL;
                }
            }
        }

        // ---- Phase 1: Corridor skeleton ----
        let ring_margin = SVC_CORRIDOR_WIDTH;
        let inner_y0 = y_margin + ring_margin;
        let inner_y1 = hl.saturating_sub(y_margin + ring_margin);

        // Spine: centered, clipped to inner boundary (inside ring margin)
        for x in spine_left..spine_right.min(hw) {
            for y in inner_y0..inner_y1 {
                if grid[x][y] != CELL_HULL {
                    grid[x][y] = CELL_MAIN_CORRIDOR;
                }
            }
        }

        // Cross-corridors: use midship positions, clipped to inner boundary
        let cross_ys = &mid_cross_ys;
        let cc_x0 = x_margin + ring_margin;
        let cc_x1 = hw.saturating_sub(x_margin + ring_margin);
        for &cy in cross_ys.iter() {
            for x in cc_x0..cc_x1 {
                for y in cy..cy + CROSS_CORRIDOR_WIDTH {
                    if y < hl && grid[x][y] == CELL_EMPTY {
                        grid[x][y] = CELL_MAIN_CORRIDOR;
                    }
                }
            }
        }

        // ---- Phase 2: Stamp global shafts that fit within this deck's hull ----
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

        // Create corridor Room entries
        // Spine segments (between cross-corridors), clipped to hull boundary
        let mut spine_segments: Vec<(u32, usize, usize)> = Vec::new(); // (room_id, y_start, y_end)
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

        // Create Corridor table entry for the spine
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

        // Cross-corridor Room entries (clipped to ring margins)
        let mut cross_rooms: Vec<(u32, usize)> = Vec::new(); // (room_id, y_start)
        for &cy in cross_ys.iter() {
            if cy < inner_y0 || cy + CROSS_CORRIDOR_WIDTH > inner_y1 {
                continue;
            }
            let cc_room_x0 = x_margin + ring_margin;
            let cc_room_x1 = hw.saturating_sub(x_margin + ring_margin);
            let cc_w = cc_room_x1.saturating_sub(cc_room_x0);
            if cc_w < MIN_ROOM_DIM {
                continue;
            }
            let cc_id = next_id();
            ctx.db.room().insert(Room {
                id: cc_id,
                node_id: 0,
                name: format!("Cross-Corridor D{} Y{}", deck + 1, cy),
                room_type: room_types::CROSS_CORRIDOR,
                deck,
                x: cc_room_x0 as f32 + cc_w as f32 / 2.0,
                y: cy as f32 + CROSS_CORRIDOR_WIDTH as f32 / 2.0,
                width: cc_w as f32,
                height: CROSS_CORRIDOR_WIDTH as f32,
                capacity: 0,
            });
            ctx.db.corridor().insert(Corridor {
                id: 0,
                deck,
                corridor_type: corridor_types::BRANCH,
                x: cc_room_x0 as f32,
                y: cy as f32,
                width: cc_w as f32,
                length: CROSS_CORRIDOR_WIDTH as f32,
                orientation: 0,
                carries: carries_flags::CREW_PATH,
            });
            cross_rooms.push((cc_id, cy));
        }

        // ---- Corridor-to-corridor doors ----

        // Spine segments ↔ cross-corridors
        for &(cc_id, cy) in &cross_rooms {
            // Find spine segments adjacent to this cross-corridor
            for &(seg_id, seg_y0, seg_y1) in &spine_segments {
                if seg_y1 == cy {
                    // Segment above cross-corridor
                    let dx = spine_left as f32 + SPINE_WIDTH as f32 / 2.0;
                    let dy = cy as f32;
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
                        door_y: dy,
                    });
                }
                if seg_y0 == cy + CROSS_CORRIDOR_WIDTH {
                    // Segment below cross-corridor
                    let dx = spine_left as f32 + SPINE_WIDTH as f32 / 2.0;
                    let dy = (cy + CROSS_CORRIDOR_WIDTH) as f32;
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
                        door_y: dy,
                    });
                }
            }
        }

        // Doors between consecutive spine segments
        for i in 0..spine_segments.len().saturating_sub(1) {
            let (seg_a, _, seg_a_end) = spine_segments[i];
            let (seg_b, seg_b_start, _) = spine_segments[i + 1];
            if seg_a_end == seg_b_start {
                // Direct adjacency (no cross-corridor between)
                let dx = spine_left as f32 + SPINE_WIDTH as f32 / 2.0;
                let dy = seg_a_end as f32;
                ctx.db.door().insert(Door {
                    id: 0,
                    room_a: seg_a,
                    room_b: seg_b,
                    wall_a: wall_sides::SOUTH,
                    wall_b: wall_sides::NORTH,
                    position_along_wall: 0.5,
                    width: SPINE_WIDTH as f32,
                    access_level: access_levels::PUBLIC,
                    door_x: dx,
                    door_y: dy,
                });
            }
        }

        // ---- Shaft Room entries + doors to corridors ----
        // Use global index to correctly map to shaft_infos
        for (global_idx, sp) in global_shaft_placements.iter().enumerate() {
            // Skip shafts that don't fit within this deck's hull boundary
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

            // Record in shaft_infos for cross-deck doors
            if global_idx < shaft_infos.len() {
                shaft_infos[global_idx].deck_room_ids[deck as usize] = Some(shaft_room_id);
                shaft_infos[global_idx].ref_x = sp.x as f32 + sp.w as f32 / 2.0;
                shaft_infos[global_idx].ref_y = sp.y as f32 + sp.h as f32 / 2.0;
                shaft_infos[global_idx].ref_w = sp.w as f32;
                shaft_infos[global_idx].ref_h = sp.h as f32;
            }

            // Connect shaft to adjacent corridor
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
                spine_left,
                spine_right,
                access,
            );
        }

        // ---- Phase 3: Find attachment strips ----
        let strips = find_attachment_strips(
            &grid,
            hw,
            hl,
            spine_left,
            spine_right,
            cross_ys,
            &spine_segments,
            &cross_rooms,
            x_margin,
            y_margin,
        );

        // ---- Phase 4: Collect room requests for this deck (greedy fill) ----
        let primary_zone = deck_zone_map[deck as usize] as usize;
        let mut deck_requests: Vec<RoomRequest> = Vec::new();
        let total_strip_area: usize = strips.iter().map(|s| s.w * s.h).sum();
        let mut filled_area = 0.0f32;
        let area_budget = total_strip_area as f32 * 0.95; // leave 5% for packing waste

        // Pull from primary zone first
        let cursor = &mut zone_cursors[primary_zone];
        while *cursor < zone_requests[primary_zone].len() && filled_area < area_budget {
            let req = zone_requests[primary_zone][*cursor].clone();
            filled_area += req.target_area;
            deck_requests.push(req);
            *cursor += 1;
        }

        // If space remains, pull from adjacent/overflow zones (HAB fills gaps)
        if filled_area < area_budget * 0.7 {
            // Try HAB overflow first, then other zones with remaining rooms
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

        // Sort: largest first for best BSP packing
        deck_requests.sort_by(|a, b| {
            b.target_area
                .partial_cmp(&a.target_area)
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        // ---- Phase 5: BSP pack rooms into attachment strips ----
        let mut placed_rooms: Vec<(u32, usize, usize, usize, usize, u8)> = Vec::new();
        let mut request_idx = 0;
        let total_request_area: f32 = deck_requests.iter().map(|r| r.target_area).sum();

        for strip in &strips {
            if request_idx >= deck_requests.len() {
                break;
            }
            // BSP subdivide this strip and pack rooms
            let mut sub_rects: Vec<(usize, usize, usize, usize)> = Vec::new();
            bsp_subdivide(
                strip.x,
                strip.y,
                strip.w,
                strip.h,
                &deck_requests[request_idx..],
                &mut sub_rects,
            );

            for (rx, ry, rw, rh) in &sub_rects {
                if request_idx >= deck_requests.len() {
                    break;
                }

                // Skip if any cell overlaps a shaft or corridor
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

                let req = &deck_requests[request_idx];
                let room_id = next_id();

                // Stamp grid
                for gx in *rx..(*rx + *rw).min(hw) {
                    for gy in *ry..(*ry + *rh).min(hl) {
                        if grid[gx][gy] == CELL_EMPTY {
                            grid[gx][gy] = CELL_ROOM_BASE + (room_id as u8 % 246);
                        }
                    }
                }

                // Create room
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

                // Create door to adjacent corridor (if strip touches a corridor)
                if strip.corridor_room_id != 0 {
                    let (door_x, door_y, wall_room, wall_corr) =
                        compute_door_position(*rx, *ry, *rw, *rh, strip);
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: room_id,
                        room_b: strip.corridor_room_id,
                        wall_a: wall_room,
                        wall_b: wall_corr,
                        position_along_wall: 0.5,
                        width: 3.0_f32.min(*rw as f32).min(*rh as f32),
                        access_level: access_levels::PUBLIC,
                        door_x,
                        door_y,
                    });
                } else {
                    // Non-spine strip: connect to nearest adjacent placed room
                    for &(adj_id, ax, ay, aw, ah, _) in placed_rooms.iter() {
                        if let Some((dx, dy, wa, wb)) =
                            find_shared_edge(*rx, *ry, *rw, *rh, ax, ay, aw, ah)
                        {
                            ctx.db.door().insert(Door {
                                id: 0,
                                room_a: room_id,
                                room_b: adj_id,
                                wall_a: wa,
                                wall_b: wb,
                                position_along_wall: 0.5,
                                width: 3.0_f32.min(*rw as f32).min(*rh as f32),
                                access_level: access_levels::PUBLIC,
                                door_x: dx,
                                door_y: dy,
                            });
                            break;
                        }
                    }
                }

                placed_rooms.push((room_id, *rx, *ry, *rw, *rh, req.room_type));
                request_idx += 1;
            }
        }

        // ---- Phase 5b: Overflow filling ----
        // Try to place remaining rooms in empty grid regions
        if request_idx < deck_requests.len() {
            let ring_margin_of = SVC_CORRIDOR_WIDTH;
            let fill_x0 = x_margin + ring_margin_of;
            let fill_x1 = hw.saturating_sub(x_margin + ring_margin_of);
            let fill_y0 = y_margin + ring_margin_of;
            let fill_y1 = hl.saturating_sub(y_margin + ring_margin_of);

            let mut y = fill_y0;
            while y < fill_y1 && request_idx < deck_requests.len() {
                let mut x = fill_x0;
                while x < fill_x1 && request_idx < deck_requests.len() {
                    if grid[x][y] != CELL_EMPTY {
                        x += 1;
                        continue;
                    }
                    // Expand to largest empty rectangle from this cell
                    let mut max_w = 0;
                    for dx in 0..(fill_x1 - x) {
                        if grid[x + dx][y] != CELL_EMPTY {
                            break;
                        }
                        max_w = dx + 1;
                    }
                    let mut max_h = fill_y1 - y;
                    for dy in 0..max_h {
                        let row_clear = (0..max_w).all(|dx| grid[x + dx][y + dy] == CELL_EMPTY);
                        if !row_clear {
                            max_h = dy;
                            break;
                        }
                    }

                    if max_w >= MIN_ROOM_DIM && max_h >= MIN_ROOM_DIM {
                        let req = &deck_requests[request_idx];
                        let target_side = (req.target_area.sqrt() as usize).max(MIN_ROOM_DIM);
                        let rw = max_w.min(target_side.max(MIN_ROOM_DIM));
                        let rh = max_h.min(
                            ((req.target_area / rw as f32).ceil() as usize)
                                .max(MIN_ROOM_DIM)
                                .min(max_h),
                        );

                        if rw >= MIN_ROOM_DIM && rh >= MIN_ROOM_DIM {
                            // Check adjacency to corridors first
                            let mut has_door = false;

                            // Try spine segments
                            for &(seg_id, seg_y0, seg_y1) in &spine_segments {
                                let sx = (hw / 2).saturating_sub(SPINE_WIDTH / 2);
                                if let Some((dx, dy, wa, wb)) = find_shared_edge(
                                    x,
                                    y,
                                    rw,
                                    rh,
                                    sx,
                                    seg_y0,
                                    SPINE_WIDTH,
                                    seg_y1 - seg_y0,
                                ) {
                                    has_door = true;
                                    // Will create door after room insert
                                    // Store for later
                                    let room_id = next_id();
                                    for gx in x..(x + rw) {
                                        for gy in y..(y + rh) {
                                            grid[gx][gy] = CELL_ROOM_BASE + (room_id as u8 % 246);
                                        }
                                    }
                                    ctx.db.room().insert(Room {
                                        id: room_id,
                                        node_id: req.node_id,
                                        name: req.name.clone(),
                                        room_type: req.room_type,
                                        deck,
                                        x: x as f32 + rw as f32 / 2.0,
                                        y: y as f32 + rh as f32 / 2.0,
                                        width: rw as f32,
                                        height: rh as f32,
                                        capacity: req.capacity,
                                    });
                                    ctx.db.door().insert(Door {
                                        id: 0,
                                        room_a: room_id,
                                        room_b: seg_id,
                                        wall_a: wa,
                                        wall_b: wb,
                                        position_along_wall: 0.5,
                                        width: 3.0_f32.min(rw as f32).min(rh as f32),
                                        access_level: access_levels::PUBLIC,
                                        door_x: dx,
                                        door_y: dy,
                                    });
                                    placed_rooms.push((room_id, x, y, rw, rh, req.room_type));
                                    request_idx += 1;
                                    break;
                                }
                            }
                            if has_door {
                                x += rw;
                                continue;
                            }

                            // Try cross-corridors
                            for &(cc_id, cy) in &cross_rooms {
                                let cc_x0 = x_margin + ring_margin_of;
                                let cc_w = hw.saturating_sub(x_margin + ring_margin_of) - cc_x0;
                                if let Some((dx, dy, wa, wb)) = find_shared_edge(
                                    x,
                                    y,
                                    rw,
                                    rh,
                                    cc_x0,
                                    cy,
                                    cc_w,
                                    CROSS_CORRIDOR_WIDTH,
                                ) {
                                    has_door = true;
                                    let room_id = next_id();
                                    for gx in x..(x + rw) {
                                        for gy in y..(y + rh) {
                                            grid[gx][gy] = CELL_ROOM_BASE + (room_id as u8 % 246);
                                        }
                                    }
                                    ctx.db.room().insert(Room {
                                        id: room_id,
                                        node_id: req.node_id,
                                        name: req.name.clone(),
                                        room_type: req.room_type,
                                        deck,
                                        x: x as f32 + rw as f32 / 2.0,
                                        y: y as f32 + rh as f32 / 2.0,
                                        width: rw as f32,
                                        height: rh as f32,
                                        capacity: req.capacity,
                                    });
                                    ctx.db.door().insert(Door {
                                        id: 0,
                                        room_a: room_id,
                                        room_b: cc_id,
                                        wall_a: wa,
                                        wall_b: wb,
                                        position_along_wall: 0.5,
                                        width: 3.0_f32.min(rw as f32).min(rh as f32),
                                        access_level: access_levels::PUBLIC,
                                        door_x: dx,
                                        door_y: dy,
                                    });
                                    placed_rooms.push((room_id, x, y, rw, rh, req.room_type));
                                    request_idx += 1;
                                    break;
                                }
                            }
                            if has_door {
                                x += rw;
                                continue;
                            }

                            // Try adjacent already-placed rooms
                            for &(adj_id, ax, ay, aw, ah, _) in placed_rooms.iter() {
                                if let Some((dx, dy, wa, wb)) =
                                    find_shared_edge(x, y, rw, rh, ax, ay, aw, ah)
                                {
                                    has_door = true;
                                    let room_id = next_id();
                                    for gx in x..(x + rw) {
                                        for gy in y..(y + rh) {
                                            grid[gx][gy] = CELL_ROOM_BASE + (room_id as u8 % 246);
                                        }
                                    }
                                    ctx.db.room().insert(Room {
                                        id: room_id,
                                        node_id: req.node_id,
                                        name: req.name.clone(),
                                        room_type: req.room_type,
                                        deck,
                                        x: x as f32 + rw as f32 / 2.0,
                                        y: y as f32 + rh as f32 / 2.0,
                                        width: rw as f32,
                                        height: rh as f32,
                                        capacity: req.capacity,
                                    });
                                    ctx.db.door().insert(Door {
                                        id: 0,
                                        room_a: room_id,
                                        room_b: adj_id,
                                        wall_a: wa,
                                        wall_b: wb,
                                        position_along_wall: 0.5,
                                        width: 3.0_f32.min(rw as f32).min(rh as f32),
                                        access_level: access_levels::PUBLIC,
                                        door_x: dx,
                                        door_y: dy,
                                    });
                                    placed_rooms.push((room_id, x, y, rw, rh, req.room_type));
                                    request_idx += 1;
                                    break;
                                }
                            }
                            // Skip if no adjacency found (room would be isolated)
                            if !has_door {
                                // Don't place this room — move to next cell
                            }
                        }
                    }
                    x += max_w.max(1);
                }
                y += 1;
            }
        }

        // ---- Phase 5c: Filler room backfill ----
        // Fill remaining empty space with utility rooms from the filler pool.
        {
            let ring_margin_of = SVC_CORRIDOR_WIDTH;
            let fill_x0 = x_margin + ring_margin_of;
            let fill_x1 = hw.saturating_sub(x_margin + ring_margin_of);
            let fill_y0 = y_margin + ring_margin_of;
            let fill_y1 = hl.saturating_sub(y_margin + ring_margin_of);
            let mut filler_idx = 0usize;
            let mut filler_count = 0u32;

            let mut y = fill_y0;
            while y < fill_y1 {
                let mut x = fill_x0;
                while x < fill_x1 {
                    if grid[x][y] != CELL_EMPTY {
                        x += 1;
                        continue;
                    }
                    // Expand to largest empty rectangle
                    let mut max_w = 0;
                    for dx in 0..(fill_x1 - x) {
                        if grid[x + dx][y] != CELL_EMPTY {
                            break;
                        }
                        max_w = dx + 1;
                    }
                    let mut max_h = fill_y1 - y;
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

                        if rw >= MIN_ROOM_DIM && rh >= MIN_ROOM_DIM {
                            // Check adjacency before placing
                            let mut has_adj = false;
                            // Check spine
                            for &(_seg_id, seg_y0, seg_y1) in &spine_segments {
                                let sx = (hw / 2).saturating_sub(SPINE_WIDTH / 2);
                                if find_shared_edge(
                                    x,
                                    y,
                                    rw,
                                    rh,
                                    sx,
                                    seg_y0,
                                    SPINE_WIDTH,
                                    seg_y1 - seg_y0,
                                )
                                .is_some()
                                {
                                    has_adj = true;
                                    break;
                                }
                            }
                            if !has_adj {
                                for &(_, cy) in &cross_rooms {
                                    let cc_x0 = x_margin + ring_margin_of;
                                    let cc_w = hw.saturating_sub(x_margin + ring_margin_of) - cc_x0;
                                    if find_shared_edge(
                                        x,
                                        y,
                                        rw,
                                        rh,
                                        cc_x0,
                                        cy,
                                        cc_w,
                                        CROSS_CORRIDOR_WIDTH,
                                    )
                                    .is_some()
                                    {
                                        has_adj = true;
                                        break;
                                    }
                                }
                            }
                            if !has_adj {
                                for &(_, ax, ay, aw, ah, _) in placed_rooms.iter() {
                                    if find_shared_edge(x, y, rw, rh, ax, ay, aw, ah).is_some() {
                                        has_adj = true;
                                        break;
                                    }
                                }
                            }

                            if has_adj {
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
                            }
                        }
                    }
                    x += max_w.max(1);
                }
                y += 1;
            }
            if filler_count > 0 {
                log::info!("Deck {}: placed {} filler rooms", deck, filler_count);
            }
        }

        // ---- Phase 6: Perimeter service corridor ----
        // Wrap a 2-cell-wide corridor ring around the outermost placed content.
        let perimeter_rooms = wrap_perimeter_corridor(
            ctx,
            &mut grid,
            hw,
            hl,
            x_margin,
            y_margin,
            deck,
            &mut next_id,
            &placed_rooms,
            &spine_segments,
            &cross_rooms,
        );

        // ---- Phase 7: Room-to-room doors (adjacent logical pairs) ----
        for i in 0..placed_rooms.len() {
            for j in (i + 1)..placed_rooms.len() {
                let (id_a, ax, ay, aw, ah, rt_a) = placed_rooms[i];
                let (id_b, bx, by, bw, bh, rt_b) = placed_rooms[j];
                if !should_have_room_door(rt_a, rt_b) {
                    continue;
                }
                // Check adjacency (shared edge)
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

        // ---- Phase 7b: Rescue orphan rooms (ensure every room has ≥1 door) ----
        // Run multiple passes — each pass may connect rooms that enable further connections.
        {
            let mut has_door: std::collections::HashSet<u32> = std::collections::HashSet::new();
            for door in ctx.db.door().iter() {
                has_door.insert(door.room_a);
                has_door.insert(door.room_b);
            }

            loop {
                let before = has_door.len();
                for &(room_id, rx, ry, rw, rh, _rt) in &placed_rooms {
                    if has_door.contains(&room_id) {
                        continue;
                    }
                    // Try spine segments
                    let mut connected = false;
                    for &(seg_id, seg_y0, seg_y1) in &spine_segments {
                        let sx = (hw / 2).saturating_sub(SPINE_WIDTH / 2);
                        if let Some((dx, dy, wa, wb)) = find_shared_edge(
                            rx,
                            ry,
                            rw,
                            rh,
                            sx,
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
                                width: 3.0_f32.min(rw as f32).min(rh as f32),
                                access_level: access_levels::PUBLIC,
                                door_x: dx,
                                door_y: dy,
                            });
                            connected = true;
                            has_door.insert(room_id);
                            break;
                        }
                    }
                    if connected {
                        continue;
                    }

                    // Try cross-corridors
                    for &(cc_id, cy) in &cross_rooms {
                        let cc_x0 = x_margin + SVC_CORRIDOR_WIDTH;
                        let cc_w = hw.saturating_sub(x_margin + SVC_CORRIDOR_WIDTH) - cc_x0;
                        if let Some((dx, dy, wa, wb)) =
                            find_shared_edge(rx, ry, rw, rh, cc_x0, cy, cc_w, CROSS_CORRIDOR_WIDTH)
                        {
                            ctx.db.door().insert(Door {
                                id: 0,
                                room_a: room_id,
                                room_b: cc_id,
                                wall_a: wa,
                                wall_b: wb,
                                position_along_wall: 0.5,
                                width: 3.0_f32.min(rw as f32).min(rh as f32),
                                access_level: access_levels::PUBLIC,
                                door_x: dx,
                                door_y: dy,
                            });
                            connected = true;
                            has_door.insert(room_id);
                            break;
                        }
                    }
                    if connected {
                        continue;
                    }

                    // Try any adjacent placed room that already has a door
                    for &(adj_id, ax, ay, aw, ah, _) in &placed_rooms {
                        if adj_id == room_id || !has_door.contains(&adj_id) {
                            continue;
                        }
                        if let Some((dx, dy, wa, wb)) =
                            find_shared_edge(rx, ry, rw, rh, ax, ay, aw, ah)
                        {
                            ctx.db.door().insert(Door {
                                id: 0,
                                room_a: room_id,
                                room_b: adj_id,
                                wall_a: wa,
                                wall_b: wb,
                                position_along_wall: 0.5,
                                width: 3.0_f32.min(rw as f32).min(rh as f32),
                                access_level: access_levels::PUBLIC,
                                door_x: dx,
                                door_y: dy,
                            });
                            connected = true;
                            has_door.insert(room_id);
                            break;
                        }
                    }
                    // If still not connected, try ring segments
                    if !connected {
                        for &ring_id in &perimeter_rooms {
                            if let Some(pr) = ctx.db.room().id().find(ring_id) {
                                let prx = (pr.x - pr.width / 2.0) as usize;
                                let pry = (pr.y - pr.height / 2.0) as usize;
                                let prw = pr.width as usize;
                                let prh = pr.height as usize;
                                if let Some((dx, dy, wa, wb)) =
                                    find_shared_edge(rx, ry, rw, rh, prx, pry, prw, prh)
                                {
                                    ctx.db.door().insert(Door {
                                        id: 0,
                                        room_a: room_id,
                                        room_b: ring_id,
                                        wall_a: wa,
                                        wall_b: wb,
                                        position_along_wall: 0.5,
                                        width: 3.0_f32.min(rw as f32).min(rh as f32),
                                        access_level: access_levels::CREW_ONLY,
                                        door_x: dx,
                                        door_y: dy,
                                    });
                                    has_door.insert(room_id);
                                    break;
                                }
                            }
                        }
                    }
                }
                if has_door.len() == before {
                    break; // No new rooms rescued — converged
                }
            } // end loop
        }

        // ---- Grid dump (debug) ----
        let max_rows = 60;
        let mut dump = format!(
            "Deck {} grid ({}x{}, {} rooms, {} strips, {} cross-corridors):\n",
            deck + 1,
            hw,
            hl,
            placed_rooms.len(),
            strips.len(),
            cross_ys.len(),
        );
        for y in 0..hl.min(max_rows) {
            for x in 0..hw {
                let ch = match grid[x][y] {
                    CELL_EMPTY => '.',
                    CELL_MAIN_CORRIDOR => '=',
                    CELL_SERVICE_CORRIDOR => '-',
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
            "Deck {}: placed {}/{} rooms ({:.0}/{:.0}m² area, {} strip area available)",
            deck + 1,
            request_idx.min(deck_requests.len()),
            deck_requests.len(),
            deck_requests
                .iter()
                .take(request_idx)
                .map(|r| r.target_area)
                .sum::<f32>(),
            total_request_area,
            total_strip_area,
        );
    } // end per-deck loop

    // ---- VerticalShaft table entries + cross-deck doors ----
    for si in &shaft_infos {
        // Only create shaft entry if it was placed on at least one deck
        let placed_decks: Vec<String> = si
            .deck_room_ids
            .iter()
            .enumerate()
            .filter_map(|(d, rid)| rid.map(|_| d.to_string()))
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

        // Cross-deck doors between consecutive shaft rooms
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

    // Log final stats
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

/// Compute shaft placements by distributing templates across cross-corridor intersections.
/// Main elevators go starboard of spine, service elevators near hull edge,
/// ladders go port of spine. Shafts are distributed evenly along the deck length.
fn compute_shaft_placements(
    templates: &[(&'static str, u8, bool, usize, usize)],
    spine_right: usize,
    spine_left: usize,
    cross_ys: &[usize],
    hw: usize,
    hl: usize,
) -> Vec<ShaftPlacement> {
    let mut placements = Vec::new();
    let spine_left_edge = spine_left;
    let cross_end_offset = CROSS_CORRIDOR_WIDTH;

    if cross_ys.is_empty() {
        // Minimal deck: place first elevator next to spine
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

    // Separate templates by type for placement strategy
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

    // Distribute main elevators evenly across cross-corridor positions (starboard of spine)
    let num_positions = cross_ys.len();
    for (i, (name, st, is_main, w, h)) in main_elevators.iter().enumerate() {
        let cross_idx = if main_elevators.len() <= num_positions {
            // Distribute evenly: map i to a cross-corridor index
            i * num_positions / main_elevators.len()
        } else {
            i % num_positions
        };
        let cy = cross_ys[cross_idx.min(num_positions - 1)];
        // Stack multiple elevators at same intersection by offsetting Y
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

    // Service elevators: near starboard hull edge, distributed evenly
    let svc_x = hw.saturating_sub(SVC_CORRIDOR_WIDTH + 2); // leave room for perimeter corridor
    if svc_x > spine_right {
        for (i, (name, st, is_main, w, h)) in service_elevators.iter().enumerate() {
            let cross_idx = if service_elevators.len() <= num_positions {
                (i * num_positions / service_elevators.len().max(1)).min(num_positions - 1)
            } else {
                i % num_positions
            };
            let cy = cross_ys[cross_idx];
            placements.push(ShaftPlacement {
                x: svc_x.saturating_sub(*w),
                y: cy + cross_end_offset,
                w: *w,
                h: *h,
                shaft_type: *st,
                name,
                is_main: *is_main,
            });
        }
    }

    // Ladders: port side of spine, distributed evenly
    for (i, (name, st, is_main, w, h)) in ladders.iter().enumerate() {
        let cross_idx = if ladders.len() <= num_positions {
            (i * num_positions / ladders.len().max(1)).min(num_positions - 1)
        } else {
            i % num_positions
        };
        let cy = cross_ys[cross_idx];
        placements.push(ShaftPlacement {
            x: spine_left_edge.saturating_sub(*w),
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

/// Find attachment strips: empty rectangular areas directly adjacent to corridor walls.
#[allow(clippy::too_many_arguments)]
fn find_attachment_strips(
    grid: &[Vec<u8>],
    hw: usize,
    hl: usize,
    spine_left: usize,
    spine_right: usize,
    cross_ys: &[usize],
    spine_segments: &[(u32, usize, usize)],
    cross_rooms: &[(u32, usize)],
    x_margin: usize,
    y_margin: usize,
) -> Vec<AttachmentStrip> {
    let mut strips = Vec::new();

    // Reserve space for perimeter service corridor ring
    let ring_margin = SVC_CORRIDOR_WIDTH;

    // Y boundaries clipped to hull + ring margin
    let y_lo = y_margin + ring_margin;
    let y_hi = hl.saturating_sub(y_margin + ring_margin);

    // Build Y segment boundaries from cross-corridors
    let mut y_boundaries: Vec<usize> = vec![y_lo];
    for &cy in cross_ys {
        if cy >= y_lo && cy + CROSS_CORRIDOR_WIDTH <= y_hi {
            y_boundaries.push(cy);
            y_boundaries.push(cy + CROSS_CORRIDOR_WIDTH);
        }
    }
    y_boundaries.push(y_hi);

    let port_x = x_margin + ring_margin;
    let stbd_x_max = hw.saturating_sub(x_margin + ring_margin);

    // For each Y segment (between cross-corridors), scan BOTH sides for
    // clear rectangular regions, splitting around shaft obstacles in both
    // X and Y dimensions.
    for chunk in y_boundaries.chunks(2) {
        if chunk.len() < 2 || chunk[0] >= chunk[1] {
            continue;
        }
        let seg_y0 = chunk[0];
        let seg_y1 = chunk[1];

        // Scan both port and starboard sides
        let sides: [(usize, usize, u8); 2] = [
            (port_x, spine_left, wall_sides::WEST),      // port
            (spine_right, stbd_x_max, wall_sides::EAST), // starboard
        ];

        for &(x_start, x_end, wall_side) in &sides {
            if x_end <= x_start {
                continue;
            }
            // Find clear sub-rectangles by scanning Y rows for shaft obstacles
            let sub_rects = find_clear_subrects(grid, x_start, x_end, seg_y0, seg_y1);
            for (rx, ry, rw, rh) in sub_rects {
                if rw < MIN_ROOM_DIM || rh < MIN_ROOM_DIM {
                    continue;
                }
                // Strips touching spine get a spine corridor door
                let touches_spine = (wall_side == wall_sides::WEST && rx + rw == spine_left)
                    || (wall_side == wall_sides::EAST && rx == spine_right);
                let corridor_id = if touches_spine {
                    find_corridor_for_strip(
                        if wall_side == wall_sides::WEST {
                            spine_left
                        } else {
                            spine_right
                        },
                        ry,
                        ry + rh,
                        spine_segments,
                        cross_rooms,
                    )
                } else {
                    0
                };
                let door_x = if wall_side == wall_sides::WEST {
                    rx + rw
                } else {
                    rx
                };
                strips.push(AttachmentStrip {
                    corridor_room_id: corridor_id,
                    x: rx,
                    y: ry,
                    w: rw,
                    h: rh,
                    wall_side,
                    door_x,
                    door_y: ry + rh / 2,
                });
            }
        }
    }

    // Sort strips: spine-touching first (they have corridor_room_id != 0),
    // then by Y for balanced filling, alternating port/starboard.
    strips.sort_by(|a, b| {
        // Spine-touching strips first
        let a_spine = if a.corridor_room_id != 0 { 0 } else { 1 };
        let b_spine = if b.corridor_room_id != 0 { 0 } else { 1 };
        let spine_cmp = a_spine.cmp(&b_spine);
        if spine_cmp != core::cmp::Ordering::Equal {
            return spine_cmp;
        }
        let y_cmp = a.y.cmp(&b.y);
        if y_cmp != core::cmp::Ordering::Equal {
            return y_cmp;
        }
        a.wall_side.cmp(&b.wall_side)
    });
    strips
}

/// Find clear sub-rectangles within a region by splitting around obstacles.
/// Returns Vec of (x, y, w, h) for each clear rectangle.
fn find_clear_subrects(
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

    // Scan columns to find X-runs of clear columns, then for each run
    // scan rows to find Y-sub-runs
    let mut col_start = x0;
    let mut x = x0;
    while x <= x1 {
        // Check if this column has ANY non-empty cells in the Y range
        let col_has_obstacle = if x < x1 {
            (y0..y1).any(|y| y >= grid[x].len() || grid[x][y] != CELL_EMPTY)
        } else {
            true // sentinel: force flush at end
        };

        if col_has_obstacle {
            let run_w = x - col_start;
            if run_w >= MIN_ROOM_DIM {
                // This column range is fully clear — emit as one rect
                results.push((col_start, y0, run_w, y1 - y0));
            } else if run_w > 0 && run_w < MIN_ROOM_DIM {
                // Narrow clear band — skip (too small for a room)
            }

            if x < x1 {
                // This column has obstacles — try to find clear Y-sub-ranges
                // within just this column (and adjacent obstacle columns)
                // by scanning rows
                let obs_col_start = x;
                // Find end of obstacle column block
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

                // For obstacle columns, find Y sub-ranges that ARE clear
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

/// Scan how many empty columns exist starting from x in the given y range.
fn scan_empty_width(grid: &[Vec<u8>], start_x: usize, y: usize, max_w: usize, h: usize) -> usize {
    let hw = grid.len();
    for dx in 0..max_w {
        let x = start_x + dx;
        if x >= hw {
            return dx;
        }
        // Check if this column is fully empty in the y range
        for dy in 0..h {
            if y + dy >= grid[x].len() || grid[x][y + dy] != CELL_EMPTY {
                return dx;
            }
        }
    }
    max_w
}

/// Find which corridor room a strip connects to (spine segment or cross-corridor).
fn find_corridor_for_strip(
    _edge_x: usize,
    y0: usize,
    y1: usize,
    spine_segments: &[(u32, usize, usize)],
    _cross_rooms: &[(u32, usize)],
) -> u32 {
    let mid_y = (y0 + y1) / 2;
    // Find spine segment containing mid_y
    for &(seg_id, seg_y0, seg_y1) in spine_segments {
        if mid_y >= seg_y0 && mid_y < seg_y1 {
            return seg_id;
        }
    }
    // Fallback: first spine segment
    spine_segments.first().map(|s| s.0).unwrap_or(0)
}

/// BSP subdivision of a rectangular area into sub-rectangles for room placement.
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
        // Single room fills the rectangle (capped at 1.5× target area)
        let target = requests[0].target_area;
        let max_area = target * 1.5;
        let actual_area = (w * h) as f32;
        if actual_area <= max_area || (w <= MIN_ROOM_DIM + 1 && h <= MIN_ROOM_DIM + 1) {
            out.push((x, y, w, h));
        } else {
            // Shrink to fit — split off excess
            if w > h {
                let new_w = ((target / h as f32) as usize).max(MIN_ROOM_DIM).min(w);
                out.push((x, y, new_w, h));
            } else {
                let new_h = ((target / w as f32) as usize).max(MIN_ROOM_DIM).min(h);
                out.push((x, y, w, new_h));
            }
        }
        return;
    }

    // Split the rectangle and distribute requests.
    // ONLY split along Y (horizontal rows) so every room spans the full strip width
    // and maintains contact with the corridor wall. Vertical X-splits create inner
    // rooms that are hidden behind outer rooms with no corridor access.
    let split_at = requests.len() / 2;
    let area_ratio = requests[..split_at]
        .iter()
        .map(|r| r.target_area)
        .sum::<f32>()
        / requests.iter().map(|r| r.target_area).sum::<f32>();

    // Horizontal split (split along Y) — creates rows, all full-width
    let split_y = (h as f32 * area_ratio).round() as usize;
    let split_y = split_y
        .max(MIN_ROOM_DIM)
        .min(h.saturating_sub(MIN_ROOM_DIM));
    if split_y >= MIN_ROOM_DIM && h - split_y >= MIN_ROOM_DIM {
        bsp_subdivide(x, y, w, split_y, &requests[..split_at], out);
        bsp_subdivide(x, y + split_y, w, h - split_y, &requests[split_at..], out);
    } else {
        out.push((x, y, w, h));
    }
}

/// Compute door position for a room adjacent to a corridor via an attachment strip.
fn compute_door_position(
    rx: usize,
    ry: usize,
    rw: usize,
    rh: usize,
    strip: &AttachmentStrip,
) -> (f32, f32, u8, u8) {
    match strip.wall_side {
        wall_sides::WEST => {
            // Room is west of corridor (port side) — door on room's east wall
            let dx = (rx + rw) as f32;
            let dy = ry as f32 + rh as f32 / 2.0;
            (dx, dy, wall_sides::EAST, wall_sides::WEST)
        }
        wall_sides::EAST => {
            // Room is east of corridor (starboard side) — door on room's west wall
            let dx = rx as f32;
            let dy = ry as f32 + rh as f32 / 2.0;
            (dx, dy, wall_sides::WEST, wall_sides::EAST)
        }
        wall_sides::NORTH => {
            let dx = rx as f32 + rw as f32 / 2.0;
            let dy = (ry + rh) as f32;
            (dx, dy, wall_sides::SOUTH, wall_sides::NORTH)
        }
        wall_sides::SOUTH => {
            let dx = rx as f32 + rw as f32 / 2.0;
            let dy = ry as f32;
            (dx, dy, wall_sides::NORTH, wall_sides::SOUTH)
        }
        _ => {
            let dx = rx as f32 + rw as f32 / 2.0;
            let dy = ry as f32 + rh as f32 / 2.0;
            (dx, dy, wall_sides::EAST, wall_sides::WEST)
        }
    }
}

/// Connect a shaft room to its adjacent corridor.
#[allow(clippy::too_many_arguments)]
fn connect_shaft_to_corridor(
    ctx: &ReducerContext,
    shaft_room_id: u32,
    sp: &ShaftPlacement,
    spine_segments: &[(u32, usize, usize)],
    cross_rooms: &[(u32, usize)],
    spine_left: usize,
    spine_right: usize,
    access: u8,
) {
    // Check if shaft overlaps a cross-corridor (same Y range)
    for &(cc_id, cy) in cross_rooms {
        if sp.y < cy + CROSS_CORRIDOR_WIDTH && sp.y + sp.h > cy {
            ctx.db.door().insert(Door {
                id: 0,
                room_a: shaft_room_id,
                room_b: cc_id,
                wall_a: wall_sides::SOUTH,
                wall_b: wall_sides::NORTH,
                position_along_wall: 0.5,
                width: sp.w.min(sp.h) as f32,
                access_level: access,
                door_x: sp.x as f32 + sp.w as f32 / 2.0,
                door_y: sp.y as f32 + sp.h as f32 / 2.0,
            });
            return;
        }
    }

    // Check if adjacent to spine
    if sp.x == spine_right || sp.x + sp.w == spine_left {
        let mid_y = sp.y + sp.h / 2;
        for &(seg_id, seg_y0, seg_y1) in spine_segments {
            if mid_y >= seg_y0 && mid_y < seg_y1 {
                let dx = if sp.x == spine_right {
                    spine_right as f32
                } else {
                    spine_left as f32
                };
                ctx.db.door().insert(Door {
                    id: 0,
                    room_a: shaft_room_id,
                    room_b: seg_id,
                    wall_a: if sp.x == spine_right {
                        wall_sides::WEST
                    } else {
                        wall_sides::EAST
                    },
                    wall_b: if sp.x == spine_right {
                        wall_sides::EAST
                    } else {
                        wall_sides::WEST
                    },
                    position_along_wall: 0.5,
                    width: sp.h.min(sp.w) as f32,
                    access_level: access,
                    door_x: dx,
                    door_y: mid_y as f32,
                });
                return;
            }
        }
    }
}

/// Wrap a perimeter service corridor around placed content on a deck.
/// Places a SVC_CORRIDOR_WIDTH ring at a fixed offset from the hull boundary.
#[allow(clippy::too_many_arguments)]
fn wrap_perimeter_corridor(
    ctx: &ReducerContext,
    grid: &mut [Vec<u8>],
    hw: usize,
    hl: usize,
    x_margin: usize,
    y_margin: usize,
    deck: i32,
    next_id: &mut impl FnMut() -> u32,
    placed_rooms: &[(u32, usize, usize, usize, usize, u8)],
    spine_segments: &[(u32, usize, usize)],
    cross_rooms: &[(u32, usize)],
) -> Vec<u32> {
    // The ring sits just inside the hull boundary
    let ring_x0 = x_margin;
    let ring_x1 = hw.saturating_sub(x_margin);
    let ring_y0 = y_margin;
    let ring_y1 = hl.saturating_sub(y_margin);

    if ring_x1 <= ring_x0 + 2 * SVC_CORRIDOR_WIDTH + SPINE_WIDTH
        || ring_y1 <= ring_y0 + 2 * SVC_CORRIDOR_WIDTH
    {
        return Vec::new();
    }

    // Inner boundary = where rooms/corridors live
    let inner_x0 = ring_x0 + SVC_CORRIDOR_WIDTH;
    let inner_x1 = ring_x1.saturating_sub(SVC_CORRIDOR_WIDTH);
    let inner_y0 = ring_y0 + SVC_CORRIDOR_WIDTH;
    let inner_y1 = ring_y1.saturating_sub(SVC_CORRIDOR_WIDTH);

    // Stamp ring cells: everything between hull edge and inner boundary
    for x in ring_x0..ring_x1 {
        for y in ring_y0..ring_y1 {
            let in_interior = x >= inner_x0 && x < inner_x1 && y >= inner_y0 && y < inner_y1;
            if !in_interior && grid[x][y] == CELL_EMPTY {
                grid[x][y] = CELL_SERVICE_CORRIDOR;
            }
        }
    }

    let mut perimeter_ids = Vec::new();

    // North (fore) side: ring_x0..ring_x1, ring_y0..inner_y0
    let north_w = ring_x1 - ring_x0;
    let north_h = inner_y0 - ring_y0;
    if north_h >= 1 && north_w >= 1 {
        let id = next_id();
        ctx.db.room().insert(Room {
            id,
            node_id: 0,
            name: format!("Service Ring North D{}", deck + 1),
            room_type: room_types::SERVICE_CORRIDOR,
            deck,
            x: ring_x0 as f32 + north_w as f32 / 2.0,
            y: ring_y0 as f32 + north_h as f32 / 2.0,
            width: north_w as f32,
            height: north_h as f32,
            capacity: 0,
        });
        perimeter_ids.push(id);
    }

    // South (aft) side: ring_x0..ring_x1, inner_y1..ring_y1
    let south_h = ring_y1 - inner_y1;
    if south_h >= 1 && north_w >= 1 {
        let id = next_id();
        ctx.db.room().insert(Room {
            id,
            node_id: 0,
            name: format!("Service Ring South D{}", deck + 1),
            room_type: room_types::SERVICE_CORRIDOR,
            deck,
            x: ring_x0 as f32 + north_w as f32 / 2.0,
            y: inner_y1 as f32 + south_h as f32 / 2.0,
            width: north_w as f32,
            height: south_h as f32,
            capacity: 0,
        });
        perimeter_ids.push(id);
    }

    // West (port) side: ring_x0..inner_x0, inner_y0..inner_y1
    let west_w = inner_x0 - ring_x0;
    let side_h = inner_y1 - inner_y0;
    if west_w >= 1 && side_h >= 1 {
        let id = next_id();
        ctx.db.room().insert(Room {
            id,
            node_id: 0,
            name: format!("Service Ring West D{}", deck + 1),
            room_type: room_types::SERVICE_CORRIDOR,
            deck,
            x: ring_x0 as f32 + west_w as f32 / 2.0,
            y: inner_y0 as f32 + side_h as f32 / 2.0,
            width: west_w as f32,
            height: side_h as f32,
            capacity: 0,
        });
        perimeter_ids.push(id);
    }

    // East (starboard) side: inner_x1..ring_x1, inner_y0..inner_y1
    let east_w = ring_x1 - inner_x1;
    if east_w >= 1 && side_h >= 1 {
        let id = next_id();
        ctx.db.room().insert(Room {
            id,
            node_id: 0,
            name: format!("Service Ring East D{}", deck + 1),
            room_type: room_types::SERVICE_CORRIDOR,
            deck,
            x: inner_x1 as f32 + east_w as f32 / 2.0,
            y: inner_y0 as f32 + side_h as f32 / 2.0,
            width: east_w as f32,
            height: side_h as f32,
            capacity: 0,
        });
        perimeter_ids.push(id);
    }

    // Create Corridor table entry for perimeter
    ctx.db.corridor().insert(Corridor {
        id: 0,
        deck,
        corridor_type: corridor_types::SERVICE,
        x: ring_x0 as f32,
        y: ring_y0 as f32,
        width: (ring_x1 - ring_x0) as f32,
        length: (ring_y1 - ring_y0) as f32,
        orientation: 0,
        carries: carries_flags::POWER
            | carries_flags::WATER
            | carries_flags::HVAC
            | carries_flags::COOLANT,
    });

    // Connect ring segments to each other at corners
    for i in 0..perimeter_ids.len() {
        for j in (i + 1)..perimeter_ids.len() {
            let ra = ctx.db.room().id().find(perimeter_ids[i]);
            let rb = ctx.db.room().id().find(perimeter_ids[j]);
            if let (Some(a), Some(b)) = (ra, rb) {
                let ax = (a.x - a.width / 2.0) as usize;
                let ay = (a.y - a.height / 2.0) as usize;
                let aw = a.width as usize;
                let ah = a.height as usize;
                let bx = (b.x - b.width / 2.0) as usize;
                let by = (b.y - b.height / 2.0) as usize;
                let bw = b.width as usize;
                let bh = b.height as usize;
                if let Some((dx, dy, wa, wb)) = find_shared_edge(ax, ay, aw, ah, bx, by, bw, bh) {
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: perimeter_ids[i],
                        room_b: perimeter_ids[j],
                        wall_a: wa,
                        wall_b: wb,
                        position_along_wall: 0.5,
                        width: SVC_CORRIDOR_WIDTH as f32,
                        access_level: access_levels::CREW_ONLY,
                        door_x: dx,
                        door_y: dy,
                    });
                }
            }
        }
    }

    // Connect ring to adjacent spine/cross-corridor/rooms
    // Track placed rooms that already got a service door (limit to 1 per room)
    let mut rooms_with_svc_door: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for &pid in &perimeter_ids {
        if let Some(pr) = ctx.db.room().id().find(pid) {
            let px = (pr.x - pr.width / 2.0) as usize;
            let py = (pr.y - pr.height / 2.0) as usize;
            let pw = pr.width as usize;
            let ph = pr.height as usize;

            // Spine segments
            for &(seg_id, seg_y0, seg_y1) in spine_segments {
                let sx = (hw / 2).saturating_sub(SPINE_WIDTH / 2);
                if let Some((dx, dy, wa, wb)) =
                    find_shared_edge(px, py, pw, ph, sx, seg_y0, SPINE_WIDTH, seg_y1 - seg_y0)
                {
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: pid,
                        room_b: seg_id,
                        wall_a: wa,
                        wall_b: wb,
                        position_along_wall: 0.5,
                        width: SVC_CORRIDOR_WIDTH as f32,
                        access_level: access_levels::CREW_ONLY,
                        door_x: dx,
                        door_y: dy,
                    });
                }
            }

            // Cross-corridors (use inner bounds matching actual Room geometry)
            for &(cc_id, cy) in cross_rooms {
                let cc_x0 = inner_x0;
                let cc_w = inner_x1.saturating_sub(inner_x0);
                if let Some((dx, dy, wa, wb)) =
                    find_shared_edge(px, py, pw, ph, cc_x0, cy, cc_w, CROSS_CORRIDOR_WIDTH)
                {
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: pid,
                        room_b: cc_id,
                        wall_a: wa,
                        wall_b: wb,
                        position_along_wall: 0.5,
                        width: SVC_CORRIDOR_WIDTH as f32,
                        access_level: access_levels::CREW_ONLY,
                        door_x: dx,
                        door_y: dy,
                    });
                }
            }

            // Placed rooms — skip habitation, limit to 1 service door per room
            for &(room_id, rx, ry, rw, rh, rt) in placed_rooms {
                if is_habitation(rt) || rooms_with_svc_door.contains(&room_id) {
                    continue;
                }
                if let Some((dx, dy, wa, wb)) = find_shared_edge(px, py, pw, ph, rx, ry, rw, rh) {
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: pid,
                        room_b: room_id,
                        wall_a: wa,
                        wall_b: wb,
                        position_along_wall: 0.5,
                        width: SVC_CORRIDOR_WIDTH as f32,
                        access_level: access_levels::CREW_ONLY,
                        door_x: dx,
                        door_y: dy,
                    });
                    rooms_with_svc_door.insert(room_id);
                }
            }
        }
    }

    perimeter_ids
}

/// Find a shared edge between two adjacent rooms. Returns (door_x, door_y, wall_a, wall_b).
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
    // A's east wall touches B's west wall
    if ax + aw == bx {
        let overlap_y0 = ay.max(by);
        let overlap_y1 = (ay + ah).min(by + bh);
        if overlap_y1 > overlap_y0 {
            let dy = (overlap_y0 + overlap_y1) as f32 / 2.0;
            return Some(((ax + aw) as f32, dy, wall_sides::EAST, wall_sides::WEST));
        }
    }
    // A's west wall touches B's east wall
    if bx + bw == ax {
        let overlap_y0 = ay.max(by);
        let overlap_y1 = (ay + ah).min(by + bh);
        if overlap_y1 > overlap_y0 {
            let dy = (overlap_y0 + overlap_y1) as f32 / 2.0;
            return Some((ax as f32, dy, wall_sides::WEST, wall_sides::EAST));
        }
    }
    // A's south wall touches B's north wall
    if ay + ah == by {
        let overlap_x0 = ax.max(bx);
        let overlap_x1 = (ax + aw).min(bx + bw);
        if overlap_x1 > overlap_x0 {
            let dx = (overlap_x0 + overlap_x1) as f32 / 2.0;
            return Some((dx, (ay + ah) as f32, wall_sides::SOUTH, wall_sides::NORTH));
        }
    }
    // A's north wall touches B's south wall
    if by + bh == ay {
        let overlap_x0 = ax.max(bx);
        let overlap_x1 = (ax + aw).min(bx + bw);
        if overlap_x1 > overlap_x0 {
            let dx = (overlap_x0 + overlap_x1) as f32 / 2.0;
            return Some((dx, ay as f32, wall_sides::NORTH, wall_sides::SOUTH));
        }
    }
    None
}

/// Compute shaft templates scaled to population.
/// Returns: Vec of (name, shaft_type, is_main, width, height)
fn compute_shaft_templates(total_pop: u32) -> Vec<(&'static str, u8, bool, usize, usize)> {
    let main_count = (total_pop as f32 / 200.0).ceil().max(2.0) as usize;
    let svc_count = (total_pop as f32 / 500.0).ceil().max(1.0) as usize;
    let ladder_count = (total_pop as f32 / 500.0).ceil().max(2.0) as usize;

    // Name pools for generated shafts
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

/// Compute hull dimensions using iterative overhead calculation.
/// Uses perimeter service corridor estimate instead of fixed starboard strip.
fn compute_hull_dimensions(
    room_area_per_deck: f32,
    shaft_area_per_deck: f32,
    max_room_area: f32,
) -> (usize, usize) {
    let aspect_ratio = 3.5f32;
    let mut mult = 1.4f32;

    // Minimum beam: largest room must fit in one strip
    // strip_width = (beam - SPINE_WIDTH - 2*SVC_CORRIDOR_WIDTH) / 2
    // strip_width >= sqrt(max_room_area) for aspect-ratio-1 fit
    let min_strip_w = (max_room_area.sqrt()).max(MIN_ROOM_DIM as f32);
    let min_beam =
        (min_strip_w * 2.0 + SPINE_WIDTH as f32 + 2.0 * SVC_CORRIDOR_WIDTH as f32).max(30.0);

    for _ in 0..5 {
        let apd = room_area_per_deck * mult;
        let b = (apd.sqrt() / aspect_ratio.sqrt()).max(min_beam);
        let l = (apd / b).max(100.0);

        let num_cross = (l / 35.0).round().max(1.0);
        // Spine + cross-corridors (full width now) + perimeter corridor ring
        let corridor_area = SPINE_WIDTH as f32 * l
            + num_cross * CROSS_CORRIDOR_WIDTH as f32 * b
            + 2.0 * (b + l) * SVC_CORRIDOR_WIDTH as f32;

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

/// Auto-compute optimal deck count from room area and population constraints.
/// Finds the smallest deck count where fill ratio is ≤ 85% and max walking
/// distance to an elevator is ≤ 50m.
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

    // Minimum 7 decks so every zone gets ≥1 deck
    let mut best = 7u32;
    for d in 7..=30u32 {
        let room_per_deck = total_room_area / d as f32;
        let (b, l) = compute_hull_dimensions(room_per_deck, shaft_area_per_deck, max_room_area);

        let num_cross = (l as f32 / 35.0).round().max(1.0) as usize;
        let usable_width = b.saturating_sub(SPINE_WIDTH + 2 * SVC_CORRIDOR_WIDTH);
        let usable_length =
            l.saturating_sub(2 * SVC_CORRIDOR_WIDTH) - num_cross * CROSS_CORRIDOR_WIDTH;
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
