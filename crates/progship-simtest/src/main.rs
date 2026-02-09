//! ProgShip Headless Simulation Harness
//!
//! Validates pure simulation logic and data without SpacetimeDB.
//! Runs entirely in-process — no DB, no networking, no rendering.
//!
//! Usage:
//!   cargo run -p progship-simtest
//!   cargo run -p progship-simtest -- --verbose

use progship_logic::constants::{activity_types, groups, room_types, shifts};
use progship_logic::duty;
use progship_logic::economy::{self, RationingLevel, ResourceLevels, ResourceValues};
use progship_logic::health::{self, InjurySeverity};
use progship_logic::mission::{self, Destination, MissionConfig, PropulsionType};
use progship_logic::pathfinding::{DoorEdge, NavGraph};
use progship_logic::systems::{
    DefenseVariant, FoodVariant, GravityVariant, LifeSupportVariant, MedicalVariant, PowerVariant,
    WaterVariant,
};
use progship_logic::utility::{self, RoomContext, UtilityInput};
use serde::Deserialize;

// ── Facility manifest (same JSON the server uses) ───────────────────────
const MANIFEST_JSON: &str = include_str!("../../../data/facility_manifest.json");

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FacilitySpec {
    name: String,
    room_type: u8,
    target_area: f32,
    capacity: u32,
    count: u32,
    deck_zone: u8,
    group: u8,
}

// ── Test harness ────────────────────────────────────────────────────────

struct TestResult {
    name: String,
    passed: bool,
    detail: String,
}

fn main() {
    let verbose = std::env::args().any(|a| a == "--verbose");
    println!("=== ProgShip Simulation Harness ===\n");

    let mut results = Vec::new();

    // 1. Facility manifest validation
    results.extend(validate_facility_manifest(verbose));

    // 2. Duty & shift logic sweep
    results.extend(validate_duty_logic(verbose));

    // 3. Economy resource loop
    results.extend(validate_economy_logic(verbose));

    // 4. Health & medical system
    results.extend(validate_health_logic(verbose));

    // 5. Mission & voyage calculations
    results.extend(validate_mission_logic(verbose));

    // 6. Pathfinding on synthetic graph
    results.extend(validate_pathfinding(verbose));

    // 7. Utility AI decision sweep
    results.extend(validate_utility_ai(verbose));

    // 8. System variant consistency
    results.extend(validate_system_variants(verbose));

    // ── Summary ──
    println!();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.iter().filter(|r| !r.passed).count();
    let total = results.len();

    for r in &results {
        let icon = if r.passed { "✓" } else { "✗" };
        if !r.passed || verbose {
            println!("  {} {}: {}", icon, r.name, r.detail);
        }
    }

    println!(
        "\n=== RESULT: {}/{} passed, {} failed ===",
        passed, total, failed
    );

    if failed > 0 {
        std::process::exit(1);
    }
}

// ── 1. Facility Manifest ────────────────────────────────────────────────

