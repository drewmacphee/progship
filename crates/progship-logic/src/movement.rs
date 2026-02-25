//! Pure movement logic — room bounds, door traversal, wall-sliding.
//!
//! Algorithm: "extended bounds with smooth room transition"
//! 1. Correct starting position if outside room bounds (NPC push edge case)
//! 2. Compute desired target = start + delta
//! 3. For each wall, extend bounds at door openings (remove radius inset)
//! 4. Clamp target to extended bounds — player smoothly enters overlap zone
//! 5. If player center crosses a wall, change room_id (no position jump)
//! 6. Walls without doors use normal radius-inset bounds (wall slide)

/// Decode packed cell mask bytes into axis-aligned rects.
/// Each rect = 4 × u16 (x0, y0, x1, y1) = 8 bytes.
pub fn decode_cell_rects(cells: &[u8]) -> Vec<(u16, u16, u16, u16)> {
    let mut rects = Vec::with_capacity(cells.len() / 8);
    let mut i = 0;
    while i + 7 < cells.len() {
        let x0 = u16::from_le_bytes([cells[i], cells[i + 1]]);
        let y0 = u16::from_le_bytes([cells[i + 2], cells[i + 3]]);
        let x1 = u16::from_le_bytes([cells[i + 4], cells[i + 5]]);
        let y1 = u16::from_le_bytes([cells[i + 6], cells[i + 7]]);
        rects.push((x0, y0, x1, y1));
        i += 8;
    }
    rects
}

/// Check if a point is inside any of the cell mask rects.
pub fn cell_mask_contains(cells: &[u8], x: f32, y: f32) -> bool {
    let mut i = 0;
    while i + 7 < cells.len() {
        let x0 = u16::from_le_bytes([cells[i], cells[i + 1]]) as f32;
        let y0 = u16::from_le_bytes([cells[i + 2], cells[i + 3]]) as f32;
        let x1 = u16::from_le_bytes([cells[i + 4], cells[i + 5]]) as f32;
        let y1 = u16::from_le_bytes([cells[i + 6], cells[i + 7]]) as f32;
        if x >= x0 && x < x1 && y >= y0 && y < y1 {
            return true;
        }
        i += 8;
    }
    false
}

/// Axis-aligned bounding box for a room (center + half-extents).
#[derive(Debug, Clone, Copy)]
pub struct RoomBounds {
    pub id: u32,
    pub cx: f32,
    pub cy: f32,
    pub half_w: f32,
    pub half_h: f32,
}

impl RoomBounds {
    pub fn new(id: u32, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            id,
            cx: x,
            cy: y,
            half_w: width / 2.0,
            half_h: height / 2.0,
        }
    }

    pub fn min_x(&self) -> f32 {
        self.cx - self.half_w
    }
    pub fn max_x(&self) -> f32 {
        self.cx + self.half_w
    }
    pub fn min_y(&self) -> f32 {
        self.cy - self.half_h
    }
    pub fn max_y(&self) -> f32 {
        self.cy + self.half_h
    }

    /// Check if a point (with radius padding) is inside this room.
    pub fn contains(&self, x: f32, y: f32, radius: f32) -> bool {
        x >= self.min_x() + radius
            && x <= self.max_x() - radius
            && y >= self.min_y() + radius
            && y <= self.max_y() - radius
    }

    /// Clamp a point to stay inside this room (with radius padding).
    pub fn clamp(&self, x: f32, y: f32, radius: f32) -> (f32, f32) {
        (
            x.clamp(self.min_x() + radius, self.max_x() - radius),
            y.clamp(self.min_y() + radius, self.max_y() - radius),
        )
    }

    fn x_lo(&self, r: f32) -> f32 {
        self.min_x() + r
    }
    fn x_hi(&self, r: f32) -> f32 {
        self.max_x() - r
    }
    fn y_lo(&self, r: f32) -> f32 {
        self.min_y() + r
    }
    fn y_hi(&self, r: f32) -> f32 {
        self.max_y() - r
    }
}

/// Minimal door info needed for traversal checks.
#[derive(Debug, Clone, Copy)]
pub struct DoorInfo {
    pub room_a: u32,
    pub room_b: u32,
    pub door_x: f32,
    pub door_y: f32,
    pub width: f32,
}

