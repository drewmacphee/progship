//! Ship structure components: Room, Deck, ShipSystem, etc.

use super::common::{BoundingBox, Vec3};
use serde::{Deserialize, Serialize};

/// Room component - represents a physical space on the ship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub room_type: RoomType,
    /// Local bounds within the room (for positioning people inside)
    pub bounds: BoundingBox,
    /// World position of room center on the deck (meters from ship center)
    pub world_x: f32,
    pub world_y: f32,
    /// Which deck this room is on
    pub deck_level: i32,
    /// Maximum occupancy
    pub capacity: u32,
    /// Legacy deck_id for backwards compatibility
    pub deck_id: u32,
}

impl Room {
    pub fn new(name: impl Into<String>, room_type: RoomType, width: f32, depth: f32) -> Self {
        Self {
            name: name.into(),
            room_type,
            bounds: BoundingBox::from_size(width, depth, 3.0), // width, depth, height
            world_x: 0.0,
            world_y: 0.0,
            deck_level: 0,
            capacity: ((width * depth) / 4.0).max(1.0) as u32, // ~4 sq meters per person
            deck_id: 0,
        }
    }

    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.world_x = x;
        self.world_y = y;
        self
    }

    pub fn with_deck(mut self, deck_id: u32) -> Self {
        self.deck_id = deck_id;
        self.deck_level = deck_id as i32;
        self
    }

    pub fn with_deck_level(mut self, level: i32) -> Self {
        self.deck_level = level;
        self.deck_id = level as u32;
        self
    }

    pub fn with_capacity(mut self, capacity: u32) -> Self {
        self.capacity = capacity;
        self
    }

    /// Room width (x dimension)
    pub fn width(&self) -> f32 {
        self.bounds.max.x - self.bounds.min.x
    }

    /// Room depth (y dimension)  
    pub fn depth(&self) -> f32 {
        self.bounds.max.y - self.bounds.min.y
    }

    /// Get world-space bounding box for this room
    pub fn world_bounds(&self) -> (f32, f32, f32, f32) {
        let hw = self.width() / 2.0;
        let hd = self.depth() / 2.0;
        (
            self.world_x - hw, // min_x
            self.world_y - hd, // min_y
            self.world_x + hw, // max_x
            self.world_y + hd, // max_y
        )
    }

    /// Check if this room is adjacent to another room (within 1 meter)
    pub fn is_adjacent_to(&self, other: &Room) -> bool {
        if self.deck_level != other.deck_level {
            return false;
        }

        let (ax1, ay1, ax2, ay2) = self.world_bounds();
        let (bx1, by1, bx2, by2) = other.world_bounds();

        // Check if rooms are within 1 meter of each other
        let gap = 1.0;
        let _x_adjacent = ax2 >= bx1 - gap && ax1 <= bx2 + gap;
        let _y_adjacent = ay2 >= by1 - gap && ay1 <= by2 + gap;
        let x_overlap = ax2 > bx1 && ax1 < bx2;
        let y_overlap = ay2 > by1 && ay1 < by2;

        // Adjacent means sharing an edge (overlap in one dimension, touching in other)
        (x_overlap
            && (ay2 >= by1 - gap && ay2 <= by1 + gap || ay1 >= by2 - gap && ay1 <= by2 + gap))
            || (y_overlap
                && (ax2 >= bx1 - gap && ax2 <= bx1 + gap || ax1 >= bx2 - gap && ax1 <= bx2 + gap))
    }

    /// Get a random position inside this room (local coordinates)
    pub fn random_position(&self, rng: &mut impl rand::Rng) -> Vec3 {
        Vec3::new(
            rng.gen_range(self.bounds.min.x + 0.5..self.bounds.max.x - 0.5),
            rng.gen_range(self.bounds.min.y + 0.5..self.bounds.max.y - 0.5),
            0.0, // Keep on floor
        )
    }

    /// Convert local position to world position
    /// Local coords: (0,0) is bottom-left corner, (width, depth) is top-right
    pub fn local_to_world(&self, local: Vec3) -> Vec3 {
        Vec3::new(
            self.world_x + local.x - self.width() / 2.0,
            self.world_y + local.y - self.depth() / 2.0,
            local.z,
        )
    }

    /// Get the door position (local coords) for entering/exiting this room
    /// Doors are typically at the edge closest to the corridor (y=0)
    pub fn door_position(&self) -> Vec3 {
        // Door is centered on x, at the edge closest to corridor (y=0 in world coords)
        let door_y = if self.world_y > 0.0 {
            // Room is on starboard (positive y), door at min y edge
            0.5 // Just inside the min y edge
        } else {
            // Room is on port (negative y) or center, door at max y edge
            self.depth() - 0.5 // Just inside the max y edge
        };

        Vec3::new(self.width() / 2.0, door_y, 0.0)
    }

    /// Get the door position in world coordinates
    pub fn door_world_position(&self) -> Vec3 {
        self.local_to_world(self.door_position())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoomType {
    // Command
    Bridge,
    ConferenceRoom,

    // Engineering
    Engineering,
    ReactorRoom,
    MaintenanceBay,

    // Living
    Quarters,
    QuartersCrew,
    QuartersOfficer,
    QuartersPassenger,

    // Services
    Mess,
    Galley,
    Medical,
    Recreation,
    Gym,

    // Utility
    Cargo,
    Storage,
    Airlock,
    Corridor,
    Elevator,

    // Science
    Laboratory,
    Observatory,

    // Life Support
    LifeSupport,
    Hydroponics,
    WaterRecycling,
}

impl RoomType {
    /// What activities typically happen here?
    pub fn typical_activities(&self) -> Vec<super::people::ActivityType> {
        use super::people::ActivityType;
        match self {
            RoomType::Bridge | RoomType::Engineering | RoomType::ReactorRoom => {
                vec![ActivityType::Working]
            }
            RoomType::Quarters
            | RoomType::QuartersCrew
            | RoomType::QuartersOfficer
            | RoomType::QuartersPassenger => {
                vec![
                    ActivityType::Sleeping,
                    ActivityType::Relaxing,
                    ActivityType::Hygiene,
                ]
            }
            RoomType::Mess | RoomType::Galley => {
                vec![ActivityType::Eating, ActivityType::Socializing]
            }
            RoomType::Medical => vec![ActivityType::Working],
            RoomType::Recreation | RoomType::Gym => {
                vec![ActivityType::Relaxing, ActivityType::Socializing]
            }
            RoomType::Corridor | RoomType::Elevator => vec![ActivityType::Traveling],
            _ => vec![ActivityType::Working],
        }
    }
}

/// Connections to other rooms (for pathfinding)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoomConnections {
    /// IDs of connected rooms
    pub connected_to: Vec<u32>,
}

