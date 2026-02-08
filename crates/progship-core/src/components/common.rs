//! Common components used across multiple entity types.

use hecs::Entity;
use serde::{Deserialize, Serialize};

/// 3D position vector
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn distance_squared(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn distance(&self, other: &Self) -> f32 {
        self.distance_squared(other).sqrt()
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            Self::ZERO
        }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

/// Axis-aligned bounding box
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_size(width: f32, height: f32, depth: f32) -> Self {
        Self {
            min: Vec3::ZERO,
            max: Vec3::new(width, height, depth),
        }
    }

    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn depth(&self) -> f32 {
        self.max.z - self.min.z
    }

    pub fn contains(&self, point: &Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }
}

/// Spatial position component - where an entity is located
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    /// Local position within the room
    pub local: Vec3,
    /// The room entity this position is relative to
    /// Serialized as u64 for save/load (Entity is not directly serializable)
    #[serde(skip)]
    pub room: Option<Entity>,
    /// Room ID for serialization
    pub room_id: u32,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            local: Vec3::ZERO,
            room: None,
            room_id: 0,
        }
    }
}

impl Position {
    pub fn new(x: f32, y: f32, room_id: u32) -> Self {
        Self {
            local: Vec3::new(x, y, 0.0),
            room: None,
            room_id,
        }
    }

    pub fn with_room(mut self, room: Entity) -> Self {
        self.room = Some(room);
        self
    }
}

/// Movement component - present only while entity is moving
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movement {
    /// Target position in current room
    pub destination: Vec3,
    /// Final destination in the target room
    pub final_destination: Vec3,
    /// Movement speed in units per second
    pub speed: f32,
    /// Path of room IDs to traverse (for inter-room movement)
    pub path: Vec<u32>,
    /// Current index in path
    pub path_index: usize,
    /// Door position to enter next room (local coords of next room)
    pub next_door_position: Option<Vec3>,
    /// Entry door positions for each room in path
    pub entry_door_positions: Vec<Vec3>,
    /// Exit door positions for each room in path
    pub exit_door_positions: Vec<Vec3>,
}

impl Movement {
    pub fn new(destination: Vec3, speed: f32) -> Self {
        Self {
            destination,
            final_destination: destination,
            speed,
            path: Vec::new(),
            path_index: 0,
            next_door_position: None,
            entry_door_positions: Vec::new(),
            exit_door_positions: Vec::new(),
        }
    }

    pub fn with_path(mut self, path: Vec<u32>) -> Self {
        self.path = path;
        self
    }
}

/// Name component for entities that have names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Name {
    pub given: String,
    pub family: String,
    pub nickname: Option<String>,
}

impl Name {
    pub fn new(given: impl Into<String>, family: impl Into<String>) -> Self {
        Self {
            given: given.into(),
            family: family.into(),
            nickname: None,
        }
    }

    pub fn with_nickname(mut self, nickname: impl Into<String>) -> Self {
        self.nickname = Some(nickname.into());
        self
    }

    pub fn full_name(&self) -> String {
        format!("{} {}", self.given, self.family)
    }

    pub fn display_name(&self) -> &str {
        self.nickname.as_deref().unwrap_or(&self.given)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_operations() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        
        let sum = a + b;
        assert_eq!(sum.x, 5.0);
        assert_eq!(sum.y, 7.0);
        assert_eq!(sum.z, 9.0);

        let diff = b - a;
        assert_eq!(diff.x, 3.0);
        
        let scaled = a * 2.0;
        assert_eq!(scaled.x, 2.0);
        assert_eq!(scaled.y, 4.0);
    }

    #[test]
    fn test_vec3_normalize() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let n = v.normalize();
        assert!((n.length() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_bounding_box_contains() {
        let bb = BoundingBox::from_size(10.0, 10.0, 10.0);
        assert!(bb.contains(&Vec3::new(5.0, 5.0, 5.0)));
        assert!(!bb.contains(&Vec3::new(15.0, 5.0, 5.0)));
    }

    #[test]
    fn test_name() {
        let name = Name::new("John", "Doe").with_nickname("JD");
        assert_eq!(name.full_name(), "John Doe");
        assert_eq!(name.display_name(), "JD");
    }
}
