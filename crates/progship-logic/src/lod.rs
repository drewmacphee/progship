//! Level-of-detail (LOD) simulation tiers for population scale-up.
//!
//! At 5,000+ agents, running every system at full frequency for every
//! agent is prohibitive. This module provides pure functions that
//! classify agents into simulation tiers based on spatial relevance,
//! then control which systems run for each tier.
//!
//! # Tiers
//!
//! | Tier | Who | Activity | Movement | Needs | Social |
//! |------|-----|----------|----------|-------|--------|
//! | `Full` | Same deck as camera | 1 Hz | 60 Hz | 0.1 Hz | 1 Hz |
//! | `Nearby` | Adjacent decks | 1 Hz | 10 Hz | 0.1 Hz | 0.5 Hz |
//! | `Background` | All other decks | 0.1 Hz | skip | 0.1 Hz | skip |
//! | `Dormant` | Sleeping off-camera | skip | skip | 0.01 Hz | skip |
//!
//! # Usage
//!
//! ```
//! use progship_logic::lod::{LodTier, LodSystem, classify_agent, should_update, LodConfig};
//!
//! let config = LodConfig::default();
//! let tier = classify_agent(3, 3, &[2, 3, 4], true);
//! assert_eq!(tier, LodTier::Full);
//! assert!(should_update(tier, LodSystem::Movement, 0, &config));
//! ```

use serde::{Deserialize, Serialize};

/// Simulation tier for an agent, determining update frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LodTier {
    /// On the currently viewed deck — full simulation.
    Full,
    /// On an adjacent deck — reduced movement, full activity.
    Nearby,
    /// Far from camera — minimal updates, no movement interpolation.
    Background,
    /// Sleeping agent far from camera — near-zero updates.
    Dormant,
}

/// Simulation subsystem, each with its own per-tier frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodSystem {
    /// Position interpolation (T0, nominally 60 Hz).
    Movement,
    /// Activity state machine (T1, nominally 1 Hz).
    Activity,
    /// Needs decay (T2, nominally 0.1 Hz).
    Needs,
    /// Social / conversation checks.
    Social,
    /// Atmosphere exposure effects.
    Atmosphere,
}

/// Per-system tick intervals for each LOD tier.
///
/// An interval of `N` means "run once every N ticks". A value of `0`
/// means the system is skipped entirely for that tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LodConfig {
    /// Tick intervals for [`LodTier::Full`] agents.
    pub full: TierIntervals,
    /// Tick intervals for [`LodTier::Nearby`] agents.
    pub nearby: TierIntervals,
    /// Tick intervals for [`LodTier::Background`] agents.
    pub background: TierIntervals,
    /// Tick intervals for [`LodTier::Dormant`] agents.
    pub dormant: TierIntervals,
}

/// Per-system tick intervals within a single tier.
///
/// An interval of `1` means every tick, `10` means every 10th tick,
/// and `0` means the system is disabled for this tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierIntervals {
    pub movement: u32,
    pub activity: u32,
    pub needs: u32,
    pub social: u32,
    pub atmosphere: u32,
}

impl Default for LodConfig {
    fn default() -> Self {
        Self {
            full: TierIntervals {
                movement: 1,  // every tick (60 Hz)
                activity: 60, // ~1 Hz
                needs: 600,   // ~0.1 Hz
                social: 60,   // ~1 Hz
                atmosphere: 600,
            },
            nearby: TierIntervals {
                movement: 6,  // ~10 Hz
                activity: 60, // ~1 Hz
                needs: 600,   // ~0.1 Hz
                social: 120,  // ~0.5 Hz
                atmosphere: 600,
            },
            background: TierIntervals {
                movement: 0,   // skip
                activity: 600, // ~0.1 Hz
                needs: 600,    // ~0.1 Hz
                social: 0,     // skip
                atmosphere: 1200,
            },
            dormant: TierIntervals {
                movement: 0,   // skip
                activity: 0,   // skip
                needs: 6000,   // ~0.01 Hz
                social: 0,     // skip
                atmosphere: 0, // skip
            },
        }
    }
}

