//! Corridor-first ship layout generation.
//!
//! Pipeline: hull sizing → corridor skeleton → shafts at intersections →
//! attachment strip scanning → BSP room packing → room-to-room doors.
//! Rooms are ONLY placed adjacent to corridors, guaranteeing connectivity
//! by construction (no BFS cleanup needed).

use super::doors::should_have_room_door;
use super::facilities::deck_range_for_zone;
use super::hull::{hull_length, hull_width};
use super::treemap::RoomRequest;
use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

// Grid cell type markers
const CELL_EMPTY: u8 = 0;
const CELL_MAIN_CORRIDOR: u8 = 1;
const CELL_SERVICE_CORRIDOR: u8 = 2;
const CELL_SHAFT: u8 = 3;
const CELL_ROOM_BASE: u8 = 10;

// Corridor geometry
const SPINE_WIDTH: usize = 3;
const CROSS_CORRIDOR_WIDTH: usize = 3;
const SVC_CORRIDOR_WIDTH: usize = 2;
const MIN_ROOM_DIM: usize = 4;

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

pub(super) fn layout_ship(ctx: &ReducerContext, deck_count: u32) {
    let nodes: Vec<GraphNode> = ctx.db.graph_node().iter().collect();

    // ---- Hull sizing from total room area ----
    let total_area: f32 = nodes.iter().map(|n| n.required_area).sum();
    let gross_area = total_area * 1.4; // 40% overhead for corridors/walls
    let area_per_deck = gross_area / deck_count as f32;
    let ship_beam = (area_per_deck.sqrt() / 2.45).max(30.0) as usize;
    let ship_length = (area_per_deck / ship_beam as f32).max(100.0) as usize;
    log::info!(
        "Hull sizing: {:.0}m² total room area, {:.0}m² gross, {}×{} hull ({} decks)",
        total_area, gross_area, ship_beam, ship_length, deck_count
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

    // We'll define shaft templates and fill per-deck room IDs
    let shaft_templates: Vec<(&str, u8, bool)> = vec![
        ("Fore Elevator", shaft_types::ELEVATOR, true),
        ("Aft Elevator", shaft_types::ELEVATOR, true),
        ("Service Elevator", shaft_types::SERVICE_ELEVATOR, false),
        ("Ladder A", shaft_types::LADDER, false),
        ("Ladder B", shaft_types::LADDER, false),
    ];
    let mut shaft_infos: Vec<ShaftInfo> = shaft_templates
        .iter()
        .map(|(name, st, is_main)| ShaftInfo {
            name,
            shaft_type: *st,
            is_main: *is_main,
            deck_room_ids: vec![None; deck_count as usize],
            ref_x: 0.0,
            ref_y: 0.0,
            ref_w: 3.0,
            ref_h: 3.0,
        })
        .collect();

    // ---- Per-deck generation ----
    for deck in 0..deck_count as i32 {
        let hw: usize = hull_width(deck as u32, deck_count, ship_beam);
        let hl: usize = hull_length(deck as u32, deck_count, ship_length);

        if hw < 12 || hl < 30 {
            log::warn!("Deck {} too small ({}×{}), skipping", deck + 1, hw, hl);
            continue;
        }

        let mut grid: Vec<Vec<u8>> = vec![vec![CELL_EMPTY; hl]; hw];

        // ---- Phase 1: Corridor skeleton ----

        // Spine: centered, full deck length
        let spine_left = hw / 2 - SPINE_WIDTH / 2;
        let spine_right = spine_left + SPINE_WIDTH;
        for x in spine_left..spine_right.min(hw) {
            for y in 0..hl {
                grid[x][y] = CELL_MAIN_CORRIDOR;
            }
        }

        // Cross-corridors: proportionally spaced, 1 per ~35 cells
        let num_cross = ((hl as f32 / 35.0).round() as usize).max(1);
        let cross_spacing = hl / (num_cross + 1);
        let mut cross_ys: Vec<usize> = Vec::new();
        for i in 1..=num_cross {
            let cy = i * cross_spacing;
            if cy + CROSS_CORRIDOR_WIDTH <= hl {
                cross_ys.push(cy);
            }
        }

        // Stamp cross-corridors
        let svc_left = hw - SVC_CORRIDOR_WIDTH;
        for &cy in &cross_ys {
            for x in 0..svc_left.min(hw) {
                for y in cy..cy + CROSS_CORRIDOR_WIDTH {
                    if grid[x][y] == CELL_EMPTY {
                        grid[x][y] = CELL_MAIN_CORRIDOR;
                    }
                }
            }
        }

        // Service corridor: starboard edge, full length
        for x in svc_left..hw {
            for y in 0..hl {
                if grid[x][y] == CELL_EMPTY {
                    grid[x][y] = CELL_SERVICE_CORRIDOR;
                }
            }
        }

        // ---- Phase 2: Shafts at corridor intersections ----
        let shaft_placements = compute_shaft_placements(
            spine_right, svc_left, &cross_ys, hw, hl,
        );

        for sp in &shaft_placements {
            for sx in sp.x..((sp.x + sp.w).min(hw)) {
                for sy in sp.y..((sp.y + sp.h).min(hl)) {
                    grid[sx][sy] = CELL_SHAFT;
                }
            }
        }

        // Create corridor Room entries
        // Spine segments (between cross-corridors)
        let mut spine_segments: Vec<(u32, usize, usize)> = Vec::new(); // (room_id, y_start, y_end)
        {
            let mut seg_boundaries: Vec<usize> = vec![0];
            for &cy in &cross_ys {
                seg_boundaries.push(cy);
                seg_boundaries.push(cy + CROSS_CORRIDOR_WIDTH);
            }
            seg_boundaries.push(hl);

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
            y: 0.0,
            width: SPINE_WIDTH as f32,
            length: hl as f32,
            orientation: 1,
            carries: carries_flags::CREW_PATH | carries_flags::POWER | carries_flags::DATA,
        });

        // Cross-corridor Room entries
        let mut cross_rooms: Vec<(u32, usize)> = Vec::new(); // (room_id, y_start)
        for &cy in &cross_ys {
            let cc_id = next_id();
            ctx.db.room().insert(Room {
                id: cc_id,
                node_id: 0,
                name: format!("Cross-Corridor D{} Y{}", deck + 1, cy),
                room_type: room_types::CROSS_CORRIDOR,
                deck,
                x: svc_left as f32 / 2.0,
                y: cy as f32 + CROSS_CORRIDOR_WIDTH as f32 / 2.0,
                width: svc_left as f32,
                height: CROSS_CORRIDOR_WIDTH as f32,
                capacity: 0,
            });
            ctx.db.corridor().insert(Corridor {
                id: 0,
                deck,
                corridor_type: corridor_types::BRANCH,
                x: 0.0,
                y: cy as f32,
                width: svc_left as f32,
                length: CROSS_CORRIDOR_WIDTH as f32,
                orientation: 0,
                carries: carries_flags::CREW_PATH,
            });
            cross_rooms.push((cc_id, cy));
        }

        // Service corridor Room entry
        let svc_id = next_id();
        ctx.db.room().insert(Room {
            id: svc_id,
            node_id: 0,
            name: format!("Service Corridor D{}", deck + 1),
            room_type: room_types::SERVICE_CORRIDOR,
            deck,
            x: svc_left as f32 + SVC_CORRIDOR_WIDTH as f32 / 2.0,
            y: hl as f32 / 2.0,
            width: SVC_CORRIDOR_WIDTH as f32,
            height: hl as f32,
            capacity: 0,
        });
        ctx.db.corridor().insert(Corridor {
            id: 0,
            deck,
            corridor_type: corridor_types::SERVICE,
            x: svc_left as f32,
            y: 0.0,
            width: SVC_CORRIDOR_WIDTH as f32,
            length: hl as f32,
            orientation: 1,
            carries: carries_flags::POWER
                | carries_flags::WATER
                | carries_flags::HVAC
                | carries_flags::COOLANT,
        });

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
            // Cross-corridor ↔ service corridor
            let dx = svc_left as f32;
            let dy = cy as f32 + CROSS_CORRIDOR_WIDTH as f32 / 2.0;
            ctx.db.door().insert(Door {
                id: 0,
                room_a: cc_id,
                room_b: svc_id,
                wall_a: wall_sides::EAST,
                wall_b: wall_sides::WEST,
                position_along_wall: 0.5,
                width: CROSS_CORRIDOR_WIDTH as f32,
                access_level: access_levels::CREW_ONLY,
                door_x: dx,
                door_y: dy,
            });
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
        for (si, sp) in shaft_placements.iter().enumerate() {
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
            if si < shaft_infos.len() {
                shaft_infos[si].deck_room_ids[deck as usize] = Some(shaft_room_id);
                if deck == deck_count as i32 / 2 {
                    shaft_infos[si].ref_x = sp.x as f32 + sp.w as f32 / 2.0;
                    shaft_infos[si].ref_y = sp.y as f32 + sp.h as f32 / 2.0;
                    shaft_infos[si].ref_w = sp.w as f32;
                    shaft_infos[si].ref_h = sp.h as f32;
                }
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
                svc_id,
                spine_left,
                spine_right,
                svc_left,
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
            svc_left,
            &cross_ys,
            &spine_segments,
            &cross_rooms,
        );

        // ---- Phase 4: Collect room requests for this deck ----
        let mut deck_requests: Vec<RoomRequest> = Vec::new();
        for zone in 0..7u8 {
            let (lo, hi) = deck_range_for_zone(zone, deck_count);
            if (deck as u32) >= lo && (deck as u32) < hi {
                // How many decks in this zone?
                let zone_decks = hi.saturating_sub(lo).max(1);
                let deck_index_in_zone = (deck as u32).saturating_sub(lo);
                let requests = &zone_requests[zone as usize];
                // Distribute requests round-robin across zone decks
                for (i, req) in requests.iter().enumerate() {
                    if (i as u32 % zone_decks) == deck_index_in_zone {
                        deck_requests.push(req.clone());
                    }
                }
            }
        }
        // Sort: largest first
        deck_requests.sort_by(|a, b| {
            b.target_area
                .partial_cmp(&a.target_area)
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        // ---- Phase 5: BSP pack rooms into attachment strips ----
        let mut placed_rooms: Vec<(u32, usize, usize, usize, usize, u8)> = Vec::new();
        let mut request_idx = 0;
        let total_strip_area: usize = strips.iter().map(|s| s.w * s.h).sum();
        let total_request_area: f32 = deck_requests.iter().map(|r| r.target_area).sum();

        for strip in &strips {
            if request_idx >= deck_requests.len() {
                break;
            }
            // BSP subdivide this strip and pack rooms
            let mut sub_rects: Vec<(usize, usize, usize, usize)> = Vec::new();
            bsp_subdivide(strip.x, strip.y, strip.w, strip.h, &deck_requests[request_idx..], &mut sub_rects);

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
                    if has_conflict { break; }
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

                // Create door to adjacent corridor (atomically)
                let (door_x, door_y, wall_room, wall_corr) =
                    compute_door_position(*rx, *ry, *rw, *rh, &strip);
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

                placed_rooms.push((room_id, *rx, *ry, *rw, *rh, req.room_type));
                request_idx += 1;
            }
        }

        // ---- Phase 6: Room-to-room doors (adjacent logical pairs) ----
        for i in 0..placed_rooms.len() {
            for j in (i + 1)..placed_rooms.len() {
                let (id_a, ax, ay, aw, ah, rt_a) = placed_rooms[i];
                let (id_b, bx, by, bw, bh, rt_b) = placed_rooms[j];
                if !should_have_room_door(rt_a, rt_b) {
                    continue;
                }
                // Check adjacency (shared edge)
                if let Some((dx, dy, wa, wb)) =
                    find_shared_edge(ax, ay, aw, ah, bx, by, bw, bh)
                {
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
                    v => {
                        let idx = (v - CELL_ROOM_BASE) as usize;
                        (b'A' + (idx % 26) as u8) as char
                    }
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
            deck_requests.iter().take(request_idx).map(|r| r.target_area).sum::<f32>(),
            total_request_area,
            total_strip_area,
        );
    } // end per-deck loop

    // ---- VerticalShaft table entries + cross-deck doors ----
    let decks_str = (0..deck_count)
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(",");

    for si in &shaft_infos {
        ctx.db.vertical_shaft().insert(VerticalShaft {
            id: 0,
            shaft_type: si.shaft_type,
            name: si.name.to_string(),
            x: si.ref_x,
            y: si.ref_y,
            decks_served: decks_str.clone(),
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

/// Compute shaft placements at corridor intersections.
fn compute_shaft_placements(
    spine_right: usize,
    svc_left: usize,
    cross_ys: &[usize],
    hw: usize,
    hl: usize,
) -> Vec<ShaftPlacement> {
    let mut placements = Vec::new();

    if cross_ys.is_empty() {
        // Minimal deck: place one elevator next to spine
        placements.push(ShaftPlacement {
            x: spine_right,
            y: hl / 4,
            w: 3,
            h: 3,
            shaft_type: shaft_types::ELEVATOR,
            name: "Fore Elevator",
            is_main: true,
        });
        return placements;
    }

    // Place shafts BESIDE cross-corridor intersections, not ON them.
    // Offset below the cross-corridor so they don't block corridor traffic.
    let cross_end_offset = CROSS_CORRIDOR_WIDTH; // place just below cross-corridor

    // Fore elevator: starboard of spine, just below first cross-corridor
    placements.push(ShaftPlacement {
        x: spine_right,
        y: cross_ys[0] + cross_end_offset,
        w: 3,
        h: 3,
        shaft_type: shaft_types::ELEVATOR,
        name: "Fore Elevator",
        is_main: true,
    });

    // Aft elevator: starboard of spine, just below last cross-corridor
    let last_cy = *cross_ys.last().unwrap();
    placements.push(ShaftPlacement {
        x: spine_right,
        y: last_cy + cross_end_offset,
        w: 3,
        h: 3,
        shaft_type: shaft_types::ELEVATOR,
        name: "Aft Elevator",
        is_main: true,
    });

    // Service elevator: beside service corridor, just below middle cross-corridor
    if svc_left >= 2 {
        let svc_elev_y = cross_ys[cross_ys.len() / 2];
        placements.push(ShaftPlacement {
            x: svc_left - 2,
            y: svc_elev_y + cross_end_offset,
            w: 2,
            h: 2,
            shaft_type: shaft_types::SERVICE_ELEVATOR,
            name: "Service Elevator",
            is_main: false,
        });
    }

    // Ladders: port side of spine, just below intermediate cross-corridors
    let spine_left_edge = (hw / 2).saturating_sub(SPINE_WIDTH / 2);
    let ladder_positions: Vec<usize> = cross_ys
        .iter()
        .enumerate()
        .filter(|(i, _)| *i > 0 && *i < cross_ys.len() - 1)
        .map(|(_, cy)| *cy)
        .take(2)
        .collect();

    for (li, &cy) in ladder_positions.iter().enumerate() {
        let name = if li == 0 { "Ladder A" } else { "Ladder B" };
        placements.push(ShaftPlacement {
            x: spine_left_edge.saturating_sub(2),
            y: cy + cross_end_offset,
            w: 2,
            h: 2,
            shaft_type: shaft_types::LADDER,
            name,
            is_main: false,
        });
    }

    placements
}

/// Find attachment strips: empty rectangular areas directly adjacent to corridor walls.
fn find_attachment_strips(
    grid: &[Vec<u8>],
    _hw: usize,
    hl: usize,
    spine_left: usize,
    spine_right: usize,
    svc_left: usize,
    cross_ys: &[usize],
    spine_segments: &[(u32, usize, usize)],
    cross_rooms: &[(u32, usize)],
) -> Vec<AttachmentStrip> {
    let mut strips = Vec::new();

    // Port side of spine (x < spine_left)
    // Between consecutive cross-corridors (and before first / after last)
    let mut y_boundaries: Vec<usize> = vec![0];
    for &cy in cross_ys {
        y_boundaries.push(cy);
        y_boundaries.push(cy + CROSS_CORRIDOR_WIDTH);
    }
    y_boundaries.push(hl);

    for chunk in y_boundaries.chunks(2) {
        if chunk.len() < 2 || chunk[0] >= chunk[1] {
            continue;
        }
        let y0 = chunk[0];
        let y1 = chunk[1];
        let strip_h = y1 - y0;
        let strip_w = spine_left; // port side width

        if strip_w >= MIN_ROOM_DIM && strip_h >= MIN_ROOM_DIM {
            // Find which corridor this strip connects to
            let corridor_id = find_corridor_for_strip(
                spine_left, y0, y1, spine_segments, cross_rooms,
            );
            strips.push(AttachmentStrip {
                corridor_room_id: corridor_id,
                x: 0,
                y: y0,
                w: strip_w,
                h: strip_h,
                wall_side: wall_sides::WEST,
                door_x: spine_left,
                door_y: y0 + strip_h / 2,
            });
        }
    }

    // Starboard side of spine (spine_right..svc_left)
    for chunk in y_boundaries.chunks(2) {
        if chunk.len() < 2 || chunk[0] >= chunk[1] {
            continue;
        }
        let y0 = chunk[0];
        let y1 = chunk[1];
        let strip_h = y1 - y0;
        let strip_x = spine_right;
        let strip_w = svc_left.saturating_sub(spine_right);

        if strip_w >= MIN_ROOM_DIM && strip_h >= MIN_ROOM_DIM {
            // Exclude shaft areas — scan for actual empty width
            let actual_w = scan_empty_width(grid, strip_x, y0, strip_w, strip_h);
            if actual_w >= MIN_ROOM_DIM {
                let corridor_id = find_corridor_for_strip(
                    spine_right, y0, y1, spine_segments, cross_rooms,
                );
                strips.push(AttachmentStrip {
                    corridor_room_id: corridor_id,
                    x: strip_x,
                    y: y0,
                    w: actual_w,
                    h: strip_h,
                    wall_side: wall_sides::EAST,
                    door_x: spine_right,
                    door_y: y0 + strip_h / 2,
                });
            }
        }
    }

    // Sort strips largest-first for better room placement
    strips.sort_by(|a, b| (b.w * b.h).cmp(&(a.w * a.h)));
    strips
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

    // Split the rectangle and distribute requests
    let split_at = requests.len() / 2;
    let area_ratio = requests[..split_at]
        .iter()
        .map(|r| r.target_area)
        .sum::<f32>()
        / requests.iter().map(|r| r.target_area).sum::<f32>();

    if w >= h {
        // Vertical split (split along X)
        let split_x = (w as f32 * area_ratio).round() as usize;
        let split_x = split_x.max(MIN_ROOM_DIM).min(w.saturating_sub(MIN_ROOM_DIM));
        if split_x >= MIN_ROOM_DIM && w - split_x >= MIN_ROOM_DIM {
            bsp_subdivide(x, y, split_x, h, &requests[..split_at], out);
            bsp_subdivide(x + split_x, y, w - split_x, h, &requests[split_at..], out);
        } else {
            // Can't split further — assign first request
            out.push((x, y, w, h));
        }
    } else {
        // Horizontal split (split along Y)
        let split_y = (h as f32 * area_ratio).round() as usize;
        let split_y = split_y.max(MIN_ROOM_DIM).min(h.saturating_sub(MIN_ROOM_DIM));
        if split_y >= MIN_ROOM_DIM && h - split_y >= MIN_ROOM_DIM {
            bsp_subdivide(x, y, w, split_y, &requests[..split_at], out);
            bsp_subdivide(x, y + split_y, w, h - split_y, &requests[split_at..], out);
        } else {
            out.push((x, y, w, h));
        }
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
fn connect_shaft_to_corridor(
    ctx: &ReducerContext,
    shaft_room_id: u32,
    sp: &ShaftPlacement,
    spine_segments: &[(u32, usize, usize)],
    cross_rooms: &[(u32, usize)],
    svc_id: u32,
    spine_left: usize,
    spine_right: usize,
    svc_left: usize,
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

    // Check if adjacent to service corridor
    if sp.x + sp.w >= svc_left {
        ctx.db.door().insert(Door {
            id: 0,
            room_a: shaft_room_id,
            room_b: svc_id,
            wall_a: wall_sides::EAST,
            wall_b: wall_sides::WEST,
            position_along_wall: 0.5,
            width: sp.h.min(sp.w) as f32,
            access_level: access,
            door_x: svc_left as f32,
            door_y: sp.y as f32 + sp.h as f32 / 2.0,
        });
    }
}

/// Find a shared edge between two adjacent rooms. Returns (door_x, door_y, wall_a, wall_b).
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
