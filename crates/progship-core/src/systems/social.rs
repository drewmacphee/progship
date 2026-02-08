//! Social system - conversations, relationships, and social interactions

use hecs::World;
use rand::Rng;
use serde::{Serialize, Deserialize};
use crate::components::{
    Person, Position, Needs, Personality, Crew, Passenger,
    Relationship, Conversation, ConversationTopic, ConversationState,
    InConversation, Tone, Activity, ActivityType, Name,
};

/// Relationship graph (singleton, stored in engine)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelationshipGraph {
    pub relationships: Vec<Relationship>,
}

impl RelationshipGraph {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get or create relationship between two people
    pub fn get_or_create(&mut self, person_a: u32, person_b: u32) -> &mut Relationship {
        // Normalize IDs (smaller first)
        let (a, b) = if person_a < person_b { (person_a, person_b) } else { (person_b, person_a) };
        
        // Find existing
        if let Some(idx) = self.relationships.iter().position(|r| r.person_a_id == a && r.person_b_id == b) {
            return &mut self.relationships[idx];
        }
        
        // Create new
        self.relationships.push(Relationship::new(a, b));
        self.relationships.last_mut().unwrap()
    }
    
    /// Get relationship if it exists
    pub fn get(&self, person_a: u32, person_b: u32) -> Option<&Relationship> {
        let (a, b) = if person_a < person_b { (person_a, person_b) } else { (person_b, person_a) };
        self.relationships.iter().find(|r| r.person_a_id == a && r.person_b_id == b)
    }
}

/// Active conversations (singleton, stored in engine)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversationManager {
    pub conversations: Vec<(u32, Conversation)>, // (conversation_id, conversation)
    next_id: u32,
}

impl ConversationManager {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Start a new conversation
    pub fn start_conversation(
        &mut self,
        participants: Vec<u32>,
        topic: ConversationTopic,
        started_at: f64,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        
        let conv = Conversation::new(participants, topic, started_at);
        self.conversations.push((id, conv));
        id
    }
    
    /// Get active conversation by ID
    pub fn get(&self, id: u32) -> Option<&Conversation> {
        self.conversations.iter().find(|(cid, _)| *cid == id).map(|(_, c)| c)
    }
    
    /// Get mutable conversation by ID
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Conversation> {
        self.conversations.iter_mut().find(|(cid, _)| *cid == id).map(|(_, c)| c)
    }
    
    /// Remove ended conversations
    pub fn cleanup(&mut self) -> Vec<(u32, Vec<u32>)> {
        let ended: Vec<(u32, Vec<u32>)> = self.conversations
            .iter()
            .filter(|(_, c)| c.state == ConversationState::Ended)
            .map(|(id, c)| (*id, c.participants.clone()))
            .collect();
        self.conversations.retain(|(_, c)| c.state != ConversationState::Ended);
        ended
    }
    
    /// Get conversation count
    pub fn active_count(&self) -> usize {
        self.conversations.iter().filter(|(_, c)| c.state == ConversationState::Active).count()
    }
}

/// Find people who are close enough to talk
pub fn find_nearby_pairs(
    world: &World,
    room_entities: &[hecs::Entity],
) -> Vec<(u32, u32, u32)> { // (person_a_idx, person_b_idx, room_id)
    let mut pairs = Vec::new();
    
    // Group people by room
    let mut people_by_room: std::collections::HashMap<u32, Vec<(u32, hecs::Entity)>> = std::collections::HashMap::new();
    
    let mut person_idx: u32 = 0;
    for (entity, (_, pos)) in world.query::<(&Person, &Position)>().iter() {
        people_by_room
            .entry(pos.room_id)
            .or_default()
            .push((person_idx, entity));
        person_idx += 1;
    }
    
    // Find pairs in the same room
    for (room_id, people) in &people_by_room {
        for i in 0..people.len() {
            for j in (i + 1)..people.len() {
                pairs.push((people[i].0, people[j].0, *room_id));
            }
        }
    }
    
    let _ = room_entities; // May use later for distance calculations
    pairs
}

