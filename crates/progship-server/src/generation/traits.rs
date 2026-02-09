//! Generation trait definitions using enums (SpacetimeDB WASM can't use dyn).
//!
//! Defines the pluggable interfaces for ship generation: hull shape,
//! infrastructure layout, room packing, and door placement.
//! Each enum variant wraps configuration for one strategy.

use serde::{Deserialize, Serialize};

// ============================================================================
// HULL SHAPE
// ============================================================================

/// Hull shape strategy — determines deck boundary dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HullShape {
    /// Rectangular hull with bow/stern taper.
    Rectangular(RectangularConfig),
}

/// Configuration for a rectangular hull with taper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RectangularConfig {
    /// Maximum beam (width) in meters at equator decks.
    pub ship_beam: usize,
    /// Maximum length in meters at equator decks.
    pub ship_length: usize,
    /// Beam at bow (top) decks.
    pub bow_beam: usize,
    /// Length at bow (top) decks.
    pub bow_length: usize,
    /// Beam at stern (bottom) decks.
    pub stern_beam: usize,
    /// Length at stern (bottom) decks.
    pub stern_length: usize,
    /// Number of bow taper decks.
    pub bow_taper_decks: u32,
    /// Number of stern taper decks.
    pub stern_taper_decks: u32,
}

impl Default for RectangularConfig {
    fn default() -> Self {
        Self {
            ship_beam: 65,
            ship_length: 400,
            bow_beam: 40,
            bow_length: 200,
            stern_beam: 50,
            stern_length: 300,
            bow_taper_decks: 2,
            stern_taper_decks: 2,
        }
    }
}

impl HullShape {
    /// Get hull width for a given deck.
    pub fn width(&self, deck: u32, deck_count: u32) -> usize {
        match self {
            HullShape::Rectangular(cfg) => {
                if deck < cfg.bow_taper_decks {
                    cfg.bow_beam
                } else if deck >= deck_count.saturating_sub(cfg.stern_taper_decks) {
                    cfg.stern_beam
                } else {
                    cfg.ship_beam
                }
            }
        }
    }

    /// Get hull length for a given deck.
    pub fn length(&self, deck: u32, deck_count: u32) -> usize {
        match self {
            HullShape::Rectangular(cfg) => {
                if deck < cfg.bow_taper_decks {
                    cfg.bow_length
                } else if deck >= deck_count.saturating_sub(cfg.stern_taper_decks) {
                    cfg.stern_length
                } else {
                    cfg.ship_length
                }
            }
        }
    }

    /// Check if a point is inside the hull boundary for a given deck.
    pub fn contains(&self, deck: u32, deck_count: u32, x: usize, y: usize) -> bool {
        x < self.width(deck, deck_count) && y < self.length(deck, deck_count)
    }
}

// ============================================================================
// INFRASTRUCTURE LAYOUT
// ============================================================================

/// Infrastructure layout strategy — how corridors and shafts are stamped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InfraLayout {
    /// Central spine with perpendicular cross corridors.
    Spine(SpineConfig),
}

/// Configuration for spine-based infrastructure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpineConfig {
    /// Width of the main spine corridor in meters.
    pub spine_width: usize,
    /// Width of cross corridors in meters.
    pub cross_width: usize,
    /// Spacing between cross corridors in meters.
    pub cross_spacing: usize,
    /// Width of service corridors in meters.
    pub service_width: usize,
}

impl Default for SpineConfig {
    fn default() -> Self {
        Self {
            spine_width: 3,
            cross_width: 3,
            cross_spacing: 50,
            service_width: 2,
        }
    }
}

// ============================================================================
// ROOM PACKER
// ============================================================================

/// Room packing strategy — how rooms are placed within zones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomPacker {
    /// Squarified treemap for variable-size rooms.
    Treemap(TreemapConfig),
    /// Grid-based packer for uniform small rooms (cabins, cells).
    Grid(GridPackerConfig),
}

/// Configuration for treemap packing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreemapConfig {
    /// Maximum area cap as multiple of target area (e.g., 1.5 = 150%).
    pub cap_factor: f32,
    /// Minimum room dimension in meters.
    pub min_dim: usize,
}

impl Default for TreemapConfig {
    fn default() -> Self {
        Self {
            cap_factor: 1.5,
            min_dim: 3,
        }
    }
}

/// Configuration for grid-based packing (fixed-size rooms).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridPackerConfig {
    /// Room width in meters.
    pub room_width: usize,
    /// Room height in meters.
    pub room_height: usize,
    /// Gap between rooms in meters (for walls).
    pub gap: usize,
}

