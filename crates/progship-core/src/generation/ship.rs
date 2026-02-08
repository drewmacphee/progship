//! Ship generation - creates a realistic connected ship layout

use hecs::World;
use rand::Rng;
use crate::components::{Room, RoomType, RoomConnections, Deck, ShipSystem, SystemType, ResourceFlow, ResourceType};

/// Configuration for ship generation
#[derive(Debug, Clone)]
pub struct ShipConfig {
    pub name: String,
    pub num_decks: u32,
    pub rooms_per_deck: u32,
    pub passenger_capacity: u32,
    pub crew_size: u32,
    /// Ship length in meters (bow to stern)
    pub ship_length: f32,
    /// Ship width in meters (port to starboard)  
    pub ship_width: f32,
}

impl Default for ShipConfig {
    fn default() -> Self {
        Self {
            name: "Colony Ship".to_string(),
            num_decks: 5,
            rooms_per_deck: 10,
            passenger_capacity: 4000,
            crew_size: 1000,
            ship_length: 200.0,
            ship_width: 40.0,
        }
    }
}

/// Check if a point is inside the ship's elliptical hull
fn point_in_hull(x: f32, y: f32, half_length: f32, half_width: f32) -> bool {
    (x * x) / (half_length * half_length) + (y * y) / (half_width * half_width) <= 1.0
}

/// Get the hull width at a given x position
fn hull_width_at_x(x: f32, half_length: f32, half_width: f32) -> f32 {
    if x.abs() >= half_length {
        return 0.0;
    }
    // Ellipse equation: y = b * sqrt(1 - x²/a²)
    let ratio = 1.0 - (x * x) / (half_length * half_length);
    if ratio <= 0.0 {
        0.0
    } else {
        2.0 * half_width * ratio.sqrt()
    }
}

/// Get the hull x-extent at a given y position (returns max absolute x value)
fn hull_x_at_y(y: f32, half_length: f32, half_width: f32) -> f32 {
    if y.abs() >= half_width {
        return 0.0;
    }
    // Ellipse equation: x = a * sqrt(1 - y²/b²)
    let ratio = 1.0 - (y * y) / (half_width * half_width);
    if ratio <= 0.0 {
        0.0
    } else {
        half_length * ratio.sqrt()
    }
}

