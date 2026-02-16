//! Pure movement logic — room bounds, door traversal, wall-sliding.

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

/// Pre-computed door context for a room.
struct DoorCtx {
    door: DoorInfo,
    wall: DoorWall,
    other_id: u32,
}

/// Try to enter `dest` through `door`. Returns clamped position inside dest
/// if the target coordinates reach through the door opening.
///
/// The player's perpendicular coordinate must already be near the door
/// opening — we only allow minor clamping (within 1 unit of the opening edge),
/// NOT teleporting across the wall to the door.
fn try_enter(
    nx: f32,
    ny: f32,
    r: f32,
    door: &DoorInfo,
    wall: DoorWall,
    dest: &RoomBounds,
) -> Option<(f32, f32)> {
    let half = door.width / 2.0 - r;
    if half <= 0.0 {
        return None;
    }
    // Check perpendicular coordinate is within or near the door opening.
    // Allow 1.0 unit of slack for smooth approach, but not arbitrary teleportation.
    let slack = 1.0;
    let (cx, cy) = match wall {
        DoorWall::MinX | DoorWall::MaxX => {
            if ny < door.door_y - half - slack || ny > door.door_y + half + slack {
                return None;
            }
            (nx, ny.clamp(door.door_y - half, door.door_y + half))
        }
        DoorWall::MinY | DoorWall::MaxY => {
            if nx < door.door_x - half - slack || nx > door.door_x + half + slack {
                return None;
            }
            (nx.clamp(door.door_x - half, door.door_x + half), ny)
        }
    };
    // Clamp to dest room, then check if result is valid
    let (fx, fy) = dest.clamp(cx, cy, r);
    if dest.contains(fx, fy, r) {
        Some((fx, fy))
    } else {
        None
    }
}

/// Try all doors for a target position. Returns DoorTraversal on success.
fn try_doors(
    nx: f32,
    ny: f32,
    r: f32,
    doors: &[DoorCtx],
    door_rooms: &dyn Fn(u32) -> Option<RoomBounds>,
) -> Option<MoveResult> {
    // Try closest door first
    let mut best: Option<(usize, f32)> = None;
    for (i, dc) in doors.iter().enumerate() {
        let dist = ((nx - dc.door.door_x).powi(2) + (ny - dc.door.door_y).powi(2)).sqrt();
        if best.is_none() || dist < best.unwrap().1 {
            best = Some((i, dist));
        }
    }
    // Try best first, then all others
    if let Some((best_i, _)) = best {
        let dc = &doors[best_i];
        if let Some(dest) = door_rooms(dc.other_id) {
            if let Some((fx, fy)) = try_enter(nx, ny, r, &dc.door, dc.wall, &dest) {
                return Some(MoveResult::DoorTraversal {
                    room_id: dc.other_id,
                    x: fx,
                    y: fy,
                });
            }
        }
        // Try remaining doors
        for (i, dc) in doors.iter().enumerate() {
            if i == best_i {
                continue;
            }
            if let Some(dest) = door_rooms(dc.other_id) {
                if let Some((fx, fy)) = try_enter(nx, ny, r, &dc.door, dc.wall, &dest) {
                    return Some(MoveResult::DoorTraversal {
                        room_id: dc.other_id,
                        x: fx,
                        y: fy,
                    });
                }
            }
        }
    }
    None
}

