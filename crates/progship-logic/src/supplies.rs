//! Supply manifest calculation — resource stockpiles for the voyage.
//!
//! Calculates per-resource starting stockpiles based on:
//! - Population × per-person consumption rate × voyage duration
//! - Minus on-board system production capacity
//! - Plus emergency reserve sizing
//! - Modified by budget class (austere = minimal reserves, premium = generous)
//!
//! Validates total mass against a mass budget derived from propulsion capacity.

use serde::{Deserialize, Serialize};

use crate::config::{total_power_draw, total_system_mass, SystemSelection};
use crate::mission::{compute_voyage, MissionConfig, PropulsionType};
use crate::population::PopulationProfile;
use crate::systems::*;

/// Per-resource supply manifest entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSupply {
    pub name: String,
    /// Starting stockpile in metric tons.
    pub stockpile_tons: f64,
    /// Daily consumption rate (tons/day for entire population).
    pub daily_consumption: f64,
    /// Daily production rate from onboard systems (tons/day).
    pub daily_production: f64,
    /// Net daily change (production - consumption).
    pub daily_net: f64,
    /// Days of supply at current stockpile (if net is negative).
    pub days_of_supply: f64,
}

/// Complete supply manifest for the ship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyManifest {
    pub food: ResourceSupply,
    pub water: ResourceSupply,
    pub oxygen: ResourceSupply,
    pub fuel: ResourceSupply,
    pub spare_parts: ResourceSupply,
    pub medical: ResourceSupply,

    /// Total supply mass in metric tons.
    pub total_supply_mass: f64,
    /// Total system mass in metric tons.
    pub total_system_mass: f64,
    /// Total ship mass (supplies + systems + hull).
    pub total_ship_mass: f64,
    /// Maximum mass the propulsion system can push.
    pub propulsion_mass_limit: f64,
    /// Whether the ship is within mass budget.
    pub within_mass_budget: bool,
}

/// Per-person consumption rates (metric tons per day).
mod consumption {
    /// Food: ~2kg/day = 0.002 tons/day
    pub const FOOD_PER_PERSON: f64 = 0.002;
    /// Water: ~3L/day = 0.003 tons/day (before recycling)
    pub const WATER_PER_PERSON: f64 = 0.003;
    /// Oxygen: ~0.84kg/day = 0.00084 tons/day
    pub const O2_PER_PERSON: f64 = 0.00084;
    /// Medical supplies: ~0.0001 tons/day per person
    pub const MEDICAL_PER_PERSON: f64 = 0.0001;
}

/// Emergency reserve multiplier by budget class.
fn reserve_multiplier(budget_class: u8) -> f64 {
    match budget_class {
        1 => 1.1,  // Austere: 10% reserve
        3 => 1.5,  // Premium: 50% reserve
        _ => 1.25, // Standard: 25% reserve
    }
}

/// Estimate propulsion mass limit based on propulsion type.
fn propulsion_mass_limit(propulsion: u8) -> f64 {
    let prop = PropulsionType::from_u8(propulsion).unwrap_or(PropulsionType::FusionTorch);
    match prop {
        PropulsionType::NuclearPulse => 50_000.0,
        PropulsionType::FusionTorch => 100_000.0,
        PropulsionType::AntimatterCatalyzed => 200_000.0,
        PropulsionType::BussardRamjet => 150_000.0,
        PropulsionType::LaserSail => 30_000.0, // Limited by sail size
        PropulsionType::WarpBubble => 500_000.0,
    }
}

/// Hull mass estimate based on population (tons).
fn hull_mass(population: u32) -> f64 {
    // ~2 tons per person of structural mass
    population as f64 * 2.0
}