impl TierIntervals {
    /// Get the tick interval for a specific system.
    pub fn interval_for(&self, system: LodSystem) -> u32 {
        match system {
            LodSystem::Movement => self.movement,
            LodSystem::Activity => self.activity,
            LodSystem::Needs => self.needs,
            LodSystem::Social => self.social,
            LodSystem::Atmosphere => self.atmosphere,
        }
    }
}

impl LodConfig {
    /// Get the intervals for a specific tier.
    pub fn intervals_for(&self, tier: LodTier) -> &TierIntervals {
        match tier {
            LodTier::Full => &self.full,
            LodTier::Nearby => &self.nearby,
            LodTier::Background => &self.background,
            LodTier::Dormant => &self.dormant,
        }
    }
}

/// Classify an agent into a LOD tier based on deck proximity and state.
///
/// # Arguments
///
/// * `agent_deck` — the deck the agent is currently on
/// * `camera_deck` — the deck the player/camera is viewing
/// * `adjacent_decks` — decks considered "nearby" (e.g. connected by shaft)
/// * `is_sleeping` — whether the agent is currently in a sleep activity
pub fn classify_agent(
    agent_deck: u32,
    camera_deck: u32,
    adjacent_decks: &[u32],
    is_sleeping: bool,
) -> LodTier {
    if agent_deck == camera_deck {
        return LodTier::Full;
    }
    if adjacent_decks.contains(&agent_deck) {
        return LodTier::Nearby;
    }
    if is_sleeping {
        return LodTier::Dormant;
    }
    LodTier::Background
}

/// Determine whether a system should update for a given tier on this tick.
///
/// Returns `true` if `tick % interval == 0` (or if interval is 1).
/// Returns `false` if the interval is 0 (system disabled for this tier).
///
/// To avoid all agents in the same tier updating on the same tick,
/// use [`should_update_staggered`] which offsets by agent ID.
pub fn should_update(tier: LodTier, system: LodSystem, tick: u64, config: &LodConfig) -> bool {
    let interval = config.intervals_for(tier).interval_for(system);
    if interval == 0 {
        return false;
    }
    tick.is_multiple_of(u64::from(interval))
}

/// Like [`should_update`] but staggers updates across agents to avoid
/// "thundering herd" spikes where all background agents update on the
/// same tick.
///
/// Uses `agent_id % interval` as an offset so agents spread evenly
/// across the interval window.
pub fn should_update_staggered(
    tier: LodTier,
    system: LodSystem,
    tick: u64,
    agent_id: u32,
    config: &LodConfig,
) -> bool {
    let interval = config.intervals_for(tier).interval_for(system);
    if interval == 0 {
        return false;
    }
    let offset = u64::from(agent_id % interval);
    tick % u64::from(interval) == offset
}

/// Summary statistics for agent tier distribution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LodStats {
    pub full_count: usize,
    pub nearby_count: usize,
    pub background_count: usize,
    pub dormant_count: usize,
}

impl LodStats {
    /// Total number of agents across all tiers.
    pub fn total(&self) -> usize {
        self.full_count + self.nearby_count + self.background_count + self.dormant_count
    }

    /// Estimated relative simulation cost (full = 1.0 per agent).
    ///
    /// Nearby ≈ 0.5, background ≈ 0.1, dormant ≈ 0.01.
    pub fn estimated_cost(&self) -> f64 {
        self.full_count as f64
            + self.nearby_count as f64 * 0.5
            + self.background_count as f64 * 0.1
            + self.dormant_count as f64 * 0.01
    }
}

/// Classify a batch of agents and return tier distribution stats.
///
/// Each agent is represented as `(deck_id, is_sleeping)`.
pub fn classify_batch(
    agents: &[(u32, bool)],
    camera_deck: u32,
    adjacent_decks: &[u32],
) -> LodStats {
    let mut stats = LodStats::default();
    for &(deck, sleeping) in agents {
        match classify_agent(deck, camera_deck, adjacent_decks, sleeping) {
            LodTier::Full => stats.full_count += 1,
            LodTier::Nearby => stats.nearby_count += 1,
            LodTier::Background => stats.background_count += 1,
            LodTier::Dormant => stats.dormant_count += 1,
        }
    }
    stats
}

