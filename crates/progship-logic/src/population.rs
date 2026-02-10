//! Population & crew sizing — derives departure population and crew allocation.
//!
//! Given a `MissionConfig` (colony target pop, voyage duration, system crew needs),
//! this module calculates:
//! - Departure population (back-calculated from arrival target via growth rate)
//! - Total crew required (system crew + overhead departments)
//! - Per-department crew allocation
//! - Genetic diversity validation

use serde::{Deserialize, Serialize};

use crate::config::{total_system_crew, SystemSelection};
use crate::constants::departments;
use crate::mission::{compute_voyage, MissionConfig};

/// Population breakdown for the ship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationProfile {
    /// Total people at departure.
    pub departure_total: u32,
    /// Total crew (all departments).
    pub total_crew: u32,
    /// Total passengers (non-crew).
    pub total_passengers: u32,
    /// Target colony population on arrival.
    pub arrival_target: u32,
    /// Estimated population on arrival (with growth).
    pub estimated_arrival: u32,
    /// Per-department crew counts.
    pub department_crew: DepartmentCrew,
    /// Whether genetic diversity minimum is met.
    pub genetic_diversity_ok: bool,
}

/// Crew allocated to each department.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepartmentCrew {
    pub command: u32,
    pub engineering: u32,
    pub medical: u32,
    pub science: u32,
    pub security: u32,
    pub operations: u32,
    pub civilian: u32,
}

impl DepartmentCrew {
    pub fn total(&self) -> u32 {
        self.command
            + self.engineering
            + self.medical
            + self.science
            + self.security
            + self.operations
            + self.civilian
    }

    /// Get crew count by department ID.
    pub fn by_department(&self, dept: u8) -> u32 {
        match dept {
            departments::COMMAND => self.command,
            departments::ENGINEERING => self.engineering,
            departments::MEDICAL => self.medical,
            departments::SCIENCE => self.science,
            departments::SECURITY => self.security,
            departments::OPERATIONS => self.operations,
            departments::CIVILIAN => self.civilian,
            _ => 0,
        }
    }
}

/// Minimum viable population for genetic diversity (500-year rule).
const MIN_GENETIC_DIVERSITY: u32 = 160;

/// Annual population growth rate (births - deaths) during voyage.
const ANNUAL_GROWTH_RATE: f64 = 0.005; // 0.5% per year

/// Calculate departure population from arrival target using compound growth.
///
/// arrival = departure × (1 + rate)^years
/// departure = arrival / (1 + rate)^years
pub fn departure_population(arrival_target: u32, voyage_years: f64) -> u32 {
    if voyage_years <= 0.0 {
        return arrival_target;
    }
    let factor = (1.0 + ANNUAL_GROWTH_RATE).powf(voyage_years);
    let departure = arrival_target as f64 / factor;
    // Minimum viable population
    (departure.ceil() as u32).max(MIN_GENETIC_DIVERSITY)
}

/// Estimated arrival population given departure and voyage duration.
pub fn estimated_arrival(departure: u32, voyage_years: f64) -> u32 {
    if voyage_years <= 0.0 {
        return departure;
    }
    let factor = (1.0 + ANNUAL_GROWTH_RATE).powf(voyage_years);
    (departure as f64 * factor).floor() as u32
}

/// Calculate crew requirements from system selections + overhead.
pub fn compute_crew(system_crew: u32, departure_pop: u32, budget_class: u8) -> DepartmentCrew {
    // System operators go to engineering
    let engineering = system_crew;

    // Command: ~2% of population, minimum 10
    let command = ((departure_pop as f32 * 0.02).ceil() as u32).max(10);

    // Medical: 1 per 50 people (austere), 1 per 30 (standard), 1 per 20 (premium)
    let medical_ratio = match budget_class {
        1 => 50,
        3 => 20,
        _ => 30,
    };
    let medical = (departure_pop / medical_ratio).max(5);

    // Science: 1-3% based on mission, minimum 5
    let science = ((departure_pop as f32 * 0.015).ceil() as u32).max(5);

    // Security: 1 per 100 (austere), 1 per 50 (standard), 1 per 40 (premium)
    let security_ratio = match budget_class {
        1 => 100,
        3 => 40,
        _ => 50,
    };
    let security = (departure_pop / security_ratio).max(5);

    // Operations: logistics, maintenance, food service — ~5% of pop
    let operations = ((departure_pop as f32 * 0.05).ceil() as u32).max(10);

    // Civilian: teachers, counselors, administrators — ~3% of pop
    let civilian = ((departure_pop as f32 * 0.03).ceil() as u32).max(5);

    DepartmentCrew {
        command,
        engineering,
        medical,
        science,
        security,
        operations,
        civilian,
    }
}

