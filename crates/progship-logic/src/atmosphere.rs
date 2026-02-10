//! Per-room atmosphere simulation — O2, CO2, temperature, pressure.
//!
//! Each room tracks its own atmospheric conditions. Gas flows between
//! connected rooms based on pressure differentials. Life support systems
//! actively regulate atmosphere through vent connections.
//!
//! This replaces the simpler per-deck atmosphere model with room-level
//! granularity, enabling scenarios like localized fires, sealed rooms
//! during emergencies, and HVAC failure isolation.

use serde::{Deserialize, Serialize};

/// Atmospheric state for a single room.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RoomAtmosphere {
    /// Oxygen level (0.0 = vacuum, 0.21 = normal Earth, 1.0 = pure O2).
    pub o2: f32,
    /// Carbon dioxide level (0.0 = none, 0.0004 = Earth normal, >0.05 = dangerous).
    pub co2: f32,
    /// Temperature in Celsius.
    pub temperature: f32,
    /// Pressure in atmospheres (1.0 = Earth sea level).
    pub pressure: f32,
    /// Whether the room is sealed (doors closed, no gas exchange).
    pub sealed: bool,
    /// Whether life support vents are connected to this room.
    pub has_life_support: bool,
    /// Whether there is an active fire in the room.
    pub fire: bool,
}

impl Default for RoomAtmosphere {
    fn default() -> Self {
        Self {
            o2: 0.21,
            co2: 0.0004,
            temperature: 22.0,
            pressure: 1.0,
            sealed: false,
            has_life_support: true,
            fire: false,
        }
    }
}

/// Atmospheric danger levels for crew safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtmosphereHazard {
    /// All parameters within safe range.
    Safe,
    /// Minor deviations — discomfort but not dangerous.
    Warning,
    /// Dangerous — immediate health effects.
    Danger,
    /// Lethal — rapid incapacitation and death.
    Lethal,
}

/// Configuration constants for atmosphere simulation.
pub mod atmo_constants {
    /// Normal O2 level for ship atmosphere.
    pub const NORMAL_O2: f32 = 0.21;
    /// Minimum safe O2 level (below = Warning).
    pub const LOW_O2: f32 = 0.16;
    /// Critical O2 level (below = Danger).
    pub const CRITICAL_O2: f32 = 0.10;
    /// Lethal O2 level.
    pub const LETHAL_O2: f32 = 0.06;

    /// Normal CO2 level.
    pub const NORMAL_CO2: f32 = 0.0004;
    /// Elevated CO2 (headaches, drowsiness).
    pub const HIGH_CO2: f32 = 0.02;
    /// Dangerous CO2 (impaired judgment).
    pub const DANGER_CO2: f32 = 0.05;
    /// Lethal CO2.
    pub const LETHAL_CO2: f32 = 0.10;

    /// Normal temperature range.
    pub const TEMP_MIN_SAFE: f32 = 15.0;
    pub const TEMP_MAX_SAFE: f32 = 30.0;
    pub const TEMP_MIN_DANGER: f32 = 5.0;
    pub const TEMP_MAX_DANGER: f32 = 45.0;

    /// Normal pressure range.
    pub const PRESSURE_MIN_SAFE: f32 = 0.8;
    pub const PRESSURE_MAX_SAFE: f32 = 1.2;

    /// O2 consumed per person per hour.
    pub const O2_CONSUMPTION_PER_PERSON: f32 = 0.0008;
    /// CO2 produced per person per hour.
    pub const CO2_PRODUCTION_PER_PERSON: f32 = 0.0008;

    /// O2 consumed by fire per hour (per unit of fire).
    pub const FIRE_O2_CONSUMPTION: f32 = 0.02;
    /// CO2 produced by fire per hour.
    pub const FIRE_CO2_PRODUCTION: f32 = 0.015;
    /// Temperature increase from fire per hour.
    pub const FIRE_TEMP_INCREASE: f32 = 15.0;

    /// Gas exchange rate between adjacent rooms (per hour, fraction of differential).
    pub const GAS_EXCHANGE_RATE: f32 = 0.3;

