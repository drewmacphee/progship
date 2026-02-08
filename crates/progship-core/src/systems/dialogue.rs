//! Dialogue generation - creates conversation content based on topics and personalities

use rand::Rng;
use crate::components::{ConversationTopic, Personality, Tone, Faction};

/// A line of dialogue with speaker context
#[derive(Debug, Clone)]
pub struct DialogueLine {
    pub text: String,
    pub tone: Tone,
}

/// Generate dialogue for a conversation topic
pub fn generate_dialogue(
    topic: ConversationTopic,
    personality: &Personality,
    faction: Option<Faction>,
    relationship_strength: f32,
    rng: &mut impl Rng,
) -> DialogueLine {
    let templates = get_templates(topic);
    let template = &templates[rng.gen_range(0..templates.len())];
    
    // Determine tone based on personality
    let tone = determine_tone(personality, topic, rng);
    
    // Fill in template with contextual words
    let text = fill_template(template, personality, faction, relationship_strength, rng);
    
    DialogueLine { text, tone }
}

/// Get dialogue templates for a topic
fn get_templates(topic: ConversationTopic) -> Vec<&'static str> {
    match topic {
        ConversationTopic::Greeting => vec![
            "Hello there!",
            "Good to see you.",
            "Hey, how's it going?",
            "Hi!",
        ],
        ConversationTopic::Gossip => vec![
            "Did you hear about {subject}?",
            "I heard something interesting about {subject}.",
            "You won't believe what happened in {location}.",
            "Between you and me, {rumor}.",
            "Have you noticed {observation}?",
        ],
        ConversationTopic::Work => vec![
            "The {system} needs attention.",
            "My shift was {quality} today.",
            "I've been working on {task}.",
            "Did you see the latest reports from {department}?",
            "We should coordinate on {project}.",
        ],
        ConversationTopic::Personal => vec![
            "I've been feeling {mood} lately.",
            "I miss {place} sometimes.",
            "Do you ever think about {thought}?",
            "I had the strangest dream about {dream}.",
            "Back home, we used to {activity}.",
        ],
        ConversationTopic::Complaint => vec![
            "We need to talk about {issue}.",
            "I didn't appreciate {complaint}.",
            "There's been a problem with {problem}.",
            "I think there's been a misunderstanding.",
            "This isn't working for me.",
        ],
        ConversationTopic::Request => vec![
            "Could you help me with {task}?",
            "I was wondering if you could {suggestion}.",
            "Do you have a moment?",
            "I need a favor.",
        ],
        ConversationTopic::Flirtation => vec![
            "I enjoy spending time with you.",
            "You have such a {compliment}.",
            "I was hoping we could {suggestion}.",
            "There's something I've been meaning to tell you.",
            "Do you want to grab a meal together?",
        ],
        ConversationTopic::Argument => vec![
            "That's not what I said!",
            "You're not listening to me.",
            "We clearly disagree on this.",
            "I can't believe you think that.",
            "Let's just calm down.",
        ],
        ConversationTopic::Farewell => vec![
            "I should get going.",
            "It was nice talking to you.",
            "See you around.",
            "Take care.",
            "Until next time.",
        ],
    }
}

/// Determine the tone based on personality
fn determine_tone(personality: &Personality, topic: ConversationTopic, rng: &mut impl Rng) -> Tone {
    // High extraversion = more excited
    // High neuroticism = more annoyed/sad
    // High agreeableness = more friendly
    
    let base_chance: f32 = rng.gen();
    
    if personality.neuroticism > 0.5 && base_chance < 0.3 {
        return Tone::Annoyed;
    }
    
    if personality.extraversion > 0.5 && base_chance < 0.4 {
        return Tone::Excited;
    }
    
    if personality.agreeableness > 0.5 && base_chance < 0.3 {
        return Tone::Friendly;
    }
    
    if topic == ConversationTopic::Argument {
        return Tone::Angry;
    }
    
    if topic == ConversationTopic::Flirtation {
        return Tone::Flirty;
    }
    
    Tone::Neutral
}

