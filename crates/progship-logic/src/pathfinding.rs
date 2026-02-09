//! Pure pathfinding over the door connectivity graph.
//!
//! `NavGraph` holds a pre-built adjacency list from door data and provides
//! BFS pathfinding with an optional LRU-style cache.

use std::collections::{HashMap, HashSet, VecDeque};

/// A door edge in the navigation graph.
#[derive(Debug, Clone, Copy)]
pub struct DoorEdge {
    pub room_a: u32,
    pub room_b: u32,
    pub door_x: f32,
    pub door_y: f32,
}

/// A single waypoint in a path: walk to this door position, enter this room.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Waypoint {
    pub door_x: f32,
    pub door_y: f32,
    pub room_id: u32,
}

/// Pre-built navigation graph with BFS pathfinding and path cache.
pub struct NavGraph {
    /// room_id → list of (neighbor_room_id, door_x, door_y)
    adj: HashMap<u32, Vec<(u32, f32, f32)>>,
    /// (from, to) → cached path. Simple bounded cache.
    cache: HashMap<(u32, u32), Vec<Waypoint>>,
    cache_capacity: usize,
}

impl NavGraph {
    /// Build a navigation graph from door edges.
    pub fn from_doors(doors: &[DoorEdge]) -> Self {
        Self::from_doors_with_cache(doors, 256)
    }

    /// Build a navigation graph with a specific cache capacity.
    pub fn from_doors_with_cache(doors: &[DoorEdge], cache_capacity: usize) -> Self {
        let mut adj: HashMap<u32, Vec<(u32, f32, f32)>> = HashMap::new();
        for door in doors {
            adj.entry(door.room_a)
                .or_default()
                .push((door.room_b, door.door_x, door.door_y));
            adj.entry(door.room_b)
                .or_default()
                .push((door.room_a, door.door_x, door.door_y));
        }
        Self {
            adj,
            cache: HashMap::new(),
            cache_capacity,
        }
    }

    /// Find a path from `from_room` to `to_room` via BFS.
    ///
    /// Returns a list of waypoints (door positions + room entered).
    /// Returns empty vec if same room. Returns `None` if unreachable.
    pub fn find_path(&mut self, from_room: u32, to_room: u32) -> Option<Vec<Waypoint>> {
        if from_room == to_room {
            return Some(vec![]);
        }

        // Check cache
        let key = (from_room, to_room);
        if let Some(cached) = self.cache.get(&key) {
            return Some(cached.clone());
        }

        // BFS
        let result = self.bfs(from_room, to_room);

        // Cache result if found
        if let Some(ref path) = result {
            if self.cache.len() >= self.cache_capacity {
                // Evict oldest entry (arbitrary — HashMap iteration order)
                if let Some(&evict_key) = self.cache.keys().next() {
                    self.cache.remove(&evict_key);
                }
            }
            self.cache.insert(key, path.clone());
        }

        result
    }