/// Result of attempting to move a player.
#[derive(Debug, Clone, PartialEq)]
pub enum MoveResult {
    /// Stayed in same room, position updated.
    InRoom { x: f32, y: f32 },
    /// Passed through a door into another room.
    DoorTraversal { room_id: u32, x: f32, y: f32 },
    /// Blocked — slid along wall in current room.
    WallSlide { x: f32, y: f32 },
}

/// A movement request: position and direction.
#[derive(Debug, Clone, Copy)]
pub struct MoveInput {
    pub px: f32,
    pub py: f32,
    pub dx: f32,
    pub dy: f32,
    pub player_radius: f32,
}

/// Which wall of a room a door sits on.
#[derive(Debug, Clone, Copy, PartialEq)]
enum DoorWall {
    MinX,
    MaxX,
    MinY,
    MaxY,
}

fn door_wall(door: &DoorInfo, room: &RoomBounds) -> DoorWall {
    let d = [
        (door.door_x - room.min_x()).abs(),
        (door.door_x - room.max_x()).abs(),
        (door.door_y - room.min_y()).abs(),
        (door.door_y - room.max_y()).abs(),
    ];
    let min_val = d[0].min(d[1]).min(d[2]).min(d[3]);
    if (d[0] - min_val).abs() < 0.01 {
        DoorWall::MinX
    } else if (d[1] - min_val).abs() < 0.01 {
        DoorWall::MaxX
    } else if (d[2] - min_val).abs() < 0.01 {
        DoorWall::MinY
    } else {
        DoorWall::MaxY
    }
}

/// Check if a door on the given wall is reachable at the player's perpendicular position.
/// Returns the door info if the player is within the door opening.
fn door_at_wall(
    wall: DoorWall,
    perp: f32,
    r: f32,
    room_doors: &[(DoorInfo, DoorWall, u32)],
) -> Option<(DoorInfo, u32)> {
    for &(door, dw, other_id) in room_doors {
        if dw != wall {
            continue;
        }
        let half_open = door.width / 2.0 - r;
        if half_open <= 0.0 {
            continue;
        }
        let center = match wall {
            DoorWall::MinX | DoorWall::MaxX => door.door_y,
            DoorWall::MinY | DoorWall::MaxY => door.door_x,
        };
        if perp >= center - half_open && perp <= center + half_open {
            return Some((door, other_id));
        }
    }
    None
}

