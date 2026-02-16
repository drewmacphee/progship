//! Pure movement logic — room bounds, door traversal, wall-sliding.
//!
//! Algorithm: "clamp then slide"
//! 1. Correct starting position into room bounds (handles NPC push edge case)
//! 2. Compute desired target = start + delta
//! 3. Clamp target into room bounds on EACH axis independently
//! 4. If an axis was clamped AND there's a reachable door on that wall, traverse
//! 5. Otherwise keep the clamped position (smooth wall slide)

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

/// Check if `py` (perpendicular coord) is within the door opening ± slack.
fn in_door_opening(py_coord: f32, door_center: f32, half_open: f32, slack: f32) -> bool {
    py_coord >= door_center - half_open - slack && py_coord <= door_center + half_open + slack
}

/// Compute the result of a movement within the given room, checking doors
/// for room transitions. Uses per-axis clamping with door checks on clamped walls.
pub fn compute_move(
    input: &MoveInput,
    current_room: &RoomBounds,
    doors: &[DoorInfo],
    door_rooms: &dyn Fn(u32) -> Option<RoomBounds>,
) -> MoveResult {
    let r = input.player_radius;

    // 0. Correct starting position into room if outside (NPC push edge case)
    let (px, py) = if current_room.contains(input.px, input.py, r) {
        (input.px, input.py)
    } else {
        current_room.clamp(input.px, input.py, r)
    };

    // 1. Desired target
    let tx = px + input.dx;
    let ty = py + input.dy;

    // 2. Clamp each axis independently
    let cx = tx.clamp(current_room.x_lo(r), current_room.x_hi(r));
    let cy = ty.clamp(current_room.y_lo(r), current_room.y_hi(r));

    let x_clamped = (cx - tx).abs() > 0.001;
    let y_clamped = (cy - ty).abs() > 0.001;

    // 3. If nothing was clamped, free movement inside room
    if !x_clamped && !y_clamped {
        return MoveResult::InRoom { x: cx, y: cy };
    }

    // 4. Pre-compute door info for this room
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

    // 5. For each clamped axis, check if there's a reachable door on that wall.
    // A door is reachable if:
    //   a) it's on the wall we hit (matching DoorWall variant)
    //   b) we're moving TOWARD that wall (dx/dy sign matches)
    //   c) the perpendicular coordinate is within the door opening
    // Only try the axis that was actually clamped (player hit that wall).

    let slack = 1.0; // approach tolerance for perpendicular coordinate

    // Try X-axis door (player hit left or right wall)
    if x_clamped {
        let hit_wall = if tx < current_room.x_lo(r) {
            Some(DoorWall::MinX)
        } else if tx > current_room.x_hi(r) {
            Some(DoorWall::MaxX)
        } else {
            None
        };

        if let Some(wall) = hit_wall {
            for &(door, dw, other_id) in &room_doors {
                if dw != wall {
                    continue;
                }
                let half_open = door.width / 2.0 - r;
                if half_open <= 0.0 {
                    continue;
                }
                // Perpendicular check: is player's Y near the door opening?
                // Use the CLAMPED cy (where the player actually ends up on Y axis)
                if !in_door_opening(cy, door.door_y, half_open, slack) {
                    continue;
                }
                if let Some(dest) = door_rooms(other_id) {
                    let enter_y = cy.clamp(door.door_y - half_open, door.door_y + half_open);
                    let (fx, fy) = dest.clamp(tx, enter_y, r);
                    if dest.contains(fx, fy, r) {
                        return MoveResult::DoorTraversal {
                            room_id: other_id,
                            x: fx,
                            y: fy,
                        };
                    }
                }
            }
        }
    }

    // Try Y-axis door (player hit top or bottom wall)
    if y_clamped {
        let hit_wall = if ty < current_room.y_lo(r) {
            Some(DoorWall::MinY)
        } else if ty > current_room.y_hi(r) {
            Some(DoorWall::MaxY)
        } else {
            None
        };

        if let Some(wall) = hit_wall {
            for &(door, dw, other_id) in &room_doors {
                if dw != wall {
                    continue;
                }
                let half_open = door.width / 2.0 - r;
                if half_open <= 0.0 {
                    continue;
                }
                // Perpendicular check: is player's X near the door opening?
                if !in_door_opening(cx, door.door_x, half_open, slack) {
                    continue;
                }
                if let Some(dest) = door_rooms(other_id) {
                    let enter_x = cx.clamp(door.door_x - half_open, door.door_x + half_open);
                    let (fx, fy) = dest.clamp(enter_x, ty, r);
                    if dest.contains(fx, fy, r) {
                        return MoveResult::DoorTraversal {
                            room_id: other_id,
                            x: fx,
                            y: fy,
                        };
                    }
                }
            }
        }
    }

    // 6. No door traversal — return clamped position (wall slide)
    MoveResult::WallSlide { x: cx, y: cy }
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
                assert!((x - 11.0).abs() < 0.1, "x={x}");
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
}