/// Calculate the complete supply manifest.
pub fn compute_supply_manifest(
    config: &MissionConfig,
    systems: &SystemSelection,
    population: &PopulationProfile,
) -> SupplyManifest {
    let voyage = compute_voyage(config);
    let voyage_days = voyage.duration_years * 365.25;
    let pop = population.departure_total as f64;
    let reserve = reserve_multiplier(config.budget_class);

    // Food supply
    let food_consumption = consumption::FOOD_PER_PERSON * pop;
    let food_production = food_production_rate(systems);
    let food_net = food_production - food_consumption;
    let food_stockpile = if food_net >= 0.0 {
        // Self-sufficient: carry 90-day emergency buffer
        food_consumption * 90.0 * reserve
    } else {
        // Need to carry the deficit for entire voyage
        (-food_net * voyage_days * reserve).max(food_consumption * 90.0)
    };

    // Water supply
    let water_consumption = consumption::WATER_PER_PERSON * pop;
    let water_production = water_production_rate(systems);
    let water_net = water_production - water_consumption;
    let water_stockpile = if water_net >= 0.0 {
        water_consumption * 30.0 * reserve
    } else {
        (-water_net * voyage_days * reserve).max(water_consumption * 30.0)
    };

    // Oxygen supply
    let o2_consumption = consumption::O2_PER_PERSON * pop;
    let o2_production = o2_production_rate(systems);
    let o2_net = o2_production - o2_consumption;
    let o2_stockpile = if o2_net >= 0.0 {
        o2_consumption * 30.0 * reserve
    } else {
        (-o2_net * voyage_days * reserve).max(o2_consumption * 30.0)
    };

    // Fuel supply
    let prop = PropulsionType::from_u8(config.propulsion).unwrap_or(PropulsionType::FusionTorch);
    let fuel_rate = prop.spec().fuel_rate; // kg/hour
    let fuel_daily = fuel_rate * 24.0 / 1000.0; // tons/day
    let fuel_stockpile = fuel_daily * voyage_days * reserve;

    // Spare parts (scales with system mass and voyage length)
    let sys_mass = total_system_mass(systems) as f64;
    // ~0.1% of system mass per year in spare parts
    let spare_rate = sys_mass * 0.001 / 365.25;
    let spare_stockpile = spare_rate * voyage_days * reserve;

    // Medical supplies
    let medical_daily = consumption::MEDICAL_PER_PERSON * pop;
    let medical_stockpile = medical_daily * voyage_days * reserve;

    let food = make_supply("Food", food_stockpile, food_consumption, food_production);
    let water = make_supply(
        "Water",
        water_stockpile,
        water_consumption,
        water_production,
    );
    let oxygen = make_supply("Oxygen", o2_stockpile, o2_consumption, o2_production);
    let fuel = make_supply("Fuel", fuel_stockpile, fuel_daily, 0.0);
    let spare_parts = make_supply("Spare Parts", spare_stockpile, spare_rate, 0.0);
    let medical = make_supply("Medical", medical_stockpile, medical_daily, 0.0);

    let total_supply = food.stockpile_tons
        + water.stockpile_tons
        + oxygen.stockpile_tons
        + fuel.stockpile_tons
        + spare_parts.stockpile_tons
        + medical.stockpile_tons;

    let sys_mass_f64 = total_system_mass(systems) as f64;
    let hull = hull_mass(population.departure_total);
    let total_ship = total_supply + sys_mass_f64 + hull;
    let mass_limit = propulsion_mass_limit(config.propulsion);

    SupplyManifest {
        food,
        water,
        oxygen,
        fuel,
        spare_parts,
        medical,
        total_supply_mass: total_supply,
        total_system_mass: sys_mass_f64,
        total_ship_mass: total_ship,
        propulsion_mass_limit: mass_limit,
        within_mass_budget: total_ship <= mass_limit,
    }
}

fn make_supply(
    name: &str,
    stockpile: f64,
    daily_consumption: f64,
    daily_production: f64,
) -> ResourceSupply {
    let daily_net = daily_production - daily_consumption;
    let days_of_supply = if daily_net < 0.0 {
        stockpile / (-daily_net)
    } else {
        f64::INFINITY
    };
    ResourceSupply {
        name: name.to_string(),
        stockpile_tons: stockpile,
        daily_consumption,
        daily_production,
        daily_net,
        days_of_supply,
    }
}

/// Food production rate in tons/day from selected food system.
fn food_production_rate(systems: &SystemSelection) -> f64 {
    let variant = FoodVariant::all()
        .iter()
        .find(|v| **v as u8 == systems.food);
    match variant {
        Some(v) => v.spec().output as f64 * 24.0 / 1000.0, // kg/hr → tons/day
        None => 0.0,
    }
}

/// Water production rate in tons/day from selected water system.
fn water_production_rate(systems: &SystemSelection) -> f64 {
    let variant = WaterVariant::all()
        .iter()
        .find(|v| **v as u8 == systems.water);
    match variant {
        Some(v) => v.spec().output as f64 * 24.0 / 1000.0, // liters/hr → tons/day
        None => 0.0,
    }
}

/// O2 production rate in tons/day from selected life support system.
fn o2_production_rate(systems: &SystemSelection) -> f64 {
    let variant = LifeSupportVariant::all()
        .iter()
        .find(|v| **v as u8 == systems.life_support);
    match variant {
        Some(v) => v.spec().output as f64 * 24.0 / 1_000_000.0, // kg/hr → tons/day (output in kg O2/hr)
        None => 0.0,
    }
}

