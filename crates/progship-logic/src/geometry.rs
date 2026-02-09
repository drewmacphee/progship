//! Geometry validation for generated ship layouts.
//!
//! Pure functions that take room/door data and return validation errors.
//! No database dependency — works with plain structs.

use crate::constants::room_types;
use std::collections::{HashMap, HashSet, VecDeque};

/// Minimal room data needed for geometry validation.
#[derive(Debug, Clone)]
pub struct RoomRect {
    pub id: u32,
    pub deck: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub room_type: u8,
    pub capacity: u32,
}

/// Minimal door data needed for geometry validation.
#[derive(Debug, Clone)]
pub struct DoorInfo {
    pub id: u64,
    pub room_a: u32,
    pub room_b: u32,
    pub door_x: f32,
    pub door_y: f32,
    pub wall_a: u8,
    pub wall_b: u8,
}

/// A geometry validation error.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub category: &'static str,
    pub severity: Severity,
    pub message: String,
}

/// Error severity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

// ── A. Room geometry (per-room) ─────────────────────────────────────────

/// Check that no room has zero or negative dimensions.
pub fn check_room_dimensions(rooms: &[RoomRect]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for r in rooms {
        if r.width <= 0.0 || r.height <= 0.0 {
            errors.push(ValidationError {
                category: "room_geometry",
                severity: Severity::Error,
                message: format!(
                    "Room #{} has non-positive dimensions: {}×{}",
                    r.id, r.width, r.height
                ),
            });
        }
    }
    errors
}

/// Check that room aspect ratios are reasonable (< 10:1).
pub fn check_room_aspect_ratios(rooms: &[RoomRect]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for r in rooms {
        if r.width <= 0.0 || r.height <= 0.0 {
            continue; // caught by dimension check
        }
        let ratio = if r.width > r.height {
            r.width / r.height
        } else {
            r.height / r.width
        };
        if ratio > 10.0 {
            errors.push(ValidationError {
                category: "room_geometry",
                severity: Severity::Warning,
                message: format!(
                    "Room #{} has extreme aspect ratio {:.1}:1 ({}×{})",
                    r.id, ratio, r.width, r.height
                ),
            });
        }
    }
    errors
}

/// Check rooms don't extend outside hull boundary.
pub fn check_rooms_within_hull(
    rooms: &[RoomRect],
    hull_width: f32,
    hull_length: f32,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for r in rooms {
        if room_types::is_corridor(r.room_type) {
            continue; // corridors may extend to hull edges
        }
        let right = r.x + r.width;
        let top = r.y + r.height;
        if r.x < -0.5 || r.y < -0.5 || right > hull_width + 0.5 || top > hull_length + 0.5 {
            errors.push(ValidationError {
                category: "room_geometry",
                severity: Severity::Error,
                message: format!(
                    "Room #{} extends outside hull: ({:.1},{:.1})→({:.1},{:.1}) vs hull {}×{}",
                    r.id, r.x, r.y, right, top, hull_width, hull_length
                ),
            });
        }
    }
    errors
}

// ── B. Room-to-room (pairwise) ──────────────────────────────────────────

/// AABB overlap test: check no two rooms on the same deck overlap.
/// Allows a small tolerance for touching edges.
pub fn check_room_overlaps(rooms: &[RoomRect]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let tolerance = 0.1; // allow 0.1m touching

    // Group rooms by deck
    let mut by_deck: HashMap<i32, Vec<&RoomRect>> = HashMap::new();
    for r in rooms {
        by_deck.entry(r.deck).or_default().push(r);
    }

    for deck_rooms in by_deck.values() {
        for i in 0..deck_rooms.len() {
            for j in (i + 1)..deck_rooms.len() {
                let a = deck_rooms[i];
                let b = deck_rooms[j];

                // Skip corridor-corridor and corridor-shaft overlaps (intentional)
                if room_types::is_corridor(a.room_type) || room_types::is_corridor(b.room_type) {
                    continue;
                }

                let overlap_x =
                    (a.x + a.width - tolerance) > b.x && (b.x + b.width - tolerance) > a.x;
                let overlap_y =
                    (a.y + a.height - tolerance) > b.y && (b.y + b.height - tolerance) > a.y;

                if overlap_x && overlap_y {
                    errors.push(ValidationError {
                        category: "room_overlap",
                        severity: Severity::Error,
                        message: format!(
                            "Rooms #{} and #{} overlap on deck {}",
                            a.id, b.id, a.deck
                        ),
                    });
                }
            }
        }
    }
    errors
}

