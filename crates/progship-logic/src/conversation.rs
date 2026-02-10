//! Conversation memory and gossip propagation.
//!
//! NPCs remember recent conversations and avoid repeating topics.
//! Information shared in conversations can spread through social
//! networks via gossip.
//!
//! # Memory Model
//!
//! Each relationship stores the last N conversation summaries.
//! When two NPCs meet, the system checks recent memory to:
//! - Avoid repeating the same topic within 24 hours
//! - Select contextually appropriate greetings
//! - Propagate gossip about events
//!
//! ```
//! use progship_logic::conversation::{
//!     ConversationMemory, ConversationRecord, Topic, Tone,
//!     choose_greeting, should_avoid_topic,
//! };
//!
//! let mut memory = ConversationMemory::new(5);
//! let record = ConversationRecord {
//!     topic: Topic::SmallTalk,
//!     tone: Tone::Friendly,
//!     hour: 100.0,
//!     initiated_by_self: true,
//!     gossip: None,
//! };
//! memory.add(42, record);
//! assert!(should_avoid_topic(&memory, 42, Topic::SmallTalk, 110.0));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Conversation topics that NPCs can discuss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Topic {
    /// Weather-equivalent small talk (ship conditions, daily life).
    SmallTalk,
    /// Work-related discussion (duties, shifts, problems).
    Work,
    /// Personal matters (family, feelings, goals).
    Personal,
    /// Ship events (emergencies, celebrations, discoveries).
    ShipEvent,
    /// Gossip about other crew members.
    Gossip,
    /// Complaints (food, conditions, management).
    Complaint,
    /// Hobbies and recreational interests.
    Hobby,
    /// Health and wellbeing.
    Health,
    /// Politics and ship governance.
    Politics,
    /// Technical/scientific discussion.
    Technical,
}

impl Topic {
    /// All topic variants for iteration.
    pub const ALL: [Topic; 10] = [
        Topic::SmallTalk,
        Topic::Work,
        Topic::Personal,
        Topic::ShipEvent,
        Topic::Gossip,
        Topic::Complaint,
        Topic::Hobby,
        Topic::Health,
        Topic::Politics,
        Topic::Technical,
    ];
}

/// Emotional tone of a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tone {
    Friendly,
    Neutral,
    Tense,
    Hostile,
    Sympathetic,
    Excited,
}

/// A record of a single conversation with another person.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRecord {
    /// What was discussed.
    pub topic: Topic,
    /// How the conversation felt.
    pub tone: Tone,
    /// Simulation hour when the conversation occurred.
    pub hour: f64,
    /// Whether this NPC initiated the conversation.
    pub initiated_by_self: bool,
    /// Optional gossip payload (event_id or person_id being discussed).
    pub gossip: Option<GossipItem>,
}

/// A piece of gossip that can spread between NPCs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipItem {
    /// What kind of gossip this is.
    pub kind: GossipKind,
    /// The subject entity ID (person_id or event_id).
    pub subject_id: u32,
    /// A brief description for context.
    pub description: String,
    /// Simulation hour when the gossip originated.
    pub origin_hour: f64,
    /// How many hops this gossip has traveled.
    pub hops: u32,
}

/// Types of gossip that can spread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GossipKind {
    /// An event that happened (fire, accident, celebration).
    Event,
    /// Gossip about a specific person.
    PersonalRumor,
    /// News about ship operations or policy changes.
    ShipNews,
}

/// Per-relationship conversation memory with configurable capacity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMemory {
    /// Max records per relationship.
    max_per_relationship: usize,
    /// Map from other person's ID to their conversation history.
    records: HashMap<u32, Vec<ConversationRecord>>,
}

impl ConversationMemory {
    /// Create a new memory with the given capacity per relationship.
    pub fn new(max_per_relationship: usize) -> Self {
        Self {
            max_per_relationship,
            records: HashMap::new(),
        }
    }

    /// Add a conversation record for a specific person.
    /// Evicts the oldest record if at capacity.
    pub fn add(&mut self, person_id: u32, record: ConversationRecord) {
        let history = self.records.entry(person_id).or_default();
        if history.len() >= self.max_per_relationship {
            history.remove(0);
        }
        history.push(record);
    }

    /// Get conversation history with a specific person.
    pub fn history_with(&self, person_id: u32) -> &[ConversationRecord] {
        self.records.get(&person_id).map_or(&[], |v| v.as_slice())
    }

    /// Get the most recent conversation with a person.
    pub fn last_conversation_with(&self, person_id: u32) -> Option<&ConversationRecord> {
        self.records.get(&person_id).and_then(|v| v.last())
    }