/// Partition agent indices into per-tier buckets for batch processing.
///
/// Returns `(full, nearby, background, dormant)` vectors of indices
/// into the original `agents` slice.
pub fn partition_by_tier(
    agents: &[(u32, bool)],
    camera_deck: u32,
    adjacent_decks: &[u32],
) -> (Vec<usize>, Vec<usize>, Vec<usize>, Vec<usize>) {
    let mut full = Vec::new();
    let mut nearby = Vec::new();
    let mut background = Vec::new();
    let mut dormant = Vec::new();

    for (i, &(deck, sleeping)) in agents.iter().enumerate() {
        match classify_agent(deck, camera_deck, adjacent_decks, sleeping) {
            LodTier::Full => full.push(i),
            LodTier::Nearby => nearby.push(i),
            LodTier::Background => background.push(i),
            LodTier::Dormant => dormant.push(i),
        }
    }

    (full, nearby, background, dormant)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_deck_is_full() {
        assert_eq!(classify_agent(5, 5, &[4, 6], false), LodTier::Full);
    }

    #[test]
    fn adjacent_deck_is_nearby() {
        assert_eq!(classify_agent(4, 5, &[4, 6], false), LodTier::Nearby);
        assert_eq!(classify_agent(6, 5, &[4, 6], false), LodTier::Nearby);
    }

    #[test]
    fn far_deck_is_background() {
        assert_eq!(classify_agent(1, 5, &[4, 6], false), LodTier::Background);
    }

    #[test]
    fn sleeping_far_agent_is_dormant() {
        assert_eq!(classify_agent(1, 5, &[4, 6], true), LodTier::Dormant);
    }

    #[test]
    fn sleeping_on_camera_deck_still_full() {
        // Even sleeping agents on the visible deck get full updates
        assert_eq!(classify_agent(5, 5, &[4, 6], true), LodTier::Full);
    }

    #[test]
    fn sleeping_on_adjacent_deck_still_nearby() {
        assert_eq!(classify_agent(4, 5, &[4, 6], true), LodTier::Nearby);
    }

    #[test]
    fn full_tier_updates_movement() {
        let config = LodConfig::default();
        assert!(should_update(
            LodTier::Full,
            LodSystem::Movement,
            0,
            &config
        ));
        assert!(should_update(
            LodTier::Full,
            LodSystem::Movement,
            1,
            &config
        ));
    }

    #[test]
    fn background_skips_movement() {
        let config = LodConfig::default();
        assert!(!should_update(
            LodTier::Background,
            LodSystem::Movement,
            0,
            &config
        ));
        assert!(!should_update(
            LodTier::Background,
            LodSystem::Movement,
            100,
            &config
        ));
    }

    #[test]
    fn dormant_only_updates_needs() {
        let config = LodConfig::default();
        // Needs updates at interval 6000 → tick 0
        assert!(should_update(
            LodTier::Dormant,
            LodSystem::Needs,
            0,
            &config
        ));
        // All others disabled
        assert!(!should_update(
            LodTier::Dormant,
            LodSystem::Movement,
            0,
            &config
        ));
        assert!(!should_update(
            LodTier::Dormant,
            LodSystem::Activity,
            0,
            &config
        ));
        assert!(!should_update(
            LodTier::Dormant,
            LodSystem::Social,
            0,
            &config
        ));
        assert!(!should_update(
            LodTier::Dormant,
            LodSystem::Atmosphere,
            0,
            &config
        ));
    }

    #[test]
    fn nearby_activity_interval() {
        let config = LodConfig::default();
        // Activity at interval 60
        assert!(should_update(
            LodTier::Nearby,
            LodSystem::Activity,
            0,
            &config
        ));
        assert!(!should_update(
            LodTier::Nearby,
            LodSystem::Activity,
            1,
            &config
        ));
        assert!(should_update(
            LodTier::Nearby,
            LodSystem::Activity,
            60,
            &config
        ));
    }

    #[test]
    fn staggered_spreads_updates() {
        let config = LodConfig::default();
        // Background needs interval = 600
        // Agent 0 updates at tick 0, agent 1 at tick 1, agent 599 at tick 599
        assert!(should_update_staggered(
            LodTier::Background,
            LodSystem::Needs,
            0,
            0,
            &config
        ));
        assert!(!should_update_staggered(
            LodTier::Background,
            LodSystem::Needs,
            0,
            1,
            &config
        ));
        assert!(should_update_staggered(
            LodTier::Background,
            LodSystem::Needs,
            1,
            1,
            &config
        ));
    }

    #[test]
    fn staggered_disabled_system_always_false() {
        let config = LodConfig::default();
        assert!(!should_update_staggered(
            LodTier::Dormant,
            LodSystem::Movement,
            0,
            0,
            &config
        ));
    }

    #[test]
    fn classify_batch_counts() {
        let agents = vec![
            (5, false), // full (same deck)
            (4, false), // nearby
            (6, false), // nearby
            (1, false), // background
            (10, true), // dormant (sleeping + far)
            (5, true),  // full (same deck, sleeping)
        ];
        let stats = classify_batch(&agents, 5, &[4, 6]);
        assert_eq!(stats.full_count, 2);
        assert_eq!(stats.nearby_count, 2);
        assert_eq!(stats.background_count, 1);
        assert_eq!(stats.dormant_count, 1);
        assert_eq!(stats.total(), 6);
    }

    #[test]
    fn estimated_cost_scales() {
        let stats = LodStats {
            full_count: 100,
            nearby_count: 200,
            background_count: 4000,
            dormant_count: 700,
        };
        // 100*1.0 + 200*0.5 + 4000*0.1 + 700*0.01 = 100 + 100 + 400 + 7 = 607
        assert!((stats.estimated_cost() - 607.0).abs() < 0.01);
        // Much less than 5000 (if all were full)
        assert!(stats.estimated_cost() < stats.total() as f64);
    }

    #[test]
    fn partition_returns_correct_indices() {
        let agents = vec![
            (5, false), // 0 → full
            (4, false), // 1 → nearby
            (1, false), // 2 → background
            (10, true), // 3 → dormant
        ];
        let (full, nearby, bg, dorm) = partition_by_tier(&agents, 5, &[4, 6]);
        assert_eq!(full, vec![0]);
        assert_eq!(nearby, vec![1]);
        assert_eq!(bg, vec![2]);
        assert_eq!(dorm, vec![3]);
    }

    #[test]
    fn large_population_cost_reduction() {
        // Simulate a realistic 5000-person ship where camera is on deck 10
        // of a 21-deck ship. Adjacent decks: 9, 11.
        let mut agents = Vec::new();
        // ~240 people per deck × 21 decks
        for deck in 0..21u32 {
            for _ in 0..240 {
                let sleeping = deck % 3 == 0; // ~1/3 sleeping
                agents.push((deck, sleeping));
            }
        }
        let stats = classify_batch(&agents, 10, &[9, 11]);
        assert_eq!(stats.total(), 5040);
        // Only ~240 on camera deck
        assert_eq!(stats.full_count, 240);
        // 480 nearby (decks 9, 11)
        assert_eq!(stats.nearby_count, 480);
        // Effective cost should be well under 5040
        assert!(stats.estimated_cost() < 2000.0);
    }

    #[test]
    fn config_default_values() {
        let config = LodConfig::default();
        // Full tier: movement every tick
        assert_eq!(config.full.movement, 1);
        // Dormant tier: needs at 6000
        assert_eq!(config.dormant.needs, 6000);
        // Background: movement disabled
        assert_eq!(config.background.movement, 0);
        // Intervals_for works
        assert_eq!(config.intervals_for(LodTier::Full).activity, 60);
    }

    #[test]
    fn empty_adjacent_decks() {
        // No adjacent decks — everything not on camera deck is background/dormant
        assert_eq!(classify_agent(4, 5, &[], false), LodTier::Background);
        assert_eq!(classify_agent(4, 5, &[], true), LodTier::Dormant);
        assert_eq!(classify_agent(5, 5, &[], false), LodTier::Full);
    }
}
