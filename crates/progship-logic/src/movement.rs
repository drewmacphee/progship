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

/// Check if a door is on a vertical wall (left/right) of the room.
fn door_on_vertical_wall(door: &DoorInfo, room: &RoomBounds) -> bool {
    let dist_left = (door.door_x - room.min_x()).abs();
    let dist_right = (door.door_x - room.max_x()).abs();
    let dist_top = (door.door_y - room.min_y()).abs();
    let dist_bottom = (door.door_y - room.max_y()).abs();
    dist_left.min(dist_right) < dist_top.min(dist_bottom)
}

/// Try to traverse through a door into the destination room.
/// Returns Some((room_id, x, y)) if successful.
fn try_door_traversal(
    new_x: f32,
    new_y: f32,
    radius: f32,
    door: &DoorInfo,
    current: &RoomBounds,
    door_rooms: &dyn Fn(u32) -> Option<RoomBounds>,
) -> Option<(u32, f32, f32)> {
    let other_id = if door.room_a == current.id {
        door.room_b
    } else {
        door.room_a
    };
    let dest = door_rooms(other_id)?;
    let on_vert = door_on_vertical_wall(door, current);
    let half_door = door.width / 2.0;

    // Clamp perpendicular axis to door opening
    let (pass_x, pass_y) = if on_vert {
        (
            new_x,
            new_y.clamp(door.door_y - half_door, door.door_y + half_door),
        )
    } else {
        (
            new_x.clamp(door.door_x - half_door, door.door_x + half_door),
            new_y,
        )
    };

    if dest.contains(pass_x, pass_y, radius) {
        return Some((other_id, pass_x, pass_y));
    }

    // In the doorway between rooms — allow movement in the traversal axis
    // spanning both rooms, keep perpendicular clamped to door width
    let (fx, fy) = if on_vert {
        let span_min = current.min_x().min(dest.min_x()) + radius;
        let span_max = current.max_x().max(dest.max_x()) - radius;
        (pass_x.clamp(span_min, span_max), pass_y)
    } else {
        let span_min = current.min_y().min(dest.min_y()) + radius;
        let span_max = current.max_y().max(dest.max_y()) - radius;
        (pass_x, pass_y.clamp(span_min, span_max))
    };

    // Only accept if the doorway position makes sense (not clamped back to start)
    if on_vert {
        let beyond_current = fx < current.min_x() + radius || fx > current.max_x() - radius;
        if beyond_current || dest.contains(fx, fy, radius) {
            return Some((other_id, fx, fy));
        }
    } else {
        let beyond_current = fy < current.min_y() + radius || fy > current.max_y() - radius;
        if beyond_current || dest.contains(fx, fy, radius) {
            return Some((other_id, fx, fy));
        }
    }

    None
}

