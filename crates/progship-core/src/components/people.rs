//! People-related components: Person, Crew, Passenger, Needs, Personality, etc.

use serde::{Deserialize, Serialize};

/// Marker component identifying an entity as a person
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Person;

/// Needs that drive behavior - all values 0.0 (satisfied) to 1.0 (desperate)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Needs {
    pub hunger: f32,
    pub fatigue: f32,
    pub social: f32,
    pub comfort: f32,
    pub hygiene: f32,
}

impl Needs {
    /// Returns the most urgent need above the threshold
    pub fn most_urgent(&self, threshold: f32) -> Option<NeedType> {
        let needs = [
            (NeedType::Hunger, self.hunger),
            (NeedType::Fatigue, self.fatigue),
            (NeedType::Social, self.social),
            (NeedType::Comfort, self.comfort),
            (NeedType::Hygiene, self.hygiene),
        ];

        needs
            .iter()
            .filter(|(_, v)| *v > threshold)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(t, _)| *t)
    }

    /// Apply decay over time (needs increase)
    pub fn decay(&mut self, hours: f32) {
        // Rates: how many hours until need reaches 1.0 from 0.0
        self.hunger = (self.hunger + hours / 8.0).clamp(0.0, 1.0);   // Hungry after 8 hours
        self.fatigue = (self.fatigue + hours / 16.0).clamp(0.0, 1.0); // Tired after 16 hours
        self.social = (self.social + hours / 48.0).clamp(0.0, 1.0);   // Lonely after 48 hours
        self.comfort = (self.comfort + hours / 24.0).clamp(0.0, 1.0); // Uncomfortable after 24 hours
        self.hygiene = (self.hygiene + hours / 12.0).clamp(0.0, 1.0); // Needs shower after 12 hours
    }

    /// Satisfy a specific need
    pub fn satisfy(&mut self, need: NeedType, amount: f32) {
        let value = match need {
            NeedType::Hunger => &mut self.hunger,
            NeedType::Fatigue => &mut self.fatigue,
            NeedType::Social => &mut self.social,
            NeedType::Comfort => &mut self.comfort,
            NeedType::Hygiene => &mut self.hygiene,
        };
        *value = (*value - amount).clamp(0.0, 1.0);
    }
}

/// Types of needs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NeedType {
    Hunger,
    Fatigue,
    Social,
    Comfort,
    Hygiene,
}

/// Big Five personality traits - values from -1.0 to 1.0
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Personality {
    pub openness: f32,
    pub conscientiousness: f32,
    pub extraversion: f32,
    pub agreeableness: f32,
    pub neuroticism: f32,
}

impl Personality {
    /// Generate a random personality
    pub fn random(rng: &mut impl rand::Rng) -> Self {
        Self {
            openness: rng.gen_range(-1.0..=1.0),
            conscientiousness: rng.gen_range(-1.0..=1.0),
            extraversion: rng.gen_range(-1.0..=1.0),
            agreeableness: rng.gen_range(-1.0..=1.0),
            neuroticism: rng.gen_range(-1.0..=1.0),
        }
    }
}

/// Skills that affect job performance
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Skills {
    pub engineering: f32,
    pub medical: f32,
    pub piloting: f32,
    pub science: f32,
    pub social: f32,
    pub combat: f32,
}