impl RoomConnections {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connect(&mut self, room_id: u32) {
        if !self.connected_to.contains(&room_id) {
            self.connected_to.push(room_id);
        }
    }

    pub fn is_connected(&self, room_id: u32) -> bool {
        self.connected_to.contains(&room_id)
    }
}

/// Deck component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    pub name: String,
    pub level: i32,
}

impl Deck {
    pub fn new(name: impl Into<String>, level: i32) -> Self {
        Self {
            name: name.into(),
            level,
        }
    }
}

/// Ship system component (power, life support, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipSystem {
    pub name: String,
    pub system_type: SystemType,
    pub health: f32,
    pub status: SystemStatus,
}

impl ShipSystem {
    pub fn new(name: impl Into<String>, system_type: SystemType) -> Self {
        Self {
            name: name.into(),
            system_type,
            health: 1.0,
            status: SystemStatus::Nominal,
        }
    }

    /// Update status based on health
    pub fn update_status(&mut self) {
        self.status = match self.health {
            h if h >= 0.9 => SystemStatus::Nominal,
            h if h >= 0.5 => SystemStatus::Degraded,
            h if h >= 0.1 => SystemStatus::Critical,
            _ => SystemStatus::Offline,
        };
    }

    /// Apply degradation over time
    pub fn degrade(&mut self, hours: f32, rate: f32) {
        self.health = (self.health - hours * rate).clamp(0.0, 1.0);
        self.update_status();
    }

