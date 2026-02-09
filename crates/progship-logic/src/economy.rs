//! Pure economy logic — resource scarcity, rationing, production rates.

/// Resource levels as fractions of capacity (0.0 = empty, 1.0 = full).
#[derive(Debug, Clone, Default)]
pub struct ResourceLevels {
    pub food: f32,
    pub water: f32,
    pub oxygen: f32,
    pub power: f32,
    pub fuel: f32,
    pub spare_parts: f32,
}

/// Raw resource values for computing levels.
#[derive(Debug, Clone)]
pub struct ResourceValues {
    pub food: f32,
    pub food_cap: f32,
    pub water: f32,
    pub water_cap: f32,
    pub oxygen: f32,
    pub oxygen_cap: f32,
    pub power: f32,
    pub power_cap: f32,
    pub fuel: f32,
    pub fuel_cap: f32,
    pub spare_parts: f32,
    pub spare_parts_cap: f32,
}

/// Compute resource levels as fraction of capacity.
pub fn compute_levels(vals: &ResourceValues) -> ResourceLevels {
    ResourceLevels {
        food: safe_ratio(vals.food, vals.food_cap),
        water: safe_ratio(vals.water, vals.water_cap),
        oxygen: safe_ratio(vals.oxygen, vals.oxygen_cap),
        power: safe_ratio(vals.power, vals.power_cap),
        fuel: safe_ratio(vals.fuel, vals.fuel_cap),
        spare_parts: safe_ratio(vals.spare_parts, vals.spare_parts_cap),
    }
}

fn safe_ratio(current: f32, cap: f32) -> f32 {
    if cap <= 0.0 {
        0.0
    } else {
        (current / cap).clamp(0.0, 1.0)
    }
}

/// Rationing level based on the most critical consumable resource.
/// 0 = normal, 1 = light rationing, 2 = heavy rationing, 3 = emergency
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RationingLevel {
    Normal = 0,
    Light = 1,
    Heavy = 2,
    Emergency = 3,
}

/// Determine rationing level from resource levels.
/// Uses the worst of food, water, oxygen (the life-critical consumables).
pub fn compute_rationing(levels: &ResourceLevels) -> RationingLevel {
    let worst = levels.food.min(levels.water).min(levels.oxygen);
    if worst > 0.5 {
        RationingLevel::Normal
    } else if worst > 0.25 {
        RationingLevel::Light
    } else if worst > 0.1 {
        RationingLevel::Heavy
    } else {
        RationingLevel::Emergency
    }
}

/// Consumption multiplier based on rationing level.
/// Light rationing reduces consumption, heavy reduces more.
pub fn rationing_consumption_factor(level: RationingLevel) -> f32 {
    match level {
        RationingLevel::Normal => 1.0,
        RationingLevel::Light => 0.8,
        RationingLevel::Heavy => 0.5,
        RationingLevel::Emergency => 0.3,
    }
}

/// Morale penalty per tick from rationing.
pub fn rationing_morale_penalty(level: RationingLevel) -> f32 {
    match level {
        RationingLevel::Normal => 0.0,
        RationingLevel::Light => 0.005,
        RationingLevel::Heavy => 0.02,
        RationingLevel::Emergency => 0.05,
    }
}

/// Hunger penalty modifier from rationing — reduced food means slower satiation.
pub fn rationing_hunger_factor(level: RationingLevel) -> f32 {
    match level {
        RationingLevel::Normal => 1.0,
        RationingLevel::Light => 1.2,     // 20% hungrier
        RationingLevel::Heavy => 1.5,     // 50% hungrier
        RationingLevel::Emergency => 2.0, // Twice as hungry
    }
}

/// Detect which resources are in shortage (below 20% capacity).
/// Returns a list of (resource_name, level) for resources in shortage.
pub fn detect_shortages(levels: &ResourceLevels) -> Vec<(&'static str, f32)> {
    let threshold = 0.2;
    let mut shortages = Vec::new();
    if levels.food < threshold {
        shortages.push(("food", levels.food));
    }
    if levels.water < threshold {
        shortages.push(("water", levels.water));
    }
    if levels.oxygen < threshold {
        shortages.push(("oxygen", levels.oxygen));
    }
    if levels.power < threshold {
        shortages.push(("power", levels.power));
    }
    if levels.spare_parts < threshold {
        shortages.push(("spare_parts", levels.spare_parts));
    }
    shortages
}