    /// Count total conversations across all relationships.
    pub fn total_conversations(&self) -> usize {
        self.records.values().map(|v| v.len()).sum()
    }

    /// Get all known gossip items (deduplicated by subject_id + kind).
    pub fn known_gossip(&self) -> Vec<&GossipItem> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for records in self.records.values() {
            for record in records {
                if let Some(ref gossip) = record.gossip {
                    let key = (gossip.kind, gossip.subject_id);
                    if seen.insert(key) {
                        result.push(gossip);
                    }
                }
            }
        }
        result
    }

    /// Count unique people this NPC has talked to.
    pub fn unique_contacts(&self) -> usize {
        self.records.len()
    }
}

/// Check whether a topic should be avoided with a specific person.
///
/// Returns `true` if the same topic was discussed within the last
/// `cooldown_hours` (default 24).
pub fn should_avoid_topic(
    memory: &ConversationMemory,
    person_id: u32,
    topic: Topic,
    current_hour: f64,
) -> bool {
    should_avoid_topic_with_cooldown(memory, person_id, topic, current_hour, 24.0)
}

/// Like [`should_avoid_topic`] but with a custom cooldown period.
pub fn should_avoid_topic_with_cooldown(
    memory: &ConversationMemory,
    person_id: u32,
    topic: Topic,
    current_hour: f64,
    cooldown_hours: f64,
) -> bool {
    let history = memory.history_with(person_id);
    history
        .iter()
        .any(|r| r.topic == topic && (current_hour - r.hour) < cooldown_hours)
}

/// Select available topics that haven't been discussed recently.
pub fn available_topics(
    memory: &ConversationMemory,
    person_id: u32,
    current_hour: f64,
) -> Vec<Topic> {
    Topic::ALL
        .iter()
        .copied()
        .filter(|&t| !should_avoid_topic(memory, person_id, t, current_hour))
        .collect()
}

/// Greeting style based on relationship history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GreetingStyle {
    /// First meeting ever.
    FirstMeeting,
    /// Haven't talked in a long time (>72 hours).
    LongTimeSince,
    /// Normal re-encounter.
    Regular,
    /// Last conversation was tense/hostile.
    Cautious,
    /// Last conversation was friendly/excited.
    Warm,
}

/// Choose an appropriate greeting style based on conversation history.
pub fn choose_greeting(
    memory: &ConversationMemory,
    person_id: u32,
    current_hour: f64,
) -> GreetingStyle {
    match memory.last_conversation_with(person_id) {
        None => GreetingStyle::FirstMeeting,
        Some(last) => {
            let hours_since = current_hour - last.hour;
            if hours_since > 72.0 {
                GreetingStyle::LongTimeSince
            } else {
                match last.tone {
                    Tone::Hostile | Tone::Tense => GreetingStyle::Cautious,
                    Tone::Friendly | Tone::Excited | Tone::Sympathetic => GreetingStyle::Warm,
                    Tone::Neutral => GreetingStyle::Regular,
                }
            }
        }
    }
}

/// Determine whether gossip should be shared with another person.
///
/// Gossip spreads if:
/// - The receiver doesn't already know it (not in their memory)
/// - The gossip hasn't expired (< max_age_hours old)
/// - The gossip hasn't traveled too many hops (< max_hops)
pub fn should_share_gossip(
    gossip: &GossipItem,
    receiver_memory: &ConversationMemory,
    current_hour: f64,
    max_age_hours: f64,
    max_hops: u32,
) -> bool {
    // Too old
    if current_hour - gossip.origin_hour > max_age_hours {
        return false;
    }
    // Too many hops
    if gossip.hops >= max_hops {
        return false;
    }
    // Receiver already knows
    let known = receiver_memory.known_gossip();
    !known
        .iter()
        .any(|g| g.kind == gossip.kind && g.subject_id == gossip.subject_id)
}

/// Create a new gossip item from an event.
pub fn create_event_gossip(event_id: u32, description: &str, current_hour: f64) -> GossipItem {
    GossipItem {
        kind: GossipKind::Event,
        subject_id: event_id,
        description: description.to_string(),
        origin_hour: current_hour,
        hops: 0,
    }
}

/// Propagate gossip by incrementing hop count.
pub fn propagate_gossip(gossip: &GossipItem) -> GossipItem {
    GossipItem {
        kind: gossip.kind,
        subject_id: gossip.subject_id,
        description: gossip.description.clone(),
        origin_hour: gossip.origin_hour,
        hops: gossip.hops + 1,
    }
}