impl Default for GridPackerConfig {
    fn default() -> Self {
        Self {
            room_width: 4,
            room_height: 4,
            gap: 0,
        }
    }
}

// ============================================================================
// DOOR PLACER
// ============================================================================

/// Door placement strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DoorPlacer {
    /// Grid-aligned door placement on shared walls.
    GridAligned(GridDoorConfig),
}

/// Configuration for grid-aligned door placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridDoorConfig {
    /// Door width in meters.
    pub door_width: f32,
}

impl Default for GridDoorConfig {
    fn default() -> Self {
        Self { door_width: 1.5 }
    }
}

// ============================================================================
// SHIP GENERATION CONFIG (combines all strategies)
// ============================================================================

/// Complete ship generation configuration — all strategy selections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipGenConfig {
    pub hull: HullShape,
    pub infrastructure: InfraLayout,
    pub room_packer: RoomPacker,
    pub door_placer: DoorPlacer,
}

impl Default for ShipGenConfig {
    fn default() -> Self {
        Self {
            hull: HullShape::Rectangular(RectangularConfig::default()),
            infrastructure: InfraLayout::Spine(SpineConfig::default()),
            room_packer: RoomPacker::Treemap(TreemapConfig::default()),
            door_placer: DoorPlacer::GridAligned(GridDoorConfig::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rectangular_hull_width() {
        let hull = HullShape::Rectangular(RectangularConfig::default());
        assert_eq!(hull.width(0, 20), 40); // Bow taper
        assert_eq!(hull.width(1, 20), 40); // Bow taper
        assert_eq!(hull.width(10, 20), 65); // Equator
        assert_eq!(hull.width(18, 20), 50); // Stern taper
        assert_eq!(hull.width(19, 20), 50); // Stern taper
    }

    #[test]
    fn test_rectangular_hull_length() {
        let hull = HullShape::Rectangular(RectangularConfig::default());
        assert_eq!(hull.length(0, 20), 200); // Bow taper
        assert_eq!(hull.length(10, 20), 400); // Equator
        assert_eq!(hull.length(19, 20), 300); // Stern taper
    }

    #[test]
    fn test_hull_contains() {
        let hull = HullShape::Rectangular(RectangularConfig::default());
        assert!(hull.contains(10, 20, 30, 200)); // Inside equator
        assert!(!hull.contains(10, 20, 65, 200)); // Outside width
        assert!(!hull.contains(10, 20, 30, 400)); // Outside length
        assert!(hull.contains(0, 20, 39, 199)); // Inside bow
        assert!(!hull.contains(0, 20, 40, 0)); // Outside bow
    }

    #[test]
    fn test_default_ship_gen_config() {
        let config = ShipGenConfig::default();
        match &config.hull {
            HullShape::Rectangular(cfg) => assert_eq!(cfg.ship_beam, 65),
        }
        match &config.room_packer {
            RoomPacker::Treemap(cfg) => assert!((cfg.cap_factor - 1.5).abs() < 0.01),
            RoomPacker::Grid(_) => panic!("Expected treemap"),
        }
    }

    #[test]
    fn test_custom_hull_shape() {
        let hull = HullShape::Rectangular(RectangularConfig {
            ship_beam: 100,
            ship_length: 600,
            bow_beam: 60,
            bow_length: 300,
            stern_beam: 70,
            stern_length: 400,
            bow_taper_decks: 3,
            stern_taper_decks: 3,
        });
        assert_eq!(hull.width(0, 20), 60); // Custom bow
        assert_eq!(hull.width(2, 20), 60); // Still bow (3 taper decks)
        assert_eq!(hull.width(3, 20), 100); // Equator
        assert_eq!(hull.width(17, 20), 70); // Custom stern
        assert_eq!(hull.length(10, 20), 600);
    }

    #[test]
    fn test_hull_matches_legacy() {
        // Verify default config matches the old hull.rs hardcoded values
        let hull = HullShape::Rectangular(RectangularConfig::default());
        let deck_count = 20;
        for deck in 0..deck_count {
            let old_w = crate::generation::hull::hull_width(deck, deck_count, 65);
            let new_w = hull.width(deck, deck_count);
            assert_eq!(old_w, new_w, "Width mismatch at deck {deck}");

            let old_l = crate::generation::hull::hull_length(deck, deck_count, 400);
            let new_l = hull.length(deck, deck_count);
            assert_eq!(old_l, new_l, "Length mismatch at deck {deck}");
        }
    }
}
