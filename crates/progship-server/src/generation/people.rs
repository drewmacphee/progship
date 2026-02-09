//! Crew and passenger generation with deterministic name assignment.
//!
//! Generates crew members and passengers with skills, needs, and personality traits.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

// Name pools for generation (deterministic, no rand needed)
const GIVEN_NAMES: &[&str] = &[
    "Alex", "Jordan", "Morgan", "Casey", "Riley", "Quinn", "Avery", "Taylor", "Skyler", "Kai",
    "Rowan", "Sage", "River", "Phoenix", "Eden", "Harper", "Blake", "Logan", "Reese", "Cameron",
    "Dakota", "Emery", "Finley", "Hayden", "Jaden", "Kendall", "Lane", "Marley", "Noel", "Parker",
    "Remy", "Shay", "Tatum", "Val", "Wren", "Zion", "Arden", "Bay", "Cedar", "Drew", "Ellis",
    "Flynn", "Grey", "Hollis", "Indigo", "Jules", "Kit", "Lark", "Milan", "Nico", "Oakley",
    "Peyton", "Raven", "Sol", "Teagan", "Uri", "Vesper", "Winter", "Xen", "Yael", "Zephyr", "Ash",
    "Briar", "Cove", "Dune", "Ever", "Fern", "Glen", "Haven", "Ivy", "Jade", "Kestrel", "Linden",
    "Moss", "North", "Onyx", "Pine", "Rain", "Stone", "Thorn",
];

const FAMILY_NAMES: &[&str] = &[
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

pub fn generate_crew(ctx: &ReducerContext, count: u32) {
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

pub fn generate_passengers(ctx: &ReducerContext, count: u32, _deck_count: u32) {
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

        let spawn_room = ctx.db.room().id().find(passenger_room_id);
        let (sx, sy, sw, sh, spawn_rid) = spawn_room
            .map(|r| (r.x, r.y, r.width, r.height, r.id))
            .unwrap_or((0.0, 100.0, 30.0, 50.0, 0));
        let spread_x = ((i % 8) as f32 - 4.0) * 2.0;
        let spread_y = ((i / 8) as f32).rem_euclid(sh - 2.0) - (sh / 2.0 - 1.0);
        ctx.db.position().insert(Position {
            person_id,
            room_id: spawn_rid,
            x: sx + spread_x.clamp(-sw / 2.0 + 0.5, sw / 2.0 - 0.5),
            y: sy + spread_y.clamp(-sh / 2.0 + 0.5, sh / 2.0 - 0.5),
            z: 0.0,
        });

        ctx.db.needs().insert(Needs {
            person_id,
            hunger: 0.2 + (i % 3) as f32 * 0.1,
            fatigue: 0.3 + (i % 5) as f32 * 0.05,
            social: 0.4 + (i % 4) as f32 * 0.1,
            comfort: 0.15 + (i % 5) as f32 * 0.05,
            hygiene: 0.15 + (i % 6) as f32 * 0.03,
            health: 1.0,
            morale: 0.6 + (i % 7) as f32 * 0.05,
        });

        let base = (i as f32 * 0.382_034) % 1.0;
        ctx.db.personality().insert(Personality {
            person_id,
            openness: 0.4 + base * 0.3,
            conscientiousness: 0.3 + ((base * 3.0) % 1.0) * 0.4,
            extraversion: 0.4 + ((base * 5.0) % 1.0) * 0.3,
            agreeableness: 0.5 + ((base * 7.0) % 1.0) * 0.2,
            neuroticism: 0.3 + ((base * 11.0) % 1.0) * 0.2,
        });

        let cabin = match i % 10 {
            0 | 1 => cabin_classes::FIRST,
            2 | 3 | 4 => cabin_classes::STANDARD,
            _ => cabin_classes::STEERAGE,
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