/// Fill in template placeholders
fn fill_template(
    template: &str,
    personality: &Personality,
    faction: Option<Faction>,
    _relationship_strength: f32,
    rng: &mut impl Rng,
) -> String {
    let mut result = template.to_string();
    
    // Subject replacements
    let subjects = vec!["the new crew rotation", "that incident yesterday", 
                        "the captain's announcement", "the supply situation"];
    let rumors = vec!["someone saw something strange in cargo bay 3",
                      "the engines have been making odd sounds",
                      "there might be a celebration planned",
                      "we're ahead of schedule"];
    let observations = vec!["how quiet it's been", "the food quality lately",
                           "the tension between shifts", "something off about the air"];
    let locations = vec!["engineering", "the mess hall", "deck 3", "medical"];
    
    // Work replacements
    let systems = vec!["life support", "navigation", "power grid", "comms array"];
    let qualities = vec!["exhausting", "productive", "strange", "routine"];
    let tasks = vec!["diagnostics", "maintenance reports", "crew schedules", "inventory"];
    let departments = vec!["Command", "Engineering", "Science", "Medical"];
    let projects = vec!["the upcoming drill", "resource allocation", "shift changes"];
    
    // Personal replacements
    let moods = if personality.neuroticism > 0.5 {
        vec!["anxious", "restless", "overwhelmed"]
    } else if personality.extraversion > 0.5 {
        vec!["energetic", "social", "excited"]
    } else {
        vec!["contemplative", "peaceful", "nostalgic"]
    };
    
    let places = vec!["Earth", "home", "open skies", "real sunlight"];
    let thoughts = vec!["what we left behind", "where we're going", "our purpose here"];
    let dreams = vec!["space whales", "my family", "an endless corridor", "stars"];
    let activities = vec!["gather for meals", "watch the sunset", "play in the garden"];
    
    // Ship replacements
    let conditions = vec!["steady", "quiet", "a bit tense", "efficient"];
    let viewpoints = vec!["observation deck", "the bridge", "my quarters"];
    let shipnews = vec!["course adjustments", "a minor issue", "good progress"];
    
    // Philosophy replacements
    let events = vec!["we arrive", "the journey ends", "we're gone"];
    let concepts = vec!["our place in the cosmos", "the nature of time", "fate"];
    let beliefs = vec!["destiny", "chance", "something greater"];
    let wonders = vec!["what waits for us", "if we're alone", "the meaning of it all"];
    let descriptions = vec!["vast", "beautiful", "mysterious", "indifferent"];
    
    // Conflict replacements
    let issues = vec!["what happened earlier", "your behavior", "this situation"];
    let complaints = vec!["what you said", "how that was handled", "being ignored"];
    let problems = vec!["communication", "the workload", "priorities"];
    
    // Romance replacements  
    let compliments = vec!["kind smile", "interesting perspective", "calming presence"];
    let suggestions = vec!["talk more", "explore the ship together", "share a meal"];
    
    // Faction-specific vocabulary adjustments
    let _dept_word = faction.map(|f| {
        if f.is_crew() { "department" } else { "area" }
    }).unwrap_or("section");
    
    // Replace placeholders
    result = result.replace("{subject}", subjects[rng.gen_range(0..subjects.len())]);
    result = result.replace("{rumor}", rumors[rng.gen_range(0..rumors.len())]);
    result = result.replace("{observation}", observations[rng.gen_range(0..observations.len())]);
    result = result.replace("{location}", locations[rng.gen_range(0..locations.len())]);
    result = result.replace("{system}", systems[rng.gen_range(0..systems.len())]);
    result = result.replace("{quality}", qualities[rng.gen_range(0..qualities.len())]);
    result = result.replace("{task}", tasks[rng.gen_range(0..tasks.len())]);
    result = result.replace("{department}", departments[rng.gen_range(0..departments.len())]);
    result = result.replace("{project}", projects[rng.gen_range(0..projects.len())]);
    result = result.replace("{mood}", moods[rng.gen_range(0..moods.len())]);
    result = result.replace("{place}", places[rng.gen_range(0..places.len())]);
    result = result.replace("{thought}", thoughts[rng.gen_range(0..thoughts.len())]);
    result = result.replace("{dream}", dreams[rng.gen_range(0..dreams.len())]);
    result = result.replace("{activity}", activities[rng.gen_range(0..activities.len())]);
    result = result.replace("{condition}", conditions[rng.gen_range(0..conditions.len())]);
    result = result.replace("{viewpoint}", viewpoints[rng.gen_range(0..viewpoints.len())]);
    result = result.replace("{shipnews}", shipnews[rng.gen_range(0..shipnews.len())]);
    result = result.replace("{event}", events[rng.gen_range(0..events.len())]);
    result = result.replace("{concept}", concepts[rng.gen_range(0..concepts.len())]);
    result = result.replace("{belief}", beliefs[rng.gen_range(0..beliefs.len())]);
    result = result.replace("{wonder}", wonders[rng.gen_range(0..wonders.len())]);
    result = result.replace("{description}", descriptions[rng.gen_range(0..descriptions.len())]);
    result = result.replace("{issue}", issues[rng.gen_range(0..issues.len())]);
    result = result.replace("{complaint}", complaints[rng.gen_range(0..complaints.len())]);
    result = result.replace("{problem}", problems[rng.gen_range(0..problems.len())]);
    result = result.replace("{compliment}", compliments[rng.gen_range(0..compliments.len())]);
    result = result.replace("{suggestion}", suggestions[rng.gen_range(0..suggestions.len())]);
    
    result
}

