//! Player-facing ship configuration builder.
//!
//! Before launching a game, the player selects mission parameters
//! through a configuration screen. This module provides the data
//! model and validation logic for that screen, independent of any
//! UI framework.
//!
//! # Configuration Flow
//!
//! 1. Player picks a destination (star system)
//! 2. Sets mission priorities (safety, speed, comfort, science, self-sufficiency)
//! 3. Adjusts tech level and budget class
//! 4. Reviews auto-selected systems (can override individual choices)
//! 5. Sees derived values: population, crew, voyage duration, mass budget
//! 6. Launches generation or randomizes
//!
//! ```
//! use progship_logic::ship_config::{ShipConfigBuilder, validate_config};
//!
//! let mut builder = ShipConfigBuilder::default();
//! builder.destination = 0; // Proxima Centauri
//! builder.colony_target = 5000;
//! let errors = validate_config(&builder);
//! assert!(errors.is_empty());
//! ```

use serde::{Deserialize, Serialize};

/// Player-editable ship configuration before generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipConfigBuilder {
    /// Destination star system (index into Destination enum).
    pub destination: u8,
    /// Target colony population at arrival.
    pub colony_target: u32,
    /// Technology level (1=basic, 2=standard, 3=advanced).
    pub tech_level: u8,
    /// Budget class (1=austere, 2=standard, 3=premium).
    pub budget_class: u8,
    /// Mission priority weights (each 0–100, should sum to ~100).
    pub priority_safety: u8,
    pub priority_speed: u8,
    pub priority_comfort: u8,
    pub priority_science: u8,
    pub priority_self_sufficiency: u8,
    /// Propulsion type override (None = auto-select based on tech/budget).
    pub propulsion_override: Option<u8>,
    /// Random seed for generation (None = random).
    pub seed: Option<u64>,
    /// Ship name.
    pub ship_name: String,
}

impl Default for ShipConfigBuilder {
    fn default() -> Self {
        Self {
            destination: 0, // Proxima Centauri
            colony_target: 5000,
            tech_level: 2,
            budget_class: 2,
            priority_safety: 25,
            priority_speed: 20,
            priority_comfort: 20,
            priority_science: 15,
            priority_self_sufficiency: 20,
            propulsion_override: None,
            seed: None,
            ship_name: "ISV Prometheus".to_string(),
        }
    }
}

/// A destination star system with key parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestinationInfo {
    pub id: u8,
    pub name: &'static str,
    pub distance_ly: f32,
    pub habitability: f32,
    pub description: &'static str,
}

/// All available destination star systems.
pub fn destinations() -> Vec<DestinationInfo> {
    vec![
        DestinationInfo {
            id: 0,
            name: "Proxima Centauri",
            distance_ly: 4.24,
            habitability: 0.6,
            description: "Closest star system. Proxima Centauri b is a rocky exoplanet in the habitable zone.",
        },
        DestinationInfo {
            id: 1,
            name: "Tau Ceti",
            distance_ly: 11.9,
            habitability: 0.7,
            description: "Sun-like star with multiple planet candidates. Moderate voyage length.",
        },
        DestinationInfo {
            id: 2,
            name: "TRAPPIST-1",
            distance_ly: 39.6,
            habitability: 0.8,
            description: "Seven Earth-sized planets, three in habitable zone. Long voyage but excellent prospects.",
        },
        DestinationInfo {
            id: 3,
            name: "Epsilon Eridani",
            distance_ly: 10.5,
            habitability: 0.5,
            description: "Young star with confirmed dust disk. Moderate distance, uncertain habitability.",
        },
        DestinationInfo {
            id: 4,
            name: "Barnard's Star",
            distance_ly: 5.96,
            habitability: 0.4,
            description: "Red dwarf with one confirmed planet. Short voyage, harsh conditions.",
        },
        DestinationInfo {
            id: 5,
            name: "Luyten's Star",
            distance_ly: 12.4,
            habitability: 0.65,
            description: "Red dwarf with super-Earth in habitable zone.",
        },
        DestinationInfo {
            id: 6,
            name: "Kepler-442",
            distance_ly: 1206.0,
            habitability: 0.95,
            description: "Multi-generational voyage to one of the most habitable known exoplanets.",
        },
        DestinationInfo {
            id: 7,
            name: "Wolf 1061",
            distance_ly: 13.8,
            habitability: 0.55,
            description: "Three confirmed planets. Moderate distance, one in habitable zone.",
        },
    ]
}

/// Configuration validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// Colony target too small for genetic diversity.
    PopulationTooSmall(u32),
    /// Colony target unreasonably large.
    PopulationTooLarge(u32),
    /// Tech level out of range.
    InvalidTechLevel(u8),
    /// Budget class out of range.
    InvalidBudgetClass(u8),
    /// Unknown destination.
    InvalidDestination(u8),
    /// Priority sum too far from 100.
    PrioritySumInvalid(u16),
    /// Ship name empty.
    EmptyShipName,
}