/// Generate a ship layout in the ECS world
pub fn generate_ship(world: &mut World, config: &ShipConfig, rng: &mut impl Rng) -> ShipLayout {
    let mut layout = ShipLayout::new(&config.name, config.ship_length, config.ship_width);
    
    let half_length = config.ship_length / 2.0;
    let half_width = config.ship_width / 2.0;
    let corridor_width = 3.0;
    
    // Generate each deck
    for deck_idx in 0..config.num_decks {
        let deck_level = deck_idx as i32;
        let deck_name = deck_name_for_level(deck_level, config.num_decks);
        
        let deck_entity = world.spawn((Deck::new(&deck_name, deck_level),));
        layout.decks.push(deck_entity);
        
        // Corridor starts where hull is wide enough
        let corridor_start_x = find_hull_x_for_width(half_length, half_width, corridor_width + 4.0);
        let corridor_length = corridor_start_x * 2.0 - 4.0; // Leave margin at ends
        
        // Create central corridor
        let corridor_entity = create_corridor(
            world,
            &format!("{} Main Corridor", deck_name),
            deck_level,
            corridor_length,
            corridor_width,
            &mut layout,
        );
        let corridor_room_id = layout.rooms.len() as u32 - 1;
        
        // Get room types for this deck
        let room_types = deck_room_distribution(deck_level, config.rooms_per_deck, config.num_decks);
        
        // Filter out corridors
        let actual_rooms: Vec<_> = room_types.iter()
            .filter(|rt| **rt != RoomType::Corridor)
            .collect();
        
        // Split rooms between port (negative y) and starboard (positive y)
        let port_rooms: Vec<_> = actual_rooms.iter().enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, rt)| *rt)
            .collect();
        let starboard_rooms: Vec<_> = actual_rooms.iter().enumerate()
            .filter(|(i, _)| i % 2 == 1)
            .map(|(_, rt)| *rt)
            .collect();
        
        // Place port-side rooms (negative y)
        // Rooms are placed along the corridor (x-axis) with depths extending to hull (y-axis)
        let num_port = port_rooms.len();
        for (slot_idx, room_type) in port_rooms.iter().enumerate() {
            // First, calculate the room depth (toward hull)
            // Use center position to get approximate available depth
            let approx_x = 0.0; // We'll refine this
            let available_width = hull_width_at_x(approx_x, half_length, half_width);
            let max_depth = (available_width - corridor_width) / 2.0 - 0.5;
            
            if max_depth < 4.0 { continue; }
            
            let room_depth = max_depth.min(15.0).max(4.0); // Cap depth at 15m for more rooms
            let room_y = -(corridor_width / 2.0 + room_depth / 2.0);
            let outer_y = room_y - room_depth / 2.0;  // Outer edge (most negative y)
            
            // Find hull x-extent at the outer y position
            let hull_x_extent = hull_x_at_y(outer_y, half_length, half_width);
            
            // Divide the available x-range among rooms on this side
            let usable_x = hull_x_extent * 2.0 - 2.0; // Leave 1m margin on each end
            let slot_width = usable_x / num_port as f32;
            let room_width = (slot_width - 0.5).max(4.0);
            let room_x = -usable_x / 2.0 + slot_width * (slot_idx as f32 + 0.5);
            
            // Verify corners fit
            if !point_in_hull(room_x - room_width/2.0, outer_y, half_length, half_width) ||
               !point_in_hull(room_x + room_width/2.0, outer_y, half_length, half_width) {
                continue;
            }
            
            let room_name = generate_room_name(**room_type, slot_idx * 2);
            let room = Room::new(&room_name, **room_type, room_width, room_depth)
                .with_deck_level(deck_level)
                .with_position(room_x, room_y);
            
            let room_id = layout.rooms.len() as u32;
            let room_entity = world.spawn((room, RoomConnections::new()));
            
            if let Ok(mut conn) = world.get::<&mut RoomConnections>(room_entity) {
                conn.connect(corridor_room_id);
            }
            if let Ok(mut corridor_conn) = world.get::<&mut RoomConnections>(corridor_entity) {
                corridor_conn.connect(room_id);
            }
            
            layout.rooms.push(room_entity);
        }
        
        // Place starboard-side rooms (positive y)
        let num_starboard = starboard_rooms.len();
        for (slot_idx, room_type) in starboard_rooms.iter().enumerate() {
            // First, calculate the room depth (toward hull)
            let approx_x = 0.0;
            let available_width = hull_width_at_x(approx_x, half_length, half_width);
            let max_depth = (available_width - corridor_width) / 2.0 - 0.5;
            
            if max_depth < 4.0 { continue; }
            
            let room_depth = max_depth.min(15.0).max(4.0); // Cap depth at 15m
            let room_y = corridor_width / 2.0 + room_depth / 2.0;
            let outer_y = room_y + room_depth / 2.0;  // Outer edge (most positive y)
            
            // Find hull x-extent at the outer y position
            let hull_x_extent = hull_x_at_y(outer_y, half_length, half_width);
            
            // Divide the available x-range among rooms on this side
            let usable_x = hull_x_extent * 2.0 - 2.0; // Leave 1m margin on each end
            let slot_width = usable_x / num_starboard as f32;
            let room_width = (slot_width - 0.5).max(4.0);
            let room_x = -usable_x / 2.0 + slot_width * (slot_idx as f32 + 0.5);
            
            // Verify corners fit
            if !point_in_hull(room_x - room_width/2.0, outer_y, half_length, half_width) ||
               !point_in_hull(room_x + room_width/2.0, outer_y, half_length, half_width) {
                continue;
            }
            
            let room_name = generate_room_name(**room_type, slot_idx * 2 + 1);
            let room = Room::new(&room_name, **room_type, room_width, room_depth)
                .with_deck_level(deck_level)
                .with_position(room_x, room_y);
            
            let room_id = layout.rooms.len() as u32;
            let room_entity = world.spawn((room, RoomConnections::new()));
            
            if let Ok(mut conn) = world.get::<&mut RoomConnections>(room_entity) {
                conn.connect(corridor_room_id);
            }
            if let Ok(mut corridor_conn) = world.get::<&mut RoomConnections>(corridor_entity) {
                corridor_conn.connect(room_id);
            }
            
            layout.rooms.push(room_entity);
        }
        
        // Add elevator at center of each deck
        let elevator_room = Room::new("Elevator", RoomType::Elevator, 4.0, 4.0)
            .with_deck_level(deck_level)
            .with_position(0.0, 0.0);
        
        let elevator_entity = world.spawn((
            elevator_room,
            RoomConnections::new(),
        ));
        
        // Connect elevator to corridor
        if let Ok(mut conn) = world.get::<&mut RoomConnections>(elevator_entity) {
            conn.connect(corridor_room_id);
        }
        if let Ok(mut corridor_conn) = world.get::<&mut RoomConnections>(corridor_entity) {
            corridor_conn.connect(layout.rooms.len() as u32);
        }
        
        layout.rooms.push(elevator_entity);
        layout.elevators.push(elevator_entity);
    }
    
    // Connect elevators across decks
    for i in 1..layout.elevators.len() {
        let current_elevator = layout.elevators[i];
        let prev_elevator = layout.elevators[i - 1];
        
        // Find their room IDs
        let current_id = layout.rooms.iter().position(|&e| e == current_elevator).unwrap_or(0) as u32;
        let prev_id = layout.rooms.iter().position(|&e| e == prev_elevator).unwrap_or(0) as u32;
        
        if let Ok(mut conn) = world.get::<&mut RoomConnections>(current_elevator) {
            conn.connect(prev_id);
        }
        if let Ok(mut conn) = world.get::<&mut RoomConnections>(prev_elevator) {
            conn.connect(current_id);
        }
    }
    
    // Generate ship systems
    generate_ship_systems(world, &mut layout, config);
    
    layout
}

