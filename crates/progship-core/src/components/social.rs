//! Social components: Relationships, Conversations, Memories

use serde::{Deserialize, Serialize};

/// Relationship between two people (stored as separate entity or in a graph)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub person_a_id: u32,
    pub person_b_id: u32,
    pub relationship_type: RelationshipType,
    /// -1.0 (hostile) to 1.0 (close)
    pub strength: f32,
    /// 0.0 (stranger) to 1.0 (intimate knowledge)
    pub familiarity: f32,
    /// Simulation time of last interaction
    pub last_interaction: f64,
}

impl Relationship {
    pub fn new(person_a_id: u32, person_b_id: u32) -> Self {
        Self {
            person_a_id,
            person_b_id,
            relationship_type: RelationshipType::Stranger,
            strength: 0.0,
            familiarity: 0.0,
            last_interaction: 0.0,
        }
    }

    /// Update relationship type based on strength and familiarity
    pub fn update_type(&mut self) {
        self.relationship_type = match (self.strength, self.familiarity) {
            (s, _) if s < -0.5 => RelationshipType::Enemy,
            (s, _) if s < -0.2 => RelationshipType::Rival,
            (_, f) if f < 0.1 => RelationshipType::Stranger,
            (_, f) if f < 0.3 => RelationshipType::Acquaintance,
            (s, f) if s > 0.7 && f > 0.7 => RelationshipType::CloseFriend,
            (s, _) if s > 0.3 => RelationshipType::Friend,
            _ => RelationshipType::Acquaintance,
        };
    }

    /// Decay relationship over time without interaction
    pub fn decay(&mut self, hours_since_interaction: f64) {
        // Strength decays toward neutral, familiarity decays slowly
        let decay_rate = 0.001; // per hour
        let hours = hours_since_interaction as f32;
        
        if self.strength > 0.0 {
            self.strength = (self.strength - hours * decay_rate).max(0.0);
        } else if self.strength < 0.0 {
            self.strength = (self.strength + hours * decay_rate).min(0.0);
        }
        
        self.familiarity = (self.familiarity - hours * decay_rate * 0.1).max(0.0);
        self.update_type();
    }

    /// Record an interaction
    pub fn interact(&mut self, current_time: f64, quality: f32) {
        self.last_interaction = current_time;
        self.familiarity = (self.familiarity + 0.05).min(1.0);
        self.strength = (self.strength + quality * 0.1).clamp(-1.0, 1.0);
        self.update_type();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipType {
    Stranger,
    Acquaintance,
    Colleague,
    Friend,
    CloseFriend,
    Romantic,
    Family,
    Rival,
    Enemy,
}

/// Marker component for entities currently in a conversation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InConversation {
    pub conversation_id: u32,
}

/// Conversation entity component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub participants: Vec<u32>,
    pub topic: ConversationTopic,
    pub started_at: f64,
    pub state: ConversationState,
    pub exchanges: Vec<Exchange>,
}

impl Conversation {
    pub fn new(participants: Vec<u32>, topic: ConversationTopic, started_at: f64) -> Self {
        Self {
            participants,
            topic,
            started_at,
            state: ConversationState::Active,
            exchanges: Vec::new(),
        }
    }

    pub fn add_exchange(&mut self, speaker_id: u32, content_id: u32, timestamp: f64, tone: Tone) {
        self.exchanges.push(Exchange {
            speaker_id,
            content_id,
            timestamp,
            tone,
        });
    }

    pub fn end(&mut self) {
        self.state = ConversationState::Ended;
    }

    pub fn duration(&self, current_time: f64) -> f64 {
        current_time - self.started_at
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConversationTopic {
    Greeting,
    Work,
    Gossip,
    Personal,
    Complaint,
    Request,
    Flirtation,
    Argument,
    Farewell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConversationState {
    Active,
    Paused,
    Ended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    pub speaker_id: u32,
    /// Reference to dialogue content (template ID or generated)
    pub content_id: u32,
    pub timestamp: f64,
    pub tone: Tone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tone {
    Neutral,
    Friendly,
    Formal,
    Annoyed,
    Excited,
    Sad,
    Angry,
    Flirty,
    Sarcastic,
}

/// Memory of a significant event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub person_id: u32,
    pub event_type: MemoryType,
    pub timestamp: f64,
    /// -1.0 (traumatic) to 1.0 (joyful)
    pub emotional_impact: f32,
    pub related_people: Vec<u32>,
    pub related_location_id: Option<u32>,
    /// How much the memory has faded (0.0 = fresh, 1.0 = forgotten)
    pub decay: f32,
}

impl Memory {
    pub fn new(person_id: u32, event_type: MemoryType, timestamp: f64, impact: f32) -> Self {
        Self {
            person_id,
            event_type,
            timestamp,
            emotional_impact: impact,
            related_people: Vec::new(),
            related_location_id: None,
            decay: 0.0,
        }
    }

    pub fn with_people(mut self, people: Vec<u32>) -> Self {
        self.related_people = people;
        self
    }

    pub fn with_location(mut self, location_id: u32) -> Self {
        self.related_location_id = Some(location_id);
        self
    }

    /// Apply memory decay over time
    pub fn apply_decay(&mut self, hours: f64) {
        // Stronger emotional memories decay slower
        let base_rate = 0.001; // per hour
        let rate = base_rate / (1.0 + self.emotional_impact.abs());
        self.decay = (self.decay + hours as f32 * rate).min(1.0);
    }

    /// Is this memory still significant?
    pub fn is_significant(&self) -> bool {
        self.decay < 0.8 && self.emotional_impact.abs() > 0.2
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryType {
    Conversation,
    SharedMeal,
    WorkedTogether,
    Helped,
    Conflict,
    RomanticMoment,
    Witnessed,
    Achievement,
    Failure,
    Accident,
    Celebration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_interaction() {
        let mut rel = Relationship::new(1, 2);
        assert_eq!(rel.relationship_type, RelationshipType::Stranger);
        
        // Positive interactions
        for i in 0..10 {
            rel.interact(i as f64, 0.5);
        }
        
        assert!(rel.familiarity > 0.3);
        assert!(rel.strength > 0.3);
        assert_eq!(rel.relationship_type, RelationshipType::Friend);
    }

    #[test]
    fn test_relationship_decay() {
        let mut rel = Relationship::new(1, 2);
        rel.strength = 0.5;
        rel.familiarity = 0.5;
        rel.update_type();
        
        rel.decay(100.0); // 100 hours without interaction
        assert!(rel.strength < 0.5);
    }

    #[test]
    fn test_memory_decay() {
        let mut memory = Memory::new(1, MemoryType::Conversation, 0.0, 0.3);
        assert!(memory.is_significant());
        
        // With impact=0.3, rate = 0.001/(1+0.3) ≈ 0.00077/hr
        // Need ~1040 hours to reach decay=0.8
        memory.apply_decay(1500.0);
        assert!(!memory.is_significant());
    }
}
