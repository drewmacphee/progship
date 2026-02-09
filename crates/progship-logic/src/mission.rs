//! Mission configuration — destination, propulsion, voyage profile.
//!
//! Defines the high-level mission parameters that drive ship generation:
//! where the colony ship is going, how it gets there, and what it needs.

use serde::{Deserialize, Serialize};

// ============================================================================
// DESTINATIONS
// ============================================================================

/// Target star system for the colony mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Destination {
    /// Proxima Centauri b — closest, harsh conditions.
    ProximaCentauri = 0,
    /// Barnard's Star — cold, rocky planets.
    BarnardsStar = 1,
    /// Wolf 359 — dim red dwarf, low habitability.
    Wolf359 = 2,
    /// Luyten's Star — temperate super-earth candidate.
    LuytensStar = 3,
    /// Tau Ceti e — habitable zone, promising.
    TauCeti = 4,
    /// Epsilon Eridani — young system, resource-rich.
    EpsilonEridani = 5,
    /// 61 Cygni — binary system, extreme conditions.
    SixtyOneCygni = 6,
    /// Kepler-442b — distant, high habitability.
    Kepler442b = 7,
}

/// Destination metadata.
#[derive(Debug, Clone)]
pub struct DestinationInfo {
    pub name: &'static str,
    pub distance_ly: f64,
    pub habitability: f32,
    pub resource_richness: f32,
}

impl Destination {
    pub fn info(&self) -> DestinationInfo {
        match self {
            Self::ProximaCentauri => DestinationInfo {
                name: "Proxima Centauri b",
                distance_ly: 4.24,
                habitability: 0.4,
                resource_richness: 0.5,
            },
            Self::BarnardsStar => DestinationInfo {
                name: "Barnard's Star",
                distance_ly: 5.98,
                habitability: 0.3,
                resource_richness: 0.6,
            },
            Self::Wolf359 => DestinationInfo {
                name: "Wolf 359",
                distance_ly: 7.86,
                habitability: 0.2,
                resource_richness: 0.4,
            },
            Self::LuytensStar => DestinationInfo {
                name: "Luyten's Star",
                distance_ly: 12.36,
                habitability: 0.7,
                resource_richness: 0.6,
            },
            Self::TauCeti => DestinationInfo {
                name: "Tau Ceti e",
                distance_ly: 11.91,
                habitability: 0.8,
                resource_richness: 0.7,
            },
            Self::EpsilonEridani => DestinationInfo {
                name: "Epsilon Eridani",
                distance_ly: 10.47,
                habitability: 0.5,
                resource_richness: 0.9,
            },
            Self::SixtyOneCygni => DestinationInfo {
                name: "61 Cygni",
                distance_ly: 11.41,
                habitability: 0.3,
                resource_richness: 0.5,
            },
            Self::Kepler442b => DestinationInfo {
                name: "Kepler-442b",
                distance_ly: 100.0,
                habitability: 0.9,
                resource_richness: 0.8,
            },
        }
    }

    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::ProximaCentauri),
            1 => Some(Self::BarnardsStar),
            2 => Some(Self::Wolf359),
            3 => Some(Self::LuytensStar),
            4 => Some(Self::TauCeti),
            5 => Some(Self::EpsilonEridani),
            6 => Some(Self::SixtyOneCygni),
            7 => Some(Self::Kepler442b),
            _ => None,
        }
    }
}

// ============================================================================
// PROPULSION
// ============================================================================

/// Propulsion system type determining cruise velocity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PropulsionType {
    /// Nuclear pulse (Orion-type). Slow but proven.
    NuclearPulse = 0,
    /// Fusion torch drive. Moderate speed.
    FusionTorch = 1,
    /// Antimatter catalyzed fusion. Fast.
    AntimatterCatalyzed = 2,
    /// Bussard ramjet. Uses interstellar hydrogen.
    BussardRamjet = 3,
    /// Laser sail with deceleration stage. Very fast.
    LaserSail = 4,
    /// Alcubierre-inspired warp bubble. Fastest, experimental.
    WarpBubble = 5,
}