/// Find the best door for the player's current position and movement direction.
fn find_best_door(
    px: f32,
    py: f32,
    dx: f32,
    dy: f32,
    current_id: u32,
    doors: &[DoorInfo],
) -> Option<&DoorInfo> {
    let mut best: Option<(usize, f32)> = None;
    for (i, door) in doors.iter().enumerate() {
        if door.room_a != current_id && door.room_b != current_id {
            continue;
        }
        let dist = ((px - door.door_x).powi(2) + (py - door.door_y).powi(2)).sqrt();
        let door_zone = (door.width / 2.0 + 2.0).max(3.0);
        if dist > door_zone {
            continue;
        }
        // Skip doors we're moving away from
        let to_x = door.door_x - px;
        let to_y = door.door_y - py;
        let dot = to_x * dx + to_y * dy;
        if dot < -0.001 && dist > 0.5 {
            continue;
        }
        if best.is_none() || dist < best.unwrap().1 {
            best = Some((i, dist));
        }
    }
    best.map(|(i, _)| &doors[i])
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
    let new_x = input.px + input.dx;
    let new_y = input.py + input.dy;
    let r = input.player_radius;

    // Fast path: still inside current room
    if current_room.contains(new_x, new_y, r) {
        return MoveResult::InRoom { x: new_x, y: new_y };
    }

    // Try door traversal with full movement
    if let Some(door) = find_best_door(
        input.px,
        input.py,
        input.dx,
        input.dy,
        current_room.id,
        doors,
    ) {
        if let Some((room_id, x, y)) =
            try_door_traversal(new_x, new_y, r, door, current_room, door_rooms)
        {
            return MoveResult::DoorTraversal { room_id, x, y };
        }
    }

    // Wall-slide: try each axis independently to allow sliding along walls.
    // This prevents the "stuck on corners" feeling.
    let try_x = input.px + input.dx;
    let try_y = input.py + input.dy;

    // Try X-only movement
    let x_ok = current_room.contains(try_x, input.py, r);
    // Try Y-only movement
    let y_ok = current_room.contains(input.px, try_y, r);

    let (slide_x, slide_y) = match (x_ok, y_ok) {
        (true, true) => (try_x, try_y), // Both axes valid (shouldn't reach here)
        (true, false) => {
            // X movement is fine, Y is blocked — check if Y-only leads to a door
            if let Some(door) =
                find_best_door(input.px, input.py, 0.0, input.dy, current_room.id, doors)
            {
                if let Some((room_id, x, y)) =
                    try_door_traversal(input.px, try_y, r, door, current_room, door_rooms)
                {
                    return MoveResult::DoorTraversal { room_id, x, y };
                }
            }
            (try_x, input.py)
        }
        (false, true) => {
            // Y movement is fine, X is blocked — check if X-only leads to a door
            if let Some(door) =
                find_best_door(input.px, input.py, input.dx, 0.0, current_room.id, doors)
            {
                if let Some((room_id, x, y)) =
                    try_door_traversal(try_x, input.py, r, door, current_room, door_rooms)
                {
                    return MoveResult::DoorTraversal { room_id, x, y };
                }
            }
            (input.px, try_y)
        }
        (false, false) => {
            // Both axes blocked — try each axis with door traversal
            if input.dx.abs() > input.dy.abs() {
                if let Some(door) =
                    find_best_door(input.px, input.py, input.dx, 0.0, current_room.id, doors)
                {
                    if let Some((room_id, x, y)) =
                        try_door_traversal(try_x, input.py, r, door, current_room, door_rooms)
                    {
                        return MoveResult::DoorTraversal { room_id, x, y };
                    }
                }
                if let Some(door) =
                    find_best_door(input.px, input.py, 0.0, input.dy, current_room.id, doors)
                {
                    if let Some((room_id, x, y)) =
                        try_door_traversal(input.px, try_y, r, door, current_room, door_rooms)
                    {
                        return MoveResult::DoorTraversal { room_id, x, y };
                    }
                }
            } else {
                if let Some(door) =
                    find_best_door(input.px, input.py, 0.0, input.dy, current_room.id, doors)
                {
                    if let Some((room_id, x, y)) =
                        try_door_traversal(input.px, try_y, r, door, current_room, door_rooms)
                    {
                        return MoveResult::DoorTraversal { room_id, x, y };
                    }
                }
                if let Some(door) =
                    find_best_door(input.px, input.py, input.dx, 0.0, current_room.id, doors)
                {
                    if let Some((room_id, x, y)) =
                        try_door_traversal(try_x, input.py, r, door, current_room, door_rooms)
                    {
                        return MoveResult::DoorTraversal { room_id, x, y };
                    }
                }
            }
            // Clamp to room bounds
            current_room.clamp(try_x, try_y, r)
        }
    };

    if slide_x != input.px || slide_y != input.py {
        MoveResult::WallSlide {
            x: slide_x,
            y: slide_y,
        }
    } else {
        MoveResult::WallSlide {
            x: input.px,
            y: input.py,
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
            player_radius: 0.5,
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
                assert!(x <= 10.0 + 5.0 - 0.5);
                assert!((y - 10.0).abs() < 0.01);
            }
            _ => panic!("Expected WallSlide, got {:?}", result),
        }
    }

    #[test]
    fn test_diagonal_wall_slide() {
        // Moving diagonally into right wall should slide along the Y axis
        let room = simple_room(1, 10.0, 10.0, 10.0, 10.0);
        let result = compute_move(&mi(14.0, 10.0, 2.0, 2.0), &room, &[], &|_| None);
        match result {
            MoveResult::WallSlide { x, y } => {
                // X should be clamped, but Y should advance
                assert!(x <= 14.5 + 0.01, "X should be clamped, got {}", x);
                assert!(
                    (y - 12.0).abs() < 0.01,
                    "Y should advance to 12.0, got {}",
                    y
                );
            }
            _ => panic!("Expected WallSlide, got {:?}", result),
        }
    }

    #[test]
    fn test_door_traversal() {
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
            MoveResult::DoorTraversal { room_id, .. } => {
                assert_eq!(room_id, 2);
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
        assert!(room.contains(10.0, 10.0, 0.5)); // center
        assert!(room.contains(19.0, 19.0, 0.5)); // near corner
        assert!(!room.contains(20.5, 10.0, 0.5)); // outside
    }

    #[test]
    fn test_room_clamp() {
        let room = simple_room(1, 10.0, 10.0, 20.0, 20.0);
        let (x, y) = room.clamp(100.0, -100.0, 0.5);
        assert!((x - 19.5).abs() < 0.01);
        assert!((y - 0.5).abs() < 0.01);
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
            MoveResult::InRoom { y, .. } => {
                assert!(y > 52.8, "Y should advance, got {}", y);
            }
            other => panic!(
                "Expected DoorTraversal or advancing InRoom, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_corner_slide_into_door() {
        // Player near a corner of a corridor with a door on the adjacent wall.
        // Moving diagonally should slide along the wall and eventually enter the door.
        let corridor = simple_room(1, 10.0, 10.0, 3.0, 20.0);
        let room = simple_room(2, 15.0, 18.0, 8.0, 6.0);
        let door = DoorInfo {
            room_a: 1,
            room_b: 2,
            door_x: 11.5,
            door_y: 18.0,
            width: 2.0,
        };
        let lookup = |id: u32| -> Option<RoomBounds> {
            if id == 2 {
                Some(room)
            } else {
                None
            }
        };

        // Moving diagonally right+up near top of corridor, should slide up (not get stuck)
        let result = compute_move(&mi(10.0, 17.0, 3.0, 1.0), &corridor, &[door], &lookup);
        match result {
            MoveResult::WallSlide { y, .. } => {
                assert!(y > 17.0, "Y should advance, got {}", y);
            }
            MoveResult::DoorTraversal { room_id, .. } => {
                assert_eq!(room_id, 2);
            }
            other => panic!("Expected slide or traversal, got {:?}", other),
        }
    }
}
