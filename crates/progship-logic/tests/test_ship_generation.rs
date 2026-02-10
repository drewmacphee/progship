//! Integration tests for the full ship generation pipeline.
//!
//! Exercises: MissionConfig → SystemSelection → PopulationProfile
//! → FacilityManifest → SupplyManifest → ServiceDecks
//!
//! All tests are pure logic — no SpacetimeDB, no rendering.

use progship_logic::config::{
    select_systems, total_power_draw, total_system_crew, total_system_mass, SystemOverrides,
    SystemSelection,
};
use progship_logic::manifest::{
    estimate_deck_count, generate_manifest, total_area, total_rooms, RoomRequirement,
};
use progship_logic::mission::{
    compute_voyage, Destination, MissionConfig, MissionPriority, PropulsionType,
};
use progship_logic::population::{compute_population, PopulationProfile};
use progship_logic::service_decks::{
    generate_service_deck_rooms, plan_service_decks, total_deck_count, ServiceDeckConfig,
};
use progship_logic::supplies::{compute_supply_manifest, power_balance};

// ── Helpers ────────────────────────────────────────────────────────────

fn default_config() -> MissionConfig {
    MissionConfig {
        destination: Destination::ProximaCentauri as u8,
        colony_target_pop: 5000,
        tech_level: 2,
        budget_class: 2,
        mission_priority: MissionPriority {
            safety: 0.4,
            speed: 0.1,
            comfort: 0.2,
            science: 0.0,
            self_sufficiency: 0.3,
        },
        seed: 42,
        propulsion: PropulsionType::FusionTorch as u8,
    }
}

/// Run the full pipeline and return all intermediate outputs.
fn run_pipeline(
    config: &MissionConfig,
) -> (SystemSelection, PopulationProfile, Vec<RoomRequirement>) {
    let systems = select_systems(config, &SystemOverrides::default());
    let population = compute_population(config, &systems);
    let manifest = generate_manifest(&systems, &population, config.budget_class);
    (systems, population, manifest)
}

// ── Pipeline coherence tests ───────────────────────────────────────────

#[test]
fn pipeline_runs_without_panic() {
    let config = default_config();
    let (systems, population, manifest) = run_pipeline(&config);
    let _supplies = compute_supply_manifest(&config, &systems, &population);

    assert!(population.departure_total > 0);
    assert!(!manifest.is_empty());
}

#[test]
fn deterministic_output() {
    let config = default_config();
    let (sys1, pop1, man1) = run_pipeline(&config);
    let (sys2, pop2, man2) = run_pipeline(&config);

    assert_eq!(sys1.power, sys2.power);
    assert_eq!(sys1.life_support, sys2.life_support);
    assert_eq!(sys1.food, sys2.food);
    assert_eq!(sys1.water, sys2.water);
    assert_eq!(sys1.defense, sys2.defense);
    assert_eq!(sys1.medical, sys2.medical);
    assert_eq!(sys1.gravity, sys2.gravity);
    assert_eq!(pop1.departure_total, pop2.departure_total);
    assert_eq!(pop1.total_crew, pop2.total_crew);
    assert_eq!(man1.len(), man2.len());
}

#[test]
fn different_seeds_produce_variation() {
    // Test that across 50 seeds, we see at least some system variation
    let mut distinct_selections = std::collections::HashSet::new();
    for seed in 0..50 {
        let mut config = default_config();
        config.seed = seed;
        let (sys, _, _) = run_pipeline(&config);
        distinct_selections.insert((
            sys.power,
            sys.life_support,
            sys.food,
            sys.water,
            sys.defense,
            sys.medical,
            sys.gravity,
        ));
    }
    // With 50 seeds, we should see at least 2 distinct system configurations
    assert!(
        distinct_selections.len() >= 2,
        "50 seeds produced only {} distinct system configs",
        distinct_selections.len()
    );
}

// ── Population tests ───────────────────────────────────────────────────

#[test]
fn population_meets_genetic_diversity() {
    let config = default_config();
    let (_, pop, _) = run_pipeline(&config);
    assert!(pop.genetic_diversity_ok);
    assert!(pop.departure_total >= 160);
}

#[test]
fn crew_less_than_total_population() {
    let config = default_config();
    let (_, pop, _) = run_pipeline(&config);
    assert!(
        pop.total_crew < pop.departure_total,
        "Crew ({}) should be less than total population ({})",
        pop.total_crew,
        pop.departure_total
    );
}