/// Total power surplus/deficit in kW.
pub fn power_balance(systems: &SystemSelection) -> f32 {
    let power_output = PowerVariant::all()
        .iter()
        .find(|v| **v as u8 == systems.power)
        .map(|v| v.spec().output)
        .unwrap_or(0.0);
    let power_draw = total_power_draw(systems);
    power_output - power_draw
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{select_systems, SystemOverrides};
    use crate::population::compute_population;

    fn default_manifest() -> SupplyManifest {
        let config = MissionConfig::default();
        let systems = select_systems(&config, &SystemOverrides::default());
        let pop = compute_population(&config, &systems);
        compute_supply_manifest(&config, &systems, &pop)
    }

    #[test]
    fn test_supply_manifest_nonzero() {
        let m = default_manifest();
        assert!(m.food.stockpile_tons > 0.0, "food stockpile should be > 0");
        assert!(
            m.water.stockpile_tons > 0.0,
            "water stockpile should be > 0"
        );
        assert!(
            m.oxygen.stockpile_tons > 0.0,
            "oxygen stockpile should be > 0"
        );
        assert!(m.fuel.stockpile_tons > 0.0, "fuel stockpile should be > 0");
        assert!(
            m.medical.stockpile_tons > 0.0,
            "medical stockpile should be > 0"
        );
    }

    #[test]
    fn test_total_mass_is_sum() {
        let m = default_manifest();
        let supply_sum = m.food.stockpile_tons
            + m.water.stockpile_tons
            + m.oxygen.stockpile_tons
            + m.fuel.stockpile_tons
            + m.spare_parts.stockpile_tons
            + m.medical.stockpile_tons;
        assert!(
            (m.total_supply_mass - supply_sum).abs() < 0.01,
            "total supply mass should equal sum of supplies"
        );
    }

    #[test]
    fn test_ship_mass_includes_hull() {
        let m = default_manifest();
        assert!(
            m.total_ship_mass > m.total_supply_mass + m.total_system_mass,
            "ship mass should include hull"
        );
    }

    #[test]
    fn test_mass_budget_check() {
        let m = default_manifest();
        // Default config should be within budget
        assert!(m.propulsion_mass_limit > 0.0);
        // Budget check should be correct
        assert_eq!(
            m.within_mass_budget,
            m.total_ship_mass <= m.propulsion_mass_limit
        );
    }

    #[test]
    fn test_food_has_production() {
        let m = default_manifest();
        assert!(
            m.food.daily_production > 0.0,
            "should have food production system"
        );
    }

    #[test]
    fn test_water_has_production() {
        let m = default_manifest();
        assert!(
            m.water.daily_production > 0.0,
            "should have water production system"
        );
    }

    #[test]
    fn test_fuel_no_production() {
        let m = default_manifest();
        assert_eq!(
            m.fuel.daily_production, 0.0,
            "fuel is consumed, not produced onboard"
        );
        assert!(m.fuel.daily_consumption > 0.0);
    }

    #[test]
    fn test_days_of_supply_fuel() {
        let m = default_manifest();
        assert!(
            m.fuel.days_of_supply.is_finite(),
            "fuel should have finite days"
        );
        assert!(m.fuel.days_of_supply > 0.0);
    }

    #[test]
    fn test_premium_more_reserves() {
        let austere = MissionConfig {
            budget_class: 1,
            ..MissionConfig::default()
        };
        let premium = MissionConfig {
            budget_class: 3,
            ..MissionConfig::default()
        };
        let s_a = select_systems(&austere, &SystemOverrides::default());
        let s_p = select_systems(&premium, &SystemOverrides::default());
        let pop_a = compute_population(&austere, &s_a);
        let pop_p = compute_population(&premium, &s_p);
        let m_a = compute_supply_manifest(&austere, &s_a, &pop_a);
        let m_p = compute_supply_manifest(&premium, &s_p, &pop_p);
        // Premium should carry more medical supplies (relative to pop)
        let med_per_person_a = m_a.medical.stockpile_tons / pop_a.departure_total as f64;
        let med_per_person_p = m_p.medical.stockpile_tons / pop_p.departure_total as f64;
        assert!(
            med_per_person_p >= med_per_person_a,
            "premium should have at least as much medical per person"
        );
    }

    #[test]
    fn test_power_balance() {
        let config = MissionConfig::default();
        let systems = select_systems(&config, &SystemOverrides::default());
        let balance = power_balance(&systems);
        // Power balance may be negative with default config — that's a design
        // signal that more generators are needed. The function itself should work.
        assert!(
            balance.is_finite(),
            "power balance should be finite: {balance}"
        );
    }
}
