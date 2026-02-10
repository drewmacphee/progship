//! O'Neill cylinder ship geometry and layout.
//!
//! An O'Neill cylinder is a rotating habitat with a cylindrical interior.
//! The living surface is on the inside of the cylinder, and artificial
//! gravity is produced by rotation.
//!
//! # Layout
//!
//! The cylinder is divided into:
//! - **Sectors**: Longitudinal slices (like orange segments)
//! - **Rings**: Concentric rings at different radii
//! - **Levels**: Along the cylinder's axis (fore/aft)
//!
//! ```text
//!     ┌──────────────────────┐
//!     │  Sector 0  │ Sector 1 │  ← Cross-section
//!     │            │          │
//!     ├────────────┼──────────┤
//!     │  Sector 3  │ Sector 2 │
//!     │            │          │
//!     └──────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```
//! use progship_logic::cylinder::{CylinderConfig, generate_cylinder_layout, CylinderRoom};
//!
//! let config = CylinderConfig::default();
//! let rooms = generate_cylinder_layout(&config);
//! assert!(!rooms.is_empty());
//! ```

use serde::{Deserialize, Serialize};

/// Configuration for an O'Neill cylinder habitat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CylinderConfig {
    /// Cylinder length in meters (along axis).
    pub length: f32,
    /// Inner radius in meters (living surface).
    pub inner_radius: f32,
    /// Outer radius in meters (hull).
    pub outer_radius: f32,
    /// Number of longitudinal sectors (like orange segments).
    pub sector_count: u32,
    /// Number of axial levels (rings along the length).
    pub level_count: u32,
    /// Number of radial layers (from inner surface outward).
    pub radial_layers: u32,
    /// Width of ring corridors in meters.
    pub corridor_width: f32,
    /// Rotation rate in RPM (determines artificial gravity).
    pub rotation_rpm: f32,
}

impl Default for CylinderConfig {
    fn default() -> Self {
        Self {
            length: 800.0,
            inner_radius: 200.0,
            outer_radius: 250.0,
            sector_count: 6,
            level_count: 10,
            radial_layers: 3,
            corridor_width: 3.0,
            rotation_rpm: 1.0,
        }
    }
}

impl CylinderConfig {
    /// Calculate artificial gravity at the inner surface (in g).
    ///
    /// g = ω²r where ω = 2π × RPM/60.
    pub fn surface_gravity(&self) -> f32 {
        let omega = 2.0 * std::f32::consts::PI * self.rotation_rpm / 60.0;
        let accel = omega * omega * self.inner_radius;
        accel / 9.81 // convert to g
    }

    /// Calculate the arc length of one sector at the inner surface.
    pub fn sector_arc_length(&self) -> f32 {
        2.0 * std::f32::consts::PI * self.inner_radius / self.sector_count as f32
    }

    /// Calculate the axial length of one level.
    pub fn level_length(&self) -> f32 {
        self.length / self.level_count as f32
    }

    /// Calculate the radial thickness of one layer.
    pub fn layer_thickness(&self) -> f32 {
        (self.outer_radius - self.inner_radius) / self.radial_layers as f32
    }

    /// Total habitable surface area (inner surface, m²).
    pub fn habitable_area(&self) -> f32 {
        2.0 * std::f32::consts::PI * self.inner_radius * self.length
    }

    /// Total number of room slots (sector × level × layer).
    pub fn total_slots(&self) -> u32 {
        self.sector_count * self.level_count * self.radial_layers
    }
}

/// A room in the O'Neill cylinder, positioned by sector/level/layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CylinderRoom {
    /// Room ID.
    pub id: u32,
    /// Sector index (0..sector_count).
    pub sector: u32,
    /// Axial level index (0..level_count).
    pub level: u32,
    /// Radial layer (0 = inner/habitable surface, higher = further from surface).
    pub layer: u32,
    /// Room type (from constants::room_types).
    pub room_type: u8,
    /// Room width in meters (arc length at this radius).
    pub width: f32,
    /// Room depth in meters (along cylinder axis).
    pub depth: f32,
    /// Room height in meters (radial direction).
    pub height: f32,
    /// Whether this is a corridor connecting adjacent rooms.
    pub is_corridor: bool,
}

impl CylinderRoom {
    /// Floor area in m².
    pub fn area(&self) -> f32 {
        self.width * self.depth
    }

    /// Volume in m³.
    pub fn volume(&self) -> f32 {
        self.width * self.depth * self.height
    }