/// Generate a greeting based on personality and relationship
pub fn generate_greeting(
    personality: &Personality,
    relationship_strength: f32,
    rng: &mut impl Rng,
) -> String {
    let greetings = if relationship_strength > 0.7 {
        vec!["Hey!", "Good to see you!", "There you are!", "Hey friend!"]
    } else if relationship_strength > 0.3 {
        vec!["Hello.", "Hi there.", "Good day.", "Hey."]
    } else {
        vec!["Hello.", "Greetings.", "Hi.", "Good day."]
    };
    
    let mut greeting = greetings[rng.gen_range(0..greetings.len())].to_string();
    
    // Personality modifiers
    if personality.extraversion > 0.7 {
        greeting.push_str(" How are you?");
    } else if personality.agreeableness > 0.7 {
        greeting.push_str(" Nice to see you.");
    }
    
    greeting
}

/// Generate a farewell based on personality
pub fn generate_farewell(
    personality: &Personality,
    relationship_strength: f32,
    rng: &mut impl Rng,
) -> String {
    let farewells = if relationship_strength > 0.7 {
        vec!["See you later!", "Take care!", "Until next time!", "Catch you around!"]
    } else if relationship_strength > 0.3 {
        vec!["Goodbye.", "See you.", "Later.", "Take care."]
    } else {
        vec!["Goodbye.", "Farewell.", "Until we meet again.", "Good day."]
    };
    
    let mut farewell = farewells[rng.gen_range(0..farewells.len())].to_string();
    
    if personality.agreeableness > 0.7 && rng.gen_bool(0.5) {
        farewell.push_str(" It was nice talking!");
    }
    
    farewell
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn test_generate_dialogue() {
        let mut rng = StdRng::seed_from_u64(42);
        let personality = Personality::default();
        
        let line = generate_dialogue(
            ConversationTopic::Work,
            &personality,
            None,
            0.5,
            &mut rng,
        );
        
        assert!(!line.text.is_empty());
        assert!(!line.text.contains("{"));
    }
    
    #[test]
    fn test_greeting_varies_by_relationship() {
        let mut rng = StdRng::seed_from_u64(42);
        let personality = Personality::default();
        
        let close_greeting = generate_greeting(&personality, 0.9, &mut rng);
        let distant_greeting = generate_greeting(&personality, 0.1, &mut rng);
        
        // Both should be non-empty
        assert!(!close_greeting.is_empty());
        assert!(!distant_greeting.is_empty());
    }
}
