//! System variant definitions — specs for all configurable ship system types.
//!
//! Each ship system category (power, life support, food, water, etc.) has
//! multiple variants with different performance characteristics. The selection
//! algorithm (WS7-3) will choose variants based on MissionConfig parameters.

use serde::{Deserialize, Serialize};

/// Common spec shared by all system variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSpec {
    pub name: &'static str,
    pub description: &'static str,
    /// Output capacity (units/hour, varies by category).
    pub output: f32,
    /// Crew required to operate.
    pub crew_needed: u32,
    /// Power draw in kW.
    pub power_draw: f32,
    /// Mean time between failures in hours.
    pub mtbf_hours: f64,
    /// Mass in metric tons.
    pub mass_tons: f32,
    /// Room type required for installation.
    pub room_type: u8,
    /// Minimum tech level required.
    pub min_tech_level: u8,
}

// ============================================================================
// POWER SYSTEMS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PowerVariant {
    FissionReactor = 0,
    FusionReactor = 1,
    AntimatterReactor = 2,
    SolarArray = 3,
    RTG = 4,
}

impl PowerVariant {
    pub fn spec(&self) -> SystemSpec {
        use crate::constants::room_types;
        match self {
            Self::FissionReactor => SystemSpec {
                name: "Fission Reactor",
                description: "Uranium-based fission. Reliable, moderate output.",
                output: 500.0, // kW
                crew_needed: 6,
                power_draw: 0.0, // Produces power
                mtbf_hours: 40000.0,
                mass_tons: 200.0,
                room_type: room_types::REACTOR,
                min_tech_level: 1,
            },
            Self::FusionReactor => SystemSpec {
                name: "Fusion Reactor",
                description: "Deuterium-tritium fusion. High output, clean.",
                output: 2000.0,
                crew_needed: 10,
                power_draw: 0.0,
                mtbf_hours: 30000.0,
                mass_tons: 300.0,
                room_type: room_types::REACTOR,
                min_tech_level: 2,
            },
            Self::AntimatterReactor => SystemSpec {
                name: "Antimatter Reactor",
                description: "Matter-antimatter annihilation. Extreme output.",
                output: 10000.0,
                crew_needed: 15,
                power_draw: 0.0,
                mtbf_hours: 15000.0,
                mass_tons: 150.0,
                room_type: room_types::REACTOR,
                min_tech_level: 4,
            },
            Self::SolarArray => SystemSpec {
                name: "Solar Array",
                description: "Photovoltaic panels. Low output, zero fuel.",
                output: 50.0,
                crew_needed: 2,
                power_draw: 0.0,
                mtbf_hours: 100000.0,
                mass_tons: 50.0,
                room_type: room_types::ENGINEERING,
                min_tech_level: 1,
            },
            Self::RTG => SystemSpec {
                name: "Radioisotope Thermoelectric Generator",
                description: "Plutonium decay. Emergency backup, very reliable.",
                output: 20.0,
                crew_needed: 1,
                power_draw: 0.0,
                mtbf_hours: 200000.0,
                mass_tons: 5.0,
                room_type: room_types::BACKUP_REACTOR,
                min_tech_level: 1,
            },
        }
    }

    pub fn all() -> &'static [PowerVariant] {
        &[
            Self::FissionReactor,
            Self::FusionReactor,
            Self::AntimatterReactor,
            Self::SolarArray,
            Self::RTG,
        ]
    }
}

// ============================================================================
// LIFE SUPPORT SYSTEMS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum LifeSupportVariant {
    BasicElectrolysis = 0,
    AdvancedMOXIE = 1,
    BioregenerativeLSS = 2,
    CryogenicSeparation = 3,
}

impl LifeSupportVariant {
    pub fn spec(&self) -> SystemSpec {
        use crate::constants::room_types;
        match self {
            Self::BasicElectrolysis => SystemSpec {
                name: "Basic Electrolysis",
                description: "Water electrolysis for O2. Simple, proven.",
                output: 50.0, // kg O2/hour per unit
                crew_needed: 3,
                power_draw: 100.0,
                mtbf_hours: 50000.0,
                mass_tons: 20.0,
                room_type: room_types::ATMOSPHERE_PROCESSING,
                min_tech_level: 1,
            },
            Self::AdvancedMOXIE => SystemSpec {
                name: "Advanced MOXIE",
                description: "CO2 electrolysis. Dual-purpose O2 + CO removal.",
                output: 80.0,
                crew_needed: 4,
                power_draw: 150.0,
                mtbf_hours: 35000.0,
                mass_tons: 30.0,
                room_type: room_types::ATMOSPHERE_PROCESSING,
                min_tech_level: 2,
            },
            Self::BioregenerativeLSS => SystemSpec {
                name: "Bioregenerative Life Support",
                description: "Plant-based O2 + food production. Self-sustaining.",
                output: 120.0,
                crew_needed: 8,
                power_draw: 200.0,
                mtbf_hours: 20000.0,
                mass_tons: 100.0,
                room_type: room_types::HYDROPONICS,
                min_tech_level: 3,
            },
            Self::CryogenicSeparation => SystemSpec {
                name: "Cryogenic Air Separation",
                description: "Industrial-grade atmosphere processing. High capacity.",
                output: 200.0,
                crew_needed: 5,
                power_draw: 300.0,
                mtbf_hours: 40000.0,
                mass_tons: 60.0,
                room_type: room_types::ATMOSPHERE_PROCESSING,
                min_tech_level: 2,
            },
        }
    }