    /// Life support O2 production rate per hour.
    pub const LS_O2_PRODUCTION: f32 = 0.01;
    /// Life support CO2 scrubbing rate per hour.
    pub const LS_CO2_SCRUBBING: f32 = 0.01;
    /// Life support temperature regulation rate (degrees per hour toward target).
    pub const LS_TEMP_REGULATION: f32 = 2.0;
    /// Target temperature for life support.
    pub const LS_TARGET_TEMP: f32 = 22.0;
}

/// Assess the hazard level of a room's atmosphere.
pub fn assess_hazard(atmo: &RoomAtmosphere) -> AtmosphereHazard {
    use atmo_constants::*;

    // Check each parameter and return the worst hazard level
    let mut worst = AtmosphereHazard::Safe;

    // O2 check
    if atmo.o2 < LETHAL_O2 {
        return AtmosphereHazard::Lethal;
    } else if atmo.o2 < CRITICAL_O2 {
        worst = AtmosphereHazard::Danger;
    } else if atmo.o2 < LOW_O2 {
        worst = worst_hazard(worst, AtmosphereHazard::Warning);
    }

    // CO2 check
    if atmo.co2 > LETHAL_CO2 {
        return AtmosphereHazard::Lethal;
    } else if atmo.co2 > DANGER_CO2 {
        worst = worst_hazard(worst, AtmosphereHazard::Danger);
    } else if atmo.co2 > HIGH_CO2 {
        worst = worst_hazard(worst, AtmosphereHazard::Warning);
    }

    // Temperature check
    if atmo.temperature < TEMP_MIN_DANGER || atmo.temperature > TEMP_MAX_DANGER {
        worst = worst_hazard(worst, AtmosphereHazard::Danger);
    } else if atmo.temperature < TEMP_MIN_SAFE || atmo.temperature > TEMP_MAX_SAFE {
        worst = worst_hazard(worst, AtmosphereHazard::Warning);
    }

    // Pressure check
    if atmo.pressure < 0.3 || atmo.pressure > 2.0 {
        worst = worst_hazard(worst, AtmosphereHazard::Danger);
    } else if atmo.pressure < PRESSURE_MIN_SAFE || atmo.pressure > PRESSURE_MAX_SAFE {
        worst = worst_hazard(worst, AtmosphereHazard::Warning);
    }

    worst
}

fn worst_hazard(a: AtmosphereHazard, b: AtmosphereHazard) -> AtmosphereHazard {
    let rank = |h: AtmosphereHazard| match h {
        AtmosphereHazard::Safe => 0,
        AtmosphereHazard::Warning => 1,
        AtmosphereHazard::Danger => 2,
        AtmosphereHazard::Lethal => 3,
    };
    if rank(b) > rank(a) {
        b
    } else {
        a
    }
}

/// Update a single room's atmosphere for one time step.
///
/// `dt` is the time step in hours. `occupants` is the number of people in the room.
/// This handles breathing, fire effects, and life support regulation.
/// Gas exchange between rooms is handled separately by `exchange_gas`.
pub fn update_room_atmosphere(atmo: &mut RoomAtmosphere, occupants: u32, dt: f32) {
    use atmo_constants::*;

    // Breathing: people consume O2 and produce CO2
    let breathing_o2 = O2_CONSUMPTION_PER_PERSON * occupants as f32 * dt;
    let breathing_co2 = CO2_PRODUCTION_PER_PERSON * occupants as f32 * dt;
    atmo.o2 = (atmo.o2 - breathing_o2).max(0.0);
    atmo.co2 = (atmo.co2 + breathing_co2).min(1.0);

    // Fire effects
    if atmo.fire && atmo.o2 > 0.05 {
        atmo.o2 = (atmo.o2 - FIRE_O2_CONSUMPTION * dt).max(0.0);
        atmo.co2 = (atmo.co2 + FIRE_CO2_PRODUCTION * dt).min(1.0);
        atmo.temperature += FIRE_TEMP_INCREASE * dt;
        // Fire self-extinguishes if O2 drops too low
        if atmo.o2 < 0.05 {
            atmo.fire = false;
        }
    }

    // Life support regulation (only if connected and not sealed)
    if atmo.has_life_support && !atmo.sealed {
        // Produce O2 (up to normal)
        if atmo.o2 < NORMAL_O2 {
            atmo.o2 = (atmo.o2 + LS_O2_PRODUCTION * dt).min(NORMAL_O2);
        }
        // Scrub CO2 (down to normal)
        if atmo.co2 > NORMAL_CO2 {
            atmo.co2 = (atmo.co2 - LS_CO2_SCRUBBING * dt).max(NORMAL_CO2);
        }
        // Temperature regulation toward target
        let temp_diff = LS_TARGET_TEMP - atmo.temperature;
        if temp_diff.abs() > 0.1 {
            let adjust = temp_diff.signum() * LS_TEMP_REGULATION * dt;
            if temp_diff.abs() > adjust.abs() {
                atmo.temperature += adjust;
            } else {
                atmo.temperature = LS_TARGET_TEMP;
            }
        }
    }

    // Pressure clamp (simplified — no vacuum simulation yet)
    atmo.pressure = atmo.pressure.clamp(0.0, 3.0);
}