/// Configuration for gossip propagation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipConfig {
    /// Maximum age of gossip before it stops spreading (hours).
    pub max_age_hours: f64,
    /// Maximum hops gossip can travel through the social network.
    pub max_hops: u32,
    /// Probability of sharing gossip in a conversation (0.0–1.0).
    pub share_probability: f64,
    /// Maximum conversations remembered per relationship.
    pub memory_capacity: usize,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            max_age_hours: 168.0, // 1 week
            max_hops: 5,
            share_probability: 0.3,
            memory_capacity: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(topic: Topic, tone: Tone, hour: f64) -> ConversationRecord {
        ConversationRecord {
            topic,
            tone,
            hour,
            initiated_by_self: true,
            gossip: None,
        }
    }

    #[test]
    fn memory_add_and_retrieve() {
        let mut mem = ConversationMemory::new(5);
        mem.add(42, make_record(Topic::SmallTalk, Tone::Friendly, 100.0));
        assert_eq!(mem.history_with(42).len(), 1);
        assert_eq!(mem.history_with(99).len(), 0);
    }

    #[test]
    fn memory_evicts_oldest() {
        let mut mem = ConversationMemory::new(3);
        mem.add(42, make_record(Topic::SmallTalk, Tone::Friendly, 100.0));
        mem.add(42, make_record(Topic::Work, Tone::Neutral, 110.0));
        mem.add(42, make_record(Topic::Personal, Tone::Sympathetic, 120.0));
        mem.add(42, make_record(Topic::Hobby, Tone::Excited, 130.0));
        let history = mem.history_with(42);
        assert_eq!(history.len(), 3);
        // Oldest (SmallTalk at 100) should be evicted
        assert_eq!(history[0].topic, Topic::Work);
    }

    #[test]
    fn avoid_recent_topic() {
        let mut mem = ConversationMemory::new(5);
        mem.add(42, make_record(Topic::Work, Tone::Neutral, 100.0));
        // Within 24 hours — should avoid
        assert!(should_avoid_topic(&mem, 42, Topic::Work, 110.0));
        // After 24 hours — okay
        assert!(!should_avoid_topic(&mem, 42, Topic::Work, 125.0));
    }

    #[test]
    fn different_topic_not_avoided() {
        let mut mem = ConversationMemory::new(5);
        mem.add(42, make_record(Topic::Work, Tone::Neutral, 100.0));
        assert!(!should_avoid_topic(&mem, 42, Topic::SmallTalk, 110.0));
    }

    #[test]
    fn different_person_not_avoided() {
        let mut mem = ConversationMemory::new(5);
        mem.add(42, make_record(Topic::Work, Tone::Neutral, 100.0));
        assert!(!should_avoid_topic(&mem, 99, Topic::Work, 110.0));
    }

    #[test]
    fn available_topics_filters_recent() {
        let mut mem = ConversationMemory::new(5);
        mem.add(42, make_record(Topic::SmallTalk, Tone::Friendly, 100.0));
        mem.add(42, make_record(Topic::Work, Tone::Neutral, 100.0));
        let available = available_topics(&mem, 42, 110.0);
        assert!(!available.contains(&Topic::SmallTalk));
        assert!(!available.contains(&Topic::Work));
        assert!(available.contains(&Topic::Personal));
        assert_eq!(available.len(), 8); // 10 total - 2 avoided
    }

    #[test]
    fn greeting_first_meeting() {
        let mem = ConversationMemory::new(5);
        assert_eq!(
            choose_greeting(&mem, 42, 100.0),
            GreetingStyle::FirstMeeting
        );
    }

    #[test]
    fn greeting_warm_after_friendly() {
        let mut mem = ConversationMemory::new(5);
        mem.add(42, make_record(Topic::SmallTalk, Tone::Friendly, 100.0));
        assert_eq!(choose_greeting(&mem, 42, 110.0), GreetingStyle::Warm);
    }

    #[test]
    fn greeting_cautious_after_hostile() {
        let mut mem = ConversationMemory::new(5);
        mem.add(42, make_record(Topic::Complaint, Tone::Hostile, 100.0));
        assert_eq!(choose_greeting(&mem, 42, 110.0), GreetingStyle::Cautious);
    }

    #[test]
    fn greeting_long_time_since() {
        let mut mem = ConversationMemory::new(5);
        mem.add(42, make_record(Topic::SmallTalk, Tone::Friendly, 100.0));
        assert_eq!(
            choose_greeting(&mem, 42, 200.0),
            GreetingStyle::LongTimeSince
        );
    }

    #[test]
    fn gossip_creation() {
        let gossip = create_event_gossip(1, "Fire in engine room", 500.0);
        assert_eq!(gossip.kind, GossipKind::Event);
        assert_eq!(gossip.hops, 0);
        assert_eq!(gossip.subject_id, 1);
    }

    #[test]
    fn gossip_propagation() {
        let original = create_event_gossip(1, "Fire in engine room", 500.0);
        let spread = propagate_gossip(&original);
        assert_eq!(spread.hops, 1);
        assert_eq!(spread.subject_id, original.subject_id);
    }

    #[test]
    fn gossip_sharing_fresh() {
        let gossip = create_event_gossip(1, "Fire", 500.0);
        let receiver = ConversationMemory::new(5);
        assert!(should_share_gossip(&gossip, &receiver, 510.0, 168.0, 5));
    }

    #[test]
    fn gossip_too_old_not_shared() {
        let gossip = create_event_gossip(1, "Fire", 100.0);
        let receiver = ConversationMemory::new(5);
        // 300 hours later, max age 168
        assert!(!should_share_gossip(&gossip, &receiver, 400.0, 168.0, 5));
    }

    #[test]
    fn gossip_too_many_hops() {
        let mut gossip = create_event_gossip(1, "Fire", 500.0);
        gossip.hops = 5;
        let receiver = ConversationMemory::new(5);
        assert!(!should_share_gossip(&gossip, &receiver, 510.0, 168.0, 5));
    }

    #[test]
    fn gossip_already_known() {
        let gossip = create_event_gossip(1, "Fire", 500.0);
        let mut receiver = ConversationMemory::new(5);
        // Receiver already heard about event 1
        receiver.add(
            99,
            ConversationRecord {
                topic: Topic::ShipEvent,
                tone: Tone::Excited,
                hour: 505.0,
                initiated_by_self: false,
                gossip: Some(gossip.clone()),
            },
        );
        assert!(!should_share_gossip(&gossip, &receiver, 510.0, 168.0, 5));
    }

    #[test]
    fn acceptance_no_repeated_greeting() {
        let mut mem = ConversationMemory::new(5);
        mem.add(42, make_record(Topic::SmallTalk, Tone::Friendly, 100.0));
        // Same topic within 24h → should be avoided
        assert!(
            should_avoid_topic(&mem, 42, Topic::SmallTalk, 110.0),
            "NPC should not repeat same greeting topic within 24h"
        );
    }

    #[test]
    fn acceptance_gossip_spreads() {
        // NPC A tells B about fire → B should know about it
        let gossip = create_event_gossip(1, "Fire in deck 3", 500.0);
        let propagated = propagate_gossip(&gossip);
        assert_eq!(propagated.hops, 1);

        // B can share with C (hop 1 < max 5)
        let c_memory = ConversationMemory::new(5);
        assert!(
            should_share_gossip(&propagated, &c_memory, 510.0, 168.0, 5),
            "gossip should spread through social network"
        );
    }

    #[test]
    fn total_conversations_count() {
        let mut mem = ConversationMemory::new(5);
        mem.add(42, make_record(Topic::SmallTalk, Tone::Friendly, 100.0));
        mem.add(42, make_record(Topic::Work, Tone::Neutral, 110.0));
        mem.add(99, make_record(Topic::Personal, Tone::Sympathetic, 120.0));
        assert_eq!(mem.total_conversations(), 3);
        assert_eq!(mem.unique_contacts(), 2);
    }

    #[test]
    fn known_gossip_deduplicates() {
        let mut mem = ConversationMemory::new(5);
        let gossip = create_event_gossip(1, "Fire", 500.0);
        // Hear same gossip from two people
        mem.add(
            42,
            ConversationRecord {
                topic: Topic::Gossip,
                tone: Tone::Excited,
                hour: 510.0,
                initiated_by_self: false,
                gossip: Some(gossip.clone()),
            },
        );
        mem.add(
            99,
            ConversationRecord {
                topic: Topic::Gossip,
                tone: Tone::Neutral,
                hour: 520.0,
                initiated_by_self: false,
                gossip: Some(gossip),
            },
        );
        assert_eq!(mem.known_gossip().len(), 1);
    }

    #[test]
    fn config_defaults() {
        let config = GossipConfig::default();
        assert_eq!(config.max_hops, 5);
        assert!((config.max_age_hours - 168.0).abs() < f64::EPSILON);
        assert_eq!(config.memory_capacity, 10);
    }

    #[test]
    fn all_topics() {
        assert_eq!(Topic::ALL.len(), 10);
    }
}