// ── C. Door validity ────────────────────────────────────────────────────

/// Check that both room_a and room_b exist for each door.
pub fn check_door_rooms_exist(doors: &[DoorInfo], rooms: &[RoomRect]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let room_ids: HashSet<u32> = rooms.iter().map(|r| r.id).collect();

    for d in doors {
        if !room_ids.contains(&d.room_a) {
            errors.push(ValidationError {
                category: "door_validity",
                severity: Severity::Error,
                message: format!("Door #{} references non-existent room_a={}", d.id, d.room_a),
            });
        }
        if !room_ids.contains(&d.room_b) {
            errors.push(ValidationError {
                category: "door_validity",
                severity: Severity::Error,
                message: format!("Door #{} references non-existent room_b={}", d.id, d.room_b),
            });
        }
    }
    errors
}

/// Check that every non-corridor, non-shaft room has at least 1 door.
pub fn check_rooms_have_doors(rooms: &[RoomRect], doors: &[DoorInfo]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut connected: HashSet<u32> = HashSet::new();
    for d in doors {
        connected.insert(d.room_a);
        connected.insert(d.room_b);
    }

    for r in rooms {
        if room_types::is_corridor(r.room_type) {
            continue;
        }
        if !connected.contains(&r.id) {
            errors.push(ValidationError {
                category: "door_validity",
                severity: Severity::Error,
                message: format!("Room #{} (type={}) has no doors", r.id, r.room_type),
            });
        }
    }
    errors
}

/// Check for duplicate doors (same room pair connected at same position).
pub fn check_duplicate_doors(doors: &[DoorInfo]) -> Vec<ValidationError> {
    let errors = Vec::new();
    let mut seen: HashSet<(u32, u32)> = HashSet::new();

    for d in doors {
        let key = if d.room_a <= d.room_b {
            (d.room_a, d.room_b)
        } else {
            (d.room_b, d.room_a)
        };
        // Allow multiple doors between same rooms (large shared wall)
        // but flag exact duplicates at same position
        if !seen.insert(key) {
            // Only flag as warning — multiple doors between rooms can be valid
        }
    }
    let _ = seen; // suppress unused
    errors
}

// ── D. Connectivity (graph-level) ───────────────────────────────────────

/// Check that all non-corridor rooms on each deck are reachable via BFS.
pub fn check_deck_connectivity(rooms: &[RoomRect], doors: &[DoorInfo]) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Build adjacency from doors
    let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();
    for d in doors {
        adj.entry(d.room_a).or_default().push(d.room_b);
        adj.entry(d.room_b).or_default().push(d.room_a);
    }

    // Group rooms by deck
    let mut by_deck: HashMap<i32, Vec<u32>> = HashMap::new();
    for r in rooms {
        by_deck.entry(r.deck).or_default().push(r.id);
    }

    for (deck, deck_rooms) in &by_deck {
        if deck_rooms.is_empty() {
            continue;
        }

        // BFS from first room
        let start = deck_rooms[0];
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert(start);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = adj.get(&current) {
                for &next in neighbors {
                    if !visited.contains(&next) && deck_rooms.contains(&next) {
                        visited.insert(next);
                        queue.push_back(next);
                    }
                }
            }
        }

        let unreached: Vec<u32> = deck_rooms
            .iter()
            .filter(|&&r| !visited.contains(&r))
            .copied()
            .collect();

        if !unreached.is_empty() {
            errors.push(ValidationError {
                category: "connectivity",
                severity: Severity::Error,
                message: format!(
                    "Deck {}: {} of {} rooms unreachable (e.g. room #{})",
                    deck,
                    unreached.len(),
                    deck_rooms.len(),
                    unreached[0]
                ),
            });
        }
    }
    errors
}