#[test]
fn all_departments_have_crew() {
    let config = default_config();
    let (_, pop, _) = run_pipeline(&config);
    let dc = &pop.department_crew;
    assert!(dc.command > 0, "Command has no crew");
    assert!(dc.engineering > 0, "Engineering has no crew");
    assert!(dc.medical > 0, "Medical has no crew");
    assert!(dc.science > 0, "Science has no crew");
    assert!(dc.security > 0, "Security has no crew");
    assert!(dc.operations > 0, "Operations has no crew");
    assert!(dc.civilian > 0, "Civilian has no crew");
    assert_eq!(dc.total(), pop.total_crew);
}

#[test]
fn crew_includes_system_operators() {
    let config = default_config();
    let systems = select_systems(&config, &SystemOverrides::default());
    let system_crew = total_system_crew(&systems);
    let pop = compute_population(&config, &systems);

    assert!(
        pop.department_crew.engineering >= system_crew,
        "Engineering ({}) should include system operators ({})",
        pop.department_crew.engineering,
        system_crew
    );
}

// ── Manifest tests ─────────────────────────────────────────────────────

#[test]
fn manifest_has_essential_room_types() {
    let config = default_config();
    let (_, _, manifest) = run_pipeline(&config);

    let has_room = |name: &str| manifest.iter().any(|r| r.name == name);
    assert!(has_room("Bridge"), "Missing Bridge");
    assert!(has_room("Mess Hall"), "Missing Mess Hall");
    assert!(has_room("Hospital Ward"), "Missing Hospital Ward");
    assert!(has_room("Main Engineering"), "Missing Main Engineering");
}

#[test]
fn manifest_room_counts_positive() {
    let config = default_config();
    let (_, _, manifest) = run_pipeline(&config);

    for room in &manifest {
        assert!(room.count > 0, "Room '{}' has count 0", room.name);
        assert!(room.target_area > 0.0, "Room '{}' has area 0", room.name);
    }
}

#[test]
fn manifest_total_area_reasonable() {
    let config = default_config();
    let (_, pop, manifest) = run_pipeline(&config);

    let area = total_area(&manifest);
    let rooms = total_rooms(&manifest);
    let area_per_person = area / pop.departure_total as f32;

    assert!(
        area_per_person > 5.0,
        "Too little area per person: {:.1} sq-m",
        area_per_person
    );
    assert!(
        area_per_person < 200.0,
        "Too much area per person: {:.1} sq-m",
        area_per_person
    );
    assert!(rooms > 50, "Only {} rooms total", rooms);
}

#[test]
fn manifest_scales_with_population() {
    let mut small = default_config();
    small.colony_target_pop = 500;
    let mut large = default_config();
    large.colony_target_pop = 10_000;

    let (_, _, man_small) = run_pipeline(&small);
    let (_, _, man_large) = run_pipeline(&large);

    assert!(
        total_rooms(&man_large) > total_rooms(&man_small),
        "Larger population should have more rooms"
    );
}

#[test]
fn budget_class_affects_manifest() {
    let mut austere = default_config();
    austere.budget_class = 1;
    let mut premium = default_config();
    premium.budget_class = 3;

    let (_, _, man_austere) = run_pipeline(&austere);
    let (_, _, man_premium) = run_pipeline(&premium);

    assert!(
        total_area(&man_premium) > total_area(&man_austere),
        "Premium should have more area than austere"
    );
}

// ── Deck layout tests ──────────────────────────────────────────────────

#[test]
fn deck_count_reasonable() {
    let config = default_config();
    let (_, _, manifest) = run_pipeline(&config);

    let decks = estimate_deck_count(&manifest, 65.0, 400.0);
    assert!(decks >= 3, "Too few decks: {}", decks);
    assert!(decks <= 50, "Too many decks: {}", decks);
}

#[test]
fn service_decks_placed_correctly() {
    let hab_decks = 15u32;
    let interval = 3u32;

    let positions = plan_service_decks(hab_decks, interval);
    let total = total_deck_count(hab_decks, interval);
    assert_eq!(total, hab_decks + positions.len() as u32);

    for (svc, below, above) in &positions {
        assert!(
            *svc > *below && *svc < *above,
            "Service deck {} not between {} and {}",
            svc,
            below,
            above
        );
    }
}

