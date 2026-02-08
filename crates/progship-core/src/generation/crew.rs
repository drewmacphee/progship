//! Crew and passenger generation

use hecs::{World, Entity};
use rand::Rng;
use crate::components::*;
use super::names::generate_name;

/// Generate crew members
pub fn generate_crew(
    world: &mut World,
    count: u32,
    rooms: &[Entity],
    rng: &mut impl Rng,
) -> Vec<Entity> {
    let mut crew_entities = Vec::with_capacity(count as usize);
    
    // Collect room data for positioning
    let room_data: Vec<(u32, f32, f32)> = rooms.iter().enumerate().filter_map(|(idx, &entity)| {
        world.get::<&crate::components::Room>(entity).ok().map(|room| {
            (idx as u32, room.width(), room.depth())
        })
    }).collect();
    
    for i in 0..count {
        // Determine department and rank based on distribution
        let (department, rank) = crew_assignment(i, count, rng);
        let shift = random_shift(rng);
        
        // Generate person data
        let name = generate_name(rng);
        let _age = rng.gen_range(22..60);
        let personality = Personality::random(rng);
        let skills = Skills::random(rng, Some(department.primary_skill()));
        
        // Find appropriate starting room
        let room_id = i % rooms.len() as u32;
        
        // Get room bounds for proper positioning
        let (width, depth) = room_data
            .iter()
            .find(|(id, _, _)| *id == room_id)
            .map(|(_, w, d)| (*w, *d))
            .unwrap_or((10.0, 10.0));
        
        // Position within room bounds (centered, with padding)
        let position = Position::new(
            rng.gen_range(width * 0.1..width * 0.9),
            rng.gen_range(depth * 0.1..depth * 0.9),
            room_id,
        );
        
        let crew_data = Crew::new(department, rank, shift)
            .with_station(room_id);
        
        let entity = world.spawn((
            Person,
            name,
            position,
            Needs::default(),
            personality,
            skills,
            crew_data,
        ));
        
        crew_entities.push(entity);
    }
    
    crew_entities
}

/// Generate passengers
pub fn generate_passengers(
    world: &mut World,
    count: u32,
    rooms: &[Entity],
    rng: &mut impl Rng,
) -> Vec<Entity> {
    let mut passenger_entities = Vec::with_capacity(count as usize);
    
    // Collect room data for positioning
    let room_data: Vec<(u32, f32, f32)> = rooms.iter().enumerate().filter_map(|(idx, &entity)| {
        world.get::<&crate::components::Room>(entity).ok().map(|room| {
            (idx as u32, room.width(), room.depth())
        })
    }).collect();
    
    for i in 0..count {
        // Generate person data
        let name = generate_name(rng);
        let _age = rng.gen_range(5..80);
        let personality = Personality::random(rng);
        let skills = Skills::random(rng, None);
        
        // Determine cabin class based on distribution
        let cabin_class = match rng.gen_range(0..100) {
            0..=5 => CabinClass::First,
            6..=30 => CabinClass::Standard,
            _ => CabinClass::Steerage,
        };
        
        // Find starting room
        let room_id = i % rooms.len() as u32;
        
        // Get room bounds for proper positioning
        let (width, depth) = room_data
            .iter()
            .find(|(id, _, _)| *id == room_id)
            .map(|(_, w, d)| (*w, *d))
            .unwrap_or((10.0, 10.0));
        
        // Position within room bounds (centered, with padding)
        let position = Position::new(
            rng.gen_range(width * 0.1..width * 0.9),
            rng.gen_range(depth * 0.1..depth * 0.9),
            room_id,
        );
        
        let passenger_data = Passenger::new(cabin_class);
        
        let entity = world.spawn((
            Person,
            name,
            position,
            Needs::default(),
            personality,
            skills,
            passenger_data,
        ));
        
        passenger_entities.push(entity);
    }
    
    passenger_entities
}

/// Determine department and rank for crew member
fn crew_assignment(_index: u32, _total: u32, rng: &mut impl Rng) -> (Department, Rank) {
    // Distribution: 
    // Engineering 25%, Operations 25%, Medical 10%, Science 10%, 
    // Security 15%, Command 5%, Civilian 10%
    
    let department = match rng.gen_range(0..100) {
        0..=24 => Department::Engineering,
        25..=49 => Department::Operations,
        50..=59 => Department::Medical,
        60..=69 => Department::Science,
        70..=84 => Department::Security,
        85..=89 => Department::Command,
        _ => Department::Civilian,
    };
    
    // Rank distribution (pyramid)
    let rank = match rng.gen_range(0..100) {
        0..=40 => Rank::Crewman,
        41..=65 => Rank::Specialist,
        66..=80 => Rank::Petty,
        81..=90 => Rank::Chief,
        91..=95 => Rank::Ensign,
        96..=98 => Rank::Lieutenant,
        99 => Rank::Commander,
        _ => Rank::Crewman,
    };
    
    (department, rank)
}

/// Random shift assignment (roughly equal distribution)
fn random_shift(rng: &mut impl Rng) -> Shift {
    match rng.gen_range(0..3) {
        0 => Shift::Alpha,
        1 => Shift::Beta,
        _ => Shift::Gamma,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_crew() {
        let mut world = World::new();
        let mut rng = rand::thread_rng();
        
        // Create some dummy rooms first
        let rooms: Vec<Entity> = (0..5)
            .map(|_| world.spawn(()))
            .collect();
        
        let crew = generate_crew(&mut world, 100, &rooms, &mut rng);
        
        assert_eq!(crew.len(), 100);
        
        // Verify all have required components
        for entity in &crew {
            assert!(world.get::<&Person>(*entity).is_ok());
            assert!(world.get::<&Name>(*entity).is_ok());
            assert!(world.get::<&Crew>(*entity).is_ok());
            assert!(world.get::<&Needs>(*entity).is_ok());
        }
    }

    #[test]
    fn test_generate_passengers() {
        let mut world = World::new();
        let mut rng = rand::thread_rng();
        
        let rooms: Vec<Entity> = (0..5)
            .map(|_| world.spawn(()))
            .collect();
        
        let passengers = generate_passengers(&mut world, 50, &rooms, &mut rng);
        
        assert_eq!(passengers.len(), 50);
        
        for entity in &passengers {
            assert!(world.get::<&Person>(*entity).is_ok());
            assert!(world.get::<&Passenger>(*entity).is_ok());
            // Should NOT have Crew component
            assert!(world.get::<&Crew>(*entity).is_err());
        }
    }
}