/// Propulsion specs.
#[derive(Debug, Clone)]
pub struct PropulsionSpec {
    pub name: &'static str,
    /// Cruise velocity as fraction of c.
    pub cruise_velocity_c: f64,
    /// Power draw in MW.
    pub power_draw_mw: f64,
    /// Fuel consumption rate (kg/hour).
    pub fuel_rate: f64,
    /// Crew required to operate.
    pub crew_required: u32,
    /// Reliability (MTBF in hours).
    pub mtbf_hours: f64,
}

impl PropulsionType {
    pub fn spec(&self) -> PropulsionSpec {
        match self {
            Self::NuclearPulse => PropulsionSpec {
                name: "Nuclear Pulse Drive",
                cruise_velocity_c: 0.03,
                power_draw_mw: 50.0,
                fuel_rate: 10.0,
                crew_required: 8,
                mtbf_hours: 50000.0,
            },
            Self::FusionTorch => PropulsionSpec {
                name: "Fusion Torch Drive",
                cruise_velocity_c: 0.05,
                power_draw_mw: 200.0,
                fuel_rate: 5.0,
                crew_required: 12,
                mtbf_hours: 30000.0,
            },
            Self::AntimatterCatalyzed => PropulsionSpec {
                name: "Antimatter-Catalyzed Fusion",
                cruise_velocity_c: 0.10,
                power_draw_mw: 500.0,
                fuel_rate: 0.5,
                crew_required: 15,
                mtbf_hours: 20000.0,
            },
            Self::BussardRamjet => PropulsionSpec {
                name: "Bussard Ramjet",
                cruise_velocity_c: 0.08,
                power_draw_mw: 100.0,
                fuel_rate: 0.0, // Collects fuel from space
                crew_required: 10,
                mtbf_hours: 25000.0,
            },
            Self::LaserSail => PropulsionSpec {
                name: "Laser Sail + Decel Stage",
                cruise_velocity_c: 0.15,
                power_draw_mw: 10.0, // Minimal onboard
                fuel_rate: 0.1,
                crew_required: 5,
                mtbf_hours: 100000.0,
            },
            Self::WarpBubble => PropulsionSpec {
                name: "Warp Bubble Drive",
                cruise_velocity_c: 1.0,
                power_draw_mw: 5000.0,
                fuel_rate: 50.0,
                crew_required: 20,
                mtbf_hours: 10000.0,
            },
        }
    }

    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::NuclearPulse),
            1 => Some(Self::FusionTorch),
            2 => Some(Self::AntimatterCatalyzed),
            3 => Some(Self::BussardRamjet),
            4 => Some(Self::LaserSail),
            5 => Some(Self::WarpBubble),
            _ => None,
        }
    }
}

// ============================================================================
// MISSION CONFIG
// ============================================================================

/// Top-level mission configuration that drives all ship generation decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionConfig {
    /// Target star system.
    pub destination: u8,
    /// Target colony population on arrival.
    pub colony_target_pop: u32,
    /// Technology level (1-5, affects system variants available).
    pub tech_level: u8,
    /// Budget class (1=austere, 2=standard, 3=premium).
    pub budget_class: u8,
    /// Mission priority weighting.
    pub mission_priority: MissionPriority,
    /// Random seed for deterministic generation.
    pub seed: u64,
    /// Propulsion system selection.
    pub propulsion: u8,
}

/// Mission priority weighting — what matters most for this colony.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionPriority {
    /// Weight for crew safety and redundancy.
    pub safety: f32,
    /// Weight for speed (shorter voyage).
    pub speed: f32,
    /// Weight for comfort and morale.
    pub comfort: f32,
    /// Weight for science and exploration capability.
    pub science: f32,
    /// Weight for self-sufficiency and resource independence.
    pub self_sufficiency: f32,
}

impl Default for MissionPriority {
    fn default() -> Self {
        Self {
            safety: 1.0,
            speed: 1.0,
            comfort: 1.0,
            science: 1.0,
            self_sufficiency: 1.0,
        }
    }
}

impl Default for MissionConfig {
    fn default() -> Self {
        Self {
            destination: Destination::TauCeti as u8,
            colony_target_pop: 5000,
            tech_level: 3,
            budget_class: 2,
            mission_priority: MissionPriority::default(),
            seed: 42,
            propulsion: PropulsionType::FusionTorch as u8,
        }
    }
}