/// Health impact from resource depletion.
/// Returns health damage per hour when resources hit zero.
pub fn resource_health_damage(levels: &ResourceLevels) -> f32 {
    let mut damage = 0.0;
    if levels.oxygen < 0.05 {
        damage += 0.1; // Suffocation
    }
    if levels.water < 0.02 {
        damage += 0.05; // Dehydration
    }
    if levels.food < 0.02 {
        damage += 0.02; // Starvation (slower)
    }
    damage
}

/// Production efficiency for growth chambers based on count and health.
/// Each growth chamber produces base food_rate per hour when at full health.
pub fn food_production_rate(growth_chamber_count: u32, avg_efficiency: f32) -> f32 {
    let base_rate = 5.0; // kg per hour per growth chamber
    growth_chamber_count as f32 * base_rate * avg_efficiency
}

/// Water recycling rate based on recycler count and efficiency.
/// Recyclers recover a fraction of water consumed.
pub fn water_recycling_rate(
    recycler_count: u32,
    avg_efficiency: f32,
    population: f32,
    consumption_rate_per_person: f32,
) -> f32 {
    // Each recycler can handle ~500 people at full efficiency
    let capacity_ratio = (recycler_count as f32 * 500.0 / population.max(1.0)).min(1.0);
    let recovery_rate = 0.9; // 90% water recovery at full efficiency
    population * consumption_rate_per_person * recovery_rate * avg_efficiency * capacity_ratio
}

/// Power balance: total generation minus total draw.
pub fn power_balance(total_generation: f32, total_draw: f32) -> f32 {
    total_generation - total_draw
}

/// Convert rationing level to u8 for storage.
pub fn rationing_to_u8(level: RationingLevel) -> u8 {
    level as u8
}

