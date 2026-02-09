//! Crew and passenger generation with name pools and RNG utilities.
//!
//! Generates crew members with departments/ranks/skills and passengers with
//! cabin classes/professions. Uses deterministic name distribution.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

// Name pools for generation (deterministic, no rand needed)
pub(super) const GIVEN_NAMES: &[&str] = &[
    "Alex", "Jordan", "Morgan", "Casey", "Riley", "Quinn", "Avery", "Taylor", "Skyler", "Kai",
    "Rowan", "Sage", "River", "Phoenix", "Eden", "Harper", "Blake", "Logan", "Reese", "Cameron",
    "Dakota", "Emery", "Finley", "Hayden", "Jaden", "Kendall", "Lane", "Marley", "Noel", "Parker",
    "Remy", "Shay", "Tatum", "Val", "Wren", "Zion", "Arden", "Bay", "Cedar", "Drew", "Ellis",
    "Flynn", "Grey", "Hollis", "Indigo", "Jules", "Kit", "Lark", "Milan", "Nico", "Oakley",
    "Peyton", "Raven", "Sol", "Teagan", "Uri", "Vesper", "Winter", "Xen", "Yael", "Zephyr", "Ash",
    "Briar", "Cove", "Dune", "Ever", "Fern", "Glen", "Haven", "Ivy", "Jade", "Kestrel", "Linden",
    "Moss", "North", "Onyx", "Pine", "Rain", "Stone", "Thorn",
];

pub(super) const FAMILY_NAMES: &[&str] = &[
    "Chen",
    "Nakamura",
    "Petrov",
    "Santos",
    "Kim",
    "Hansen",
    "Okafor",
    "Moreau",
    "Singh",
    "Torres",
    "Andersen",
    "Park",
    "Johansson",
    "Fernandez",
    "Larsson",
    "Novak",
    "Ibrahim",
    "Costa",
    "Yamamoto",
    "Kowalski",
    "Bakker",
    "Tanaka",
    "Müller",
    "Svensson",
    "Rossi",
    "Fischer",
    "Jansen",
    "Dubois",
    "Schmidt",
    "Popov",
    "Mendez",
    "Nguyen",
    "Ali",
    "Jensen",
    "Virtanen",
    "Colombo",
    "Takahashi",
    "Olsen",
    "Nieminen",
    "Bianchi",
    "Wagner",
    "Eriksson",
    "Morel",
    "Ivanov",
    "Ortiz",
    "Reyes",
    "Hoffmann",
    "Nilsson",
    "Russo",
    "Delgado",
    "Berger",
    "Wolf",
    "Richter",
    "Stein",
    "Hahn",
    "Krause",
    "Bauer",
    "Maier",
    "Vogt",
    "Sato",
    "Watanabe",
    "Suzuki",
    "Kato",
    "Yoshida",
    "Yamada",
    "Sasaki",
    "Hayashi",
    "Mori",
    "Ikeda",
    "Abe",
    "Ishikawa",
    "Ogawa",
    "Goto",
    "Hasegawa",
];

pub(super) struct SimpleRng {
    state: u64,
}
impl SimpleRng {
    pub fn from_name(name: &str) -> Self {
        let mut hash: u64 = 5381;
        for b in name.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(b as u64);
        }
        Self { state: hash }
    }
    pub fn next_f32(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.state >> 33) as f32) / (u32::MAX as f32)
    }
    #[allow(dead_code)]
    pub fn next_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
    #[allow(dead_code)]
    pub fn next_usize(&mut self, min: usize, max: usize) -> usize {
        if max <= min {
            return min;
        }
        let f = self.next_f32();
        let range = max - min;
        min + (f * range as f32) as usize
    }
}