    /// Effective gravity at this room's layer (fraction of surface gravity).
    ///
    /// Gravity decreases linearly toward the center: g ∝ r.
    pub fn effective_gravity(&self, config: &CylinderConfig) -> f32 {
        let layer_thickness = config.layer_thickness();
        let radius = config.inner_radius + (self.layer as f32 + 0.5) * layer_thickness;
        let surface_g = config.surface_gravity();
        surface_g * (radius / config.inner_radius)
    }
}

/// Sector purpose assignment for a balanced habitat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectorPurpose {
    /// Residential quarters and cabins.
    Residential,
    /// Agriculture, hydroponics, food production.
    Agricultural,
    /// Engineering, power, life support.
    Industrial,
    /// Parks, recreation, public spaces.
    Park,
    /// Command, medical, administration.
    Command,
    /// Mixed-use (commercial, education, social).
    MixedUse,
}

/// Assign purposes to sectors for a balanced habitat.
pub fn assign_sector_purposes(sector_count: u32) -> Vec<SectorPurpose> {
    let purposes = [
        SectorPurpose::Residential,
        SectorPurpose::Agricultural,
        SectorPurpose::Industrial,
        SectorPurpose::Park,
        SectorPurpose::Command,
        SectorPurpose::MixedUse,
    ];
    (0..sector_count)
        .map(|i| purposes[i as usize % purposes.len()])
        .collect()
}

/// Map sector purpose to primary room type for that sector.
pub fn primary_room_type(purpose: SectorPurpose) -> u8 {
    use crate::constants::room_types as rt;
    match purpose {
        SectorPurpose::Residential => rt::CABIN_SINGLE,
        SectorPurpose::Agricultural => rt::HYDROPONICS,
        SectorPurpose::Industrial => rt::ENGINEERING,
        SectorPurpose::Park => rt::ARBORETUM,
        SectorPurpose::Command => rt::BRIDGE,
        SectorPurpose::MixedUse => rt::LOUNGE,
    }
}

/// Room type for infrastructure layers (inner layers closer to hull).
pub fn infrastructure_room_type(layer: u32) -> u8 {
    use crate::constants::room_types as rt;
    match layer {
        0 => rt::CORRIDOR,        // Innermost: habitable corridors
        1 => rt::STORAGE,         // Middle: storage and utilities
        _ => rt::MAINTENANCE_BAY, // Outer: maintenance and hull access
    }
}

/// Generate the full room layout for an O'Neill cylinder.
///
/// Creates rooms for all sector×level×layer slots, with corridors
/// at layer 0 between sectors, and purpose-appropriate rooms elsewhere.
pub fn generate_cylinder_layout(config: &CylinderConfig) -> Vec<CylinderRoom> {
    let mut rooms = Vec::new();
    let mut id = 0u32;
    let purposes = assign_sector_purposes(config.sector_count);

    for sector in 0..config.sector_count {
        let purpose = purposes[sector as usize];
        for level in 0..config.level_count {
            for layer in 0..config.radial_layers {
                let layer_thickness = config.layer_thickness();
                let radius = config.inner_radius + (layer as f32 + 0.5) * layer_thickness;
                let arc_length = 2.0 * std::f32::consts::PI * radius / config.sector_count as f32;

                // Determine room type
                let (room_type, is_corridor) = if layer == 0 && level % 3 == 0 {
                    // Ring corridors every 3 levels at the surface
                    (crate::constants::room_types::CORRIDOR, true)
                } else if layer == 0 {
                    // Surface rooms — purpose-driven
                    (primary_room_type(purpose), false)
                } else {
                    // Infrastructure layers
                    (infrastructure_room_type(layer), false)
                };

                rooms.push(CylinderRoom {
                    id,
                    sector,
                    level,
                    layer,
                    room_type,
                    width: arc_length - config.corridor_width,
                    depth: config.level_length(),
                    height: layer_thickness,
                    is_corridor,
                });
                id += 1;
            }
        }
    }

    rooms
}

/// Connection between two cylinder rooms (for pathfinding).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CylinderDoor {
    pub room_a: u32,
    pub room_b: u32,
    /// Whether this door crosses a sector boundary.
    pub cross_sector: bool,
    /// Whether this door crosses a radial layer.
    pub cross_layer: bool,
}