    pub fn all() -> &'static [LifeSupportVariant] {
        &[
            Self::BasicElectrolysis,
            Self::AdvancedMOXIE,
            Self::BioregenerativeLSS,
            Self::CryogenicSeparation,
        ]
    }
}

// ============================================================================
// FOOD PRODUCTION SYSTEMS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FoodVariant {
    BasicHydroponics = 0,
    AdvancedAeroponics = 1,
    CellularAgriculture = 2,
    SyntheticFood = 3,
}

impl FoodVariant {
    pub fn spec(&self) -> SystemSpec {
        use crate::constants::room_types;
        match self {
            Self::BasicHydroponics => SystemSpec {
                name: "Basic Hydroponics",
                description: "Soilless crop growth. Moderate yield.",
                output: 5.0, // kg food/hour per growth chamber
                crew_needed: 4,
                power_draw: 50.0,
                mtbf_hours: 60000.0,
                mass_tons: 40.0,
                room_type: room_types::HYDROPONICS,
                min_tech_level: 1,
            },
            Self::AdvancedAeroponics => SystemSpec {
                name: "Advanced Aeroponics",
                description: "Mist-based root feeding. High yield, less water.",
                output: 10.0,
                crew_needed: 5,
                power_draw: 80.0,
                mtbf_hours: 40000.0,
                mass_tons: 50.0,
                room_type: room_types::HYDROPONICS,
                min_tech_level: 2,
            },
            Self::CellularAgriculture => SystemSpec {
                name: "Cellular Agriculture",
                description: "Lab-grown meat and proteins. Compact, high output.",
                output: 15.0,
                crew_needed: 6,
                power_draw: 120.0,
                mtbf_hours: 25000.0,
                mass_tons: 30.0,
                room_type: room_types::LABORATORY,
                min_tech_level: 3,
            },
            Self::SyntheticFood => SystemSpec {
                name: "Synthetic Food Processor",
                description: "Molecular assembly. Any food type, low morale.",
                output: 25.0,
                crew_needed: 3,
                power_draw: 200.0,
                mtbf_hours: 30000.0,
                mass_tons: 20.0,
                room_type: room_types::GALLEY,
                min_tech_level: 4,
            },
        }
    }

    pub fn all() -> &'static [FoodVariant] {
        &[
            Self::BasicHydroponics,
            Self::AdvancedAeroponics,
            Self::CellularAgriculture,
            Self::SyntheticFood,
        ]
    }
}

// ============================================================================
// WATER SYSTEMS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum WaterVariant {
    BasicFiltration = 0,
    ReverseOsmosis = 1,
    AdvancedDistillation = 2,
    NanoFiltration = 3,
}

impl WaterVariant {
    pub fn spec(&self) -> SystemSpec {
        use crate::constants::room_types;
        match self {
            Self::BasicFiltration => SystemSpec {
                name: "Basic Filtration",
                description: "Mechanical + chemical filtration. 80% recovery.",
                output: 100.0, // liters/hour
                crew_needed: 2,
                power_draw: 30.0,
                mtbf_hours: 70000.0,
                mass_tons: 15.0,
                room_type: room_types::WATER_RECYCLING,
                min_tech_level: 1,
            },
            Self::ReverseOsmosis => SystemSpec {
                name: "Reverse Osmosis",
                description: "Membrane-based purification. 90% recovery.",
                output: 200.0,
                crew_needed: 3,
                power_draw: 60.0,
                mtbf_hours: 50000.0,
                mass_tons: 25.0,
                room_type: room_types::WATER_RECYCLING,
                min_tech_level: 1,
            },
            Self::AdvancedDistillation => SystemSpec {
                name: "Advanced Distillation",
                description: "Vacuum distillation. 95% recovery, high purity.",
                output: 300.0,
                crew_needed: 4,
                power_draw: 100.0,
                mtbf_hours: 40000.0,
                mass_tons: 40.0,
                room_type: room_types::WATER_RECYCLING,
                min_tech_level: 2,
            },
            Self::NanoFiltration => SystemSpec {
                name: "Nano-Filtration",
                description: "Molecular-scale filtering. 99% recovery.",
                output: 500.0,
                crew_needed: 3,
                power_draw: 80.0,
                mtbf_hours: 25000.0,
                mass_tons: 20.0,
                room_type: room_types::WATER_RECYCLING,
                min_tech_level: 4,
            },
        }
    }

