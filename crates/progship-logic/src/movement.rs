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

    /// Check if a point (with radius padding) is inside this room.
    pub fn contains(&self, x: f32, y: f32, radius: f32) -> bool {
        let hw = self.half_w - radius;
        let hh = self.half_h - radius;
        x >= self.cx - hw && x <= self.cx + hw && y >= self.cy - hh && y <= self.cy + hh
    }

    /// Clamp a point to stay inside this room (with radius padding).
    pub fn clamp(&self, x: f32, y: f32, radius: f32) -> (f32, f32) {
        let hw = self.half_w - radius;
        let hh = self.half_h - radius;
        (
            x.clamp(self.cx - hw, self.cx + hw),
            y.clamp(self.cy - hh, self.cy + hh),
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

/// Compute the result of a movement within the given room, checking doors
/// for room transitions.
///
/// Pure function — no database access.
pub fn compute_move(
    input: &MoveInput,
    current_room: &RoomBounds,
    doors: &[DoorInfo],
    door_rooms: &dyn Fn(u32) -> Option<RoomBounds>,
) -> MoveResult {
    let new_x = input.px + input.dx;
    let new_y = input.py + input.dy;

    // If still inside current room, just move
    if current_room.contains(new_x, new_y, input.player_radius) {
        return MoveResult::InRoom { x: new_x, y: new_y };
    }

    // Outside room bounds — check if near a door
    for door in doors {
        let other_room_id = if door.room_a == current_room.id {
            door.room_b
        } else if door.room_b == current_room.id {
            door.room_a
        } else {
            continue;
        };

        // Use current position (not new) for door proximity — prevents skipping doors
        let dist_to_door =
            ((input.px - door.door_x).powi(2) + (input.py - door.door_y).powi(2)).sqrt();
        let door_zone = (door.width / 2.0 + 2.0).max(3.0);
        if dist_to_door > door_zone {
            continue;
        }

        let Some(dest) = door_rooms(other_room_id) else {
            continue;
        };

        // Determine door orientation by checking which wall of current room
        // the door is closest to. Compare distance to vertical walls (left/right)
        // vs horizontal walls (top/bottom).
        let dist_to_left = (door.door_x - (current_room.cx - current_room.half_w)).abs();
        let dist_to_right = (door.door_x - (current_room.cx + current_room.half_w)).abs();
        let dist_to_top = (door.door_y - (current_room.cy - current_room.half_h)).abs();
        let dist_to_bottom = (door.door_y - (current_room.cy + current_room.half_h)).abs();
        let min_vertical = dist_to_left.min(dist_to_right);
        let min_horizontal = dist_to_top.min(dist_to_bottom);
        let on_vertical_wall = min_vertical < min_horizontal;

        // Clamp perpendicular axis to door width so player can't clip through
        // adjacent walls, but allow free movement along the traversal axis
        let half_door = door.width / 2.0;
        let (pass_x, pass_y) = if on_vertical_wall {
            let cy = new_y.clamp(door.door_y - half_door, door.door_y + half_door);
            (new_x, cy)
        } else {
            let cx = new_x.clamp(door.door_x - half_door, door.door_x + half_door);
            (cx, new_y)
        };

        // Check if we've crossed into the destination room
        if dest.contains(pass_x, pass_y, input.player_radius) {
            return MoveResult::DoorTraversal {
                room_id: other_room_id,
                x: pass_x,
                y: pass_y,
            };
        }

        // Still in doorway — clamp traversal axis to span between the two rooms,
        // and perpendicular axis to the door width (already done in pass_x/pass_y).
        let (cx, cy) = current_room.clamp(pass_x, pass_y, input.player_radius);
        let (final_x, final_y) = if on_vertical_wall {
            // Door on east/west wall: bound X between current and dest room edges
            let min_x = (current_room.cx - current_room.half_w)
                .min(dest.cx - dest.half_w);
            let max_x = (current_room.cx + current_room.half_w)
                .max(dest.cx + dest.half_w);
            (pass_x.clamp(min_x + input.player_radius, max_x - input.player_radius), cy)
        } else {
            // Door on north/south wall: bound Y between current and dest room edges
            let min_y = (current_room.cy - current_room.half_h)
                .min(dest.cy - dest.half_h);
            let max_y = (current_room.cy + current_room.half_h)
                .max(dest.cy + dest.half_h);
            (cx, pass_y.clamp(min_y + input.player_radius, max_y - input.player_radius))
        };
        return MoveResult::InRoom {
            x: final_x,
            y: final_y,
        };
    }

    // No door nearby — wall-slide along current room boundary
    let (cx, cy) = current_room.clamp(new_x, new_y, input.player_radius);
    MoveResult::WallSlide { x: cx, y: cy }
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
}