/// Find the x position where hull width equals target_width
fn find_hull_x_for_width(half_length: f32, half_width: f32, target_width: f32) -> f32 {
    // From ellipse: width = 2 * b * sqrt(1 - x²/a²)
    // Solve for x: x = a * sqrt(1 - (width/2b)²)
    let ratio = target_width / (2.0 * half_width);
    if ratio >= 1.0 {
        return 0.0;
    }
    half_length * (1.0 - ratio * ratio).sqrt()
}

/// Create a central corridor
fn create_corridor(
    world: &mut World,
    name: &str,
    deck_level: i32,
    length: f32,
    width: f32,
    layout: &mut ShipLayout,
) -> hecs::Entity {
    let corridor = Room::new(name, RoomType::Corridor, length, width)
        .with_deck_level(deck_level)
        .with_position(0.0, 0.0);
    
    let entity = world.spawn((
        corridor,
        RoomConnections::new(),
    ));
    
    layout.rooms.push(entity);
    entity
}

/// Result of ship generation
#[derive(Debug)]
pub struct ShipLayout {
    pub name: String,
    pub ship_length: f32,
    pub ship_width: f32,
    pub decks: Vec<hecs::Entity>,
    pub rooms: Vec<hecs::Entity>,
    pub elevators: Vec<hecs::Entity>,
}

impl ShipLayout {
    fn new(name: &str, length: f32, width: f32) -> Self {
        Self {
            name: name.to_string(),
            ship_length: length,
            ship_width: width,
            decks: Vec::new(),
            rooms: Vec::new(),
            elevators: Vec::new(),
        }
    }
}

/// Get deck name for a level
fn deck_name_for_level(level: i32, total_decks: u32) -> String {
    match level {
        0 => "Main Deck".to_string(),
        1 => "Upper Deck".to_string(),
        n if n == (total_decks - 1) as i32 => "Top Deck".to_string(),
        n if n > 0 => format!("Deck {}", n + 1),
        _ => format!("Sublevel {}", -level),
    }
}