impl Skills {
    /// Generate random skills with optional bias toward a specialty
    pub fn random(rng: &mut impl rand::Rng, specialty: Option<SkillType>) -> Self {
        let mut skills = Self {
            engineering: rng.gen_range(0.0..0.5),
            medical: rng.gen_range(0.0..0.5),
            piloting: rng.gen_range(0.0..0.5),
            science: rng.gen_range(0.0..0.5),
            social: rng.gen_range(0.0..0.5),
            combat: rng.gen_range(0.0..0.5),
        };

        // Boost specialty
        if let Some(spec) = specialty {
            let skill = match spec {
                SkillType::Engineering => &mut skills.engineering,
                SkillType::Medical => &mut skills.medical,
                SkillType::Piloting => &mut skills.piloting,
                SkillType::Science => &mut skills.science,
                SkillType::Social => &mut skills.social,
                SkillType::Combat => &mut skills.combat,
            };
            *skill = rng.gen_range(0.6..1.0);
        }

        skills
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillType {
    Engineering,
    Medical,
    Piloting,
    Science,
    Social,
    Combat,
}

/// Crew member component - only attached to crew (not passengers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crew {
    pub department: Department,
    pub rank: Rank,
    pub shift: Shift,
    /// Room ID of duty station
    pub duty_station_id: u32,
}

impl Crew {
    pub fn new(department: Department, rank: Rank, shift: Shift) -> Self {
        Self {
            department,
            rank,
            shift,
            duty_station_id: 0,
        }
    }

