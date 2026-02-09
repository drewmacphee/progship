//! Ship layout infrastructure - corridors, shafts, room placement.
//!
//! Infrastructure-first layout: spine corridor, cross-corridors, service corridor,
//! vertical shafts, then treemap-based room placement in empty zones.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};
use super::hull::*;
use super::zones::{find_empty_zones, CELL_EMPTY, GridZone};
use super::treemap::squarified_treemap;
use super::doors::should_have_room_door;
use super::facilities::deck_range_for_zone;

/// Simple deterministic RNG for layout
pub struct SimpleRng {
    state: u64,
}
impl SimpleRng {
    pub fn from_name(name: &str) -> Self {
        let mut hash: u64 = 5381;
        for b in name.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(b as u64);
        }
        Self { state: hash }
    }
    pub fn next_f32(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let bits = (self.state >> 32) as u32;
        bits as f32 / (u32::MAX as f32)
    }
    #[allow(dead_code)]
    pub fn next_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
    #[allow(dead_code)]
    pub fn next_usize(&mut self, min: usize, max: usize) -> usize {
        if max <= min {
            return min;
        }
        let f = self.next_f32();
        let range = max - min;
        min + (f * range as f32) as usize
    }
}

/// Room request for treemap placement.
#[derive(Clone)]
pub struct RoomRequest {
    pub node_id: u64,
    pub name: String,
    pub room_type: u8,
    pub target_area: f32,
    pub capacity: u32,
    pub group: u8,
}

/// Placed room result from treemap.
#[allow(dead_code)]
pub struct PlacedRoom {
    pub room_id: u32,
    pub node_id: u64,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub room_type: u8,
}

// Grid cell values
const CELL_MAIN_CORRIDOR: u8 = 1;
const CELL_SERVICE_CORRIDOR: u8 = 2;
const CELL_SHAFT: u8 = 3;
const CELL_ROOM_BASE: u8 = 10;