/// Determine room distribution for a deck based on its level
fn deck_room_distribution(deck_level: i32, num_rooms: u32, total_decks: u32) -> Vec<RoomType> {
    let mut rooms = Vec::with_capacity(num_rooms as usize);
    
    match deck_level {
        0 => {
            // Main deck: command, common areas
            rooms.push(RoomType::Bridge);
            rooms.push(RoomType::ConferenceRoom);
            rooms.push(RoomType::Mess);
            rooms.push(RoomType::Galley);
            rooms.push(RoomType::Medical);
            rooms.push(RoomType::Recreation);
            for _ in rooms.len()..num_rooms as usize {
                rooms.push(RoomType::QuartersOfficer);
            }
        }
        1 => {
            // Deck 2: crew quarters, recreation
            rooms.push(RoomType::Gym);
            rooms.push(RoomType::Recreation);
            for _ in rooms.len()..num_rooms as usize {
                rooms.push(RoomType::QuartersCrew);
            }
        }
        n if n == (total_decks - 1) as i32 => {
            // Top deck: observation, science
            rooms.push(RoomType::Observatory);
            rooms.push(RoomType::Laboratory);
            rooms.push(RoomType::Laboratory);
            for _ in rooms.len()..num_rooms as usize {
                rooms.push(RoomType::QuartersPassenger);
            }
        }
        n if n > 1 => {
            // Middle decks: passenger quarters
            rooms.push(RoomType::Recreation);
            for _ in rooms.len()..num_rooms as usize {
                rooms.push(RoomType::QuartersPassenger);
            }
        }
        _ => {
            // Engineering deck
            rooms.push(RoomType::Engineering);
            rooms.push(RoomType::ReactorRoom);
            rooms.push(RoomType::LifeSupport);
            for _ in rooms.len()..num_rooms as usize {
                rooms.push(RoomType::Cargo);
            }
        }
    }
    
    rooms.truncate(num_rooms as usize);
    rooms
}

/// Generate a descriptive room name
fn generate_room_name(room_type: RoomType, index: usize) -> String {
    match room_type {
        RoomType::Bridge => "Bridge".to_string(),
        RoomType::Engineering => "Main Engineering".to_string(),
        RoomType::ReactorRoom => "Reactor Room".to_string(),
        RoomType::Medical => "Sickbay".to_string(),
        RoomType::Mess => "Mess Hall".to_string(),
        RoomType::Corridor => format!("Corridor {}", index + 1),
        RoomType::QuartersCrew => format!("Crew Quarters {}", (index / 2) + 1),
        RoomType::QuartersOfficer => format!("Officer Quarters {}", (index / 2) + 1),
        RoomType::QuartersPassenger => format!("Cabin {}", (index / 2) + 1),
        RoomType::Observatory => "Observation Deck".to_string(),
        RoomType::Laboratory => format!("Lab {}", (index / 2) + 1),
        RoomType::Recreation => "Rec Room".to_string(),
        RoomType::Gym => "Gymnasium".to_string(),
        RoomType::Galley => "Galley".to_string(),
        RoomType::ConferenceRoom => "Conference Room".to_string(),
        _ => format!("{:?} {}", room_type, (index / 2) + 1),
    }
}