    pub fn with_station(mut self, station_id: u32) -> Self {
        self.duty_station_id = station_id;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Department {
    Command,
    Engineering,
    Medical,
    Science,
    Security,
    Operations,
    Civilian,
}

impl Department {
    pub fn primary_skill(&self) -> SkillType {
        match self {
            Department::Command => SkillType::Social,
            Department::Engineering => SkillType::Engineering,
            Department::Medical => SkillType::Medical,
            Department::Science => SkillType::Science,
            Department::Security => SkillType::Combat,
            Department::Operations => SkillType::Social,
            Department::Civilian => SkillType::Social,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rank {
    Crewman,
    Specialist,
    Petty,
    Chief,
    Ensign,
    Lieutenant,
    Commander,
    Captain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Shift {
    /// 0600-1400
    Alpha,
    /// 1400-2200
    Beta,
    /// 2200-0600
    Gamma,
}

impl Shift {
    /// Returns true if this shift is currently active
    pub fn is_active(&self, hour: f32) -> bool {
        let hour = hour % 24.0;
        match self {
            Shift::Alpha => (6.0..14.0).contains(&hour),
            Shift::Beta => (14.0..22.0).contains(&hour),
            Shift::Gamma => hour >= 22.0 || hour < 6.0,
        }
    }
}

/// Passenger component - only attached to passengers (not crew)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Passenger {
    pub cabin_class: CabinClass,
    pub destination: String,
    pub profession: String,
}

impl Passenger {
    pub fn new(cabin_class: CabinClass) -> Self {
        Self {
            cabin_class,
            destination: String::new(),
            profession: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CabinClass {
    First,
    Standard,
    Steerage,
}

/// Current activity component - present when performing an activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub activity_type: ActivityType,
    pub started_at: f64,
    pub duration: f32,
    /// Optional target entity (workstation, person, etc.)
    pub target_id: Option<u32>,
}

impl Activity {
    pub fn new(activity_type: ActivityType, started_at: f64, duration: f32) -> Self {
        Self {
            activity_type,
            started_at,
            duration,
            target_id: None,
        }
    }

    pub fn is_complete(&self, current_time: f64) -> bool {
        current_time - self.started_at >= self.duration as f64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActivityType {
    Idle,
    Working,
    Eating,
    Sleeping,
    Socializing,
    Relaxing,
    Hygiene,
    Traveling,
    Maintenance,
    /// On duty at assigned station
    OnDuty,
    /// Off duty, free time
    OffDuty,
    /// Responding to emergency
    Emergency,
}

impl ActivityType {
    /// Returns which need this activity satisfies (if any)
    pub fn satisfies(&self) -> Option<NeedType> {
        match self {
            ActivityType::Eating => Some(NeedType::Hunger),
            ActivityType::Sleeping => Some(NeedType::Fatigue),
            ActivityType::Socializing => Some(NeedType::Social),
            ActivityType::Relaxing => Some(NeedType::Comfort),
            ActivityType::Hygiene => Some(NeedType::Hygiene),
            _ => None,
        }
    }
    
    /// Returns true if this activity can be interrupted for duty
    pub fn interruptible_for_duty(&self) -> bool {
        match self {
            ActivityType::Sleeping => false, // Don't wake for duty if very tired
            ActivityType::Emergency => false, // Emergencies take priority
            ActivityType::Maintenance => false, // Finish repairs
            _ => true,
        }
    }
}

/// Faction affiliation for social groupings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Faction {
    /// Ship's command structure
    Command,
    /// Engineering department
    Engineering,
    /// Medical staff
    Medical,
    /// Science team
    Science,
    /// Security personnel
    Security,
    /// Operations/support
    Operations,
    /// First-class passengers
    PassengersFirst,
    /// Standard passengers
    PassengersStandard,
    /// Steerage passengers
    PassengersSteerage,
}

impl Faction {
    /// Convert from Department to Faction
    pub fn from_department(dept: Department) -> Self {
        match dept {
            Department::Command => Faction::Command,
            Department::Engineering => Faction::Engineering,
            Department::Medical => Faction::Medical,
            Department::Science => Faction::Science,
            Department::Security => Faction::Security,
            Department::Operations => Faction::Operations,
            Department::Civilian => Faction::Operations,
        }
    }
    
    /// Convert from CabinClass to Faction
    pub fn from_cabin_class(class: CabinClass) -> Self {
        match class {
            CabinClass::First => Faction::PassengersFirst,
            CabinClass::Standard => Faction::PassengersStandard,
            CabinClass::Steerage => Faction::PassengersSteerage,
        }
    }
    
    /// Base affinity between factions (-1.0 to 1.0)
    /// Positive = friendly, Negative = rivalry
    pub fn affinity_with(&self, other: &Faction) -> f32 {
        if self == other {
            return 1.0; // Same faction = high affinity
        }
        
        match (self, other) {
            // Crew departments generally get along
            (Faction::Command, Faction::Security) => 0.5,
            (Faction::Engineering, Faction::Science) => 0.4,
            (Faction::Medical, _) if other.is_crew() => 0.3,
            
            // Passenger classes have some tension
            (Faction::PassengersFirst, Faction::PassengersSteerage) => -0.2,
            (Faction::PassengersSteerage, Faction::PassengersFirst) => -0.2,
            
            // Crew and passengers are neutral to slightly wary
            _ if self.is_crew() && other.is_passenger() => 0.0,
            _ if self.is_passenger() && other.is_crew() => 0.1,
            
            // Default neutral
            _ => 0.1,
        }
    }
    
    pub fn is_crew(&self) -> bool {
        matches!(self, Faction::Command | Faction::Engineering | Faction::Medical 
            | Faction::Science | Faction::Security | Faction::Operations)
    }
    
    pub fn is_passenger(&self) -> bool {
        matches!(self, Faction::PassengersFirst | Faction::PassengersStandard | Faction::PassengersSteerage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_decay() {
        let mut needs = Needs::default();
        needs.decay(8.0);
        assert!((needs.hunger - 1.0).abs() < 0.01); // Should be hungry after 8 hours
        assert!(needs.fatigue < 1.0); // Not yet exhausted
    }

    #[test]
    fn test_needs_most_urgent() {
        let mut needs = Needs::default();
        needs.hunger = 0.9;
        needs.fatigue = 0.5;
        
        assert_eq!(needs.most_urgent(0.3), Some(NeedType::Hunger));
        assert_eq!(needs.most_urgent(0.95), None);
    }

    #[test]
    fn test_shift_is_active() {
        assert!(Shift::Alpha.is_active(10.0));
        assert!(!Shift::Alpha.is_active(20.0));
        assert!(Shift::Gamma.is_active(23.0));
        assert!(Shift::Gamma.is_active(3.0));
    }

    #[test]
    fn test_activity_complete() {
        let activity = Activity::new(ActivityType::Eating, 0.0, 0.5);
        assert!(!activity.is_complete(0.3));
        assert!(activity.is_complete(0.6));
    }
}