/// Generate doors connecting adjacent rooms in the cylinder.
///
/// Rooms are connected:
/// - Axially: same sector, same layer, adjacent levels
/// - Circumferentially: same level, same layer, adjacent sectors
/// - Radially: same sector, same level, adjacent layers
pub fn generate_cylinder_doors(
    rooms: &[CylinderRoom],
    config: &CylinderConfig,
) -> Vec<CylinderDoor> {
    let mut doors = Vec::new();

    // Index rooms by (sector, level, layer) for fast lookup
    let mut index: std::collections::HashMap<(u32, u32, u32), u32> =
        std::collections::HashMap::new();
    for room in rooms {
        index.insert((room.sector, room.level, room.layer), room.id);
    }

    for room in rooms {
        // Axial neighbor (next level)
        if room.level + 1 < config.level_count {
            if let Some(&neighbor_id) = index.get(&(room.sector, room.level + 1, room.layer)) {
                doors.push(CylinderDoor {
                    room_a: room.id,
                    room_b: neighbor_id,
                    cross_sector: false,
                    cross_layer: false,
                });
            }
        }

        // Circumferential neighbor (next sector, wrapping)
        let next_sector = (room.sector + 1) % config.sector_count;
        if let Some(&neighbor_id) = index.get(&(next_sector, room.level, room.layer)) {
            doors.push(CylinderDoor {
                room_a: room.id,
                room_b: neighbor_id,
                cross_sector: true,
                cross_layer: false,
            });
        }

        // Radial neighbor (next layer)
        if room.layer + 1 < config.radial_layers {
            if let Some(&neighbor_id) = index.get(&(room.sector, room.level, room.layer + 1)) {
                doors.push(CylinderDoor {
                    room_a: room.id,
                    room_b: neighbor_id,
                    cross_sector: false,
                    cross_layer: true,
                });
            }
        }
    }

    doors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = CylinderConfig::default();
        assert_eq!(config.sector_count, 6);
        assert_eq!(config.level_count, 10);
        assert_eq!(config.radial_layers, 3);
        assert!((config.length - 800.0).abs() < f32::EPSILON);
    }

    #[test]
    fn surface_gravity_reasonable() {
        let config = CylinderConfig::default();
        let g = config.surface_gravity();
        // At 200m radius, 1 RPM: ω = 2π/60, g = ω²r/9.81
        // ≈ (0.1047)² × 200 / 9.81 ≈ 0.223g
        assert!(
            g > 0.1 && g < 1.0,
            "gravity {g} should be between 0.1 and 1.0 g"
        );
    }

    #[test]
    fn sector_arc_length() {
        let config = CylinderConfig::default();
        let arc = config.sector_arc_length();
        // 2π × 200 / 6 ≈ 209m
        assert!(arc > 200.0 && arc < 220.0, "arc {arc}");
    }

    #[test]
    fn habitable_area() {
        let config = CylinderConfig::default();
        let area = config.habitable_area();
        // 2π × 200 × 800 ≈ 1,005,310 m²
        assert!(area > 900_000.0 && area < 1_100_000.0, "area {area}");
    }

    #[test]
    fn total_slots() {
        let config = CylinderConfig::default();
        assert_eq!(config.total_slots(), 6 * 10 * 3);
    }

    #[test]
    fn generate_layout_room_count() {
        let config = CylinderConfig::default();
        let rooms = generate_cylinder_layout(&config);
        assert_eq!(rooms.len() as u32, config.total_slots());
    }

    #[test]
    fn all_rooms_have_positive_dimensions() {
        let config = CylinderConfig::default();
        let rooms = generate_cylinder_layout(&config);
        for room in &rooms {
            assert!(room.width > 0.0, "room {} width <= 0", room.id);
            assert!(room.depth > 0.0, "room {} depth <= 0", room.id);
            assert!(room.height > 0.0, "room {} height <= 0", room.id);
        }
    }

    #[test]
    fn corridors_at_surface_every_3_levels() {
        let config = CylinderConfig::default();
        let rooms = generate_cylinder_layout(&config);
        let corridors: Vec<_> = rooms.iter().filter(|r| r.is_corridor).collect();
        assert!(!corridors.is_empty());
        for c in &corridors {
            assert_eq!(c.layer, 0, "corridors should be at layer 0");
            assert_eq!(c.level % 3, 0, "corridors at levels divisible by 3");
        }
    }

    #[test]
    fn sector_purposes_assigned() {
        let purposes = assign_sector_purposes(6);
        assert_eq!(purposes.len(), 6);
        assert_eq!(purposes[0], SectorPurpose::Residential);
        assert_eq!(purposes[1], SectorPurpose::Agricultural);
    }

    #[test]
    fn sector_purposes_wrap() {
        let purposes = assign_sector_purposes(8);
        assert_eq!(purposes.len(), 8);
        // 6 types, so index 6 wraps to 0 (Residential)
        assert_eq!(purposes[6], SectorPurpose::Residential);
    }

    #[test]
    fn effective_gravity_decreases_outward() {
        let config = CylinderConfig::default();
        let rooms = generate_cylinder_layout(&config);
        // Compare layer 0 and layer 2 in same sector/level
        let layer0 = rooms
            .iter()
            .find(|r| r.sector == 0 && r.level == 1 && r.layer == 0)
            .unwrap();
        let layer2 = rooms
            .iter()
            .find(|r| r.sector == 0 && r.level == 1 && r.layer == 2)
            .unwrap();
        // Outer layer has higher radius → higher g (centripetal: g ∝ r)
        assert!(layer2.effective_gravity(&config) > layer0.effective_gravity(&config));
    }

    #[test]
    fn doors_connect_adjacent_rooms() {
        let config = CylinderConfig::default();
        let rooms = generate_cylinder_layout(&config);
        let doors = generate_cylinder_doors(&rooms, &config);
        assert!(!doors.is_empty());

        // Axial: 6 sectors × 9 level-pairs × 3 layers = 162
        // Circumferential: 6 sector-pairs × 10 levels × 3 layers = 180
        // Radial: 6 sectors × 10 levels × 2 layer-pairs = 120
        // Total = 462
        let expected = 6 * 9 * 3 + 6 * 10 * 3 + 6 * 10 * 2;
        assert_eq!(doors.len(), expected, "door count");
    }

    #[test]
    fn cross_sector_doors_exist() {
        let config = CylinderConfig::default();
        let rooms = generate_cylinder_layout(&config);
        let doors = generate_cylinder_doors(&rooms, &config);
        let cross_sector: Vec<_> = doors.iter().filter(|d| d.cross_sector).collect();
        assert!(!cross_sector.is_empty());
        // Should wrap around: sector 5 connects to sector 0
        let wrapping = cross_sector.iter().any(|d| {
            let a = rooms.iter().find(|r| r.id == d.room_a).unwrap();
            let b = rooms.iter().find(|r| r.id == d.room_b).unwrap();
            (a.sector == 5 && b.sector == 0) || (a.sector == 0 && b.sector == 5)
        });
        assert!(wrapping, "sector wrapping should exist");
    }

    #[test]
    fn cross_layer_doors_exist() {
        let config = CylinderConfig::default();
        let rooms = generate_cylinder_layout(&config);
        let doors = generate_cylinder_doors(&rooms, &config);
        assert!(doors.iter().any(|d| d.cross_layer));
    }

    #[test]
    fn room_area_and_volume() {
        let room = CylinderRoom {
            id: 0,
            sector: 0,
            level: 0,
            layer: 0,
            room_type: 0,
            width: 10.0,
            depth: 20.0,
            height: 3.0,
            is_corridor: false,
        };
        assert!((room.area() - 200.0).abs() < f32::EPSILON);
        assert!((room.volume() - 600.0).abs() < f32::EPSILON);
    }

    #[test]
    fn acceptance_generates_walkable_ship() {
        // Can generate a cylindrical ship with rooms and doors
        let config = CylinderConfig::default();
        let rooms = generate_cylinder_layout(&config);
        let doors = generate_cylinder_doors(&rooms, &config);

        // Has rooms
        assert!(rooms.len() > 100);
        // Has doors for connectivity
        assert!(doors.len() > rooms.len());
        // All rooms have valid dimensions
        for room in &rooms {
            assert!(room.area() > 0.0);
        }
        // Multiple room types present
        let types: std::collections::HashSet<u8> = rooms.iter().map(|r| r.room_type).collect();
        assert!(types.len() >= 4, "should have multiple room types");
    }

    #[test]
    fn small_cylinder_works() {
        let config = CylinderConfig {
            length: 100.0,
            inner_radius: 50.0,
            outer_radius: 60.0,
            sector_count: 3,
            level_count: 5,
            radial_layers: 2,
            corridor_width: 2.0,
            rotation_rpm: 2.0,
        };
        let rooms = generate_cylinder_layout(&config);
        assert_eq!(rooms.len() as u32, 3 * 5 * 2);
        let doors = generate_cylinder_doors(&rooms, &config);
        assert!(!doors.is_empty());
    }
}