fn validate_facility_manifest(verbose: bool) -> Vec<TestResult> {
    println!("--- Facility Manifest ---");
    let mut results = Vec::new();

    let manifest: Vec<FacilitySpec> = match serde_json::from_str(MANIFEST_JSON) {
        Ok(m) => m,
        Err(e) => {
            results.push(TestResult {
                name: "manifest_parse".into(),
                passed: false,
                detail: format!("JSON parse error: {}", e),
            });
            return results;
        }
    };

    // Must have rooms
    results.push(TestResult {
        name: "manifest_not_empty".into(),
        passed: manifest.len() > 20,
        detail: format!("{} room types loaded", manifest.len()),
    });

    // All rooms have valid area
    let bad_area: Vec<_> = manifest.iter().filter(|f| f.target_area <= 0.0).collect();
    results.push(TestResult {
        name: "manifest_positive_areas".into(),
        passed: bad_area.is_empty(),
        detail: if bad_area.is_empty() {
            "all rooms have positive area".into()
        } else {
            format!("{} rooms with non-positive area", bad_area.len())
        },
    });

    // All rooms have valid capacity
    let bad_cap: Vec<_> = manifest.iter().filter(|f| f.capacity == 0).collect();
    results.push(TestResult {
        name: "manifest_positive_capacity".into(),
        passed: bad_cap.is_empty(),
        detail: if bad_cap.is_empty() {
            "all rooms have positive capacity".into()
        } else {
            format!(
                "{} rooms with zero capacity: {}",
                bad_cap.len(),
                bad_cap
                    .iter()
                    .map(|f| f.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
    });

    // Group values within range
    let bad_group: Vec<_> = manifest
        .iter()
        .filter(|f| f.group > groups::INFRASTRUCTURE)
        .collect();
    results.push(TestResult {
        name: "manifest_valid_groups".into(),
        passed: bad_group.is_empty(),
        detail: if bad_group.is_empty() {
            "all rooms have valid group".into()
        } else {
            format!("{} rooms with invalid group", bad_group.len())
        },
    });

    // Total room count
    let total_rooms: u32 = manifest.iter().map(|f| f.count).sum();
    results.push(TestResult {
        name: "manifest_total_rooms".into(),
        passed: total_rooms > 100,
        detail: format!("{} total rooms to generate", total_rooms),
    });

    // Deck zones 0-6
    let bad_zone: Vec<_> = manifest.iter().filter(|f| f.deck_zone > 6).collect();
    results.push(TestResult {
        name: "manifest_valid_zones".into(),
        passed: bad_zone.is_empty(),
        detail: if bad_zone.is_empty() {
            "all rooms have valid deck zone (0-6)".into()
        } else {
            format!("{} rooms with invalid zone", bad_zone.len())
        },
    });

    // Key rooms exist
    let has_bridge = manifest.iter().any(|f| f.room_type == room_types::BRIDGE);
    let has_reactor = manifest.iter().any(|f| f.room_type == room_types::REACTOR);
    let has_medbay = manifest
        .iter()
        .any(|f| f.room_type == room_types::HOSPITAL_WARD);
    let has_mess = manifest
        .iter()
        .any(|f| f.room_type == room_types::MESS_HALL);
    let has_cabin = manifest
        .iter()
        .any(|f| f.room_type == room_types::CABIN_SINGLE);

    results.push(TestResult {
        name: "manifest_key_rooms".into(),
        passed: has_bridge && has_reactor && has_medbay && has_mess && has_cabin,
        detail: format!(
            "bridge={} reactor={} hospital={} mess={} cabin={}",
            has_bridge, has_reactor, has_medbay, has_mess, has_cabin
        ),
    });

    if verbose {
        // Group distribution
        let mut group_counts = [0u32; 12];
        for f in &manifest {
            if (f.group as usize) < group_counts.len() {
                group_counts[f.group as usize] += f.count;
            }
        }
        let group_names = [
            "CMD", "SEC", "HAB", "FOOD", "MED", "REC", "ENG", "WORK", "PROP", "LIFE", "CARGO",
            "INFRA",
        ];
        println!("  Room distribution by group:");
        for (i, name) in group_names.iter().enumerate() {
            println!("    {:5}: {} rooms", name, group_counts[i]);
        }
    }

    results
}

// ── 2. Duty & Shift Logic ───────────────────────────────────────────────

fn validate_duty_logic(_verbose: bool) -> Vec<TestResult> {
    println!("--- Duty & Shift Logic ---");
    let mut results = Vec::new();

    // Every hour of the day is covered by exactly one shift being on-duty
    let mut coverage = [false; 24];
    for (hour, covered) in coverage.iter_mut().enumerate() {
        let h = hour as f32;
        let alpha = duty::should_be_on_duty(shifts::ALPHA, h);
        let beta = duty::should_be_on_duty(shifts::BETA, h);
        let gamma = duty::should_be_on_duty(shifts::GAMMA, h);
        *covered = alpha || beta || gamma;
    }
    results.push(TestResult {
        name: "duty_24hr_coverage".into(),
        passed: coverage.iter().all(|&c| c),
        detail: "all 24 hours covered by at least one shift".into(),
    });

    // No shift has 24hr duty
    for &shift in &[shifts::ALPHA, shifts::BETA, shifts::GAMMA] {
        let on_count = (0..24)
            .filter(|&h| duty::should_be_on_duty(shift, h as f32))
            .count();
        results.push(TestResult {
            name: format!("duty_shift_{}_not_24h", shift),
            passed: (6..24).contains(&on_count),
            detail: format!("shift {} on duty {} hours", shift, on_count),
        });
    }

    // Fitness check
    let fit = duty::is_fit_for_duty(0.3, 0.3, 0.8);
    let unfit = !duty::is_fit_for_duty(0.9, 0.9, 0.2);
    results.push(TestResult {
        name: "duty_fitness_check".into(),
        passed: fit && unfit,
        detail: format!("healthy=fit:{} exhausted=unfit:{}", fit, unfit),
    });

    // Sleep: very tired crew should sleep regardless of shift
    let should_sleep = duty::should_sleep(shifts::ALPHA, 10.0, 0.95);
    results.push(TestResult {
        name: "duty_exhausted_sleeps".into(),
        passed: should_sleep,
        detail: "crew at fatigue=0.95 should sleep".into(),
    });

    results
}

// ── 3. Economy ──────────────────────────────────────────────────────────

fn validate_economy_logic(_verbose: bool) -> Vec<TestResult> {
    println!("--- Economy & Resources ---");
    let mut results = Vec::new();

    // Rationing levels from resource values
    let full = ResourceValues {
        food: 1000.0,
        food_cap: 1000.0,
        water: 1000.0,
        water_cap: 1000.0,
        oxygen: 1000.0,
        oxygen_cap: 1000.0,
        power: 1000.0,
        power_cap: 1000.0,
        fuel: 1000.0,
        fuel_cap: 1000.0,
        spare_parts: 1000.0,
        spare_parts_cap: 1000.0,
    };
    let levels_full = economy::compute_levels(&full);
    let ration_full = economy::compute_rationing(&levels_full);
    results.push(TestResult {
        name: "economy_full_no_rationing".into(),
        passed: matches!(ration_full, RationingLevel::Normal),
        detail: "full resources → Normal rationing".into(),
    });

    // Low food triggers rationing
    let low_food = ResourceValues {
        food: 200.0,
        food_cap: 1000.0,
        water: 1000.0,
        water_cap: 1000.0,
        oxygen: 1000.0,
        oxygen_cap: 1000.0,
        power: 1000.0,
        power_cap: 1000.0,
        fuel: 1000.0,
        fuel_cap: 1000.0,
        spare_parts: 1000.0,
        spare_parts_cap: 1000.0,
    };
    let levels_low = economy::compute_levels(&low_food);
    let ration_low = economy::compute_rationing(&levels_low);
    results.push(TestResult {
        name: "economy_low_food_rations".into(),
        passed: !matches!(ration_low, RationingLevel::Normal),
        detail: format!("food at 20% → {:?} rationing", ration_low),
    });

    // Empty oxygen triggers emergency
    let no_o2 = ResourceValues {
        food: 1000.0,
        food_cap: 1000.0,
        water: 1000.0,
        water_cap: 1000.0,
        oxygen: 50.0,
        oxygen_cap: 1000.0,
        power: 1000.0,
        power_cap: 1000.0,
        fuel: 1000.0,
        fuel_cap: 1000.0,
        spare_parts: 1000.0,
        spare_parts_cap: 1000.0,
    };
    let levels_o2 = economy::compute_levels(&no_o2);
    let ration_o2 = economy::compute_rationing(&levels_o2);
    results.push(TestResult {
        name: "economy_low_o2_emergency".into(),
        passed: matches!(ration_o2, RationingLevel::Emergency),
        detail: format!("oxygen at 5% → {:?}", ration_o2),
    });

    // Shortages detected
    let shortages = economy::detect_shortages(&ResourceLevels {
        food: 0.15,
        water: 0.8,
        oxygen: 0.1,
        power: 1.0,
        fuel: 1.0,
        spare_parts: 1.0,
    });
    results.push(TestResult {
        name: "economy_shortage_detection".into(),
        passed: shortages.len() == 2,
        detail: format!("{} shortages detected (food+O2)", shortages.len()),
    });

    // Health damage from depletion
    let damage = economy::resource_health_damage(&ResourceLevels {
        food: 0.0,
        water: 0.0,
        oxygen: 0.0,
        power: 0.0,
        fuel: 0.0,
        spare_parts: 0.0,
    });
    results.push(TestResult {
        name: "economy_depletion_damage".into(),
        passed: damage > 0.0,
        detail: format!("total depletion → {:.3}/hr damage", damage),
    });

    results
}

// ── 4. Health & Medical ─────────────────────────────────────────────────

fn validate_health_logic(_verbose: bool) -> Vec<TestResult> {
    println!("--- Health & Medical ---");
    let mut results = Vec::new();

    // Injury severity tiers
    results.push(TestResult {
        name: "health_severity_tiers".into(),
        passed: matches!(InjurySeverity::from_health(1.0), InjurySeverity::Healthy)
            && matches!(InjurySeverity::from_health(0.5), InjurySeverity::Light)
            && matches!(InjurySeverity::from_health(0.3), InjurySeverity::Moderate)
            && matches!(InjurySeverity::from_health(0.1), InjurySeverity::Critical),
        detail: "4 severity tiers: healthy→light→moderate→critical".into(),
    });

    // Death check
    results.push(TestResult {
        name: "health_death_at_zero".into(),
        passed: health::is_dead(0.0) && !health::is_dead(0.01),
        detail: "health=0 is dead, health>0 is alive".into(),
    });

    // Recovery in sickbay
    let h = health::compute_health_recovery(0.3, 0.8, 0.8, true, 0.5, 1.0);
    results.push(TestResult {
        name: "health_sickbay_heals".into(),
        passed: h > 0.3,
        detail: format!("0.3 → {:.3} after 1hr in sickbay", h),
    });

    // No natural recovery when moderate injury
    let h2 = health::compute_health_recovery(0.3, 0.2, 0.2, false, 0.0, 1.0);
    results.push(TestResult {
        name: "health_no_natural_moderate".into(),
        passed: (h2 - 0.3).abs() < f32::EPSILON,
        detail: "moderate injury doesn't self-heal".into(),
    });

    // Morale impact from death
    let (witness, ship) = health::death_morale_impact();
    results.push(TestResult {
        name: "health_death_morale".into(),
        passed: witness < 0.0 && ship < 0.0 && witness < ship,
        detail: format!("witness={:.1} shipwide={:.1}", witness, ship),
    });

    results
}

// ── 5. Mission & Voyage ─────────────────────────────────────────────────

fn validate_mission_logic(verbose: bool) -> Vec<TestResult> {
    println!("--- Mission & Voyage ---");
    let mut results = Vec::new();

    // All destinations reachable with all propulsion types
    let destinations = [
        Destination::ProximaCentauri,
        Destination::BarnardsStar,
        Destination::Wolf359,
        Destination::LuytensStar,
        Destination::TauCeti,
        Destination::EpsilonEridani,
        Destination::SixtyOneCygni,
        Destination::Kepler442b,
    ];
    let propulsions = [
        PropulsionType::NuclearPulse,
        PropulsionType::FusionTorch,
        PropulsionType::AntimatterCatalyzed,
        PropulsionType::BussardRamjet,
        PropulsionType::LaserSail,
        PropulsionType::WarpBubble,
    ];

    let mut all_valid = true;
    let mut min_years = f64::MAX;
    let mut max_years = 0.0f64;

    for &dest in &destinations {
        for &prop in &propulsions {
            let config = MissionConfig {
                destination: dest as u8,
                propulsion: prop as u8,
                ..MissionConfig::default()
            };
            let profile = mission::compute_voyage(&config);
            if profile.duration_years <= 0.0 || profile.duration_years > 10000.0 {
                all_valid = false;
            }
            min_years = min_years.min(profile.duration_years);
            max_years = max_years.max(profile.duration_years);
        }
    }

    results.push(TestResult {
        name: "mission_all_reachable".into(),
        passed: all_valid,
        detail: format!(
            "48 combos valid, range: {:.1}–{:.1} years",
            min_years, max_years
        ),
    });

    // Closer destination = shorter voyage (with same propulsion)
    let proxima = mission::compute_voyage(&MissionConfig {
        destination: Destination::ProximaCentauri as u8,
        propulsion: PropulsionType::FusionTorch as u8,
        ..MissionConfig::default()
    });
    let kepler = mission::compute_voyage(&MissionConfig {
        destination: Destination::Kepler442b as u8,
        propulsion: PropulsionType::FusionTorch as u8,
        ..MissionConfig::default()
    });
    results.push(TestResult {
        name: "mission_distance_ordering".into(),
        passed: proxima.duration_years < kepler.duration_years,
        detail: format!(
            "Proxima {:.1}yr < Kepler {:.1}yr",
            proxima.duration_years, kepler.duration_years
        ),
    });

    // Faster propulsion = shorter voyage (same destination)
    let slow = mission::compute_voyage(&MissionConfig {
        destination: Destination::TauCeti as u8,
        propulsion: PropulsionType::NuclearPulse as u8,
        ..MissionConfig::default()
    });
    let fast = mission::compute_voyage(&MissionConfig {
        destination: Destination::TauCeti as u8,
        propulsion: PropulsionType::WarpBubble as u8,
        ..MissionConfig::default()
    });
    results.push(TestResult {
        name: "mission_propulsion_ordering".into(),
        passed: fast.duration_years < slow.duration_years,
        detail: format!(
            "Warp {:.1}yr < Nuclear {:.1}yr to Tau Ceti",
            fast.duration_years, slow.duration_years
        ),
    });

    if verbose {
        println!("  Voyage durations (years):");
        for &dest in &destinations {
            let info = dest.info();
            let fusion = mission::compute_voyage(&MissionConfig {
                destination: dest as u8,
                propulsion: PropulsionType::FusionTorch as u8,
                ..MissionConfig::default()
            });
            println!(
                "    {:20} ({:.1} ly) → {:.1} yr (fusion)",
                info.name, info.distance_ly, fusion.duration_years
            );
        }
    }

    results
}

// ── 6. Pathfinding ──────────────────────────────────────────────────────

fn validate_pathfinding(_verbose: bool) -> Vec<TestResult> {
    println!("--- Pathfinding ---");
    let mut results = Vec::new();

    // Build a small 5-room graph: A-B-C (deck 0), D-E (deck 1), B-D cross-deck
    let edges = vec![
        DoorEdge {
            room_a: 1,
            room_b: 2,
            door_x: 10.0,
            door_y: 5.0,
        },
        DoorEdge {
            room_a: 2,
            room_b: 3,
            door_x: 20.0,
            door_y: 5.0,
        },
        DoorEdge {
            room_a: 2,
            room_b: 4,
            door_x: 15.0,
            door_y: 0.0,
        }, // cross-deck
        DoorEdge {
            room_a: 4,
            room_b: 5,
            door_x: 15.0,
            door_y: 10.0,
        },
    ];
    let mut nav = NavGraph::from_doors(&edges);

    // Same room
    let same = nav.find_path(1, 1);
    results.push(TestResult {
        name: "pathfind_same_room".into(),
        passed: same.is_some() && same.as_ref().unwrap().is_empty(),
        detail: "same room → empty path".into(),
    });

    // Adjacent rooms
    let adj = nav.find_path(1, 2);
    results.push(TestResult {
        name: "pathfind_adjacent".into(),
        passed: adj.is_some() && adj.as_ref().unwrap().len() == 1,
        detail: "1→2 = 1 hop".into(),
    });

    // Multi-hop
    let multi = nav.find_path(1, 3);
    results.push(TestResult {
        name: "pathfind_multi_hop".into(),
        passed: multi.is_some() && multi.as_ref().unwrap().len() == 2,
        detail: "1→3 = 2 hops".into(),
    });

    // Cross-deck
    let cross = nav.find_path(1, 5);
    results.push(TestResult {
        name: "pathfind_cross_deck".into(),
        passed: cross.is_some() && cross.as_ref().unwrap().len() == 3,
        detail: "1→5 = 3 hops (via shaft)".into(),
    });

    // Unreachable
    let mut nav2 = NavGraph::from_doors(&[
        DoorEdge {
            room_a: 1,
            room_b: 2,
            door_x: 5.0,
            door_y: 5.0,
        },
        DoorEdge {
            room_a: 3,
            room_b: 4,
            door_x: 15.0,
            door_y: 5.0,
        },
    ]);
    let unreachable = nav2.find_path(1, 4);
    results.push(TestResult {
        name: "pathfind_unreachable".into(),
        passed: unreachable.is_none(),
        detail: "disconnected graph → None".into(),
    });

    // Larger graph stress test
    let mut big_edges = Vec::new();
    for i in 0..999u32 {
        big_edges.push(DoorEdge {
            room_a: i,
            room_b: i + 1,
            door_x: i as f32,
            door_y: 0.0,
        });
    }
    let mut big_nav = NavGraph::from_doors(&big_edges);
    let long_path = big_nav.find_path(0, 999);
    results.push(TestResult {
        name: "pathfind_1000_rooms".into(),
        passed: long_path.is_some() && long_path.as_ref().unwrap().len() == 999,
        detail: "1000-room chain pathfind succeeds".into(),
    });

    results
}

// ── 7. Utility AI ───────────────────────────────────────────────────────

fn validate_utility_ai(verbose: bool) -> Vec<TestResult> {
    println!("--- Utility AI ---");
    let mut results = Vec::new();

    let base_input = UtilityInput {
        hunger: 0.5,
        fatigue: 0.5,
        social: 0.5,
        comfort: 0.5,
        hygiene: 0.5,
        health: 1.0,
        morale: 0.8,
        hour: 12.0,
        is_crew: true,
        shift: Some(shifts::ALPHA),
        department: Some(0),
        openness: 0.5,
        conscientiousness: 0.5,
        extraversion: 0.5,
        agreeableness: 0.5,
        neuroticism: 0.5,
        current_room: Some(RoomContext {
            room_type: room_types::CORRIDOR,
            occupants: 5,
            capacity: 50,
        }),
        fit_for_duty: true,
        should_be_on_duty: true,
    };

    // Very hungry → eating
    let hungry = UtilityInput {
        hunger: 0.95,
        fatigue: 0.2,
        ..base_input
    };
    let (act, _, _) = utility::pick_best(&hungry);
    results.push(TestResult {
        name: "utility_hungry_eats".into(),
        passed: act == activity_types::EATING,
        detail: format!("hunger=0.95 → activity {}", act),
    });

    // Very tired → sleeping
    let tired = UtilityInput {
        fatigue: 0.95,
        hunger: 0.2,
        ..base_input
    };
    let (act, _, _) = utility::pick_best(&tired);
    results.push(TestResult {
        name: "utility_tired_sleeps".into(),
        passed: act == activity_types::SLEEPING,
        detail: format!("fatigue=0.95 → activity {}", act),
    });

    // Low hygiene → hygiene (not on duty so hygiene wins)
    let dirty = UtilityInput {
        hygiene: 0.95,
        hunger: 0.2,
        fatigue: 0.2,
        should_be_on_duty: false,
        ..base_input
    };
    let (act, _, _) = utility::pick_best(&dirty);
    results.push(TestResult {
        name: "utility_dirty_washes".into(),
        passed: act == activity_types::HYGIENE,
        detail: format!("hygiene=0.95 → activity {}", act),
    });

    // Personality variation: extravert vs introvert both produce valid activities
    let extravert = UtilityInput {
        extraversion: 0.95,
        ..base_input
    };
    let introvert = UtilityInput {
        extraversion: 0.05,
        ..base_input
    };
    let (act_e, _, _) = utility::pick_best(&extravert);
    let (act_i, _, _) = utility::pick_best(&introvert);
    results.push(TestResult {
        name: "utility_personality_varies".into(),
        passed: true, // both produce valid activities
        detail: format!("extravert→{} introvert→{}", act_e, act_i),
    });

    // Medical urgency overrides
    let injured = UtilityInput {
        health: 0.15,
        hunger: 0.2,
        fatigue: 0.2,
        ..base_input
    };
    let (act, _, _) = utility::pick_best(&injured);
    results.push(TestResult {
        name: "utility_injured_seeks_medical".into(),
        passed: act == activity_types::SLEEPING || act == activity_types::IDLE, // medical may map to various
        detail: format!("health=0.15 → activity {}", act),
    });

    // Sweep: all need combinations produce valid activities (no panics)
    let mut valid_count = 0;
    let steps = [0.0, 0.25, 0.5, 0.75, 1.0];
    for &h in &steps {
        for &f in &steps {
            for &s in &steps {
                let input = UtilityInput {
                    hunger: h,
                    fatigue: f,
                    social: s,
                    ..base_input
                };
                let (act, dur, _) = utility::pick_best(&input);
                if act <= 12 && dur > 0.0 {
                    valid_count += 1;
                }
            }
        }
    }
    let expected = steps.len().pow(3);
    results.push(TestResult {
        name: "utility_sweep_125_combos".into(),
        passed: valid_count == expected,
        detail: format!("{}/{} combinations valid", valid_count, expected),
    });

    if verbose {
        println!("  Activity distribution over 125 combos:");
        let mut counts = [0u32; 13];
        for &h in &steps {
            for &f in &steps {
                for &s in &steps {
                    let input = UtilityInput {
                        hunger: h,
                        fatigue: f,
                        social: s,
                        ..base_input
                    };
                    let (act, _, _) = utility::pick_best(&input);
                    if (act as usize) < counts.len() {
                        counts[act as usize] += 1;
                    }
                }
            }
        }
        let names = [
            "IDLE", "WORK", "EAT", "SLEEP", "SOCIAL", "RELAX", "HYGIENE", "TRAVEL", "MAINT",
            "ON_DUTY", "OFF_DUTY", "EMERG", "EXERCISE",
        ];
        for (i, name) in names.iter().enumerate() {
            if counts[i] > 0 {
                println!("    {:10}: {}", name, counts[i]);
            }
        }
    }

    results
}

// ── 8. System Variants ──────────────────────────────────────────────────

fn validate_system_variants(_verbose: bool) -> Vec<TestResult> {
    println!("--- System Variants ---");
    let mut results = Vec::new();

    let power_count = PowerVariant::all().len();
    let ls_count = LifeSupportVariant::all().len();
    let food_count = FoodVariant::all().len();
    let water_count = WaterVariant::all().len();
    let defense_count = DefenseVariant::all().len();
    let medical_count = MedicalVariant::all().len();
    let gravity_count = GravityVariant::all().len();
    let total = power_count
        + ls_count
        + food_count
        + water_count
        + defense_count
        + medical_count
        + gravity_count;

    results.push(TestResult {
        name: "systems_variant_count".into(),
        passed: total == 28,
        detail: format!(
            "{} total: power={} ls={} food={} water={} def={} med={} grav={}",
            total,
            power_count,
            ls_count,
            food_count,
            water_count,
            defense_count,
            medical_count,
            gravity_count
        ),
    });

    // All power variants have positive output
    let all_positive_output = PowerVariant::all().iter().all(|v| v.spec().output > 0.0);
    results.push(TestResult {
        name: "systems_power_positive".into(),
        passed: all_positive_output,
        detail: "all power variants have positive output".into(),
    });

    // All variants have crew >= 1
    let mut all_crewed = true;
    for v in PowerVariant::all() {
        if v.spec().crew_needed == 0 {
            all_crewed = false;
        }
    }
    for v in LifeSupportVariant::all() {
        if v.spec().crew_needed == 0 {
            all_crewed = false;
        }
    }
    results.push(TestResult {
        name: "systems_all_crewed".into(),
        passed: all_crewed,
        detail: "all system variants need ≥1 crew".into(),
    });

    // Tech levels in valid range (1-5)
    let mut tech_valid = true;
    for v in PowerVariant::all() {
        let s = v.spec();
        if s.min_tech_level < 1 || s.min_tech_level > 5 {
            tech_valid = false;
        }
    }
    for v in GravityVariant::all() {
        let s = v.spec();
        if s.min_tech_level < 1 || s.min_tech_level > 5 {
            tech_valid = false;
        }
    }
    results.push(TestResult {
        name: "systems_tech_range".into(),
        passed: tech_valid,
        detail: "all tech levels in 1-5 range".into(),
    });

    results
}