pub fn layout_ship(ctx: &ReducerContext, deck_count: u32) {
    let ship_name = ctx
        .db
        .ship_config()
        .id()
        .find(0)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let _rng = SimpleRng::from_name(&ship_name);
    let nodes: Vec<GraphNode> = ctx.db.graph_node().iter().collect();

    // Build per-deck-zone room request lists from graph nodes
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
    // Sort each zone's requests: largest rooms first for better treemap packing
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

    // Per-deck shaft positions are computed inside the deck loop below

    /// Spine segment info for a deck: (room_id, y_start, y_end)
    struct SpineSegment {
        room_id: u32,
        y_start: usize,
        y_end: usize,
    }

    /// Cross-corridor Room info: (room_id, y_start)
    struct CrossCorridorRoom {
        room_id: u32,
        y_start: usize,
    }

    for deck in 0..deck_count as i32 {
        // Hull taper per deck
        let hull_width: usize = match deck as u32 {
            0..=1 => 40,
            d if d >= deck_count.saturating_sub(2) => 50,
            _ => SHIP_BEAM,
        };
        let hull_length: usize = match deck as u32 {
            0..=1 => 200,
            d if d >= deck_count.saturating_sub(2) => 300,
            _ => SHIP_LENGTH,
        };

        // Shaft positions relative to THIS deck's hull
        let deck_spine_cx = hull_width / 2;
        // Place elevators adjacent to spine (just outside it on starboard side)
        let fore_elev_deck = (deck_spine_cx + 2, 10usize, 3usize, 3usize);
        let aft_elev_deck = (
            deck_spine_cx + 2,
            if hull_length > 20 {
                hull_length - 14
            } else {
                hull_length / 2
            },
            3,
            3,
        );
        let svc_elev_deck = (
            hull_width.saturating_sub(5),
            100usize.min(hull_length.saturating_sub(5)),
            2,
            2,
        );
        let ladders_deck: Vec<(usize, usize, usize, usize)> = [50, 150, 250, 350]
            .iter()
            .filter(|&&ly| ly + 2 <= hull_length)
            .map(|&ly| (hull_width.saturating_sub(4), ly, 2, 2))
            .collect();

        // Allocate grid: grid[x][y], size [hull_width][hull_length]
        let mut grid: Vec<Vec<u8>> = vec![vec![CELL_EMPTY; hull_length]; hull_width];

        // ---- Step 1: Stamp corridor skeleton ----

        // Main spine: SPINE_WIDTH cells wide, centered, full length
        let spine_left = hull_width / 2 - SPINE_WIDTH / 2;
        let spine_right = spine_left + SPINE_WIDTH;
        for x in spine_left..spine_right.min(hull_width) {
            for y in 0..hull_length {
                grid[x][y] = CELL_MAIN_CORRIDOR;
            }
        }

        // Compute service corridor boundary early (needed by cross-corridors)
        let svc_left = hull_width.saturating_sub(SVC_CORRIDOR_WIDTH);

        // Cross-corridors: CROSS_CORRIDOR_WIDTH cells wide, horizontal, every CROSS_CORRIDOR_SPACING
        // Only span from x=0 to svc_left (stop before service corridor)
        let mut cross_corridor_ys: Vec<usize> = Vec::new();
        let mut cy = CROSS_CORRIDOR_SPACING;
        while cy + CROSS_CORRIDOR_WIDTH <= hull_length {
            for x in 0..svc_left {
                for dy in 0..CROSS_CORRIDOR_WIDTH {
                    let yy = cy + dy;
                    if yy < hull_length {
                        // Don't overwrite shaft cells (will be stamped later, but we
                        // pre-check to keep cross-corridor Room bounds accurate)
                        if grid[x][yy] == CELL_EMPTY {
                            grid[x][yy] = CELL_MAIN_CORRIDOR;
                        }
                    }
                }
            }
            cross_corridor_ys.push(cy);
            cy += CROSS_CORRIDOR_SPACING;
        }

        // FIX 1: Create SEGMENTED spine Room entries (one per section between cross-corridors)
        // Boundaries are: 0, cross1_start, cross1_end, cross2_start, ..., hull_length
        let mut spine_segments: Vec<SpineSegment> = Vec::new();
        {
            let mut seg_start = 0usize;
            for &ccy in &cross_corridor_ys {
                // Spine segment from seg_start to ccy (just before cross-corridor)
                if ccy > seg_start {
                    let seg_len = ccy - seg_start;
                    let sid = next_id();
                    let seg_cy = seg_start as f32 + seg_len as f32 / 2.0;
                    ctx.db.room().insert(Room {
                        id: sid,
                        node_id: 0,
                        name: format!("Deck {} Spine Seg {}", deck + 1, spine_segments.len() + 1),
                        room_type: room_types::CORRIDOR,
                        deck,
                        x: (spine_left + spine_right) as f32 / 2.0,
                        y: seg_cy,
                        width: SPINE_WIDTH as f32,
                        height: seg_len as f32,
                        capacity: 50,
                    });
                    spine_segments.push(SpineSegment {
                        room_id: sid,
                        y_start: seg_start,
                        y_end: ccy,
                    });
                }
                // Skip past the cross-corridor band (seg_start advances after it)
                seg_start = ccy + CROSS_CORRIDOR_WIDTH;
            }
            // Final segment after last cross-corridor to hull end
            if seg_start < hull_length {
                let seg_len = hull_length - seg_start;
                let sid = next_id();
                let seg_cy = seg_start as f32 + seg_len as f32 / 2.0;
                ctx.db.room().insert(Room {
                    id: sid,
                    node_id: 0,
                    name: format!("Deck {} Spine Seg {}", deck + 1, spine_segments.len() + 1),
                    room_type: room_types::CORRIDOR,
                    deck,
                    x: (spine_left + spine_right) as f32 / 2.0,
                    y: seg_cy,
                    width: SPINE_WIDTH as f32,
                    height: seg_len as f32,
                    capacity: 50,
                });
                spine_segments.push(SpineSegment {
                    room_id: sid,
                    y_start: seg_start,
                    y_end: hull_length,
                });
            }
        }

        // Corridor table entry for full spine (rendering uses Corridor table)
        ctx.db.corridor().insert(Corridor {
            id: 0,
            deck,
            corridor_type: corridor_types::MAIN,
            x: (spine_left + spine_right) as f32 / 2.0,
            y: hull_length as f32 / 2.0,
            width: SPINE_WIDTH as f32,
            length: hull_length as f32,
            orientation: 1,
            carries: carries_flags::CREW_PATH | carries_flags::POWER | carries_flags::DATA,
        });

        // Doors connecting adjacent spine segments (through cross-corridors).
        // Spine segments are not directly connected by doors here; instead:
        //   seg_a SOUTH → cross-corridor NORTH
        //   cross-corridor SOUTH → seg_b NORTH
        // Door creation between spine and cross-corridor rooms is handled elsewhere
        // in the generation pipeline; no direct spine-to-spine doors are added here.

        // FIX 2: Create Room entries for each cross-corridor
        // Width limited to svc_left (does not extend into service corridor zone)
        // Shafts may sit inside the cross-corridor — that overlap is tolerated
        let mut cross_rooms: Vec<CrossCorridorRoom> = Vec::new();
        for (ci, &ccy) in cross_corridor_ys.iter().enumerate() {
            let cross_cy_f = ccy as f32 + CROSS_CORRIDOR_WIDTH as f32 / 2.0;
            let crid = next_id();
            let cross_width = svc_left as f32;
            ctx.db.room().insert(Room {
                id: crid,
                node_id: 0,
                name: format!("Deck {} Cross-Corridor {}", deck + 1, ci + 1),
                room_type: room_types::CROSS_CORRIDOR,
                deck,
                x: cross_width / 2.0,
                y: cross_cy_f,
                width: cross_width,
                height: CROSS_CORRIDOR_WIDTH as f32,
                capacity: 20,
            });
            ctx.db.corridor().insert(Corridor {
                id: 0,
                deck,
                corridor_type: corridor_types::BRANCH,
                x: cross_width / 2.0,
                y: cross_cy_f,
                width: cross_width,
                length: CROSS_CORRIDOR_WIDTH as f32,
                orientation: 0,
                carries: carries_flags::CREW_PATH,
            });
            cross_rooms.push(CrossCorridorRoom {
                room_id: crid,
                y_start: ccy,
            });

            // Door from cross-corridor to adjacent spine segments
            // The cross-corridor sits between spine segment i and i+1
            // Connect to the segment that ends at ccy (shared edge at y=ccy)
            // and segment that starts at ccy+CROSS_CORRIDOR_WIDTH (shared edge there)
            for seg in &spine_segments {
                if seg.y_end == ccy {
                    // Spine segment's south edge at y=ccy, cross-corridor's north edge at y=ccy
                    // Door at spine center X, boundary Y = ccy
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: crid,
                        room_b: seg.room_id,
                        wall_a: wall_sides::NORTH,
                        wall_b: wall_sides::SOUTH,
                        position_along_wall: 0.5,
                        width: SPINE_WIDTH as f32,
                        access_level: access_levels::PUBLIC,
                        door_x: spine_center_x,
                        door_y: ccy as f32,
                    });
                }
                if seg.y_start == ccy + CROSS_CORRIDOR_WIDTH {
                    // Cross-corridor's south edge at y=ccy+width, spine segment's north edge there
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: crid,
                        room_b: seg.room_id,
                        wall_a: wall_sides::SOUTH,
                        wall_b: wall_sides::NORTH,
                        position_along_wall: 0.5,
                        width: SPINE_WIDTH as f32,
                        access_level: access_levels::PUBLIC,
                        door_x: spine_center_x,
                        door_y: (ccy + CROSS_CORRIDOR_WIDTH) as f32,
                    });
                }
            }
        }

        // Service corridor: SVC_CORRIDOR_WIDTH cells wide, along starboard (right) edge
        // (svc_left already computed above before cross-corridors)
        for x in svc_left..hull_width {
            for y in 0..hull_length {
                grid[x][y] = CELL_SERVICE_CORRIDOR;
            }
        }
        let svc_rid = next_id();
        ctx.db.room().insert(Room {
            id: svc_rid,
            node_id: 0,
            name: format!("Deck {} Service Corridor", deck + 1),
            room_type: room_types::SERVICE_CORRIDOR,
            deck,
            x: (svc_left as f32 + hull_width as f32) / 2.0,
            y: hull_length as f32 / 2.0,
            width: SVC_CORRIDOR_WIDTH as f32,
            height: hull_length as f32,
            capacity: 4,
        });
        ctx.db.corridor().insert(Corridor {
            id: 0,
            deck,
            corridor_type: corridor_types::SERVICE,
            x: (svc_left as f32 + hull_width as f32) / 2.0,
            y: hull_length as f32 / 2.0,
            width: SVC_CORRIDOR_WIDTH as f32,
            length: hull_length as f32,
            orientation: 1,
            carries: carries_flags::POWER
                | carries_flags::WATER
                | carries_flags::HVAC
                | carries_flags::COOLANT,
        });

        // Door connecting service corridor to each cross-corridor
        for cr in &cross_rooms {
            // Service corridor's west edge at x=svc_left, cross-corridor's east side
            // Door at the shared boundary X=svc_left, centered in the cross-corridor Y range
            let cr_cy = cr.y_start as f32 + CROSS_CORRIDOR_WIDTH as f32 / 2.0;
            ctx.db.door().insert(Door {
                id: 0,
                room_a: svc_rid,
                room_b: cr.room_id,
                wall_a: wall_sides::WEST,
                wall_b: wall_sides::EAST,
                position_along_wall: 0.5,
                width: 2.0,
                access_level: access_levels::CREW_ONLY,
                door_x: svc_left as f32,
                door_y: cr_cy,
            });
        }

        // Helper closures for finding corridor segments by Y coordinate
        let find_spine_segment = |y: usize| -> Option<&SpineSegment> {
            spine_segments
                .iter()
                .find(|s| y >= s.y_start && y < s.y_end)
        };
        let find_cross_room = |y: usize| -> Option<&CrossCorridorRoom> {
            cross_rooms
                .iter()
                .find(|c| y >= c.y_start && y < c.y_start + CROSS_CORRIDOR_WIDTH)
        };

        // ---- Step 2: Stamp vertical shaft anchors ----
        let all_shafts: Vec<(usize, usize, usize, usize, u8, u8, &str, bool)> = {
            let mut v = Vec::new();
            v.push((
                fore_elev_deck.0,
                fore_elev_deck.1,
                fore_elev_deck.2,
                fore_elev_deck.3,
                shaft_types::ELEVATOR,
                room_types::ELEVATOR_SHAFT,
                "Fore Elevator",
                true,
            ));
            v.push((
                aft_elev_deck.0,
                aft_elev_deck.1,
                aft_elev_deck.2,
                aft_elev_deck.3,
                shaft_types::ELEVATOR,
                room_types::ELEVATOR_SHAFT,
                "Aft Elevator",
                true,
            ));
            v.push((
                svc_elev_deck.0,
                svc_elev_deck.1,
                svc_elev_deck.2,
                svc_elev_deck.3,
                shaft_types::SERVICE_ELEVATOR,
                room_types::ELEVATOR_SHAFT,
                "Service Elevator",
                false,
            ));
            for (li, &(lx, ly, lw, lh)) in ladders_deck.iter().enumerate() {
                v.push((
                    lx,
                    ly,
                    lw,
                    lh,
                    shaft_types::LADDER,
                    room_types::LADDER_SHAFT,
                    match li {
                        0 => "Ladder A",
                        1 => "Ladder B",
                        2 => "Ladder C",
                        _ => "Ladder D",
                    },
                    false,
                ));
            }
            v
        };

        for &(sx, sy, sw, sh, _shaft_type, srt, sname, is_main) in &all_shafts {
            if sx + sw > hull_width || sy + sh > hull_length {
                continue;
            }

            for xx in sx..(sx + sw) {
                for yy in sy..(sy + sh) {
                    grid[xx][yy] = CELL_SHAFT;
                }
            }

            let rid = next_id();
            ctx.db.room().insert(Room {
                id: rid,
                node_id: 0,
                name: format!("{} D{}", sname, deck + 1),
                room_type: srt,
                deck,
                x: sx as f32 + sw as f32 / 2.0,
                y: sy as f32 + sh as f32 / 2.0,
                width: sw as f32,
                height: sh as f32,
                capacity: if is_main { 6 } else { 2 },
            });

            // Connect shaft to adjacent corridor via shared edge
            let access = if is_main {
                access_levels::PUBLIC
            } else {
                access_levels::CREW_ONLY
            };
            let shaft_cy = sy + sh / 2;
            let shaft_cx = sx + sw / 2;
            let shaft_center_x = sx as f32 + sw as f32 / 2.0;
            let shaft_center_y = sy as f32 + sh as f32 / 2.0;

            // First: check if shaft overlaps a cross-corridor (shaft sits inside it)
            // If so, connect to it at the shaft's north or south edge
            let mut connected = false;
            for cr in &cross_rooms {
                let cr_end = cr.y_start + CROSS_CORRIDOR_WIDTH;
                // Shaft overlaps cross-corridor if their Y ranges intersect
                if sy < cr_end && sy + sh > cr.y_start {
                    // Connect via shaft's WEST edge to the cross-corridor.
                    // Shaft is embedded inside the corridor — no corridor wall at this boundary.
                    // wall_a=WEST creates gap in shaft wall; wall_b=255 skips corridor gap.
                    let boundary_x = sx as f32;
                    ctx.db.door().insert(Door {
                        id: 0,
                        room_a: rid,
                        room_b: cr.room_id,
                        wall_a: wall_sides::WEST,
                        wall_b: 255,
                        position_along_wall: 0.5,
                        width: sh.min(CROSS_CORRIDOR_WIDTH) as f32,
                        access_level: access,
                        door_x: boundary_x,
                        door_y: shaft_center_y,
                    });
                    connected = true;
                    break;
                }
            }

            // Then check all 4 edges for adjacent corridor cells in the grid

            // SOUTH edge of shaft (y + sh): check if corridor is below
            if sy + sh < hull_length {
                let test_y = sy + sh;
                let test_x = shaft_cx.min(hull_width - 1);
                if grid[test_x][test_y] == CELL_MAIN_CORRIDOR
                    || test_y < hull_length
                        && grid[test_x.min(hull_width - 1)][test_y] == CELL_MAIN_CORRIDOR
                {
                    // Prefer a spine segment; if none, fall back to a cross-corridor room.
                    let target_room_id = find_spine_segment(test_y)
                        .map(|seg| seg.room_id)
                        .or_else(|| find_cross_room(test_y));
                    if let Some(room_id) = target_room_id {
                        let boundary_y = (sy + sh) as f32;
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: rid,
                            room_b: room_id,
                            wall_a: wall_sides::SOUTH,
                            wall_b: wall_sides::NORTH,
                            position_along_wall: 0.5,
                            width: sw as f32,
                            access_level: access,
                            door_x: shaft_center_x,
                            door_y: boundary_y,
                        });
                        connected = true;
                    }
                }
            }

            // NORTH edge of shaft (y - 1): check if corridor is above
            if sy > 0 && !connected {
                let test_y = sy - 1;
                let test_x = shaft_cx.min(hull_width - 1);
                if grid[test_x][test_y] == CELL_MAIN_CORRIDOR {
                    if let Some(seg) = find_spine_segment(test_y) {
                        let boundary_y = sy as f32;
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: rid,
                            room_b: seg.room_id,
                            wall_a: wall_sides::NORTH,
                            wall_b: wall_sides::SOUTH,
                            position_along_wall: 0.5,
                            width: sw as f32,
                            access_level: access,
                            door_x: shaft_center_x,
                            door_y: boundary_y,
                        });
                        connected = true;
                    }
                }
            }

            // EAST edge of shaft (x + sw): check if corridor is to the right
            if sx + sw < hull_width && !connected {
                let test_x = sx + sw;
                let test_y = shaft_cy.min(hull_length - 1);
                let cell = grid[test_x][test_y];
                if cell == CELL_MAIN_CORRIDOR || cell == CELL_SERVICE_CORRIDOR {
                    let boundary_x = (sx + sw) as f32;
                    let target_id = if cell == CELL_MAIN_CORRIDOR {
                        find_spine_segment(test_y).map(|s| s.room_id)
                    } else {
                        Some(svc_rid)
                    };
                    if let Some(tid) = target_id {
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: rid,
                            room_b: tid,
                            wall_a: wall_sides::EAST,
                            wall_b: wall_sides::WEST,
                            position_along_wall: 0.5,
                            width: sh as f32,
                            access_level: access,
                            door_x: boundary_x,
                            door_y: shaft_center_y,
                        });
                        connected = true;
                    }
                }
            }

            // WEST edge of shaft (x - 1): check if corridor is to the left
            if sx > 0 && !connected {
                let test_x = sx - 1;
                let test_y = shaft_cy.min(hull_length - 1);
                let cell = grid[test_x][test_y];
                if cell == CELL_MAIN_CORRIDOR || cell == CELL_SERVICE_CORRIDOR {
                    let boundary_x = sx as f32;
                    let target_id = if cell == CELL_MAIN_CORRIDOR {
                        find_spine_segment(test_y).map(|s| s.room_id)
                    } else {
                        Some(svc_rid)
                    };
                    if let Some(tid) = target_id {
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: rid,
                            room_b: tid,
                            wall_a: wall_sides::WEST,
                            wall_b: wall_sides::EAST,
                            position_along_wall: 0.5,
                            width: sh as f32,
                            access_level: access,
                            door_x: boundary_x,
                            door_y: shaft_center_y,
                        });
                        connected = true;
                    }
                }
            }

            // Shaft is either connected via cross-corridor overlap, edge adjacency, or remains isolated
        }

        // ---- Step 3: Find empty rectangular zones ----
        let zones = find_empty_zones(&grid, hull_width, hull_length);

        // ---- Step 4: Determine which rooms go on this deck ----
        let mut deck_room_requests: Vec<RoomRequest> = Vec::new();
        for zone_idx in 0..7u8 {
            let (lo, hi) = deck_range_for_zone(zone_idx, deck_count);
            if (deck as u32) >= lo && (deck as u32) < hi {
                let zone_deck_count = hi.saturating_sub(lo).max(1);
                let deck_offset = (deck as u32).saturating_sub(lo);
                let zone_reqs = &zone_requests[zone_idx as usize];
                let total_rooms = zone_reqs.len();
                let per_deck = total_rooms / zone_deck_count as usize;
                let extra = total_rooms % zone_deck_count as usize;
                let start = deck_offset as usize * per_deck + (deck_offset as usize).min(extra);
                let count = per_deck + if (deck_offset as usize) < extra { 1 } else { 0 };
                for i in start..(start + count).min(total_rooms) {
                    let rr = &zone_reqs[i];
                    deck_room_requests.push(RoomRequest {
                        node_id: rr.node_id,
                        name: rr.name.clone(),
                        room_type: rr.room_type,
                        target_area: rr.target_area,
                        capacity: rr.capacity,
                        group: rr.group,
                    });
                }
            }
        }

        if deck_room_requests.is_empty() {
            continue;
        }

        // ---- Step 5: Assign rooms to zones using squarified treemap ----
        // FIX 3: Distribute rooms PROPORTIONALLY across zones by area (not greedy)
        let mut placed_rooms: Vec<PlacedRoom> = Vec::new();
        let total_zone_area: f32 = zones
            .iter()
            .filter(|z| (z.w * z.h) as f32 >= 9.0)
            .map(|z| (z.w * z.h) as f32)
            .sum();
        let _total_room_area: f32 = deck_room_requests.iter().map(|r| r.target_area).sum();

        // Pre-allocate room counts per zone proportional to zone area
        let usable_zones: Vec<&GridZone> =
            zones.iter().filter(|z| (z.w * z.h) as f32 >= 9.0).collect();
        let mut rooms_per_zone: Vec<usize> = Vec::new();
        let mut allocated = 0usize;
        for (zi, zone) in usable_zones.iter().enumerate() {
            let zone_area = (zone.w * zone.h) as f32;
            let fraction = if total_zone_area > 0.0 {
                zone_area / total_zone_area
            } else {
                0.0
            };
            let room_count = if zi == usable_zones.len() - 1 {
                deck_room_requests.len().saturating_sub(allocated)
            } else {
                (fraction * deck_room_requests.len() as f32).round() as usize
            };
            let room_count = room_count.min(deck_room_requests.len().saturating_sub(allocated));
            rooms_per_zone.push(room_count);
            allocated += room_count;
        }

        let mut request_cursor = 0usize;
        for (zi, zone) in usable_zones.iter().enumerate() {
            if request_cursor >= deck_room_requests.len() {
                break;
            }
            let count = rooms_per_zone[zi];
            if count == 0 {
                continue;
            }

            let end = (request_cursor + count).min(deck_room_requests.len());
            let mut batch: Vec<(f32, usize)> = Vec::new();
            for i in request_cursor..end {
                batch.push((deck_room_requests[i].target_area, i));
            }
            request_cursor = end;

            if batch.is_empty() {
                continue;
            }

            let placements = squarified_treemap(&batch, zone.x, zone.y, zone.w, zone.h);

            for (orig_idx, rx, ry, rw, rh) in placements {
                if rw < 2 || rh < 2 {
                    continue;
                }
                let rr = &deck_room_requests[orig_idx];

                let cell_val = CELL_ROOM_BASE + (placed_rooms.len() % 246) as u8;
                for xx in rx..(rx + rw).min(hull_width) {
                    for yy in ry..(ry + rh).min(hull_length) {
                        if grid[xx][yy] == CELL_EMPTY {
                            grid[xx][yy] = cell_val;
                        }
                    }
                }

                let rid = next_id();
                ctx.db.room().insert(Room {
                    id: rid,
                    node_id: rr.node_id,
                    name: format!("{} D{}", rr.name, deck + 1),
                    room_type: rr.room_type,
                    deck,
                    x: rx as f32 + rw as f32 / 2.0,
                    y: ry as f32 + rh as f32 / 2.0,
                    width: rw as f32,
                    height: rh as f32,
                    capacity: rr.capacity,
                });

                placed_rooms.push(PlacedRoom {
                    room_id: rid,
                    node_id: rr.node_id,
                    x: rx,
                    y: ry,
                    w: rw,
                    h: rh,
                    room_type: rr.room_type,
                });
            }
        }

        // ---- Step 6: Generate doors ----
        let mut door_set: Vec<(u32, u32, u8)> = Vec::new();

        for pr in &placed_rooms {
            // Compute absolute door positions from shared edges.
            // Room grid coords: pr.x, pr.y, pr.w, pr.h
            // Room center: (pr.x + pr.w/2, pr.y + pr.h/2)
            let room_center_y = pr.y as f32 + pr.h as f32 / 2.0;
            let room_center_x = pr.x as f32 + pr.w as f32 / 2.0;

            // WEST edge (x - 1): room's west wall touches corridor to its left
            if pr.x > 0 {
                let test_x = pr.x - 1;
                let mid_y = pr.y + pr.h / 2;
                if mid_y < hull_length && test_x < hull_width {
                    let cell = grid[test_x][mid_y];
                    // Shared edge at x = pr.x (room's west boundary)
                    let boundary_x = pr.x as f32;
                    if cell == CELL_MAIN_CORRIDOR {
                        if let Some(seg) = find_spine_segment(mid_y) {
                            let key = (pr.room_id, seg.room_id, wall_sides::WEST);
                            if !door_set.contains(&key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: seg.room_id,
                                    wall_a: wall_sides::WEST,
                                    wall_b: wall_sides::EAST,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: boundary_x,
                                    door_y: room_center_y,
                                });
                                door_set.push(key);
                            }
                        } else if let Some(cr) = find_cross_room(mid_y) {
                            let key = (pr.room_id, cr.room_id, wall_sides::WEST);
                            if !door_set.contains(&key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: cr.room_id,
                                    wall_a: wall_sides::WEST,
                                    wall_b: wall_sides::EAST,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: boundary_x,
                                    door_y: room_center_y,
                                });
                                door_set.push(key);
                            }
                        }
                    } else if cell == CELL_SERVICE_CORRIDOR {
                        let key = (pr.room_id, svc_rid, wall_sides::WEST);
                        if !door_set.contains(&key) {
                            ctx.db.door().insert(Door {
                                id: 0,
                                room_a: pr.room_id,
                                room_b: svc_rid,
                                wall_a: wall_sides::WEST,
                                wall_b: wall_sides::EAST,
                                position_along_wall: 0.5,
                                width: 2.0,
                                access_level: access_levels::CREW_ONLY,
                                door_x: boundary_x,
                                door_y: room_center_y,
                            });
                            door_set.push(key);
                        }
                    }
                }
            }
            // EAST edge (x + w): room's east wall touches corridor to its right
            {
                let test_x = pr.x + pr.w;
                let mid_y = pr.y + pr.h / 2;
                if test_x < hull_width && mid_y < hull_length {
                    let cell = grid[test_x][mid_y];
                    // Shared edge at x = pr.x + pr.w (room's east boundary)
                    let boundary_x = (pr.x + pr.w) as f32;
                    if cell == CELL_MAIN_CORRIDOR {
                        if let Some(seg) = find_spine_segment(mid_y) {
                            let key = (pr.room_id, seg.room_id, wall_sides::EAST);
                            if !door_set.contains(&key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: seg.room_id,
                                    wall_a: wall_sides::EAST,
                                    wall_b: wall_sides::WEST,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: boundary_x,
                                    door_y: room_center_y,
                                });
                                door_set.push(key);
                            }
                        } else if let Some(cr) = find_cross_room(mid_y) {
                            let key = (pr.room_id, cr.room_id, wall_sides::EAST);
                            if !door_set.contains(&key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: cr.room_id,
                                    wall_a: wall_sides::EAST,
                                    wall_b: wall_sides::WEST,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: boundary_x,
                                    door_y: room_center_y,
                                });
                                door_set.push(key);
                            }
                        }
                    } else if cell == CELL_SERVICE_CORRIDOR {
                        let key = (pr.room_id, svc_rid, wall_sides::EAST);
                        if !door_set.contains(&key) {
                            ctx.db.door().insert(Door {
                                id: 0,
                                room_a: pr.room_id,
                                room_b: svc_rid,
                                wall_a: wall_sides::EAST,
                                wall_b: wall_sides::WEST,
                                position_along_wall: 0.5,
                                width: 2.0,
                                access_level: access_levels::CREW_ONLY,
                                door_x: boundary_x,
                                door_y: room_center_y,
                            });
                            door_set.push(key);
                        }
                    }
                }
            }
            // NORTH edge (y - 1): room's north wall touches corridor above
            if pr.y > 0 {
                let test_y = pr.y - 1;
                let mid_x = pr.x + pr.w / 2;
                if mid_x < hull_width && test_y < hull_length {
                    let cell = grid[mid_x][test_y];
                    // Shared edge at y = pr.y (room's north boundary — low Y = fore)
                    let boundary_y = pr.y as f32;
                    if cell == CELL_MAIN_CORRIDOR {
                        if let Some(seg) = find_spine_segment(test_y) {
                            let key = (pr.room_id, seg.room_id, wall_sides::NORTH);
                            if !door_set.contains(&key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: seg.room_id,
                                    wall_a: wall_sides::NORTH,
                                    wall_b: wall_sides::SOUTH,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: room_center_x,
                                    door_y: boundary_y,
                                });
                                door_set.push(key);
                            }
                        } else if let Some(cr) = find_cross_room(test_y) {
                            let key = (pr.room_id, cr.room_id, wall_sides::NORTH);
                            if !door_set.contains(&key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: cr.room_id,
                                    wall_a: wall_sides::NORTH,
                                    wall_b: wall_sides::SOUTH,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: room_center_x,
                                    door_y: boundary_y,
                                });
                                door_set.push(key);
                            }
                        }
                    }
                }
            }
            // SOUTH edge (y + h): room's south wall touches corridor below
            {
                let test_y = pr.y + pr.h;
                let mid_x = pr.x + pr.w / 2;
                if test_y < hull_length && mid_x < hull_width {
                    let cell = grid[mid_x][test_y];
                    // Shared edge at y = pr.y + pr.h (room's south boundary)
                    let boundary_y = (pr.y + pr.h) as f32;
                    if cell == CELL_MAIN_CORRIDOR {
                        if let Some(seg) = find_spine_segment(test_y) {
                            let key = (pr.room_id, seg.room_id, wall_sides::SOUTH);
                            if !door_set.contains(&key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: seg.room_id,
                                    wall_a: wall_sides::SOUTH,
                                    wall_b: wall_sides::NORTH,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: room_center_x,
                                    door_y: boundary_y,
                                });
                                door_set.push(key);
                            }
                        } else if let Some(cr) = find_cross_room(test_y) {
                            let key = (pr.room_id, cr.room_id, wall_sides::SOUTH);
                            if !door_set.contains(&key) {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: cr.room_id,
                                    wall_a: wall_sides::SOUTH,
                                    wall_b: wall_sides::NORTH,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: room_center_x,
                                    door_y: boundary_y,
                                });
                                door_set.push(key);
                            }
                        }
                    }
                }
            }
        }

        // Room-to-room doors: only for specific adjacent pairings that make logical sense
        // (e.g., galley↔mess hall, surgery↔hospital). Most rooms connect via corridors only.
        for i in 0..placed_rooms.len() {
            for j in (i + 1)..placed_rooms.len() {
                let a = &placed_rooms[i];
                let b = &placed_rooms[j];

                // Only connect rooms that should have direct internal doors
                if !should_have_room_door(a.room_type, b.room_type) {
                    continue;
                }

                // A's east edge touches B's west edge
                let boundary_x_ab = a.x + a.w;
                if boundary_x_ab == b.x
                    && boundary_x_ab > 0
                    && boundary_x_ab < hull_width
                    && a.y < b.y + b.h
                    && b.y < a.y + a.h
                {
                    let overlap_y0 = core::cmp::max(a.y, b.y);
                    let overlap_y1 = core::cmp::min(a.y + a.h, b.y + b.h);
                    if overlap_y1 > overlap_y0 {
                        let boundary_x = (a.x + a.w) as f32;
                        let mid_y = (overlap_y0 + overlap_y1) as f32 / 2.0;
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: a.room_id,
                            room_b: b.room_id,
                            wall_a: wall_sides::EAST,
                            wall_b: wall_sides::WEST,
                            position_along_wall: 0.5,
                            width: 2.0,
                            access_level: access_levels::CREW_ONLY,
                            door_x: boundary_x,
                            door_y: mid_y,
                        });
                    }
                } else if b.x + b.w == a.x
                    && a.x > 0
                    && a.x < hull_width
                    && a.y < b.y + b.h
                    && b.y < a.y + a.h
                {
                    let overlap_y0 = core::cmp::max(a.y, b.y);
                    let overlap_y1 = core::cmp::min(a.y + a.h, b.y + b.h);
                    if overlap_y1 > overlap_y0 {
                        let boundary_x = a.x as f32;
                        let mid_y = (overlap_y0 + overlap_y1) as f32 / 2.0;
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: a.room_id,
                            room_b: b.room_id,
                            wall_a: wall_sides::WEST,
                            wall_b: wall_sides::EAST,
                            position_along_wall: 0.5,
                            width: 2.0,
                            access_level: access_levels::CREW_ONLY,
                            door_x: boundary_x,
                            door_y: mid_y,
                        });
                    }
                }
                // A's south edge touches B's north edge
                let boundary_y_ab = a.y + a.h;
                if boundary_y_ab == b.y
                    && boundary_y_ab > 0
                    && boundary_y_ab < hull_length
                    && a.x < b.x + b.w
                    && b.x < a.x + a.w
                {
                    let overlap_x0 = core::cmp::max(a.x, b.x);
                    let overlap_x1 = core::cmp::min(a.x + a.w, b.x + b.w);
                    if overlap_x1 > overlap_x0 {
                        let boundary_y = (a.y + a.h) as f32;
                        let mid_x = (overlap_x0 + overlap_x1) as f32 / 2.0;
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: a.room_id,
                            room_b: b.room_id,
                            wall_a: wall_sides::SOUTH,
                            wall_b: wall_sides::NORTH,
                            position_along_wall: 0.5,
                            width: 2.0,
                            access_level: access_levels::CREW_ONLY,
                            door_x: mid_x,
                            door_y: boundary_y,
                        });
                    }
                } else if b.y + b.h == a.y
                    && a.y > 0
                    && a.y < hull_length
                    && a.x < b.x + b.w
                    && b.x < a.x + a.w
                {
                    let overlap_x0 = core::cmp::max(a.x, b.x);
                    let overlap_x1 = core::cmp::min(a.x + a.w, b.x + b.w);
                    if overlap_x1 > overlap_x0 {
                        let boundary_y = a.y as f32;
                        let mid_x = (overlap_x0 + overlap_x1) as f32 / 2.0;
                        ctx.db.door().insert(Door {
                            id: 0,
                            room_a: a.room_id,
                            room_b: b.room_id,
                            wall_a: wall_sides::NORTH,
                            wall_b: wall_sides::SOUTH,
                            position_along_wall: 0.5,
                            width: 2.0,
                            access_level: access_levels::CREW_ONLY,
                            door_x: mid_x,
                            door_y: boundary_y,
                        });
                    }
                }
            }
        }

        // Force-connect orphan rooms: only if room actually borders a corridor cell
        for pr in &placed_rooms {
            let has_door = door_set.iter().any(|&(a, _, _)| a == pr.room_id);
            if has_door {
                continue;
            }

            // Check all 4 edges for adjacent corridor cells
            let mut connected = false;

            // West edge: check cell at (pr.x - 1, mid_y)
            if pr.x > 0 {
                let test_x = pr.x - 1;
                let mid_y = pr.y + pr.h / 2;
                if test_x < hull_width && mid_y < hull_length {
                    let cell = grid[test_x][mid_y];
                    if cell == CELL_MAIN_CORRIDOR || cell == CELL_SERVICE_CORRIDOR {
                        let target = if cell == CELL_MAIN_CORRIDOR {
                            find_spine_segment(mid_y)
                                .map(|s| s.room_id)
                                .or_else(|| find_cross_room(mid_y).map(|c| c.room_id))
                        } else {
                            Some(svc_rid)
                        };
                        if let Some(tid) = target {
                            let bx = pr.x as f32;
                            if bx > 0.5 && (bx as usize) < hull_width {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: tid,
                                    wall_a: wall_sides::WEST,
                                    wall_b: wall_sides::EAST,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: bx,
                                    door_y: pr.y as f32 + pr.h as f32 / 2.0,
                                });
                                connected = true;
                            }
                        }
                    }
                }
            }
            // East edge
            if !connected {
                let test_x = pr.x + pr.w;
                let mid_y = pr.y + pr.h / 2;
                if test_x < hull_width && mid_y < hull_length {
                    let cell = grid[test_x][mid_y];
                    if cell == CELL_MAIN_CORRIDOR || cell == CELL_SERVICE_CORRIDOR {
                        let target = if cell == CELL_MAIN_CORRIDOR {
                            find_spine_segment(mid_y)
                                .map(|s| s.room_id)
                                .or_else(|| find_cross_room(mid_y).map(|c| c.room_id))
                        } else {
                            Some(svc_rid)
                        };
                        if let Some(tid) = target {
                            let bx = (pr.x + pr.w) as f32;
                            if bx > 0.5 && (bx as usize) < hull_width {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: tid,
                                    wall_a: wall_sides::EAST,
                                    wall_b: wall_sides::WEST,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: bx,
                                    door_y: pr.y as f32 + pr.h as f32 / 2.0,
                                });
                                connected = true;
                            }
                        }
                    }
                }
            }
            // North edge
            if !connected && pr.y > 0 {
                let test_y = pr.y - 1;
                let mid_x = pr.x + pr.w / 2;
                if mid_x < hull_width && test_y < hull_length {
                    let cell = grid[mid_x][test_y];
                    if cell == CELL_MAIN_CORRIDOR {
                        let target = find_spine_segment(test_y)
                            .map(|s| s.room_id)
                            .or_else(|| find_cross_room(test_y).map(|c| c.room_id));
                        if let Some(tid) = target {
                            let by = pr.y as f32;
                            if by > 0.5 {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: tid,
                                    wall_a: wall_sides::NORTH,
                                    wall_b: wall_sides::SOUTH,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: pr.x as f32 + pr.w as f32 / 2.0,
                                    door_y: by,
                                });
                                connected = true;
                            }
                        }
                    }
                }
            }
            // South edge
            if !connected {
                let test_y = pr.y + pr.h;
                let mid_x = pr.x + pr.w / 2;
                if test_y < hull_length && mid_x < hull_width {
                    let cell = grid[mid_x][test_y];
                    if cell == CELL_MAIN_CORRIDOR {
                        let target = find_spine_segment(test_y)
                            .map(|s| s.room_id)
                            .or_else(|| find_cross_room(test_y).map(|c| c.room_id));
                        if let Some(tid) = target {
                            let by = (pr.y + pr.h) as f32;
                            if (by as usize) < hull_length {
                                ctx.db.door().insert(Door {
                                    id: 0,
                                    room_a: pr.room_id,
                                    room_b: tid,
                                    wall_a: wall_sides::SOUTH,
                                    wall_b: wall_sides::NORTH,
                                    position_along_wall: 0.5,
                                    width: 2.0,
                                    access_level: access_levels::PUBLIC,
                                    door_x: pr.x as f32 + pr.w as f32 / 2.0,
                                    door_y: by,
                                });
                                connected = true;
                            }
                        }
                    }
                }
            }
            // If still not connected, this room is truly isolated — skip it
            let _ = connected;
        }

        // ASCII dump for debugging
        {
            let mut dump = format!(
                "Deck {} grid ({}x{}, {} rooms, {} spine segs, {} cross-corridors):\n",
                deck + 1,
                hull_width,
                hull_length,
                placed_rooms.len(),
                spine_segments.len(),
                cross_rooms.len()
            );
            let max_rows = hull_length.min(60);
            for y in 0..max_rows {
                for x in 0..hull_width {
                    let ch = match grid[x][y] {
                        CELL_EMPTY => '.',
                        CELL_MAIN_CORRIDOR => '=',
                        CELL_SERVICE_CORRIDOR => '-',
                        CELL_SHAFT => '#',
                        v if v >= CELL_ROOM_BASE => {
                            let idx = (v - CELL_ROOM_BASE) % 26;
                            (b'A' + idx) as char
                        }
                        _ => '.',
                    };
                    dump.push(ch);
                }
                dump.push('\n');
            }
            if hull_length > max_rows {
                dump.push_str(&format!("... ({} more rows)\n", hull_length - max_rows));
            }
            log::info!("{}", dump);
        }
    }

    // ---- Step 7: Create VerticalShaft entries and cross-deck doors ----
    let decks_str = (0..deck_count)
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(",");

    // Use standard-deck positions for VerticalShaft entries (visual markers)
    let std_spine_cx = SHIP_BEAM / 2;
    struct ShaftDef {
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        shaft_type: u8,
        name: &'static str,
        is_main: bool,
    }
    let shaft_defs = [
        ShaftDef {
            x: std_spine_cx + 2,
            y: 10,
            w: 3,
            h: 3,
            shaft_type: shaft_types::ELEVATOR,
            name: "Fore Elevator",
            is_main: true,
        },
        ShaftDef {
            x: std_spine_cx + 2,
            y: SHIP_LENGTH - 14,
            w: 3,
            h: 3,
            shaft_type: shaft_types::ELEVATOR,
            name: "Aft Elevator",
            is_main: true,
        },
        ShaftDef {
            x: SHIP_BEAM - 5,
            y: 100,
            w: 2,
            h: 2,
            shaft_type: shaft_types::SERVICE_ELEVATOR,
            name: "Service Elevator",
            is_main: false,
        },
        ShaftDef {
            x: SHIP_BEAM - 4,
            y: 50,
            w: 2,
            h: 2,
            shaft_type: shaft_types::LADDER,
            name: "Ladder A",
            is_main: false,
        },
        ShaftDef {
            x: SHIP_BEAM - 4,
            y: 150,
            w: 2,
            h: 2,
            shaft_type: shaft_types::LADDER,
            name: "Ladder B",
            is_main: false,
        },
        ShaftDef {
            x: SHIP_BEAM - 4,
            y: 250,
            w: 2,
            h: 2,
            shaft_type: shaft_types::LADDER,
            name: "Ladder C",
            is_main: false,
        },
        ShaftDef {
            x: SHIP_BEAM - 4,
            y: 350,
            w: 2,
            h: 2,
            shaft_type: shaft_types::LADDER,
            name: "Ladder D",
            is_main: false,
        },
    ];

    for sd in &shaft_defs {
        ctx.db.vertical_shaft().insert(VerticalShaft {
            id: 0,
            shaft_type: sd.shaft_type,
            name: sd.name.to_string(),
            x: sd.x as f32 + sd.w as f32 / 2.0,
            y: sd.y as f32 + sd.h as f32 / 2.0,
            decks_served: decks_str.clone(),
            width: sd.w as f32,
            height: sd.h as f32,
        });

        // Cross-deck doors between consecutive deck shaft rooms
        // Find shaft rooms by name pattern across decks
        let mut shaft_rooms_across_decks: Vec<u32> = Vec::new();
        for d in 0..deck_count {
            let search_name = format!("{} D{}", sd.name, d + 1);
            // Look up room by name match
            for room in ctx.db.room().iter() {
                if room.name == search_name {
                    shaft_rooms_across_decks.push(room.id);
                    break;
                }
            }
        }

        for i in 0..shaft_rooms_across_decks.len().saturating_sub(1) {
            let access = if sd.is_main {
                access_levels::PUBLIC
            } else {
                access_levels::CREW_ONLY
            };
            // Use actual room positions (they vary per deck due to hull taper)
            let room_a_id = shaft_rooms_across_decks[i];
            let room_b_id = shaft_rooms_across_decks[i + 1];
            if let (Some(ra), Some(rb)) = (
                ctx.db.room().id().find(room_a_id),
                ctx.db.room().id().find(room_b_id),
            ) {
                // Cross-deck door: midpoint between the two rooms' centers
                let mid_x = (ra.x + rb.x) / 2.0;
                let mid_y = (ra.y + rb.y) / 2.0;
                ctx.db.door().insert(Door {
                    id: 0,
                    room_a: room_a_id,
                    room_b: room_b_id,
                    wall_a: wall_sides::SOUTH,
                    wall_b: wall_sides::NORTH,
                    position_along_wall: 0.5,
                    width: 2.0,
                    access_level: access,
                    door_x: mid_x,
                    door_y: mid_y,
                });
            }
        }
    }
}