pub(super) fn generate_crew(ctx: &ReducerContext, count: u32) {
    let dept_cycle = [
        departments::ENGINEERING,
        departments::MEDICAL,
        departments::SCIENCE,
        departments::SECURITY,
        departments::OPERATIONS,
        departments::COMMAND,
    ];

    for i in 0..count {
        let given_idx = i as usize % GIVEN_NAMES.len();
        let family_idx = (i as usize / GIVEN_NAMES.len() + i as usize * 7) % FAMILY_NAMES.len();

        let person_id = ctx
            .db
            .person()
            .insert(Person {
                id: 0,
                given_name: GIVEN_NAMES[given_idx].to_string(),
                family_name: FAMILY_NAMES[family_idx].to_string(),
                is_crew: true,
                is_player: false,
                owner_identity: None,
            })
            .id;

        let dept = dept_cycle[i as usize % dept_cycle.len()];
        let rank = if i < 3 {
            ranks::LIEUTENANT
        } else if i < 10 {
            ranks::SPECIALIST
        } else {
            ranks::CREWMAN
        };
        let shift = (i % 3) as u8;

        // Assign duty station based on department
        let duty_room_type = match dept {
            departments::ENGINEERING => room_types::ENGINEERING,
            departments::MEDICAL => room_types::HOSPITAL_WARD,
            departments::SCIENCE => room_types::LABORATORY,
            departments::SECURITY => room_types::SECURITY_OFFICE,
            departments::COMMAND => room_types::BRIDGE,
            _ => room_types::CORRIDOR,
        };
        let duty_station_id = ctx
            .db
            .room()
            .iter()
            .find(|r| r.room_type == duty_room_type)
            .map(|r| r.id)
            .unwrap_or(0);

        // Place crew in their duty station room
        let spawn_room = ctx
            .db
            .room()
            .id()
            .find(duty_station_id)
            .or_else(|| ctx.db.room().id().find(0));
        let (sx, sy, sw, sh, spawn_rid) = spawn_room
            .map(|r| (r.x, r.y, r.width, r.height, r.id))
            .unwrap_or((0.0, 0.0, 6.0, 50.0, 0));
        let spread_x = (i % 2) as f32 * 2.0 - 1.0;
        let spread_y = (i as f32 / 2.0).rem_euclid(sh - 2.0) - (sh / 2.0 - 1.0);
        ctx.db.position().insert(Position {
            person_id,
            room_id: spawn_rid,
            x: sx + spread_x.clamp(-sw / 2.0 + 0.5, sw / 2.0 - 0.5),
            y: sy + spread_y.clamp(-sh / 2.0 + 0.5, sh / 2.0 - 0.5),
            z: 0.0,
        });

        ctx.db.needs().insert(Needs {
            person_id,
            hunger: 0.15 + (i % 5) as f32 * 0.05,
            fatigue: 0.2 + (i % 4) as f32 * 0.05,
            social: 0.3 + (i % 3) as f32 * 0.1,
            comfort: 0.1 + (i % 6) as f32 * 0.03,
            hygiene: 0.1 + (i % 7) as f32 * 0.02,
            health: 1.0,
            morale: 0.7 + (i % 5) as f32 * 0.05,
        });

        let base = (i as f32 * 0.618_034) % 1.0;
        ctx.db.personality().insert(Personality {
            person_id,
            openness: 0.3 + base * 0.4,
            conscientiousness: 0.4 + ((base * 3.0) % 1.0) * 0.3,
            extraversion: 0.3 + ((base * 5.0) % 1.0) * 0.4,
            agreeableness: 0.4 + ((base * 7.0) % 1.0) * 0.3,
            neuroticism: 0.2 + ((base * 11.0) % 1.0) * 0.3,
        });

        ctx.db.crew().insert(Crew {
            person_id,
            department: dept,
            rank,
            shift,
            duty_station_id,
            on_duty: shift == shifts::ALPHA,
        });

        let (eng, med, pilot, sci, soc, combat) = match dept {
            departments::ENGINEERING => (0.7, 0.1, 0.2, 0.3, 0.2, 0.1),
            departments::MEDICAL => (0.1, 0.8, 0.1, 0.4, 0.5, 0.1),
            departments::SCIENCE => (0.3, 0.2, 0.1, 0.8, 0.3, 0.1),
            departments::SECURITY => (0.2, 0.2, 0.2, 0.1, 0.3, 0.8),
            departments::COMMAND => (0.3, 0.2, 0.5, 0.3, 0.6, 0.3),
            _ => (0.3, 0.2, 0.2, 0.2, 0.3, 0.2),
        };
        ctx.db.skills().insert(Skills {
            person_id,
            engineering: eng,
            medical: med,
            piloting: pilot,
            science: sci,
            social: soc,
            combat,
        });

        ctx.db.activity().insert(Activity {
            person_id,
            activity_type: activity_types::IDLE,
            started_at: 0.0,
            duration: 0.5,
            target_room_id: None,
        });
    }
}