/// Full population sizing from mission config and system selection.
pub fn compute_population(config: &MissionConfig, systems: &SystemSelection) -> PopulationProfile {
    let voyage = compute_voyage(config);
    let system_crew = total_system_crew(systems);

    let dep_pop = departure_population(config.colony_target_pop, voyage.duration_years);
    let dept_crew = compute_crew(system_crew, dep_pop, config.budget_class);
    let total_crew = dept_crew.total();

    // Passengers = departure pop - crew (minimum 0)
    let total_passengers = dep_pop.saturating_sub(total_crew);

    // If crew exceeds departure pop, we need more people
    let departure_total = dep_pop.max(total_crew);

    let est_arrival = estimated_arrival(departure_total, voyage.duration_years);

    PopulationProfile {
        departure_total,
        total_crew,
        total_passengers,
        arrival_target: config.colony_target_pop,
        estimated_arrival: est_arrival,
        department_crew: dept_crew,
        genetic_diversity_ok: departure_total >= MIN_GENETIC_DIVERSITY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{select_systems, SystemOverrides};

    #[test]
    fn test_departure_short_voyage() {
        // 10-year voyage, target 5000
        let dep = departure_population(5000, 10.0);
        // 5000 / 1.005^10 ≈ 4756
        assert!(dep > 4700 && dep < 4800, "dep={dep}");
    }

    #[test]
    fn test_departure_long_voyage() {
        // 200-year voyage, target 5000
        let dep = departure_population(5000, 200.0);
        // 5000 / 1.005^200 ≈ 1845
        assert!(dep > 1700 && dep < 2000, "dep={dep}");
    }

    #[test]
    fn test_departure_very_long_voyage() {
        // 3000-year voyage, target 5000
        let dep = departure_population(5000, 3000.0);
        // Growth factor is enormous — departure would be tiny, but clamped to MIN_GENETIC_DIVERSITY
        assert_eq!(dep, MIN_GENETIC_DIVERSITY);
    }

    #[test]
    fn test_departure_zero_voyage() {
        let dep = departure_population(5000, 0.0);
        assert_eq!(dep, 5000);
    }

    #[test]
    fn test_roundtrip_growth() {
        let dep = departure_population(5000, 100.0);
        let arr = estimated_arrival(dep, 100.0);
        // Should be close to 5000 (within rounding)
        assert!((arr as i64 - 5000).unsigned_abs() < 10, "arr={arr}");
    }

    #[test]
    fn test_crew_departments_nonzero() {
        let dept = compute_crew(30, 3000, 2);
        assert!(dept.command >= 10);
        assert!(dept.engineering >= 30);
        assert!(dept.medical >= 5);
        assert!(dept.science >= 5);
        assert!(dept.security >= 5);
        assert!(dept.operations >= 10);
        assert!(dept.civilian >= 5);
    }

    #[test]
    fn test_crew_total_matches_sum() {
        let dept = compute_crew(25, 5000, 2);
        let expected = dept.command
            + dept.engineering
            + dept.medical
            + dept.science
            + dept.security
            + dept.operations
            + dept.civilian;
        assert_eq!(dept.total(), expected);
    }

    #[test]
    fn test_premium_more_medical() {
        let austere = compute_crew(20, 3000, 1);
        let premium = compute_crew(20, 3000, 3);
        assert!(premium.medical >= austere.medical);
    }

    #[test]
    fn test_genetic_diversity_flag() {
        let config = MissionConfig::default();
        let systems = select_systems(&config, &SystemOverrides::default());
        let pop = compute_population(&config, &systems);
        assert!(
            pop.genetic_diversity_ok,
            "default config should have enough people"
        );
    }

    #[test]
    fn test_population_profile_coherent() {
        let config = MissionConfig::default();
        let systems = select_systems(&config, &SystemOverrides::default());
        let pop = compute_population(&config, &systems);

        assert!(pop.departure_total > 0);
        assert!(pop.total_crew > 0);
        assert_eq!(pop.departure_total, pop.total_crew + pop.total_passengers);
        assert!(pop.estimated_arrival >= pop.departure_total);
        assert_eq!(pop.arrival_target, config.colony_target_pop);
    }

    #[test]
    fn test_by_department() {
        let dept = compute_crew(20, 2000, 2);
        assert_eq!(dept.by_department(departments::COMMAND), dept.command);
        assert_eq!(
            dept.by_department(departments::ENGINEERING),
            dept.engineering
        );
        assert_eq!(dept.by_department(departments::MEDICAL), dept.medical);
        assert_eq!(dept.by_department(99), 0); // Unknown dept
    }
}