// ============================================================================
// VOYAGE PROFILE (computed from MissionConfig)
// ============================================================================

/// Computed voyage parameters derived from mission config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoyageProfile {
    /// Distance to destination in light-years.
    pub distance_ly: f64,
    /// Cruise velocity as fraction of c.
    pub cruise_velocity_c: f64,
    /// Estimated voyage duration in years.
    pub duration_years: f64,
    /// Estimated voyage duration in hours (for simulation).
    pub duration_hours: f64,
    /// Destination habitability score.
    pub habitability: f32,
    /// Destination resource richness score.
    pub resource_richness: f32,
}

/// Compute voyage profile from mission config.
pub fn compute_voyage(config: &MissionConfig) -> VoyageProfile {
    let dest = Destination::from_u8(config.destination).unwrap_or(Destination::TauCeti);
    let dest_info = dest.info();

    let prop = PropulsionType::from_u8(config.propulsion).unwrap_or(PropulsionType::FusionTorch);
    let prop_spec = prop.spec();

    // Simple calculation: distance / velocity
    // Ignoring acceleration/deceleration phases for now
    let duration_years = dest_info.distance_ly / prop_spec.cruise_velocity_c;
    let duration_hours = duration_years * 365.25 * 24.0;

    VoyageProfile {
        distance_ly: dest_info.distance_ly,
        cruise_velocity_c: prop_spec.cruise_velocity_c,
        duration_years,
        duration_hours,
        habitability: dest_info.habitability,
        resource_richness: dest_info.resource_richness,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destination_roundtrip() {
        for i in 0..8u8 {
            let dest = Destination::from_u8(i).unwrap();
            assert_eq!(dest as u8, i);
        }
        assert!(Destination::from_u8(99).is_none());
    }

    #[test]
    fn test_destination_info() {
        let info = Destination::ProximaCentauri.info();
        assert!((info.distance_ly - 4.24).abs() < 0.01);
        assert!(info.habitability > 0.0);
    }

    #[test]
    fn test_propulsion_roundtrip() {
        for i in 0..6u8 {
            let prop = PropulsionType::from_u8(i).unwrap();
            assert_eq!(prop as u8, i);
        }
        assert!(PropulsionType::from_u8(99).is_none());
    }

    #[test]
    fn test_propulsion_velocity_ordering() {
        // Faster propulsion types should have higher velocity
        let pulse = PropulsionType::NuclearPulse.spec().cruise_velocity_c;
        let fusion = PropulsionType::FusionTorch.spec().cruise_velocity_c;
        let am = PropulsionType::AntimatterCatalyzed.spec().cruise_velocity_c;
        assert!(pulse < fusion);
        assert!(fusion < am);
    }

    #[test]
    fn test_voyage_profile_default() {
        let config = MissionConfig::default();
        let profile = compute_voyage(&config);
        // Tau Ceti at 11.91 ly, Fusion Torch at 0.05c
        assert!((profile.distance_ly - 11.91).abs() < 0.01);
        assert!((profile.cruise_velocity_c - 0.05).abs() < 0.001);
        // ~238 years
        assert!(profile.duration_years > 200.0);
        assert!(profile.duration_years < 300.0);
        assert!(profile.duration_hours > 0.0);
    }

    #[test]
    fn test_voyage_proxima_warp() {
        let config = MissionConfig {
            destination: Destination::ProximaCentauri as u8,
            propulsion: PropulsionType::WarpBubble as u8,
            ..MissionConfig::default()
        };
        let profile = compute_voyage(&config);
        // 4.24 ly at 1.0c = ~4.24 years
        assert!((profile.duration_years - 4.24).abs() < 0.1);
    }

    #[test]
    fn test_voyage_kepler_slow() {
        let config = MissionConfig {
            destination: Destination::Kepler442b as u8,
            propulsion: PropulsionType::NuclearPulse as u8,
            ..MissionConfig::default()
        };
        let profile = compute_voyage(&config);
        // 100 ly at 0.03c = ~3333 years
        assert!(profile.duration_years > 3000.0);
    }

    #[test]
    fn test_mission_priority_default() {
        let p = MissionPriority::default();
        assert_eq!(p.safety, 1.0);
        assert_eq!(p.speed, 1.0);
    }
}