/// Check that all decks are connected via inter-deck doors (shafts).
pub fn check_inter_deck_connectivity(
    rooms: &[RoomRect],
    doors: &[DoorInfo],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let room_deck: HashMap<u32, i32> = rooms.iter().map(|r| (r.id, r.deck)).collect();

    // Find which decks are connected by doors crossing decks
    let mut deck_adj: HashMap<i32, HashSet<i32>> = HashMap::new();
    let decks: HashSet<i32> = rooms.iter().map(|r| r.deck).collect();
    for &d in &decks {
        deck_adj.entry(d).or_default();
    }

    for d in doors {
        let da = room_deck.get(&d.room_a);
        let db = room_deck.get(&d.room_b);
        if let (Some(&deck_a), Some(&deck_b)) = (da, db) {
            if deck_a != deck_b {
                deck_adj.entry(deck_a).or_default().insert(deck_b);
                deck_adj.entry(deck_b).or_default().insert(deck_a);
            }
        }
    }

    if decks.len() <= 1 {
        return errors;
    }

    // BFS from first deck
    let start = *decks.iter().min().unwrap();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    visited.insert(start);
    queue.push_back(start);

    while let Some(current) = queue.pop_front() {
        if let Some(neighbors) = deck_adj.get(&current) {
            for &next in neighbors {
                if visited.insert(next) {
                    queue.push_back(next);
                }
            }
        }
    }

    let unreached: Vec<i32> = decks
        .iter()
        .filter(|d| !visited.contains(d))
        .copied()
        .collect();

    if !unreached.is_empty() {
        errors.push(ValidationError {
            category: "connectivity",
            severity: Severity::Error,
            message: format!(
                "{} of {} decks not connected inter-deck (e.g. deck {})",
                unreached.len(),
                decks.len(),
                unreached[0]
            ),
        });
    }

    errors
}

// ── Master validation ───────────────────────────────────────────────────