/// Validate a ship configuration, returning all errors found.
pub fn validate_config(config: &ShipConfigBuilder) -> Vec<ConfigError> {
    let mut errors = Vec::new();

    if config.colony_target < 160 {
        errors.push(ConfigError::PopulationTooSmall(config.colony_target));
    }
    if config.colony_target > 100_000 {
        errors.push(ConfigError::PopulationTooLarge(config.colony_target));
    }
    if config.tech_level < 1 || config.tech_level > 3 {
        errors.push(ConfigError::InvalidTechLevel(config.tech_level));
    }
    if config.budget_class < 1 || config.budget_class > 3 {
        errors.push(ConfigError::InvalidBudgetClass(config.budget_class));
    }
    if config.destination > 7 {
        errors.push(ConfigError::InvalidDestination(config.destination));
    }

    let priority_sum = config.priority_safety as u16
        + config.priority_speed as u16
        + config.priority_comfort as u16
        + config.priority_science as u16
        + config.priority_self_sufficiency as u16;
    if !(50..=200).contains(&priority_sum) {
        errors.push(ConfigError::PrioritySumInvalid(priority_sum));
    }

    if config.ship_name.trim().is_empty() {
        errors.push(ConfigError::EmptyShipName);
    }

    errors
}

/// Summary of derived ship parameters for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSummary {
    /// Destination name.
    pub destination_name: String,
    /// Estimated voyage duration in years.
    pub voyage_years: f32,
    /// Departure population (back-calculated from colony target).
    pub departure_population: u32,
    /// Total crew needed.
    pub crew_count: u32,
    /// Total passengers.
    pub passenger_count: u32,
    /// Estimated number of decks.
    pub deck_count: u32,
    /// Estimated total mass in tonnes.
    pub total_mass_tonnes: f32,
}

/// Generate a summary from a config builder.
///
/// This uses simplified estimates for the config screen preview.
/// Actual values are computed during full generation.
pub fn estimate_summary(config: &ShipConfigBuilder) -> ConfigSummary {
    let dest = destinations();
    let destination = dest.iter().find(|d| d.id == config.destination);
    let destination_name = destination.map_or("Unknown".to_string(), |d| d.name.to_string());
    let distance_ly = destination.map_or(10.0, |d| d.distance_ly);

    // Rough speed estimate based on tech level (fraction of c)
    let speed_fraction = match config.tech_level {
        1 => 0.05,
        2 => 0.10,
        3 => 0.15,
        _ => 0.10,
    };
    let voyage_years = distance_ly / speed_fraction;

    // Growth rate 0.5% per year → departure = arrival / (1.005^years)
    let growth_factor = 1.005_f32.powf(voyage_years);
    let departure_population = (config.colony_target as f32 / growth_factor).ceil() as u32;

    // Crew is ~40% of departure population
    let crew_count = (departure_population as f32 * 0.4).ceil() as u32;
    let passenger_count = departure_population.saturating_sub(crew_count);

    // Rough deck estimate: ~250 people per deck
    let deck_count = (departure_population as f32 / 250.0).ceil() as u32;

    // Mass estimate: ~50 tonnes per person (ship + supplies)
    let mass_multiplier = match config.budget_class {
        1 => 40.0,
        2 => 50.0,
        3 => 65.0,
        _ => 50.0,
    };
    let total_mass_tonnes = departure_population as f32 * mass_multiplier;

    ConfigSummary {
        destination_name,
        voyage_years,
        departure_population,
        crew_count,
        passenger_count,
        deck_count,
        total_mass_tonnes,
    }
}