/// Get typical dimensions for a room type (width along corridor, depth perpendicular)
fn room_dimensions(room_type: RoomType, rng: &mut impl Rng) -> (f32, f32) {
    match room_type {
        // Large command/common rooms
        RoomType::Bridge => (20.0, 15.0),
        RoomType::Engineering => (25.0, 18.0),
        RoomType::ReactorRoom => (15.0, 15.0),
        RoomType::Mess => (25.0, 15.0),
        RoomType::Recreation => (20.0, 12.0),
        RoomType::Gym => (20.0, 15.0),
        RoomType::Observatory => (25.0, 18.0),
        
        // Medium rooms
        RoomType::Medical => (18.0, 12.0),
        RoomType::Laboratory => (15.0, 10.0),
        RoomType::ConferenceRoom => (12.0, 10.0),
        RoomType::Galley => (15.0, 10.0),
        RoomType::Cargo => (20.0, 15.0),
        RoomType::LifeSupport => (15.0, 12.0),
        RoomType::Hydroponics => (20.0, 15.0),
        RoomType::WaterRecycling => (12.0, 10.0),
        RoomType::MaintenanceBay => (15.0, 12.0),
        RoomType::Storage => (12.0, 10.0),
        
        // Quarters - sized for occupants
        RoomType::QuartersCrew => (8.0 + rng.gen_range(0.0..2.0), 6.0),      // 48 sq m, fits 12 people
        RoomType::QuartersOfficer => (10.0 + rng.gen_range(0.0..2.0), 8.0),  // 80 sq m, fits 4 people
        RoomType::QuartersPassenger => (9.0 + rng.gen_range(0.0..2.0), 7.0), // 63 sq m, fits 8 people
        RoomType::Quarters => (8.0, 6.0),
        
        // Utility
        RoomType::Corridor => (3.0, rng.gen_range(10.0..20.0)),
        RoomType::Elevator => (4.0, 4.0),
        RoomType::Airlock => (5.0, 5.0),
    }
}