/// Check if a person can start a conversation
fn can_start_conversation(
    world: &World,
    entity: hecs::Entity,
) -> bool {
    // Not already in conversation
    if world.get::<&InConversation>(entity).is_ok() {
        return false;
    }
    
    // Not doing something that prevents talking
    if let Ok(activity) = world.get::<&Activity>(entity) {
        match activity.activity_type {
            ActivityType::Sleeping | ActivityType::Hygiene => return false,
            _ => {}
        }
    }
    
    true
}

/// Determine if two people should start a conversation
fn should_start_conversation(
    world: &World,
    entity_a: hecs::Entity,
    entity_b: hecs::Entity,
    relationships: &RelationshipGraph,
    person_a_idx: u32,
    person_b_idx: u32,
    rng: &mut impl Rng,
) -> Option<ConversationTopic> {
    // Check if both can talk
    if !can_start_conversation(world, entity_a) || !can_start_conversation(world, entity_b) {
        return None;
    }
    
    // Get social needs
    let needs_a = world.get::<&Needs>(entity_a).ok()?;
    let needs_b = world.get::<&Needs>(entity_b).ok()?;
    
    // Higher social need = more likely to talk
    let social_drive = (needs_a.social + needs_b.social) / 2.0;
    
    // Base probability scales with social need
    let base_prob = social_drive * 0.1; // 10% max per check
    
    // Relationship affects probability
    let rel_bonus = if let Some(rel) = relationships.get(person_a_idx, person_b_idx) {
        (rel.familiarity + rel.strength.max(0.0)) * 0.05
    } else {
        0.0
    };
    
    let prob = base_prob + rel_bonus;
    
    if rng.gen::<f32>() > prob {
        return None;
    }
    
    // Choose topic based on context
    let topic = if let Some(rel) = relationships.get(person_a_idx, person_b_idx) {
        if rel.familiarity < 0.2 {
            ConversationTopic::Greeting
        } else if rng.gen::<f32>() < 0.3 {
            ConversationTopic::Work
        } else if rng.gen::<f32>() < 0.5 {
            ConversationTopic::Gossip
        } else {
            ConversationTopic::Personal
        }
    } else {
        ConversationTopic::Greeting
    };
    
    Some(topic)
}

/// Main social system - triggers and manages conversations
pub fn social_system(
    world: &mut World,
    conversations: &mut ConversationManager,
    relationships: &mut RelationshipGraph,
    room_entities: &[hecs::Entity],
    current_time: f64,
    delta_hours: f32,
) {
    let mut rng = rand::thread_rng();
    
    // Collect entity list for lookup
    let people: Vec<(hecs::Entity, u32)> = world
        .query::<&Person>()
        .iter()
        .enumerate()
        .map(|(idx, (e, _))| (e, idx as u32))
        .collect();
    
    // Find nearby pairs
    let pairs = find_nearby_pairs(world, room_entities);
    
    // Try to start new conversations
    for (person_a_idx, person_b_idx, _room_id) in pairs {
        let entity_a = people.iter().find(|(_, idx)| *idx == person_a_idx).map(|(e, _)| *e);
        let entity_b = people.iter().find(|(_, idx)| *idx == person_b_idx).map(|(e, _)| *e);
        
        if let (Some(entity_a), Some(entity_b)) = (entity_a, entity_b) {
            if let Some(topic) = should_start_conversation(
                world, entity_a, entity_b, relationships, person_a_idx, person_b_idx, &mut rng
            ) {
                // Start conversation
                let conv_id = conversations.start_conversation(
                    vec![person_a_idx, person_b_idx],
                    topic,
                    current_time,
                );
                
                // Mark both as in conversation
                let _ = world.insert_one(entity_a, InConversation { conversation_id: conv_id });
                let _ = world.insert_one(entity_b, InConversation { conversation_id: conv_id });
            }
        }
    }
    
    // Progress active conversations
    progress_conversations(world, conversations, relationships, current_time, delta_hours, &mut rng);
    
    // Cleanup ended conversations and remove InConversation components
    let ended = conversations.cleanup();
    for (_conv_id, participant_indices) in ended {
        // Find entities by index and remove InConversation
        for (entity, idx) in &people {
            if participant_indices.contains(idx) {
                let _ = world.remove_one::<InConversation>(*entity);
            }
        }
    }
}

