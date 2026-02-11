//! Needs system - decays needs over time, triggers need-based behavior

use crate::components::{Needs, Person};
use hecs::World;

/// Decay needs over time (needs increase toward 1.0)
pub fn needs_system(world: &mut World, delta_hours: f32) {
    for (_, (_, needs)) in world.query_mut::<(&Person, &mut Needs)>() {
        needs.decay(delta_hours);
    }
}

/// Find people with urgent needs (above threshold)
pub fn find_urgent_needs(
    world: &World,
    threshold: f32,
) -> Vec<(hecs::Entity, crate::components::NeedType)> {
    let mut urgent = Vec::new();

    for (entity, (_, needs)) in world.query::<(&Person, &Needs)>().iter() {
        if let Some(need_type) = needs.most_urgent(threshold) {
            urgent.push((entity, need_type));
        }
    }

    urgent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_decay() {
        let mut world = World::new();

        world.spawn((Person, Needs::default()));

        // After 8 hours, hunger should be at 1.0
        needs_system(&mut world, 8.0);

        for (_, (_, needs)) in world.query::<(&Person, &Needs)>().iter() {
            assert!((needs.hunger - 1.0).abs() < 0.01);
            assert!(needs.fatigue < 1.0); // Not yet exhausted (16 hours)
        }
    }
}
