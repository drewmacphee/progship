//! Interstitial service decks — crawlspace infrastructure between habitable decks.
//!
//! Service decks are generated between every 2–3 habitable decks. They contain:
//! - HVAC nodes (ventilation routing)
//! - Power conduits (distribution nodes)
//! - Water pipes (recycling connections)
//! - Access via ladder shafts from adjacent habitable decks
//!
//! Service decks have 2m ceiling (vs 3m standard), grid-based layout,
//! and are only accessible by maintenance crew.

use serde::{Deserialize, Serialize};

use crate::constants::room_types;

/// A service deck in the ship layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDeck {
    /// Deck index in the overall deck numbering.
    pub deck_index: i32,
    /// The habitable deck above this service deck.
    pub above_deck: i32,
    /// The habitable deck below this service deck.
    pub below_deck: i32,
    /// Infrastructure rooms on this service deck.
    pub rooms: Vec<ServiceRoom>,
    /// Ladder shaft connections to adjacent habitable decks.
    pub shafts: Vec<ShaftConnection>,
}

/// A room on a service deck (crawlspace infrastructure).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRoom {
    pub id: u32,
    pub room_type: u8,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// A ladder shaft connecting a service deck to an adjacent habitable deck.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaftConnection {
    /// Service deck room ID containing the shaft.
    pub service_room_id: u32,
    /// Habitable deck room ID connected to.
    pub habitable_room_id: u32,
    /// Which habitable deck (above or below).
    pub target_deck: i32,
}

/// Configuration for service deck generation.
#[derive(Debug, Clone)]
pub struct ServiceDeckConfig {
    /// Insert a service deck every N habitable decks.
    pub interval: u32,
    /// Hull width in meters.
    pub hull_width: f32,
    /// Hull length in meters.
    pub hull_length: f32,
    /// Service corridor width in meters.
    pub corridor_width: f32,
    /// Grid cell size for infrastructure rooms.
    pub grid_cell_size: f32,
}

impl Default for ServiceDeckConfig {
    fn default() -> Self {
        Self {
            interval: 3,
            hull_width: 65.0,
            hull_length: 400.0,
            corridor_width: 2.0,
            grid_cell_size: 20.0,
        }
    }
}

/// Infrastructure room types on service decks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfraType {
    /// HVAC junction node.
    HvacNode,
    /// Power conduit junction.
    PowerConduit,
    /// Water pipe junction.
    WaterPipe,
    /// Service corridor.
    ServiceCorridor,
}

impl InfraType {
    pub fn room_type(self) -> u8 {
        room_types::SERVICE_DECK
    }

    pub fn name(self) -> &'static str {
        match self {
            InfraType::HvacNode => "HVAC Junction",
            InfraType::PowerConduit => "Power Conduit",
            InfraType::WaterPipe => "Water Pipe Junction",
            InfraType::ServiceCorridor => "Service Corridor",
        }
    }
}

/// Determine which deck indices should be service decks.
///
/// Given N habitable decks (indexed 0..N-1), inserts service decks
/// between every `interval` habitable decks.
///
/// Returns pairs of (service_deck_index, (above_hab_deck, below_hab_deck)).
pub fn plan_service_decks(hab_deck_count: u32, interval: u32) -> Vec<(i32, i32, i32)> {
    if hab_deck_count < 2 || interval == 0 {
        return vec![];
    }

    let mut result = Vec::new();
    // Habitable decks are at indices 0, 1, 2, ...
    // Service decks go between deck groups
    // With interval=3: service deck between deck 2 and 3, between 5 and 6, etc.
    let mut service_offset = 0i32;

    for i in 1..hab_deck_count {
        if i % interval == 0 {
            // Insert service deck between hab deck (i-1) and (i)
            let above_hab = (i as i32 - 1) + service_offset;
            let service_idx = above_hab + 1;
            let below_hab = service_idx + 1;
            result.push((service_idx, above_hab, below_hab));
            service_offset += 1;
        }
    }

    result
}