    /// Get neighbors of a room (for wandering to adjacent rooms).
    pub fn neighbors(&self, room_id: u32) -> &[(u32, f32, f32)] {
        self.adj.get(&room_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Check if a room exists in the graph.
    pub fn has_room(&self, room_id: u32) -> bool {
        self.adj.contains_key(&room_id)
    }

    /// Number of rooms in the graph.
    pub fn room_count(&self) -> usize {
        self.adj.len()
    }

    /// Clear the path cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Number of cached paths.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    fn bfs(&self, from_room: u32, to_room: u32) -> Option<Vec<Waypoint>> {
        let mut visited = HashSet::new();
        let mut queue: VecDeque<(u32, Vec<Waypoint>)> = VecDeque::new();
        visited.insert(from_room);
        queue.push_back((from_room, vec![]));

        while let Some((current, path)) = queue.pop_front() {
            if let Some(neighbors) = self.adj.get(&current) {
                for &(next_room, door_x, door_y) in neighbors {
                    if next_room == to_room {
                        let mut result = path.clone();
                        result.push(Waypoint {
                            door_x,
                            door_y,
                            room_id: next_room,
                        });
                        return Some(result);
                    }
                    if visited.insert(next_room) {
                        let mut new_path = path.clone();
                        new_path.push(Waypoint {
                            door_x,
                            door_y,
                            room_id: next_room,
                        });
                        queue.push_back((next_room, new_path));
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linear_graph() -> (Vec<DoorEdge>, NavGraph) {
        // A --door1--> B --door2--> C
        let doors = vec![
            DoorEdge {
                room_a: 1,
                room_b: 2,
                door_x: 10.0,
                door_y: 5.0,
            },
            DoorEdge {
                room_a: 2,
                room_b: 3,
                door_x: 20.0,
                door_y: 5.0,
            },
        ];
        let graph = NavGraph::from_doors(&doors);
        (doors, graph)
    }

    #[test]
    fn test_same_room() {
        let (_, mut graph) = linear_graph();
        let path = graph.find_path(1, 1);
        assert_eq!(path, Some(vec![]));
    }

    #[test]
    fn test_adjacent_rooms() {
        let (_, mut graph) = linear_graph();
        let path = graph.find_path(1, 2).unwrap();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0].room_id, 2);
        assert!((path[0].door_x - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_multi_hop() {
        let (_, mut graph) = linear_graph();
        let path = graph.find_path(1, 3).unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].room_id, 2);
        assert_eq!(path[1].room_id, 3);
    }

    #[test]
    fn test_reverse_direction() {
        let (_, mut graph) = linear_graph();
        let path = graph.find_path(3, 1).unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].room_id, 2);
        assert_eq!(path[1].room_id, 1);
    }

    #[test]
    fn test_unreachable() {
        let doors = vec![
            DoorEdge {
                room_a: 1,
                room_b: 2,
                door_x: 10.0,
                door_y: 5.0,
            },
            // Room 99 is isolated
        ];
        let mut graph = NavGraph::from_doors(&doors);
        assert_eq!(graph.find_path(1, 99), None);
    }

    #[test]
    fn test_cross_deck_via_shaft() {
        // Deck 0: rooms 1, 2, shaft 10
        // Deck 1: rooms 3, 4, shaft 11
        // Shaft 10 <-> Shaft 11 (cross-deck door)
        let doors = vec![
            DoorEdge {
                room_a: 1,
                room_b: 10,
                door_x: 5.0,
                door_y: 5.0,
            },
            DoorEdge {
                room_a: 2,
                room_b: 10,
                door_x: 6.0,
                door_y: 5.0,
            },
            DoorEdge {
                room_a: 10,
                room_b: 11,
                door_x: 5.0,
                door_y: 5.0,
            }, // shaft door
            DoorEdge {
                room_a: 11,
                room_b: 3,
                door_x: 5.0,
                door_y: 15.0,
            },
            DoorEdge {
                room_a: 11,
                room_b: 4,
                door_x: 6.0,
                door_y: 15.0,
            },
        ];
        let mut graph = NavGraph::from_doors(&doors);
        // Room 1 (deck 0) → Room 4 (deck 1) via shafts
        let path = graph.find_path(1, 4).unwrap();
        assert!(path.len() >= 3); // at least: shaft10, shaft11, room4
        assert_eq!(path.last().unwrap().room_id, 4);
    }

    #[test]
    fn test_cache_hit() {
        let (_, mut graph) = linear_graph();
        // First call — BFS
        let path1 = graph.find_path(1, 3).unwrap();
        assert_eq!(graph.cache_size(), 1);
        // Second call — cache hit
        let path2 = graph.find_path(1, 3).unwrap();
        assert_eq!(path1, path2);
        assert_eq!(graph.cache_size(), 1); // no new entry
    }

    #[test]
    fn test_cache_eviction() {
        let doors = vec![
            DoorEdge {
                room_a: 1,
                room_b: 2,
                door_x: 10.0,
                door_y: 5.0,
            },
            DoorEdge {
                room_a: 2,
                room_b: 3,
                door_x: 20.0,
                door_y: 5.0,
            },
            DoorEdge {
                room_a: 3,
                room_b: 4,
                door_x: 30.0,
                door_y: 5.0,
            },
        ];
        // Cache capacity of 2
        let mut graph = NavGraph::from_doors_with_cache(&doors, 2);
        graph.find_path(1, 2); // cache: {(1,2)}
        graph.find_path(1, 3); // cache: {(1,2), (1,3)}
        assert_eq!(graph.cache_size(), 2);
        graph.find_path(1, 4); // evicts one, cache still at 2
        assert_eq!(graph.cache_size(), 2);
    }

    #[test]
    fn test_neighbors() {
        let (_, graph) = linear_graph();
        let n = graph.neighbors(2);
        assert_eq!(n.len(), 2); // connected to room 1 and room 3
    }

    #[test]
    fn test_has_room() {
        let (_, graph) = linear_graph();
        assert!(graph.has_room(1));
        assert!(graph.has_room(2));
        assert!(!graph.has_room(99));
    }

    #[test]
    fn test_branching_graph() {
        //     1
        //    / \
        //   2   3
        //  / \
        // 4   5
        let doors = vec![
            DoorEdge {
                room_a: 1,
                room_b: 2,
                door_x: 5.0,
                door_y: 5.0,
            },
            DoorEdge {
                room_a: 1,
                room_b: 3,
                door_x: 15.0,
                door_y: 5.0,
            },
            DoorEdge {
                room_a: 2,
                room_b: 4,
                door_x: 3.0,
                door_y: 10.0,
            },
            DoorEdge {
                room_a: 2,
                room_b: 5,
                door_x: 7.0,
                door_y: 10.0,
            },
        ];
        let mut graph = NavGraph::from_doors(&doors);
        // Shortest path from 3 to 5: 3→1→2→5
        let path = graph.find_path(3, 5).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[2].room_id, 5);
        // 3 to 4: 3→1→2→4
        let path = graph.find_path(3, 4).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[2].room_id, 4);
    }
}