    pub fn all() -> &'static [WaterVariant] {
        &[
            Self::BasicFiltration,
            Self::ReverseOsmosis,
            Self::AdvancedDistillation,
            Self::NanoFiltration,
        ]
    }
}

// ============================================================================
// DEFENSE SYSTEMS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DefenseVariant {
    PointDefense = 0,
    ShieldGenerator = 1,
    ArmorPlating = 2,
    ECM = 3,
}

impl DefenseVariant {
    pub fn spec(&self) -> SystemSpec {
        use crate::constants::room_types;
        match self {
            Self::PointDefense => SystemSpec {
                name: "Point Defense Turrets",
                description: "Kinetic interceptors for micro-meteorites.",
                output: 1.0, // threat interceptions/hour
                crew_needed: 4,
                power_draw: 50.0,
                mtbf_hours: 30000.0,
                mass_tons: 30.0,
                room_type: room_types::CIC,
                min_tech_level: 1,
            },
            Self::ShieldGenerator => SystemSpec {
                name: "Electromagnetic Shield",
                description: "EM field deflecting charged particles and radiation.",
                output: 1.0,
                crew_needed: 3,
                power_draw: 200.0,
                mtbf_hours: 20000.0,
                mass_tons: 50.0,
                room_type: room_types::ENGINEERING,
                min_tech_level: 3,
            },
            Self::ArmorPlating => SystemSpec {
                name: "Whipple Armor",
                description: "Multi-layer impact shielding. Passive, no power.",
                output: 1.0,
                crew_needed: 0,
                power_draw: 0.0,
                mtbf_hours: 500000.0,
                mass_tons: 500.0,
                room_type: room_types::ENGINEERING,
                min_tech_level: 1,
            },
            Self::ECM => SystemSpec {
                name: "Electronic Countermeasures",
                description: "Sensor jamming and threat deception.",
                output: 1.0,
                crew_needed: 3,
                power_draw: 80.0,
                mtbf_hours: 40000.0,
                mass_tons: 10.0,
                room_type: room_types::CIC,
                min_tech_level: 2,
            },
        }
    }

    pub fn all() -> &'static [DefenseVariant] {
        &[
            Self::PointDefense,
            Self::ShieldGenerator,
            Self::ArmorPlating,
            Self::ECM,
        ]
    }
}

// ============================================================================
// MEDICAL SYSTEMS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MedicalVariant {
    BasicInfirmary = 0,
    AdvancedHospital = 1,
    AutoDoc = 2,
    CryoMedBay = 3,
}

impl MedicalVariant {
    pub fn spec(&self) -> SystemSpec {
        use crate::constants::room_types;
        match self {
            Self::BasicInfirmary => SystemSpec {
                name: "Basic Infirmary",
                description: "Standard medical ward. Handles common ailments.",
                output: 5.0, // patients/hour capacity
                crew_needed: 4,
                power_draw: 20.0,
                mtbf_hours: 80000.0,
                mass_tons: 10.0,
                room_type: room_types::HOSPITAL_WARD,
                min_tech_level: 1,
            },
            Self::AdvancedHospital => SystemSpec {
                name: "Advanced Hospital",
                description: "Full surgical suite. Handles trauma and complex cases.",
                output: 15.0,
                crew_needed: 10,
                power_draw: 60.0,
                mtbf_hours: 50000.0,
                mass_tons: 30.0,
                room_type: room_types::SURGERY,
                min_tech_level: 2,
            },
            Self::AutoDoc => SystemSpec {
                name: "AutoDoc System",
                description: "AI-assisted diagnostics and robotic surgery.",
                output: 20.0,
                crew_needed: 3,
                power_draw: 100.0,
                mtbf_hours: 30000.0,
                mass_tons: 15.0,
                room_type: room_types::HOSPITAL_WARD,
                min_tech_level: 4,
            },
            Self::CryoMedBay => SystemSpec {
                name: "Cryo-Medical Bay",
                description: "Suspended animation for critical patients.",
                output: 8.0,
                crew_needed: 5,
                power_draw: 150.0,
                mtbf_hours: 25000.0,
                mass_tons: 40.0,
                room_type: room_types::HOSPITAL_WARD,
                min_tech_level: 3,
            },
        }
    }