/// Generate the infrastructure rooms for a single service deck.
pub fn generate_service_deck_rooms(
    config: &ServiceDeckConfig,
    deck_index: i32,
    start_room_id: u32,
) -> Vec<ServiceRoom> {
    let mut rooms = Vec::new();
    let mut id = start_room_id;

    let cols = (config.hull_length / config.grid_cell_size).floor() as u32;
    let rows = (config.hull_width / config.grid_cell_size).floor() as u32;

    // Central service corridor runs the length of the deck
    rooms.push(ServiceRoom {
        id,
        room_type: room_types::SERVICE_CORRIDOR,
        name: format!("Service Corridor D{deck_index}"),
        x: 0.0,
        y: config.hull_width / 2.0 - config.corridor_width / 2.0,
        width: config.hull_length.min(cols as f32 * config.grid_cell_size),
        height: config.corridor_width,
    });
    id += 1;

    // Infrastructure nodes in a grid pattern
    let infra_types = [
        InfraType::HvacNode,
        InfraType::PowerConduit,
        InfraType::WaterPipe,
    ];

    for row in 0..rows.min(3) {
        for col in 0..cols.min(20) {
            let infra = infra_types[(col as usize + row as usize) % infra_types.len()];
            let x = col as f32 * config.grid_cell_size;
            let y = if row == 0 {
                config.hull_width / 2.0 - config.corridor_width / 2.0 - config.grid_cell_size
            } else if row == 1 {
                config.hull_width / 2.0 + config.corridor_width / 2.0
            } else {
                config.hull_width / 2.0 - config.corridor_width / 2.0 - 2.0 * config.grid_cell_size
            };

            // Skip if outside hull bounds
            if y < 0.0 || y + config.grid_cell_size > config.hull_width {
                continue;
            }

            rooms.push(ServiceRoom {
                id,
                room_type: infra.room_type(),
                name: format!("{} D{deck_index}-{row}-{col}", infra.name()),
                x,
                y,
                width: config.grid_cell_size,
                height: config.grid_cell_size,
            });
            id += 1;
        }
    }

    rooms
}

/// Total number of decks (habitable + service) for a ship.
pub fn total_deck_count(hab_decks: u32, interval: u32) -> u32 {
    let service_count = plan_service_decks(hab_decks, interval).len() as u32;
    hab_decks + service_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_service_decks_small_ship() {
        let result = plan_service_decks(2, 3);
        assert!(result.is_empty(), "2 decks shouldn't need service decks");
    }

    #[test]
    fn test_service_deck_placement() {
        let result = plan_service_decks(6, 3);
        assert_eq!(
            result.len(),
            1,
            "6 hab decks with interval 3 = 1 service deck"
        );
        let (svc, above, below) = result[0];
        assert_eq!(above, 2, "service deck should be after hab deck 2");
        assert_eq!(svc, 3, "service deck index");
        assert_eq!(below, 4, "below deck should be shifted");
    }

    #[test]
    fn test_multiple_service_decks() {
        let result = plan_service_decks(9, 3);
        assert_eq!(
            result.len(),
            2,
            "9 hab decks with interval 3 = 2 service decks"
        );
    }

    #[test]
    fn test_service_deck_every_2() {
        let result = plan_service_decks(6, 2);
        assert_eq!(
            result.len(),
            2,
            "6 hab decks with interval 2 = 2 service decks"
        );
    }

    #[test]
    fn test_generate_rooms_nonempty() {
        let config = ServiceDeckConfig::default();
        let rooms = generate_service_deck_rooms(&config, 3, 1000);
        assert!(!rooms.is_empty(), "should generate infrastructure rooms");
        // Should have at least the service corridor
        assert!(rooms
            .iter()
            .any(|r| r.room_type == room_types::SERVICE_CORRIDOR));
    }

    #[test]
    fn test_rooms_within_hull() {
        let config = ServiceDeckConfig::default();
        let rooms = generate_service_deck_rooms(&config, 5, 2000);
        for r in &rooms {
            assert!(r.x >= 0.0, "room {} x={} should be >= 0", r.name, r.x);
            assert!(
                r.x + r.width <= config.hull_length + 0.01,
                "room {} exceeds hull length",
                r.name
            );
            assert!(r.y >= 0.0, "room {} y={} should be >= 0", r.name, r.y);
            assert!(
                r.y + r.height <= config.hull_width + 0.01,
                "room {} exceeds hull width",
                r.name
            );
        }
    }

    #[test]
    fn test_total_deck_count() {
        assert_eq!(total_deck_count(6, 3), 7); // 6 hab + 1 service
        assert_eq!(total_deck_count(9, 3), 11); // 9 hab + 2 service
        assert_eq!(total_deck_count(3, 3), 3); // 3 hab + 0 service (interval not reached)
        assert_eq!(total_deck_count(4, 3), 5); // 4 hab + 1 service
    }

    #[test]
    fn test_unique_room_ids() {
        let config = ServiceDeckConfig::default();
        let rooms = generate_service_deck_rooms(&config, 3, 1000);
        let ids: Vec<u32> = rooms.iter().map(|r| r.id).collect();
        let unique: std::collections::HashSet<u32> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len(), "all room IDs should be unique");
    }

    #[test]
    fn test_infra_type_names() {
        assert_eq!(InfraType::HvacNode.name(), "HVAC Junction");
        assert_eq!(InfraType::PowerConduit.name(), "Power Conduit");
        assert_eq!(InfraType::WaterPipe.name(), "Water Pipe Junction");
    }
}