pub(super) fn generate_passengers(ctx: &ReducerContext, count: u32, _deck_count: u32) {
    let professions = [
        "Colonist",
        "Scientist",
        "Engineer",
        "Teacher",
        "Doctor",
        "Artist",
        "Farmer",
        "Merchant",
        "Writer",
        "Architect",
    ];

    // Find passenger quarters room
    let passenger_room_id = ctx
        .db
        .room()
        .iter()
        .find(|r| r.room_type == room_types::QUARTERS_PASSENGER)
        .map(|r| r.id)
        .unwrap_or(0);

    for i in 0..count {
        let given_idx = (i as usize + 40) % GIVEN_NAMES.len();
        let family_idx = (i as usize * 13 + 5) % FAMILY_NAMES.len();

        let person_id = ctx
            .db
            .person()
            .insert(Person {
                id: 0,
                given_name: GIVEN_NAMES[given_idx].to_string(),
                family_name: FAMILY_NAMES[family_idx].to_string(),
                is_crew: false,
                is_player: false,
                owner_identity: None,
            })
            .id;

        let (rx, ry, rw, rh) = ctx
            .db
            .room()
            .id()
            .find(passenger_room_id)
            .map(|r| (r.x, r.y, r.width, r.height))
            .unwrap_or((0.0, 0.0, 24.0, 18.0));
        let spread_x = ((i as f32 * 1.7) % (rw - 2.0)) - (rw / 2.0 - 1.0);
        let spread_y = ((i as f32 * 2.3) % (rh - 2.0)) - (rh / 2.0 - 1.0);
        ctx.db.position().insert(Position {
            person_id,
            room_id: passenger_room_id,
            x: rx + spread_x,
            y: ry + spread_y,
            z: 0.0,
        });

        ctx.db.needs().insert(Needs {
            person_id,
            hunger: 0.2 + (i % 4) as f32 * 0.05,
            fatigue: 0.15 + (i % 5) as f32 * 0.04,
            social: 0.4 + (i % 3) as f32 * 0.1,
            comfort: 0.2 + (i % 6) as f32 * 0.03,
            hygiene: 0.15 + (i % 7) as f32 * 0.02,
            health: 1.0,
            morale: 0.7 + (i % 4) as f32 * 0.06,
        });

        let base = ((i + 40) as f32 * 0.618_034) % 1.0;
        ctx.db.personality().insert(Personality {
            person_id,
            openness: 0.4 + base * 0.3,
            conscientiousness: 0.3 + ((base * 3.0) % 1.0) * 0.4,
            extraversion: 0.4 + ((base * 5.0) % 1.0) * 0.3,
            agreeableness: 0.5 + ((base * 7.0) % 1.0) * 0.2,
            neuroticism: 0.2 + ((base * 11.0) % 1.0) * 0.4,
        });

        let cabin = if i < count / 10 {
            cabin_classes::FIRST
        } else if i < count / 2 {
            cabin_classes::STANDARD
        } else {
            cabin_classes::STEERAGE
        };

        ctx.db.passenger().insert(Passenger {
            person_id,
            cabin_class: cabin,
            destination: "Kepler-442b".to_string(),
            profession: professions[i as usize % professions.len()].to_string(),
        });

        ctx.db.skills().insert(Skills {
            person_id,
            engineering: 0.1 + ((i as f32 * 0.3) % 0.3),
            medical: 0.1 + ((i as f32 * 0.2) % 0.2),
            piloting: 0.05,
            science: 0.2 + ((i as f32 * 0.25) % 0.3),
            social: 0.3 + ((i as f32 * 0.15) % 0.3),
            combat: 0.05,
        });

        ctx.db.activity().insert(Activity {
            person_id,
            activity_type: activity_types::IDLE,
            started_at: 0.0,
            duration: 0.5,
            target_room_id: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_pools_not_empty() {
        assert!(
            !GIVEN_NAMES.is_empty(),
            "Given names pool should not be empty"
        );
        assert!(
            !FAMILY_NAMES.is_empty(),
            "Family names pool should not be empty"
        );
        assert!(GIVEN_NAMES.len() >= 20, "Should have diverse given names");
        assert!(FAMILY_NAMES.len() >= 20, "Should have diverse family names");
    }

    #[test]
    fn test_names_are_valid() {
        for name in GIVEN_NAMES {
            assert!(!name.is_empty(), "Given names should not be empty");
            assert!(
                name.len() >= 2,
                "Given name '{}' should be at least 2 characters",
                name
            );
        }

        for name in FAMILY_NAMES {
            assert!(!name.is_empty(), "Family names should not be empty");
            assert!(
                name.len() >= 2,
                "Family name '{}' should be at least 2 characters",
                name
            );
        }
    }

    #[test]
    fn test_simple_rng_deterministic() {
        let mut rng1 = SimpleRng::from_name("TestPerson");
        let mut rng2 = SimpleRng::from_name("TestPerson");

        // Same seed should produce same sequence
        assert_eq!(rng1.next_f32(), rng2.next_f32());
        assert_eq!(rng1.next_f32(), rng2.next_f32());
        assert_eq!(rng1.next_f32(), rng2.next_f32());
    }

    #[test]
    fn test_simple_rng_different_seeds() {
        let mut rng1 = SimpleRng::from_name("Alice");
        let mut rng2 = SimpleRng::from_name("Bob");

        // Different seeds should produce different values
        let val1 = rng1.next_f32();
        let val2 = rng2.next_f32();
        assert_ne!(
            val1, val2,
            "Different seeds should produce different values"
        );
    }

    #[test]
    fn test_simple_rng_range() {
        let mut rng = SimpleRng::from_name("Test");

        for _ in 0..50 {
            let val = rng.next_f32();
            assert!(
                val >= 0.0 && val <= 1.0,
                "RNG value {} should be in [0, 1]",
                val
            );
        }
    }

    #[test]
    fn test_simple_rng_next_range() {
        let mut rng = SimpleRng::from_name("RangeTest");

        for _ in 0..50 {
            let val = rng.next_range(10.0, 20.0);
            assert!(
                val >= 10.0 && val <= 20.0,
                "Value {} should be in [10, 20]",
                val
            );
        }
    }

    #[test]
    fn test_simple_rng_next_usize() {
        let mut rng = SimpleRng::from_name("UsizeTest");

        for _ in 0..50 {
            let val = rng.next_usize(5, 15);
            assert!(val >= 5 && val < 15, "Value {} should be in [5, 15)", val);
        }
    }

    #[test]
    fn test_simple_rng_next_usize_edge_cases() {
        let mut rng = SimpleRng::from_name("EdgeTest");

        // When min == max, should return min
        let val = rng.next_usize(10, 10);
        assert_eq!(val, 10);

        // When max < min, should return min
        let val = rng.next_usize(10, 5);
        assert_eq!(val, 10);
    }

    #[test]
    fn test_name_generation_uniqueness() {
        // Generate several names and check for some diversity
        let mut names = std::collections::HashSet::new();

        for i in 0..100 {
            let given_idx = i % GIVEN_NAMES.len();
            let family_idx = (i / GIVEN_NAMES.len() + i * 7) % FAMILY_NAMES.len();
            let full_name = format!("{} {}", GIVEN_NAMES[given_idx], FAMILY_NAMES[family_idx]);
            names.insert(full_name);
        }

        // Should have good variety (at least 80% unique in first 100)
        assert!(
            names.len() >= 80,
            "Should generate diverse names, got {} unique out of 100",
            names.len()
        );
    }
}