/// Exchange gas between two connected rooms based on pressure differential.
///
/// Only operates if neither room is sealed. Equalizes O2, CO2, temperature,
/// and pressure toward the average.
pub fn exchange_gas(a: &mut RoomAtmosphere, b: &mut RoomAtmosphere, dt: f32) {
    if a.sealed || b.sealed {
        return;
    }

    let rate = atmo_constants::GAS_EXCHANGE_RATE * dt;

    // O2 exchange
    let o2_diff = b.o2 - a.o2;
    let o2_flow = o2_diff * rate;
    a.o2 = (a.o2 + o2_flow).clamp(0.0, 1.0);
    b.o2 = (b.o2 - o2_flow).clamp(0.0, 1.0);

    // CO2 exchange
    let co2_diff = b.co2 - a.co2;
    let co2_flow = co2_diff * rate;
    a.co2 = (a.co2 + co2_flow).clamp(0.0, 1.0);
    b.co2 = (b.co2 - co2_flow).clamp(0.0, 1.0);

    // Temperature exchange
    let temp_diff = b.temperature - a.temperature;
    let temp_flow = temp_diff * rate;
    a.temperature += temp_flow;
    b.temperature -= temp_flow;

    // Pressure exchange
    let press_diff = b.pressure - a.pressure;
    let press_flow = press_diff * rate;
    a.pressure = (a.pressure + press_flow).max(0.0);
    b.pressure = (b.pressure - press_flow).max(0.0);
}