/// Compute the result of a movement within the given room, checking doors
/// for room transitions. Uses axis-decomposed wall-sliding for smooth
/// diagonal movement along walls and through doorways.
pub fn compute_move(
    input: &MoveInput,
    current_room: &RoomBounds,
    doors: &[DoorInfo],
    door_rooms: &dyn Fn(u32) -> Option<RoomBounds>,
) -> MoveResult {
    let r = input.player_radius;
    let nx = input.px + input.dx;
    let ny = input.py + input.dy;

    // 1. Fast path: full move stays inside current room
    if current_room.contains(nx, ny, r) {
        return MoveResult::InRoom { x: nx, y: ny };
    }

    // Pre-compute door context
    let dctx: Vec<DoorCtx> = doors
        .iter()
        .filter(|d| d.room_a == current_room.id || d.room_b == current_room.id)
        .map(|d| {
            let other_id = if d.room_a == current_room.id {
                d.room_b
            } else {
                d.room_a
            };
            DoorCtx {
                door: *d,
                wall: door_wall(d, current_room),
                other_id,
            }
        })
        .collect();

    // 2. Try door traversal with full diagonal move
    if let Some(res) = try_doors(nx, ny, r, &dctx, door_rooms) {
        return res;
    }

    // 3. Axis-decomposed wall sliding
    let x_ok = current_room.contains(nx, input.py, r);
    let y_ok = current_room.contains(input.px, ny, r);

    match (x_ok, y_ok) {
        (true, true) => {
            // Both single-axis ok but diagonal fails — corner hit
            let (cx, cy) = current_room.clamp(nx, ny, r);
            MoveResult::WallSlide { x: cx, y: cy }
        }
        (true, false) => {
            // Y blocked — try Y-only through door, else slide X only
            if input.dy != 0.0 {
                if let Some(res) = try_doors(input.px, ny, r, &dctx, door_rooms) {
                    return res;
                }
            }
            // Slide: use the valid X, clamp Y
            let cy = ny.clamp(current_room.min_y() + r, current_room.max_y() - r);
            MoveResult::WallSlide { x: nx, y: cy }
        }
        (false, true) => {
            // X blocked — try X-only through door, else slide Y only
            if input.dx != 0.0 {
                if let Some(res) = try_doors(nx, input.py, r, &dctx, door_rooms) {
                    return res;
                }
            }
            // Slide: clamp X, use the valid Y
            let cx = nx.clamp(current_room.min_x() + r, current_room.max_x() - r);
            MoveResult::WallSlide { x: cx, y: ny }
        }
        (false, false) => {
            // Both blocked — try each axis through doors
            let (primary_x, primary_y, secondary_x, secondary_y) =
                if input.dx.abs() >= input.dy.abs() {
                    (nx, input.py, input.px, ny)
                } else {
                    (input.px, ny, nx, input.py)
                };
            if let Some(res) = try_doors(primary_x, primary_y, r, &dctx, door_rooms) {
                return res;
            }
            if let Some(res) = try_doors(secondary_x, secondary_y, r, &dctx, door_rooms) {
                return res;
            }
            // Clamp to room bounds
            let (cx, cy) = current_room.clamp(nx, ny, r);
            if cx != input.px || cy != input.py {
                MoveResult::WallSlide { x: cx, y: cy }
            } else {
                MoveResult::WallSlide {
                    x: input.px,
                    y: input.py,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_room(id: u32, x: f32, y: f32, w: f32, h: f32) -> RoomBounds {
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

    #[test]
    fn test_move_within_room() {
        let room = simple_room(1, 10.0, 10.0, 20.0, 20.0);
        let result = compute_move(&mi(10.0, 10.0, 1.0, 0.0), &room, &[], &|_| None);
        assert_eq!(result, MoveResult::InRoom { x: 11.0, y: 10.0 });
    }

    #[test]
    fn test_wall_slide_no_doors() {
        let room = simple_room(1, 10.0, 10.0, 10.0, 10.0);
        let result = compute_move(&mi(10.0, 10.0, 100.0, 0.0), &room, &[], &|_| None);
        match result {
            MoveResult::WallSlide { x, y } => {
                assert!((x - 14.6).abs() < 0.01, "x={x}");
                assert!((y - 10.0).abs() < 0.01);
            }
            _ => panic!("Expected WallSlide, got {:?}", result),
        }
    }

    #[test]
    fn test_diagonal_wall_slide_x_blocked() {
        // Moving diagonally into right wall should slide along Y
        let room = simple_room(1, 10.0, 10.0, 10.0, 10.0);
        let result = compute_move(&mi(14.0, 10.0, 2.0, 2.0), &room, &[], &|_| None);
        match result {
            MoveResult::WallSlide { x, y } => {
                assert!(x <= 14.6 + 0.01, "X should be clamped, got {x}");
                assert!((y - 12.0).abs() < 0.01, "Y should advance to 12.0, got {y}");
            }
            _ => panic!("Expected WallSlide, got {:?}", result),
        }
    }

    #[test]
    fn test_diagonal_wall_slide_y_blocked() {
        // Moving diagonally into top wall should slide along X
        let room = simple_room(1, 10.0, 10.0, 10.0, 10.0);
        let result = compute_move(&mi(10.0, 14.0, 2.0, 2.0), &room, &[], &|_| None);
        match result {
            MoveResult::WallSlide { x, y } => {
                assert!((x - 12.0).abs() < 0.01, "X should advance to 12.0, got {x}");
                assert!(y <= 14.6 + 0.01, "Y should be clamped, got {y}");
            }
            _ => panic!("Expected WallSlide, got {:?}", result),
        }
    }

    #[test]
    fn test_door_traversal_straight() {
        let room_a = simple_room(1, 5.0, 5.0, 10.0, 10.0);
        let room_b = simple_room(2, 15.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 5.0,
            width: 2.0,
        };
        let lookup = |id: u32| -> Option<RoomBounds> {
            if id == 2 {
                Some(room_b)
            } else {
                None
            }
        };
        let result = compute_move(&mi(9.0, 5.0, 2.0, 0.0), &room_a, &[door], &lookup);
        match result {
            MoveResult::DoorTraversal { room_id, x, y } => {
                assert_eq!(room_id, 2);
                assert!((x - 11.0).abs() < 0.1, "x={x}");
                assert!((y - 5.0).abs() < 0.1, "y={y}");
            }
            _ => panic!("Expected DoorTraversal, got {:?}", result),
        }
    }

    #[test]
    fn test_door_wrong_direction() {
        let room_a = simple_room(1, 5.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 5.0,
            width: 2.0,
        };
        let result = compute_move(&mi(9.0, 5.0, -2.0, 0.0), &room_a, &[door], &|_| None);
        assert_eq!(result, MoveResult::InRoom { x: 7.0, y: 5.0 });
    }

    #[test]
    fn test_room_contains() {
        let room = simple_room(1, 10.0, 10.0, 20.0, 20.0);
        assert!(room.contains(10.0, 10.0, 0.4));
        assert!(room.contains(19.0, 19.0, 0.4));
        assert!(!room.contains(20.5, 10.0, 0.4));
    }

    #[test]
    fn test_room_clamp() {
        let room = simple_room(1, 10.0, 10.0, 20.0, 20.0);
        let (x, y) = room.clamp(100.0, -100.0, 0.4);
        assert!((x - 19.6).abs() < 0.01);
        assert!((y - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_intersection_picks_correct_door() {
        let spine0 = simple_room(1, 20.0, 25.0, 3.0, 50.0);
        let cross = simple_room(3, 20.0, 51.5, 38.0, 3.0);
        let spine1 = simple_room(2, 20.0, 76.5, 3.0, 47.0);
        let door_south = DoorInfo {
            room_a: 3,
            room_b: 1,
            door_x: 20.0,
            door_y: 50.0,
            width: 3.0,
        };
        let door_north = DoorInfo {
            room_a: 3,
            room_b: 2,
            door_x: 20.0,
            door_y: 53.0,
            width: 3.0,
        };
        let lookup = |id: u32| -> Option<RoomBounds> {
            match id {
                1 => Some(spine0),
                2 => Some(spine1),
                _ => None,
            }
        };

        let result = compute_move(
            &mi(20.0, 52.8, 0.0, 0.5),
            &cross,
            &[door_south, door_north],
            &lookup,
        );
        match result {
            MoveResult::DoorTraversal { room_id, .. } => {
                assert_eq!(room_id, 2, "Should enter spine1, not spine0");
            }
            MoveResult::InRoom { y, .. } | MoveResult::WallSlide { y, .. } => {
                assert!(y > 52.8, "Y should advance, got {y}");
            }
        }
    }

    #[test]
    fn test_corridor_to_room_via_side_door() {
        // Narrow corridor with a room off to the right
        let corridor = simple_room(1, 10.0, 50.0, 3.0, 100.0);
        let room = simple_room(2, 15.0, 50.0, 8.0, 6.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 11.5,
            door_y: 50.0,
            width: 2.0,
        };
        let lookup = |id: u32| -> Option<RoomBounds> {
            if id == 2 {
                Some(room)
            } else {
                None
            }
        };

        let result = compute_move(&mi(10.5, 50.0, 2.0, 0.0), &corridor, &[door], &lookup);
        match result {
            MoveResult::DoorTraversal { room_id, x, y } => {
                assert_eq!(room_id, 2);
                assert!(x > 11.0, "x should be in room, got {x}");
                assert!((y - 50.0).abs() < 0.5, "y near 50, got {y}");
            }
            other => panic!("Expected DoorTraversal, got {:?}", other),
        }
    }

    #[test]
    fn test_wide_door_straight_through() {
        let room_a = simple_room(1, 5.0, 5.0, 10.0, 10.0);
        let room_b = simple_room(2, 15.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 5.0,
            width: 3.0,
        };
        let lookup = |id: u32| -> Option<RoomBounds> {
            if id == 2 {
                Some(room_b)
            } else {
                None
            }
        };

        let result = compute_move(&mi(9.5, 5.0, 1.0, 0.0), &room_a, &[door], &lookup);
        match result {
            MoveResult::DoorTraversal { room_id, x, y } => {
                assert_eq!(room_id, 2);
                assert!((x - 10.5).abs() < 0.1, "x={x}");
                assert!((y - 5.0).abs() < 0.1, "y={y}");
            }
            other => panic!("Expected DoorTraversal, got {:?}", other),
        }
    }

    #[test]
    fn test_door_offset_from_center() {
        // Door not centered on the wall
        let room_a = simple_room(1, 5.0, 5.0, 10.0, 10.0);
        let room_b = simple_room(2, 15.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 8.0,
            width: 2.0,
        };
        let lookup = |id: u32| -> Option<RoomBounds> {
            if id == 2 {
                Some(room_b)
            } else {
                None
            }
        };

        // Walking right at door height — should traverse
        let result = compute_move(&mi(9.5, 8.0, 1.5, 0.0), &room_a, &[door], &lookup);
        match result {
            MoveResult::DoorTraversal { room_id, .. } => assert_eq!(room_id, 2),
            other => panic!("Expected DoorTraversal, got {:?}", other),
        }

        // Walking right but NOT at door height — should wall-slide
        let result = compute_move(&mi(9.5, 3.0, 1.5, 0.0), &room_a, &[door], &lookup);
        match result {
            MoveResult::WallSlide { .. } => {} // expected
            other => panic!("Expected WallSlide, got {:?}", other),
        }
    }

    #[test]
    fn test_no_teleport_through_wall() {
        // Player far from door, big dx — should NOT teleport through wall
        let room_a = simple_room(1, 5.0, 5.0, 10.0, 10.0);
        let room_b = simple_room(2, 15.0, 5.0, 10.0, 10.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 5.0,
            width: 2.0,
        };
        let lookup = |id: u32| -> Option<RoomBounds> {
            if id == 2 {
                Some(room_b)
            } else {
                None
            }
        };

        // Walking right at y=2 (away from door at y=5) — should wall-slide
        let result = compute_move(&mi(9.0, 2.0, 5.0, 0.0), &room_a, &[door], &lookup);
        match result {
            MoveResult::WallSlide { x, .. } => {
                assert!(x <= 9.6 + 0.01, "Should clamp to wall, got x={x}");
            }
            MoveResult::DoorTraversal { .. } => {
                panic!("Should NOT traverse — player is far from door opening");
            }
            _ => {}
        }
    }
}