/// Generate ship systems (power, life support, etc.) with resource flows
fn generate_ship_systems(world: &mut World, layout: &mut ShipLayout, config: &ShipConfig) {
    let population = config.crew_size + config.passenger_capacity;
    
    // Main Reactor - produces power
    let reactor = ShipSystem::new("Main Reactor", SystemType::Power);
    let reactor_flow = ResourceFlow::new()
        .consumes(ResourceType::Fuel, 0.5) // 0.5 units fuel per hour
        .produces(ResourceType::Power, 1000.0); // 1000 kW
    world.spawn((reactor, reactor_flow));
    
    // Backup Reactor
    let backup_reactor = ShipSystem::new("Backup Reactor", SystemType::Power);
    let backup_flow = ResourceFlow::new()
        .consumes(ResourceType::Fuel, 0.2)
        .produces(ResourceType::Power, 400.0);
    world.spawn((backup_reactor, backup_flow));
    
    // Life Support - consumes power, produces oxygen
    let life_support = ShipSystem::new("Primary Life Support", SystemType::LifeSupport);
    let life_flow = ResourceFlow::new()
        .consumes(ResourceType::Power, 200.0)
        .produces(ResourceType::Oxygen, population as f32 * 0.5); // 0.5 units O2 per person per hour
    world.spawn((life_support, life_flow));
    
    // Water Recycling - consumes power, produces water
    let water_system = ShipSystem::new("Water Recycling", SystemType::WaterRecycling);
    let water_flow = ResourceFlow::new()
        .consumes(ResourceType::Power, 100.0)
        .produces(ResourceType::Water, population as f32 * 0.1);
    world.spawn((water_system, water_flow));
    
    // Hydroponics/Food Production - consumes water, power; produces food
    let food_system = ShipSystem::new("Hydroponics Bay", SystemType::FoodProduction);
    let food_flow = ResourceFlow::new()
        .consumes(ResourceType::Power, 150.0)
        .consumes(ResourceType::Water, 50.0)
        .produces(ResourceType::Food, population as f32 * 0.08);
    world.spawn((food_system, food_flow));
    
    // Gravity System - consumes power
    let gravity = ShipSystem::new("Artificial Gravity", SystemType::Gravity);
    let gravity_flow = ResourceFlow::new()
        .consumes(ResourceType::Power, 300.0);
    world.spawn((gravity, gravity_flow));
    
    // Propulsion - consumes fuel and power
    let propulsion = ShipSystem::new("Main Drive", SystemType::Propulsion);
    let propulsion_flow = ResourceFlow::new()
        .consumes(ResourceType::Power, 200.0)
        .consumes(ResourceType::Fuel, 1.0);
    world.spawn((propulsion, propulsion_flow));
    
    // Navigation - consumes power
    let nav = ShipSystem::new("Navigation Computer", SystemType::Navigation);
    let nav_flow = ResourceFlow::new()
        .consumes(ResourceType::Power, 50.0);
    world.spawn((nav, nav_flow));
    
    // Communications - consumes power
    let comms = ShipSystem::new("Subspace Communications", SystemType::Communications);
    let comms_flow = ResourceFlow::new()
        .consumes(ResourceType::Power, 30.0);
    world.spawn((comms, comms_flow));
    
    // Medical Bay - consumes power and water
    let medical = ShipSystem::new("Medical Bay Systems", SystemType::Medical);
    let medical_flow = ResourceFlow::new()
        .consumes(ResourceType::Power, 80.0)
        .consumes(ResourceType::Water, 10.0);
    world.spawn((medical, medical_flow));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ship() {
        let mut world = World::new();
        let config = ShipConfig {
            num_decks: 3,
            rooms_per_deck: 8,
            ship_length: 200.0,
            ship_width: 40.0,
            ..Default::default()
        };
        let mut rng = rand::thread_rng();
        
        let layout = generate_ship(&mut world, &config, &mut rng);
        
        assert_eq!(layout.decks.len(), 3);
        // 3 decks with corridor + elevator each = 6, plus at least some rooms
        assert!(layout.rooms.len() >= 6, "Expected at least 6 rooms, got {}", layout.rooms.len());
        assert_eq!(layout.elevators.len(), 3);
    }

    #[test]
    fn test_rooms_fit_in_hull() {
        let mut world = World::new();
        let config = ShipConfig {
            num_decks: 5,
            rooms_per_deck: 12,
            ship_length: 300.0,
            ship_width: 50.0,
            ..Default::default()
        };
        let mut rng = rand::thread_rng();
        
        let layout = generate_ship(&mut world, &config, &mut rng);
        let half_length = config.ship_length / 2.0;
        let half_width = config.ship_width / 2.0;
        
        for &room_entity in &layout.rooms {
            if let Ok(room) = world.get::<&Room>(room_entity) {
                let (min_x, min_y, max_x, max_y) = room.world_bounds();
                
                // Check all corners are in hull (with small margin for corridor)
                let margin = 0.5;
                assert!(
                    point_in_hull(min_x, min_y.abs(), half_length + margin, half_width + margin),
                    "Room {} corner ({}, {}) outside hull", room.name, min_x, min_y
                );
                assert!(
                    point_in_hull(max_x, max_y.abs(), half_length + margin, half_width + margin),
                    "Room {} corner ({}, {}) outside hull", room.name, max_x, max_y
                );
            }
        }
    }

    #[test]
    fn test_hull_width_calculation() {
        let half_length = 100.0;
        let half_width = 25.0;
        
        // At center, width should be max
        let center_width = hull_width_at_x(0.0, half_length, half_width);
        assert!((center_width - 50.0).abs() < 0.01);
        
        // At ends, width should be 0
        let end_width = hull_width_at_x(100.0, half_length, half_width);
        assert!(end_width < 0.01);
    }
    
    #[test]
    fn test_debug_room_placement() {
        let mut world = World::new();
        let config = ShipConfig {
            num_decks: 1,
            rooms_per_deck: 6,
            ship_length: 200.0,
            ship_width: 40.0,
            ..Default::default()
        };
        let mut rng = rand::thread_rng();
        
        let layout = generate_ship(&mut world, &config, &mut rng);
        
        println!("\n=== Room placement debug ===");
        println!("Ship: {}m x {}m", config.ship_length, config.ship_width);
        
        for &room_entity in &layout.rooms {
            if let Ok(room) = world.get::<&Room>(room_entity) {
                println!(
                    "{:30} pos=({:6.1}, {:6.1}) size=({:5.1} x {:5.1}) type={:?}",
                    room.name, room.world_x, room.world_y, room.width(), room.depth(), room.room_type
                );
            }
        }
        println!("=========================\n");
    }
}