/// Health damage rate from bad atmosphere (damage per hour).
pub fn atmosphere_health_damage(atmo: &RoomAtmosphere) -> f32 {
    use atmo_constants::*;

    let mut damage = 0.0;

    // Low O2 damage
    if atmo.o2 < CRITICAL_O2 {
        damage += (CRITICAL_O2 - atmo.o2) * 5.0; // Up to ~0.5/hr at 0.0 O2
    }

    // High CO2 damage
    if atmo.co2 > DANGER_CO2 {
        damage += (atmo.co2 - DANGER_CO2) * 3.0;
    }

    // Extreme temperature damage
    if atmo.temperature > TEMP_MAX_DANGER {
        damage += (atmo.temperature - TEMP_MAX_DANGER) * 0.02;
    }
    if atmo.temperature < TEMP_MIN_DANGER {
        damage += (TEMP_MIN_DANGER - atmo.temperature) * 0.02;
    }

    damage
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_atmosphere_is_safe() {
        let atmo = RoomAtmosphere::default();
        assert_eq!(assess_hazard(&atmo), AtmosphereHazard::Safe);
    }

    #[test]
    fn test_low_o2_warning() {
        let atmo = RoomAtmosphere {
            o2: 0.15,
            ..Default::default()
        };
        assert_eq!(assess_hazard(&atmo), AtmosphereHazard::Warning);
    }

    #[test]
    fn test_critical_o2_danger() {
        let atmo = RoomAtmosphere {
            o2: 0.08,
            ..Default::default()
        };
        assert_eq!(assess_hazard(&atmo), AtmosphereHazard::Danger);
    }

    #[test]
    fn test_lethal_o2() {
        let atmo = RoomAtmosphere {
            o2: 0.03,
            ..Default::default()
        };
        assert_eq!(assess_hazard(&atmo), AtmosphereHazard::Lethal);
    }

    #[test]
    fn test_high_co2_warning() {
        let atmo = RoomAtmosphere {
            co2: 0.03,
            ..Default::default()
        };
        assert_eq!(assess_hazard(&atmo), AtmosphereHazard::Warning);
    }

    #[test]
    fn test_breathing_consumes_o2() {
        let mut atmo = RoomAtmosphere::default();
        atmo.has_life_support = false; // Disable LS to see raw effect
        let o2_before = atmo.o2;
        update_room_atmosphere(&mut atmo, 10, 1.0); // 10 people, 1 hour
        assert!(atmo.o2 < o2_before, "breathing should reduce O2");
        assert!(atmo.co2 > 0.0004, "breathing should increase CO2");
    }

    #[test]
    fn test_fire_consumes_o2() {
        let mut atmo = RoomAtmosphere {
            fire: true,
            has_life_support: false,
            ..Default::default()
        };
        let o2_before = atmo.o2;
        update_room_atmosphere(&mut atmo, 0, 1.0);
        assert!(atmo.o2 < o2_before, "fire should consume O2");
        assert!(atmo.temperature > 22.0, "fire should increase temp");
    }

    #[test]
    fn test_fire_self_extinguishes() {
        let mut atmo = RoomAtmosphere {
            o2: 0.06,
            fire: true,
            has_life_support: false,
            ..Default::default()
        };
        update_room_atmosphere(&mut atmo, 0, 1.0);
        assert!(!atmo.fire, "fire should extinguish when O2 is low");
    }

    #[test]
    fn test_life_support_restores_o2() {
        let mut atmo = RoomAtmosphere {
            o2: 0.15,
            has_life_support: true,
            ..Default::default()
        };
        update_room_atmosphere(&mut atmo, 0, 1.0);
        assert!(atmo.o2 > 0.15, "LS should restore O2");
    }

    #[test]
    fn test_sealed_room_no_life_support() {
        let mut atmo = RoomAtmosphere {
            o2: 0.15,
            has_life_support: true,
            sealed: true,
            ..Default::default()
        };
        update_room_atmosphere(&mut atmo, 0, 1.0);
        // Life support can't reach sealed rooms
        assert_eq!(atmo.o2, 0.15, "sealed room should not get LS O2");
    }

    #[test]
    fn test_gas_exchange_equalizes() {
        let mut a = RoomAtmosphere {
            o2: 0.10,
            ..Default::default()
        };
        let mut b = RoomAtmosphere::default(); // o2 = 0.21

        exchange_gas(&mut a, &mut b, 1.0);

        // Both should move toward average
        assert!(a.o2 > 0.10, "low room should gain O2");
        assert!(b.o2 < 0.21, "high room should lose O2");
    }

    #[test]
    fn test_gas_exchange_sealed_blocks() {
        let mut a = RoomAtmosphere {
            o2: 0.10,
            sealed: true,
            ..Default::default()
        };
        let mut b = RoomAtmosphere::default();

        exchange_gas(&mut a, &mut b, 1.0);

        assert_eq!(a.o2, 0.10, "sealed room should not exchange gas");
        assert_eq!(b.o2, 0.21);
    }

    #[test]
    fn test_no_health_damage_normal() {
        let atmo = RoomAtmosphere::default();
        assert_eq!(atmosphere_health_damage(&atmo), 0.0);
    }

    #[test]
    fn test_health_damage_low_o2() {
        let atmo = RoomAtmosphere {
            o2: 0.05,
            ..Default::default()
        };
        let dmg = atmosphere_health_damage(&atmo);
        assert!(dmg > 0.0, "low O2 should cause damage");
    }

    #[test]
    fn test_health_damage_extreme_temp() {
        let atmo = RoomAtmosphere {
            temperature: 60.0,
            ..Default::default()
        };
        let dmg = atmosphere_health_damage(&atmo);
        assert!(dmg > 0.0, "extreme heat should cause damage");
    }

    #[test]
    fn test_temperature_regulation() {
        let mut atmo = RoomAtmosphere {
            temperature: 30.0,
            has_life_support: true,
            ..Default::default()
        };
        update_room_atmosphere(&mut atmo, 0, 1.0);
        assert!(
            atmo.temperature < 30.0,
            "LS should cool the room toward 22°C"
        );
    }
}