    pub fn all() -> &'static [MedicalVariant] {
        &[
            Self::BasicInfirmary,
            Self::AdvancedHospital,
            Self::AutoDoc,
            Self::CryoMedBay,
        ]
    }
}

// ============================================================================
// GRAVITY SYSTEMS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum GravityVariant {
    RotationalHabitat = 0,
    MagneticFloor = 1,
    ArtificialGravityPlate = 2,
}

impl GravityVariant {
    pub fn spec(&self) -> SystemSpec {
        use crate::constants::room_types;
        match self {
            Self::RotationalHabitat => SystemSpec {
                name: "Rotational Habitat",
                description: "Centrifugal force via ship rotation. No power for gravity.",
                output: 1.0, // 1g equivalent
                crew_needed: 2,
                power_draw: 10.0, // Bearings/adjustment only
                mtbf_hours: 200000.0,
                mass_tons: 0.0, // Structural, not added mass
                room_type: room_types::ENGINEERING,
                min_tech_level: 1,
            },
            Self::MagneticFloor => SystemSpec {
                name: "Magnetic Floor System",
                description: "Electromagnetic floor plates. Requires magnetic boots.",
                output: 0.3,
                crew_needed: 3,
                power_draw: 100.0,
                mtbf_hours: 50000.0,
                mass_tons: 80.0,
                room_type: room_types::ENGINEERING,
                min_tech_level: 2,
            },
            Self::ArtificialGravityPlate => SystemSpec {
                name: "Artificial Gravity Plate",
                description: "Mass-effect gravity generation. Full 1g, any orientation.",
                output: 1.0,
                crew_needed: 4,
                power_draw: 500.0,
                mtbf_hours: 20000.0,
                mass_tons: 200.0,
                room_type: room_types::ENGINEERING,
                min_tech_level: 5,
            },
        }
    }

    pub fn all() -> &'static [GravityVariant] {
        &[
            Self::RotationalHabitat,
            Self::MagneticFloor,
            Self::ArtificialGravityPlate,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_variant_count() {
        assert_eq!(PowerVariant::all().len(), 5);
    }

    #[test]
    fn test_power_output_ordering() {
        let fission = PowerVariant::FissionReactor.spec().output;
        let fusion = PowerVariant::FusionReactor.spec().output;
        let am = PowerVariant::AntimatterReactor.spec().output;
        assert!(fission < fusion);
        assert!(fusion < am);
    }

    #[test]
    fn test_all_variants_have_names() {
        for v in PowerVariant::all() {
            assert!(!v.spec().name.is_empty());
        }
        for v in LifeSupportVariant::all() {
            assert!(!v.spec().name.is_empty());
        }
        for v in FoodVariant::all() {
            assert!(!v.spec().name.is_empty());
        }
        for v in WaterVariant::all() {
            assert!(!v.spec().name.is_empty());
        }
        for v in DefenseVariant::all() {
            assert!(!v.spec().name.is_empty());
        }
        for v in MedicalVariant::all() {
            assert!(!v.spec().name.is_empty());
        }
        for v in GravityVariant::all() {
            assert!(!v.spec().name.is_empty());
        }
    }

    #[test]
    fn test_tech_level_range() {
        // All variants should have tech level 1-5
        for v in PowerVariant::all() {
            let tl = v.spec().min_tech_level;
            assert!(tl >= 1 && tl <= 5, "{}: tech level {tl}", v.spec().name);
        }
    }

    #[test]
    fn test_total_variant_count() {
        let total = PowerVariant::all().len()
            + LifeSupportVariant::all().len()
            + FoodVariant::all().len()
            + WaterVariant::all().len()
            + DefenseVariant::all().len()
            + MedicalVariant::all().len()
            + GravityVariant::all().len();
        assert_eq!(total, 28);
    }

    #[test]
    fn test_power_draw_consistency() {
        // Power generators should have 0 power draw
        for v in PowerVariant::all() {
            assert_eq!(v.spec().power_draw, 0.0, "{} draws power", v.spec().name);
        }
        // Non-power systems should draw power (except passive armor)
        for v in LifeSupportVariant::all() {
            assert!(v.spec().power_draw > 0.0, "{} has no draw", v.spec().name);
        }
    }
}