/// Compute the result of a movement within the given room, checking doors
/// for room transitions. Extends room bounds at door openings so the player
/// smoothly walks through without position jumps.
pub fn compute_move(
    input: &MoveInput,
    current_room: &RoomBounds,
    doors: &[DoorInfo],
    door_rooms: &dyn Fn(u32) -> Option<RoomBounds>,
) -> MoveResult {
    let r = input.player_radius;

    // 0. Pre-compute door info for this room
    let room_doors: Vec<(DoorInfo, DoorWall, u32)> = doors
        .iter()
        .filter(|d| d.room_a == current_room.id || d.room_b == current_room.id)
        .map(|d| {
            let other = if d.room_a == current_room.id {
                d.room_b
            } else {
                d.room_a
            };
            (*d, door_wall(d, current_room), other)
        })
        .collect();

    // 1. Correct starting position: allow overlap zone at doors
    let px = if input.px >= current_room.x_lo(r) && input.px <= current_room.x_hi(r) {
        input.px
    } else if input.px < current_room.x_lo(r) {
        // Past min_x wall — allowed if in a door opening
        if input.px >= current_room.min_x()
            && door_at_wall(DoorWall::MinX, input.py, r, &room_doors).is_some()
        {
            input.px
        } else {
            current_room.x_lo(r)
        }
    } else if input.px <= current_room.max_x()
        && door_at_wall(DoorWall::MaxX, input.py, r, &room_doors).is_some()
    {
        input.px
    } else {
        current_room.x_hi(r)
    };
    let py = if input.py >= current_room.y_lo(r) && input.py <= current_room.y_hi(r) {
        input.py
    } else if input.py < current_room.y_lo(r) {
        if input.py >= current_room.min_y()
            && door_at_wall(DoorWall::MinY, input.px, r, &room_doors).is_some()
        {
            input.py
        } else {
            current_room.y_lo(r)
        }
    } else if input.py <= current_room.max_y()
        && door_at_wall(DoorWall::MaxY, input.px, r, &room_doors).is_some()
    {
        input.py
    } else {
        current_room.y_hi(r)
    };

    // 2. Desired target
    let tx = px + input.dx;
    let ty = py + input.dy;

    // 3. Compute extended bounds — extend at door openings
    let x_lo = if door_at_wall(DoorWall::MinX, py, r, &room_doors).is_some() {
        current_room.min_x() // extend to wall edge
    } else {
        current_room.x_lo(r) // normal: wall + radius
    };
    let x_hi = if door_at_wall(DoorWall::MaxX, py, r, &room_doors).is_some() {
        current_room.max_x()
    } else {
        current_room.x_hi(r)
    };
    let y_lo = if door_at_wall(DoorWall::MinY, px, r, &room_doors).is_some() {
        current_room.min_y()
    } else {
        current_room.y_lo(r)
    };
    let y_hi = if door_at_wall(DoorWall::MaxY, px, r, &room_doors).is_some() {
        current_room.max_y()
    } else {
        current_room.y_hi(r)
    };

    // 4. Clamp to extended bounds
    let cx = tx.clamp(x_lo, x_hi);
    let cy = ty.clamp(y_lo, y_hi);

    // 5. Check if player center crossed a wall → room transition
    // Player center crosses when it goes past the wall edge (min_x/max_x/min_y/max_y).
    // We check the traversal axis: if cx is past the wall AND there's a door there.

    // X-axis: crossed min_x wall (moving left)
    if cx <= current_room.min_x() && input.dx < 0.0 {
        if let Some((door, other_id)) = door_at_wall(DoorWall::MinX, py, r, &room_doors) {
            if let Some(dest) = door_rooms(other_id) {
                let half_open = door.width / 2.0 - r;
                let fy = cy.clamp(door.door_y - half_open, door.door_y + half_open);
                // Entering dest from its MaxX side — extend that bound to wall edge
                let fx = tx.clamp(dest.x_lo(r), dest.max_x());
                let fy = fy.clamp(dest.y_lo(r), dest.y_hi(r));
                return MoveResult::DoorTraversal {
                    room_id: other_id,
                    x: fx,
                    y: fy,
                };
            }
        }
    }
    // X-axis: crossed max_x wall (moving right)
    if cx >= current_room.max_x() && input.dx > 0.0 {
        if let Some((door, other_id)) = door_at_wall(DoorWall::MaxX, py, r, &room_doors) {
            if let Some(dest) = door_rooms(other_id) {
                let half_open = door.width / 2.0 - r;
                let fy = cy.clamp(door.door_y - half_open, door.door_y + half_open);
                // Entering dest from its MinX side — extend that bound to wall edge
                let fx = tx.clamp(dest.min_x(), dest.x_hi(r));
                let fy = fy.clamp(dest.y_lo(r), dest.y_hi(r));
                return MoveResult::DoorTraversal {
                    room_id: other_id,
                    x: fx,
                    y: fy,
                };
            }
        }
    }
    // Y-axis: crossed min_y wall (moving up/north)
    if cy <= current_room.min_y() && input.dy < 0.0 {
        if let Some((door, other_id)) = door_at_wall(DoorWall::MinY, px, r, &room_doors) {
            if let Some(dest) = door_rooms(other_id) {
                let half_open = door.width / 2.0 - r;
                let fx = cx.clamp(door.door_x - half_open, door.door_x + half_open);
                // Entering dest from its MaxY side — extend that bound to wall edge
                let fx = fx.clamp(dest.x_lo(r), dest.x_hi(r));
                let fy = ty.clamp(dest.y_lo(r), dest.max_y());
                return MoveResult::DoorTraversal {
                    room_id: other_id,
                    x: fx,
                    y: fy,
                };
            }
        }
    }
    // Y-axis: crossed max_y wall (moving down/south)
    if cy >= current_room.max_y() && input.dy > 0.0 {
        if let Some((door, other_id)) = door_at_wall(DoorWall::MaxY, px, r, &room_doors) {
            if let Some(dest) = door_rooms(other_id) {
                let half_open = door.width / 2.0 - r;
                let fx = cx.clamp(door.door_x - half_open, door.door_x + half_open);
                // Entering dest from its MinY side — extend that bound to wall edge
                let fx = fx.clamp(dest.x_lo(r), dest.x_hi(r));
                let fy = ty.clamp(dest.min_y(), dest.y_hi(r));
                return MoveResult::DoorTraversal {
                    room_id: other_id,
                    x: fx,
                    y: fy,
                };
            }
        }
    }

    // 6. No wall crossing — position within (possibly extended) room bounds
    let x_clamped = (cx - tx).abs() > 0.001;
    let y_clamped = (cy - ty).abs() > 0.001;

    if x_clamped || y_clamped {
        MoveResult::WallSlide { x: cx, y: cy }
    } else {
        MoveResult::InRoom { x: cx, y: cy }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(id: u32, x: f32, y: f32, w: f32, h: f32) -> RoomBounds {
        RoomBounds::new(id, x, y, w, h)
    }

    fn mi(px: f32, py: f32, dx: f32, dy: f32) -> MoveInput {
        MoveInput {
            px,
            py,
            dx,
            dy,
            player_radius: 0.4,
        }
    }

    // --- Basic movement ---

    #[test]
    fn free_move_inside_room() {
        let r = room(1, 10.0, 10.0, 20.0, 20.0);
        let res = compute_move(&mi(10.0, 10.0, 1.0, 0.0), &r, &[], &|_| None);
        assert_eq!(res, MoveResult::InRoom { x: 11.0, y: 10.0 });
    }

    #[test]
    fn wall_clamp_no_doors() {
        let r = room(1, 10.0, 10.0, 10.0, 10.0);
        // max_x = 15, radius = 0.4, so wall at 14.6
        let res = compute_move(&mi(10.0, 10.0, 100.0, 0.0), &r, &[], &|_| None);
        match res {
            MoveResult::WallSlide { x, y } => {
                assert!((x - 14.6).abs() < 0.01, "x={x}");
                assert!((y - 10.0).abs() < 0.01, "y={y}");
            }
            _ => panic!("Expected WallSlide, got {:?}", res),
        }
    }

    // --- Wall sliding ---

    #[test]
    fn slide_along_y_when_x_blocked() {
        let r = room(1, 10.0, 10.0, 10.0, 10.0);
        let res = compute_move(&mi(14.0, 10.0, 2.0, 2.0), &r, &[], &|_| None);
        match res {
            MoveResult::WallSlide { x, y } => {
                assert!(x <= 14.6 + 0.01, "X clamped, got {x}");
                assert!((y - 12.0).abs() < 0.01, "Y advances, got {y}");
            }
            _ => panic!("Expected WallSlide, got {:?}", res),
        }
    }

    #[test]
    fn slide_along_x_when_y_blocked() {
        let r = room(1, 10.0, 10.0, 10.0, 10.0);
        let res = compute_move(&mi(10.0, 14.0, 2.0, 2.0), &r, &[], &|_| None);
        match res {
            MoveResult::WallSlide { x, y } => {
                assert!((x - 12.0).abs() < 0.01, "X advances, got {x}");
                assert!(y <= 14.6 + 0.01, "Y clamped, got {y}");
            }
            _ => panic!("Expected WallSlide, got {:?}", res),
        }
    }

    #[test]
    fn at_wall_boundary_slides_not_snaps() {
        let r = room(1, 10.0, 10.0, 10.0, 10.0);
        // Player at right wall edge, moving up only
        let res = compute_move(&mi(14.6, 10.0, 0.0, 2.0), &r, &[], &|_| None);
        match res {
            MoveResult::InRoom { x, y } | MoveResult::WallSlide { x, y } => {
                assert!((x - 14.6).abs() < 0.1, "X stays, got {x}");
                assert!((y - 12.0).abs() < 0.1, "Y advances, got {y}");
            }
            _ => panic!("got {:?}", res),
        }
    }

    #[test]
    fn at_wall_diagonal_slides_not_snaps() {
        let r = room(1, 10.0, 10.0, 10.0, 10.0);
        // Player at right wall, moving right+up — X clamped, Y free
        let res = compute_move(&mi(14.6, 10.0, 1.0, 2.0), &r, &[], &|_| None);
        match res {
            MoveResult::WallSlide { x, y } => {
                assert!((x - 14.6).abs() < 0.1, "X stays, got {x}");
                assert!((y - 12.0).abs() < 0.1, "Y advances, got {y}");
            }
            _ => panic!("got {:?}", res),
        }
    }

    #[test]
    fn outside_bounds_corrected() {
        let r = room(1, 10.0, 10.0, 10.0, 10.0);
        // Player past wall, moving up — corrected first
        let res = compute_move(&mi(14.7, 10.0, 0.0, 1.0), &r, &[], &|_| None);
        match res {
            MoveResult::InRoom { x, y } | MoveResult::WallSlide { x, y } => {
                assert!(x <= 14.6 + 0.01, "X corrected, got {x}");
                assert!((y - 11.0).abs() < 0.1, "Y advances, got {y}");
            }
            _ => panic!("got {:?}", res),
        }
    }

    // --- Door traversal ---

    #[test]
    fn straight_through_door() {
        let a = room(1, 5.0, 5.0, 10.0, 10.0);
        let b = room(2, 15.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 5.0,
            width: 2.0,
        };
        let lookup = |id: u32| if id == 2 { Some(b) } else { None };
        let res = compute_move(&mi(9.0, 5.0, 2.0, 0.0), &a, &[door], &lookup);
        match res {
            MoveResult::DoorTraversal { room_id, x, y } => {
                assert_eq!(room_id, 2);
                // Player wanted to go to x=11, lands at exactly 11 (no artificial step)
                assert!((x - 11.0).abs() < 0.01, "x at target, x={x}");
                assert!((y - 5.0).abs() < 0.1, "y={y}");
            }
            _ => panic!("Expected DoorTraversal, got {:?}", res),
        }
    }

    #[test]
    fn moving_away_from_door_stays_in_room() {
        let a = room(1, 5.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 5.0,
            width: 2.0,
        };
        let res = compute_move(&mi(9.0, 5.0, -2.0, 0.0), &a, &[door], &|_| None);
        assert_eq!(res, MoveResult::InRoom { x: 7.0, y: 5.0 });
    }

    #[test]
    fn wide_door_traversal() {
        let a = room(1, 5.0, 5.0, 10.0, 10.0);
        let b = room(2, 15.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 5.0,
            width: 3.0,
        };
        let lookup = |id: u32| if id == 2 { Some(b) } else { None };
        let res = compute_move(&mi(9.5, 5.0, 1.0, 0.0), &a, &[door], &lookup);
        match res {
            MoveResult::DoorTraversal { room_id, .. } => assert_eq!(room_id, 2),
            _ => panic!("Expected DoorTraversal, got {:?}", res),
        }
    }

    #[test]
    fn offset_door_traverse_at_opening() {
        let a = room(1, 5.0, 5.0, 10.0, 10.0);
        let b = room(2, 15.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 8.0,
            width: 2.0,
        };
        let lookup = |id: u32| if id == 2 { Some(b) } else { None };

        // At door height — traverse
        let res = compute_move(&mi(9.5, 8.0, 1.5, 0.0), &a, &[door], &lookup);
        assert!(matches!(res, MoveResult::DoorTraversal { room_id: 2, .. }));

        // Far from door height — wall slide
        let res = compute_move(&mi(9.5, 3.0, 1.5, 0.0), &a, &[door], &lookup);
        assert!(matches!(res, MoveResult::WallSlide { .. }));
    }

    #[test]
    fn no_teleport_through_wall_to_door() {
        let a = room(1, 5.0, 5.0, 10.0, 10.0);
        let b = room(2, 15.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 5.0,
            width: 2.0,
        };
        let lookup = |id: u32| if id == 2 { Some(b) } else { None };

        // Player at y=2, far from door at y=5 — should NOT traverse
        let res = compute_move(&mi(9.0, 2.0, 5.0, 0.0), &a, &[door], &lookup);
        match res {
            MoveResult::WallSlide { x, .. } => {
                assert!(x <= 9.6 + 0.01, "clamped, got x={x}");
            }
            MoveResult::DoorTraversal { .. } => {
                panic!("Should NOT traverse — player far from door");
            }
            _ => {}
        }
    }

    #[test]
    fn corridor_to_side_room() {
        let corridor = room(1, 10.0, 50.0, 3.0, 100.0);
        let side = room(2, 15.0, 50.0, 8.0, 6.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 11.5,
            door_y: 50.0,
            width: 2.0,
        };
        let lookup = |id: u32| if id == 2 { Some(side) } else { None };

        let res = compute_move(&mi(10.5, 50.0, 2.0, 0.0), &corridor, &[door], &lookup);
        match res {
            MoveResult::DoorTraversal { room_id, x, y } => {
                assert_eq!(room_id, 2);
                assert!(x > 11.0, "in room, got {x}");
                assert!((y - 50.0).abs() < 0.5, "y near 50, got {y}");
            }
            _ => panic!("Expected DoorTraversal, got {:?}", res),
        }
    }

    #[test]
    fn intersection_picks_correct_door() {
        let spine0 = room(1, 20.0, 25.0, 3.0, 50.0);
        let cross = room(3, 20.0, 51.5, 38.0, 3.0);
        let spine1 = room(2, 20.0, 76.5, 3.0, 47.0);
        let door_s = DoorInfo {
            room_a: 3,
            room_b: 1,
            door_x: 20.0,
            door_y: 50.0,
            width: 3.0,
        };
        let door_n = DoorInfo {
            room_a: 3,
            room_b: 2,
            door_x: 20.0,
            door_y: 53.0,
            width: 3.0,
        };
        let lookup = |id: u32| match id {
            1 => Some(spine0),
            2 => Some(spine1),
            _ => None,
        };

        let res = compute_move(
            &mi(20.0, 52.8, 0.0, 0.5),
            &cross,
            &[door_s, door_n],
            &lookup,
        );
        match res {
            MoveResult::DoorTraversal { room_id, .. } => {
                assert_eq!(room_id, 2, "Should enter spine1");
            }
            MoveResult::InRoom { y, .. } | MoveResult::WallSlide { y, .. } => {
                assert!(y > 52.8, "Y should advance, got {y}");
            }
        }
    }

    // --- Edge cases ---

    #[test]
    fn room_contains_and_clamp() {
        let r = room(1, 10.0, 10.0, 20.0, 20.0);
        assert!(r.contains(10.0, 10.0, 0.4));
        assert!(r.contains(19.0, 19.0, 0.4));
        assert!(!r.contains(20.5, 10.0, 0.4));
        let (x, y) = r.clamp(100.0, -100.0, 0.4);
        assert!((x - 19.6).abs() < 0.01);
        assert!((y - 0.4).abs() < 0.01);
    }

    #[test]
    fn diagonal_through_door_y_wall() {
        // Door on the top wall, player moving diagonally up-right
        let a = room(1, 5.0, 5.0, 10.0, 10.0);
        let b = room(2, 5.0, 15.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 5.0,
            door_y: 10.0,
            width: 3.0,
        };
        let lookup = |id: u32| if id == 2 { Some(b) } else { None };

        // At x=5 (door center), moving up past the wall
        let res = compute_move(&mi(5.0, 9.0, 0.5, 2.0), &a, &[door], &lookup);
        match res {
            MoveResult::DoorTraversal { room_id, .. } => assert_eq!(room_id, 2),
            _ => panic!("Expected DoorTraversal, got {:?}", res),
        }
    }

    // --- Smooth traversal (no position jump) ---

    #[test]
    fn smooth_approach_door_no_jump() {
        // Small dx should place player smoothly in overlap zone, not jump
        let a = room(1, 5.0, 5.0, 10.0, 10.0);
        let b = room(2, 15.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 5.0,
            width: 2.0,
        };
        let lookup = |id: u32| if id == 2 { Some(b) } else { None };

        // Player very close to wall with tiny dx — should reach wall edge, not jump past
        let res = compute_move(&mi(9.5, 5.0, 0.2, 0.0), &a, &[door], &lookup);
        match res {
            MoveResult::InRoom { x, .. } | MoveResult::WallSlide { x, .. } => {
                // Still in room A's extended bounds (wall at 10.0, player at 9.7)
                assert!((x - 9.7).abs() < 0.01, "smooth position, x={x}");
            }
            MoveResult::DoorTraversal { .. } => {
                panic!("Shouldn't traverse yet — haven't crossed the wall");
            }
        }
    }

    #[test]
    fn smooth_traversal_exact_target() {
        // Player walks past the wall — should land at their actual target
        let a = room(1, 5.0, 5.0, 10.0, 10.0);
        let b = room(2, 15.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 5.0,
            width: 2.0,
        };
        let lookup = |id: u32| if id == 2 { Some(b) } else { None };

        // Player at wall edge, dx=0.5 — target is 10.1, past wall at 10.0
        let res = compute_move(&mi(9.6, 5.0, 0.5, 0.0), &a, &[door], &lookup);
        match res {
            MoveResult::DoorTraversal { room_id, x, y } => {
                assert_eq!(room_id, 2);
                // Should be at the actual target (10.1), not an artificial step
                assert!((x - 10.1).abs() < 0.01, "at target, x={x}");
                assert!((y - 5.0).abs() < 0.01, "y={y}");
            }
            _ => panic!("Expected DoorTraversal, got {:?}", res),
        }
    }

    #[test]
    fn overlap_zone_in_room() {
        // Player in the overlap zone (between wall-r and wall) stays in current room
        let a = room(1, 5.0, 5.0, 10.0, 10.0);
        let b = room(2, 15.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 5.0,
            width: 2.0,
        };
        let lookup = |id: u32| if id == 2 { Some(b) } else { None };

        // Player at x=9.8 (past normal clamp of 9.6, but before wall at 10.0)
        // Moving right with tiny dx — should stay in room A
        let res = compute_move(&mi(9.8, 5.0, 0.1, 0.0), &a, &[door], &lookup);
        match res {
            MoveResult::InRoom { x, .. } | MoveResult::WallSlide { x, .. } => {
                assert!((x - 9.9).abs() < 0.01, "stays in overlap, x={x}");
            }
            MoveResult::DoorTraversal { .. } => {
                panic!("Shouldn't traverse — haven't crossed wall at 10.0");
            }
        }
    }

    #[test]
    fn corridor_to_corridor_smooth() {
        // Spine (4m wide) → Cross-corridor (3m tall) — typical corridor transition
        let spine = room(1, 37.0, 30.0, 4.0, 20.0);
        let cross = room(2, 37.0, 42.0, 38.0, 4.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 37.0,
            door_y: 40.0,
            width: 4.0,
        };
        let lookup = |id: u32| if id == 2 { Some(cross) } else { None };

        // Walking south in spine, approaching cross-corridor
        // Player at y=39.5 (near south wall at 40.0), dy=0.3
        let res = compute_move(&mi(37.0, 39.5, 0.0, 0.3), &spine, &[door], &lookup);
        match res {
            MoveResult::InRoom { y, .. } | MoveResult::WallSlide { y, .. } => {
                // Should be at 39.8, still in overlap zone (wall at 40.0)
                assert!((y - 39.8).abs() < 0.01, "smooth approach, y={y}");
            }
            MoveResult::DoorTraversal { .. } => {
                panic!("Shouldn't traverse yet — y=39.8 < wall=40.0");
            }
        }

        // Now cross the wall
        let res = compute_move(&mi(37.0, 39.9, 0.0, 0.3), &spine, &[door], &lookup);
        match res {
            MoveResult::DoorTraversal { room_id, y, .. } => {
                assert_eq!(room_id, 2);
                // At actual target y=40.2, just past the wall
                assert!((y - 40.2).abs() < 0.01, "at target, y={y}");
            }
            _ => panic!("Expected DoorTraversal, got {:?}", res),
        }
    }
}