/// Convert u8 to rationing level.
pub fn u8_to_rationing(val: u8) -> RationingLevel {
    match val {
        0 => RationingLevel::Normal,
        1 => RationingLevel::Light,
        2 => RationingLevel::Heavy,
        _ => RationingLevel::Emergency,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_levels() -> ResourceLevels {
        ResourceLevels {
            food: 1.0,
            water: 1.0,
            oxygen: 1.0,
            power: 1.0,
            fuel: 1.0,
            spare_parts: 1.0,
        }
    }

    #[test]
    fn test_compute_levels() {
        let levels = compute_levels(&ResourceValues {
            food: 50.0,
            food_cap: 100.0,
            water: 30.0,
            water_cap: 100.0,
            oxygen: 20.0,
            oxygen_cap: 100.0,
            power: 80.0,
            power_cap: 100.0,
            fuel: 90.0,
            fuel_cap: 100.0,
            spare_parts: 40.0,
            spare_parts_cap: 100.0,
        });
        assert!((levels.food - 0.5).abs() < 0.01);
        assert!((levels.water - 0.3).abs() < 0.01);
        assert!((levels.oxygen - 0.2).abs() < 0.01);
        assert!((levels.power - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_compute_levels_zero_cap() {
        let levels = compute_levels(&ResourceValues {
            food: 50.0,
            food_cap: 0.0,
            water: 30.0,
            water_cap: 0.0,
            oxygen: 0.0,
            oxygen_cap: 0.0,
            power: 0.0,
            power_cap: 0.0,
            fuel: 0.0,
            fuel_cap: 0.0,
            spare_parts: 0.0,
            spare_parts_cap: 0.0,
        });
        assert_eq!(levels.food, 0.0);
        assert_eq!(levels.water, 0.0);
    }

    #[test]
    fn test_rationing_normal() {
        let levels = full_levels();
        assert_eq!(compute_rationing(&levels), RationingLevel::Normal);
    }

    #[test]
    fn test_rationing_light() {
        let mut levels = full_levels();
        levels.food = 0.4;
        assert_eq!(compute_rationing(&levels), RationingLevel::Light);
    }

    #[test]
    fn test_rationing_heavy() {
        let mut levels = full_levels();
        levels.water = 0.2;
        assert_eq!(compute_rationing(&levels), RationingLevel::Heavy);
    }

    #[test]
    fn test_rationing_emergency() {
        let mut levels = full_levels();
        levels.oxygen = 0.05;
        assert_eq!(compute_rationing(&levels), RationingLevel::Emergency);
    }

    #[test]
    fn test_rationing_uses_worst_resource() {
        let mut levels = full_levels();
        levels.food = 0.9;
        levels.water = 0.8;
        levels.oxygen = 0.15; // This is the worst
        assert_eq!(compute_rationing(&levels), RationingLevel::Heavy);
    }

    #[test]
    fn test_consumption_factors() {
        assert_eq!(rationing_consumption_factor(RationingLevel::Normal), 1.0);
        assert!(rationing_consumption_factor(RationingLevel::Light) < 1.0);
        assert!(
            rationing_consumption_factor(RationingLevel::Heavy)
                < rationing_consumption_factor(RationingLevel::Light)
        );
        assert!(
            rationing_consumption_factor(RationingLevel::Emergency)
                < rationing_consumption_factor(RationingLevel::Heavy)
        );
    }

    #[test]
    fn test_morale_penalties() {
        assert_eq!(rationing_morale_penalty(RationingLevel::Normal), 0.0);
        assert!(rationing_morale_penalty(RationingLevel::Light) > 0.0);
        assert!(
            rationing_morale_penalty(RationingLevel::Heavy)
                > rationing_morale_penalty(RationingLevel::Light)
        );
        assert!(
            rationing_morale_penalty(RationingLevel::Emergency)
                > rationing_morale_penalty(RationingLevel::Heavy)
        );
    }

    #[test]
    fn test_hunger_factor() {
        assert_eq!(rationing_hunger_factor(RationingLevel::Normal), 1.0);
        assert!(rationing_hunger_factor(RationingLevel::Emergency) > 1.0);
    }

    #[test]
    fn test_detect_shortages_none() {
        let levels = full_levels();
        assert!(detect_shortages(&levels).is_empty());
    }

    #[test]
    fn test_detect_shortages_food() {
        let mut levels = full_levels();
        levels.food = 0.1;
        let shortages = detect_shortages(&levels);
        assert_eq!(shortages.len(), 1);
        assert_eq!(shortages[0].0, "food");
    }

    #[test]
    fn test_detect_shortages_multiple() {
        let mut levels = full_levels();
        levels.food = 0.1;
        levels.oxygen = 0.05;
        let shortages = detect_shortages(&levels);
        assert_eq!(shortages.len(), 2);
    }

    #[test]
    fn test_health_damage_none() {
        let levels = full_levels();
        assert_eq!(resource_health_damage(&levels), 0.0);
    }

    #[test]
    fn test_health_damage_suffocation() {
        let mut levels = full_levels();
        levels.oxygen = 0.02;
        assert!(resource_health_damage(&levels) > 0.05);
    }

    #[test]
    fn test_health_damage_stacking() {
        let mut levels = full_levels();
        levels.oxygen = 0.01;
        levels.water = 0.01;
        levels.food = 0.01;
        let damage = resource_health_damage(&levels);
        assert!(damage > 0.15); // All three stack
    }

    #[test]
    fn test_food_production_rate() {
        assert!((food_production_rate(2, 1.0) - 10.0).abs() < 0.01);
        assert!((food_production_rate(2, 0.5) - 5.0).abs() < 0.01);
        assert_eq!(food_production_rate(0, 1.0), 0.0);
    }

    #[test]
    fn test_rationing_roundtrip() {
        for level in [
            RationingLevel::Normal,
            RationingLevel::Light,
            RationingLevel::Heavy,
            RationingLevel::Emergency,
        ] {
            assert_eq!(u8_to_rationing(rationing_to_u8(level)), level);
        }
    }
}