/// Randomize a config builder with a random seed.
pub fn randomize_config(seed: u64) -> ShipConfigBuilder {
    // Simple hash-based randomization
    let hash = |s: u64, i: u64| -> u64 {
        let mut h = s.wrapping_mul(6364136223846793005).wrapping_add(i);
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        h
    };

    let destination = (hash(seed, 0) % 8) as u8;
    let colony_target = 1000 + (hash(seed, 1) % 9000) as u32; // 1000–10000
    let tech_level = 1 + (hash(seed, 2) % 3) as u8;
    let budget_class = 1 + (hash(seed, 3) % 3) as u8;

    // Random priorities that sum to ~100
    let mut remaining = 100u8;
    let safety = (hash(seed, 4) % 40) as u8 + 5;
    remaining = remaining.saturating_sub(safety);
    let speed = (hash(seed, 5) % remaining.min(35) as u64) as u8 + 5;
    remaining = remaining.saturating_sub(speed);
    let comfort = (hash(seed, 6) % remaining.min(30) as u64) as u8 + 5;
    remaining = remaining.saturating_sub(comfort);
    let science = (hash(seed, 7) % remaining.min(25) as u64) as u8 + 5;
    let self_suff = remaining.saturating_sub(science);

    ShipConfigBuilder {
        destination,
        colony_target,
        tech_level,
        budget_class,
        priority_safety: safety,
        priority_speed: speed,
        priority_comfort: comfort,
        priority_science: science,
        priority_self_sufficiency: self_suff,
        propulsion_override: None,
        seed: Some(seed),
        ship_name: "ISV Prometheus".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = ShipConfigBuilder::default();
        let errors = validate_config(&config);
        assert!(
            errors.is_empty(),
            "default config should be valid: {errors:?}"
        );
    }

    #[test]
    fn population_too_small() {
        let mut config = ShipConfigBuilder::default();
        config.colony_target = 50;
        let errors = validate_config(&config);
        assert!(errors.contains(&ConfigError::PopulationTooSmall(50)));
    }

    #[test]
    fn population_too_large() {
        let mut config = ShipConfigBuilder::default();
        config.colony_target = 200_000;
        let errors = validate_config(&config);
        assert!(errors.contains(&ConfigError::PopulationTooLarge(200_000)));
    }

    #[test]
    fn invalid_tech_level() {
        let mut config = ShipConfigBuilder::default();
        config.tech_level = 0;
        assert!(validate_config(&config).contains(&ConfigError::InvalidTechLevel(0)));
        config.tech_level = 4;
        assert!(validate_config(&config).contains(&ConfigError::InvalidTechLevel(4)));
    }

    #[test]
    fn invalid_budget_class() {
        let mut config = ShipConfigBuilder::default();
        config.budget_class = 0;
        assert!(validate_config(&config).contains(&ConfigError::InvalidBudgetClass(0)));
    }

    #[test]
    fn invalid_destination() {
        let mut config = ShipConfigBuilder::default();
        config.destination = 99;
        assert!(validate_config(&config).contains(&ConfigError::InvalidDestination(99)));
    }

    #[test]
    fn empty_ship_name() {
        let mut config = ShipConfigBuilder::default();
        config.ship_name = "  ".to_string();
        assert!(validate_config(&config).contains(&ConfigError::EmptyShipName));
    }

    #[test]
    fn priority_sum_invalid() {
        let mut config = ShipConfigBuilder::default();
        config.priority_safety = 0;
        config.priority_speed = 0;
        config.priority_comfort = 0;
        config.priority_science = 0;
        config.priority_self_sufficiency = 0;
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ConfigError::PrioritySumInvalid(_))));
    }

    #[test]
    fn destinations_list() {
        let dests = destinations();
        assert_eq!(dests.len(), 8);
        assert!(dests[0].distance_ly < dests[6].distance_ly);
    }

    #[test]
    fn estimate_summary_reasonable() {
        let config = ShipConfigBuilder::default();
        let summary = estimate_summary(&config);
        assert_eq!(summary.destination_name, "Proxima Centauri");
        assert!(summary.voyage_years > 10.0 && summary.voyage_years < 200.0);
        assert!(summary.departure_population > 0);
        assert!(summary.crew_count > 0);
        assert!(summary.deck_count > 0);
        assert!(summary.total_mass_tonnes > 0.0);
    }

    #[test]
    fn longer_voyage_means_smaller_departure() {
        // TRAPPIST-1 (39.6 ly) vs Proxima (4.24 ly) with same target
        let mut near = ShipConfigBuilder::default();
        near.destination = 0; // Proxima
        let mut far = ShipConfigBuilder::default();
        far.destination = 2; // TRAPPIST-1
        let near_sum = estimate_summary(&near);
        let far_sum = estimate_summary(&far);
        // Longer voyage = more growth = smaller departure needed
        assert!(far_sum.departure_population < near_sum.departure_population);
    }

    #[test]
    fn premium_budget_heavier() {
        let mut austere = ShipConfigBuilder::default();
        austere.budget_class = 1;
        let mut premium = ShipConfigBuilder::default();
        premium.budget_class = 3;
        let a_sum = estimate_summary(&austere);
        let p_sum = estimate_summary(&premium);
        assert!(p_sum.total_mass_tonnes > a_sum.total_mass_tonnes);
    }

    #[test]
    fn randomize_produces_valid() {
        for seed in 0..50 {
            let config = randomize_config(seed);
            let errors = validate_config(&config);
            assert!(
                errors.is_empty(),
                "seed {seed} produced invalid config: {errors:?}"
            );
        }
    }

    #[test]
    fn randomize_produces_variety() {
        let configs: Vec<_> = (0..20).map(randomize_config).collect();
        let destinations: std::collections::HashSet<u8> =
            configs.iter().map(|c| c.destination).collect();
        assert!(
            destinations.len() >= 3,
            "should have variety in destinations"
        );
    }

    #[test]
    fn acceptance_player_can_customize() {
        // Player sets config, sees effects, validates
        let mut config = ShipConfigBuilder::default();
        config.destination = 2; // TRAPPIST-1
        config.colony_target = 10_000;
        config.tech_level = 3;
        config.budget_class = 3;
        config.ship_name = "ISV Pioneer".to_string();

        assert!(validate_config(&config).is_empty());
        let summary = estimate_summary(&config);
        assert_eq!(summary.destination_name, "TRAPPIST-1");
        assert!(summary.crew_count > 0);
        assert!(summary.total_mass_tonnes > 0.0);
    }
}