    /// Repair the system
    pub fn repair(&mut self, amount: f32) {
        self.health = (self.health + amount).clamp(0.0, 1.0);
        self.update_status();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemType {
    Power,
    LifeSupport,
    Propulsion,
    Navigation,
    Communications,
    Weapons,
    Shields,
    Medical,
    FoodProduction,
    WaterRecycling,
    Gravity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemStatus {
    Nominal,
    Degraded,
    Critical,
    Offline,
    Destroyed,
}

/// Resource flow component - what a system consumes/produces
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceFlow {
    pub consumes: Vec<(ResourceType, f32)>,
    pub produces: Vec<(ResourceType, f32)>,
}

impl ResourceFlow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn consumes(mut self, resource: ResourceType, rate: f32) -> Self {
        self.consumes.push((resource, rate));
        self
    }

    pub fn produces(mut self, resource: ResourceType, rate: f32) -> Self {
        self.produces.push((resource, rate));
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Power,
    Water,
    Oxygen,
    Food,
    Fuel,
    Coolant,
    SpareParts,
}

/// Ship-wide resource storage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceStorage {
    pub power: f32,
    pub water: f32,
    pub oxygen: f32,
    pub food: f32,
    pub fuel: f32,
    pub coolant: f32,
    pub spare_parts: f32,
}

impl ResourceStorage {
    pub fn get(&self, resource: ResourceType) -> f32 {
        match resource {
            ResourceType::Power => self.power,
            ResourceType::Water => self.water,
            ResourceType::Oxygen => self.oxygen,
            ResourceType::Food => self.food,
            ResourceType::Fuel => self.fuel,
            ResourceType::Coolant => self.coolant,
            ResourceType::SpareParts => self.spare_parts,
        }
    }

    pub fn get_mut(&mut self, resource: ResourceType) -> &mut f32 {
        match resource {
            ResourceType::Power => &mut self.power,
            ResourceType::Water => &mut self.water,
            ResourceType::Oxygen => &mut self.oxygen,
            ResourceType::Food => &mut self.food,
            ResourceType::Fuel => &mut self.fuel,
            ResourceType::Coolant => &mut self.coolant,
            ResourceType::SpareParts => &mut self.spare_parts,
        }
    }

    pub fn consume(&mut self, resource: ResourceType, amount: f32) -> bool {
        let storage = self.get_mut(resource);
        if *storage >= amount {
            *storage -= amount;
            true
        } else {
            false
        }
    }

    pub fn produce(&mut self, resource: ResourceType, amount: f32) {
        *self.get_mut(resource) += amount;
    }
}

/// Maintenance requirements for a system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceSchedule {
    pub interval_hours: f32,
    pub last_maintenance: f64,
    pub required_skill: super::people::SkillType,
    pub duration_hours: f32,
}

impl MaintenanceSchedule {
    pub fn new(interval_hours: f32, skill: super::people::SkillType) -> Self {
        Self {
            interval_hours,
            last_maintenance: 0.0,
            required_skill: skill,
            duration_hours: 1.0,
        }
    }

    pub fn needs_maintenance(&self, current_time: f64) -> bool {
        current_time - self.last_maintenance >= self.interval_hours as f64
    }
}

/// Active maintenance task on a ship system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceTask {
    /// The system entity being repaired
    pub system_entity_id: u32,
    /// Crew member assigned to this task (entity ID, not component)
    pub assigned_crew_id: Option<u32>,
    /// Priority (0.0 - 1.0, higher = more urgent)
    pub priority: f32,
    /// Repair progress (0.0 - 1.0)
    pub progress: f32,
    /// Time when task was created
    pub created_at: f64,
}

impl MaintenanceTask {
    pub fn new(system_entity_id: u32, priority: f32, created_at: f64) -> Self {
        Self {
            system_entity_id,
            assigned_crew_id: None,
            priority,
            progress: 0.0,
            created_at,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }

    pub fn assign(&mut self, crew_id: u32) {
        self.assigned_crew_id = Some(crew_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_capacity() {
        let room = Room::new("Test", RoomType::Quarters, 10.0, 10.0);
        assert_eq!(room.capacity, 25); // 100 sq meters / 4 = 25
    }

    #[test]
    fn test_ship_system_degradation() {
        let mut system = ShipSystem::new("Reactor", SystemType::Power);
        assert_eq!(system.status, SystemStatus::Nominal);

        system.degrade(10.0, 0.05); // 50% degradation
        assert_eq!(system.status, SystemStatus::Degraded);

        system.repair(0.5);
        assert_eq!(system.status, SystemStatus::Nominal);
    }

    #[test]
    fn test_resource_storage() {
        let mut storage = ResourceStorage::default();
        storage.produce(ResourceType::Power, 100.0);
        assert!(storage.consume(ResourceType::Power, 50.0));
        assert_eq!(storage.get(ResourceType::Power), 50.0);
        assert!(!storage.consume(ResourceType::Power, 100.0)); // Not enough
    }
}