/// Progress active conversations
fn progress_conversations(
    _world: &mut World,
    conversations: &mut ConversationManager,
    relationships: &mut RelationshipGraph,
    current_time: f64,
    _delta_hours: f32,
    rng: &mut impl Rng,
) {
    for (_, conv) in &mut conversations.conversations {
        if conv.state != ConversationState::Active {
            continue;
        }
        
        let duration = conv.duration(current_time);
        
        // Conversations last 1-5 minutes (0.017 - 0.083 hours)
        let should_end = match conv.topic {
            ConversationTopic::Greeting => duration > 0.01, // Quick greeting
            ConversationTopic::Farewell => duration > 0.005,
            _ => duration > 0.05 || rng.gen::<f32>() < 0.1, // ~5% chance per tick to end
        };
        
        if should_end {
            conv.end();
            
            // Update relationships
            if conv.participants.len() >= 2 {
                let quality = match conv.topic {
                    ConversationTopic::Argument => -0.3,
                    ConversationTopic::Complaint => -0.1,
                    ConversationTopic::Greeting | ConversationTopic::Farewell => 0.1,
                    ConversationTopic::Personal | ConversationTopic::Gossip => 0.3,
                    _ => 0.2,
                };
                
                let rel = relationships.get_or_create(conv.participants[0], conv.participants[1]);
                rel.interact(current_time, quality);
            }
        }
    }
}

/// Get display text for active conversations (for chat bubbles)
pub fn get_conversation_display(
    conversations: &ConversationManager,
    person_idx: u32,
) -> Option<(ConversationTopic, Tone)> {
    for (_, conv) in &conversations.conversations {
        if conv.state == ConversationState::Active && conv.participants.contains(&person_idx) {
            // Determine tone from topic
            let tone = match conv.topic {
                ConversationTopic::Greeting | ConversationTopic::Farewell => Tone::Friendly,
                ConversationTopic::Complaint | ConversationTopic::Argument => Tone::Annoyed,
                ConversationTopic::Work => Tone::Formal,
                ConversationTopic::Flirtation => Tone::Flirty,
                _ => Tone::Neutral,
            };
            return Some((conv.topic, tone));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_graph() {
        let mut graph = RelationshipGraph::new();
        
        // Get or create should work in either order
        {
            let rel = graph.get_or_create(1, 2);
            rel.interact(0.0, 0.5);
        }
        
        // Should get same relationship with reversed IDs
        let rel = graph.get(2, 1);
        assert!(rel.is_some());
        assert!(rel.unwrap().strength > 0.0);
    }

    #[test]
    fn test_conversation_manager() {
        let mut manager = ConversationManager::new();
        
        let id = manager.start_conversation(
            vec![1, 2],
            ConversationTopic::Greeting,
            0.0,
        );
        
        assert_eq!(manager.active_count(), 1);
        
        if let Some(conv) = manager.get_mut(id) {
            conv.end();
        }
        
        manager.cleanup();
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_find_nearby_pairs() {
        let mut world = World::new();
        
        // Create 3 people in the same room
        for _ in 0..3 {
            world.spawn((
                Person,
                Position { 
                    local: crate::components::Vec3 { x: 0.0, y: 0.0, z: 0.0 },
                    room: None,
                    room_id: 0,
                },
            ));
        }
        
        // Create 1 person in a different room
        world.spawn((
            Person,
            Position { 
                local: crate::components::Vec3 { x: 0.0, y: 0.0, z: 0.0 },
                room: None,
                room_id: 1,
            },
        ));
        
        let pairs = find_nearby_pairs(&world, &[]);
        
        // 3 people in room 0 = 3 pairs (0-1, 0-2, 1-2)
        // 1 person in room 1 = 0 pairs
        assert_eq!(pairs.len(), 3);
    }
}