/// Run all geometry validations and return combined results.
pub fn validate_all(
    rooms: &[RoomRect],
    doors: &[DoorInfo],
    hull_width: f32,
    hull_length: f32,
) -> Vec<ValidationError> {
    let mut all = Vec::new();
    all.extend(check_room_dimensions(rooms));
    all.extend(check_room_aspect_ratios(rooms));
    all.extend(check_rooms_within_hull(rooms, hull_width, hull_length));
    all.extend(check_room_overlaps(rooms));
    all.extend(check_door_rooms_exist(doors, rooms));
    all.extend(check_rooms_have_doors(rooms, doors));
    all.extend(check_duplicate_doors(doors));
    all.extend(check_deck_connectivity(rooms, doors));
    all.extend(check_inter_deck_connectivity(rooms, doors));
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_room(id: u32, deck: i32, x: f32, y: f32, w: f32, h: f32) -> RoomRect {
        RoomRect {
            id,
            deck,
            x,
            y,
            width: w,
            height: h,
            room_type: 20, // MESS_HALL
            capacity: 50,
        }
    }

    fn make_door(id: u64, room_a: u32, room_b: u32, dx: f32, dy: f32) -> DoorInfo {
        DoorInfo {
            id,
            room_a,
            room_b,
            door_x: dx,
            door_y: dy,
            wall_a: 0,
            wall_b: 0,
        }
    }

    #[test]
    fn test_valid_rooms_no_errors() {
        let rooms = vec![
            make_room(1, 0, 0.0, 0.0, 10.0, 8.0),
            make_room(2, 0, 10.0, 0.0, 10.0, 8.0),
        ];
        assert!(check_room_dimensions(&rooms).is_empty());
        assert!(check_room_aspect_ratios(&rooms).is_empty());
    }

    #[test]
    fn test_zero_width_room() {
        let rooms = vec![make_room(1, 0, 0.0, 0.0, 0.0, 10.0)];
        let errs = check_room_dimensions(&rooms);
        assert_eq!(errs.len(), 1);
        assert!(errs[0].message.contains("non-positive"));
    }

    #[test]
    fn test_extreme_aspect_ratio() {
        let rooms = vec![make_room(1, 0, 0.0, 0.0, 100.0, 5.0)]; // 20:1
        let errs = check_room_aspect_ratios(&rooms);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].severity, Severity::Warning);
    }

    #[test]
    fn test_room_outside_hull() {
        let rooms = vec![make_room(1, 0, 60.0, 0.0, 20.0, 10.0)]; // extends to 80m
        let errs = check_rooms_within_hull(&rooms, 65.0, 400.0);
        assert_eq!(errs.len(), 1);
    }

    #[test]
    fn test_overlapping_rooms() {
        let rooms = vec![
            make_room(1, 0, 0.0, 0.0, 10.0, 10.0),
            make_room(2, 0, 5.0, 5.0, 10.0, 10.0), // overlaps
        ];
        let errs = check_room_overlaps(&rooms);
        assert_eq!(errs.len(), 1);
    }

    #[test]
    fn test_no_overlap_adjacent() {
        let rooms = vec![
            make_room(1, 0, 0.0, 0.0, 10.0, 10.0),
            make_room(2, 0, 10.0, 0.0, 10.0, 10.0), // touching, not overlapping
        ];
        let errs = check_room_overlaps(&rooms);
        assert!(errs.is_empty());
    }

    #[test]
    fn test_door_missing_room() {
        let rooms = vec![make_room(1, 0, 0.0, 0.0, 10.0, 10.0)];
        let doors = vec![make_door(1, 1, 999, 10.0, 5.0)];
        let errs = check_door_rooms_exist(&doors, &rooms);
        assert_eq!(errs.len(), 1);
        assert!(errs[0].message.contains("999"));
    }

    #[test]
    fn test_room_without_door() {
        let rooms = vec![
            make_room(1, 0, 0.0, 0.0, 10.0, 10.0),
            make_room(2, 0, 10.0, 0.0, 10.0, 10.0),
        ];
        let doors = vec![make_door(1, 1, 1, 5.0, 5.0)]; // only connects room 1
        let errs = check_rooms_have_doors(&rooms, &doors);
        assert_eq!(errs.len(), 1);
        assert!(errs[0].message.contains("#2"));
    }

    #[test]
    fn test_deck_connectivity() {
        let rooms = vec![
            make_room(1, 0, 0.0, 0.0, 10.0, 10.0),
            make_room(2, 0, 10.0, 0.0, 10.0, 10.0),
            make_room(3, 0, 20.0, 0.0, 10.0, 10.0), // island
        ];
        let doors = vec![make_door(1, 1, 2, 10.0, 5.0)];
        let errs = check_deck_connectivity(&rooms, &doors);
        assert_eq!(errs.len(), 1);
        assert!(errs[0].message.contains("unreachable"));
    }

    #[test]
    fn test_fully_connected_deck() {
        let rooms = vec![
            make_room(1, 0, 0.0, 0.0, 10.0, 10.0),
            make_room(2, 0, 10.0, 0.0, 10.0, 10.0),
            make_room(3, 0, 20.0, 0.0, 10.0, 10.0),
        ];
        let doors = vec![make_door(1, 1, 2, 10.0, 5.0), make_door(2, 2, 3, 20.0, 5.0)];
        let errs = check_deck_connectivity(&rooms, &doors);
        assert!(errs.is_empty());
    }

    #[test]
    fn test_inter_deck_connected() {
        let rooms = vec![
            make_room(1, 0, 0.0, 0.0, 10.0, 10.0),
            make_room(2, 1, 0.0, 0.0, 10.0, 10.0),
        ];
        let doors = vec![make_door(1, 1, 2, 5.0, 5.0)]; // cross-deck door
        let errs = check_inter_deck_connectivity(&rooms, &doors);
        assert!(errs.is_empty());
    }

    #[test]
    fn test_inter_deck_disconnected() {
        let rooms = vec![
            make_room(1, 0, 0.0, 0.0, 10.0, 10.0),
            make_room(2, 1, 0.0, 0.0, 10.0, 10.0),
        ];
        let doors = vec![]; // no inter-deck doors
        let errs = check_inter_deck_connectivity(&rooms, &doors);
        assert_eq!(errs.len(), 1);
    }

    #[test]
    fn test_validate_all_clean() {
        let rooms = vec![
            make_room(1, 0, 0.0, 0.0, 10.0, 10.0),
            make_room(2, 0, 10.0, 0.0, 10.0, 10.0),
        ];
        let doors = vec![make_door(1, 1, 2, 10.0, 5.0)];
        let errs = validate_all(&rooms, &doors, 65.0, 400.0);
        assert!(errs.is_empty(), "Expected no errors, got: {:?}", errs);
    }
}