#[test]
fn service_deck_rooms_within_hull() {
    let cfg = ServiceDeckConfig::default();

    let rooms = generate_service_deck_rooms(&cfg, 0, 1000);
    assert!(!rooms.is_empty());

    for room in &rooms {
        assert!(room.x >= 0.0);
        assert!(room.y >= 0.0);
        // x axis runs along hull length, y axis across hull width
        assert!(
            room.x + room.width <= cfg.hull_length + 0.01,
            "Room '{}' x={} w={} exceeds hull length {}",
            room.name,
            room.x,
            room.width,
            cfg.hull_length
        );
        assert!(
            room.y + room.height <= cfg.hull_width + 0.01,
            "Room '{}' y={} h={} exceeds hull width {}",
            room.name,
            room.y,
            room.height,
            cfg.hull_width
        );
    }
}

// ── Supply manifest tests ──────────────────────────────────────────────

#[test]
fn supply_manifest_non_negative() {
    let config = default_config();
    let (systems, population, _) = run_pipeline(&config);
    let supplies = compute_supply_manifest(&config, &systems, &population);

    assert!(
        supplies.food.stockpile_tons >= 0.0,
        "Negative food stockpile"
    );
    assert!(
        supplies.water.stockpile_tons >= 0.0,
        "Negative water stockpile"
    );
    assert!(
        supplies.oxygen.stockpile_tons >= 0.0,
        "Negative oxygen stockpile"
    );
    assert!(
        supplies.fuel.stockpile_tons >= 0.0,
        "Negative fuel stockpile"
    );
    assert!(
        supplies.spare_parts.stockpile_tons >= 0.0,
        "Negative spares stockpile"
    );
    assert!(
        supplies.medical.stockpile_tons >= 0.0,
        "Negative medical stockpile"
    );
    assert!(supplies.total_supply_mass > 0.0);
}

#[test]
fn supply_mass_budget_positive() {
    let config = default_config();
    let (systems, population, _) = run_pipeline(&config);
    let supplies = compute_supply_manifest(&config, &systems, &population);
    assert!(supplies.propulsion_mass_limit > 0.0);
}

#[test]
fn power_balance_finite() {
    let config = default_config();
    let systems = select_systems(&config, &SystemOverrides::default());
    assert!(power_balance(&systems).is_finite());
}

// ── System selection tests ─────────────────────────────────────────────

#[test]
fn system_mass_and_crew_positive() {
    let config = default_config();
    let systems = select_systems(&config, &SystemOverrides::default());
    assert!(total_system_crew(&systems) > 0);
    assert!(total_system_mass(&systems) > 0.0);
    assert!(total_power_draw(&systems) > 0.0);
}

#[test]
fn overrides_respected() {
    let config = default_config();
    let overrides = SystemOverrides {
        power: Some(0),
        ..Default::default()
    };
    let systems = select_systems(&config, &overrides);
    assert_eq!(systems.power, 0);
}

// ── Voyage profile tests ──────────────────────────────────────────────

#[test]
fn voyage_duration_positive() {
    let config = default_config();
    let voyage = compute_voyage(&config);
    assert!(voyage.duration_years > 0.0);
    assert!(voyage.distance_ly > 0.0);
}

#[test]
fn closer_destination_shorter_voyage() {
    let mut near = default_config();
    near.destination = Destination::ProximaCentauri as u8;
    let mut far = default_config();
    far.destination = Destination::Kepler442b as u8;

    let v_near = compute_voyage(&near);
    let v_far = compute_voyage(&far);
    assert!(v_far.duration_years > v_near.duration_years);
}

// ── Multi-seed stress test ─────────────────────────────────────────────

#[test]
fn multi_seed_pipeline_stable() {
    for seed in 0..20 {
        let mut config = default_config();
        config.seed = seed;
        let (systems, population, manifest) = run_pipeline(&config);
        let _supplies = compute_supply_manifest(&config, &systems, &population);

        assert!(population.departure_total > 0, "seed {}: empty pop", seed);
        assert!(population.genetic_diversity_ok, "seed {}: diversity", seed);
        assert!(!manifest.is_empty(), "seed {}: empty manifest", seed);
        assert!(total_rooms(&manifest) > 30, "seed {}: few rooms", seed);
    }
}
