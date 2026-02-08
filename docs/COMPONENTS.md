# ProgShip Components Reference

This document details all ECS components used in the ProgShip simulation.

## Entity Types

| Entity Type | Description | Key Components |
|-------------|-------------|----------------|
| Person | Crew or passenger | Person, Name, Position, Needs, Activity, Personality |
| Room | Ship compartment | Room, RoomConnections, Deck |
| ShipSystem | Power, life support, etc. | ShipSystem, ResourceFlow |

---

## Person Components

### `Person`
Marker component identifying an entity as a person.

```rust
pub struct Person;
```

### `Name`
Identity information.

```rust
pub struct Name {
    pub given_name: String,
    pub family_name: String,
    pub nickname: Option<String>,
}

impl Name {
    pub fn full_name(&self) -> String;
}
```

### `Position`
Current location in the ship.

```rust
pub struct Position {
    pub room_id: u32,
    pub local: Vec3,  // Position within room (meters)
}
```

### `Movement`
Present only when actively moving.

```rust
pub struct Movement {
    pub destination: Vec3,       // Current target
    pub final_destination: Vec3, // Ultimate goal
    pub speed: f32,              // m/s (default: 1.4)
    pub path: Vec<u32>,          // Room IDs to traverse
    pub path_index: usize,       // Current room index
    pub next_door_position: Option<Vec3>,
    pub entry_door_positions: Vec<Vec3>,
    pub exit_door_positions: Vec<Vec3>,
}
```

### `Needs`
Physical and psychological needs.

```rust
pub struct Needs {
    pub hunger: f32,   // 0.0 (full) to 1.0 (starving)
    pub fatigue: f32,  // 0.0 (rested) to 1.0 (exhausted)
    pub social: f32,   // 0.0 (content) to 1.0 (lonely)
    pub comfort: f32,  // 0.0 (comfortable) to 1.0 (uncomfortable)
    pub hygiene: f32,  // 0.0 (clean) to 1.0 (dirty)
}

impl Needs {
    pub fn most_urgent(&self, threshold: f32) -> Option<NeedType>;
}
```

**Decay Rates (per sim hour):**
- Hunger: 0.04 (hungry after ~25h)
- Fatigue: 0.0625 (tired after ~16h)
- Social: 0.02 (lonely after ~50h)
- Hygiene: 0.08 (dirty after ~12h)

### `Activity`
Current activity state.

```rust
pub struct Activity {
    pub activity_type: ActivityType,
    pub started_at: f64,      // Sim time
    pub duration: f32,        // Expected duration (hours)
    pub target_id: Option<u32>,
}
```

**ActivityType variants:**
| Type | Description | Duration |
|------|-------------|----------|
| `Idle` | Standing around | Variable |
| `Walking` | Moving to destination | Until arrival |
| `Sleeping` | Resting | 6-8 hours |
| `Eating` | At mess hall | 0.5-1 hour |
| `Working` | On duty | Shift length |
| `Socializing` | In conversation | 0.25-1 hour |
| `Resting` | Relaxing | 0.5-2 hours |
| `UsingFacilities` | Hygiene | 0.25-0.5 hours |
| `Entertainment` | Recreation | 0.5-2 hours |
| `OnDuty` | Active shift | 8 hours |
| `OffDuty` | Between shifts | 16 hours |
| `Emergency` | Responding to event | Until resolved |

### `Personality`
Big Five personality traits.

```rust
pub struct Personality {
    pub openness: f32,          // -1.0 to 1.0
    pub conscientiousness: f32,
    pub extraversion: f32,
    pub agreeableness: f32,
    pub neuroticism: f32,
}
```

**Effects:**
- High extraversion → more social interactions
- High neuroticism → more complaints, anxious tone
- High agreeableness → friendlier relationships
- High conscientiousness → better at duties

### `Skills`
Competency levels.

```rust
pub struct Skills {
    pub engineering: f32,  // 0.0 to 1.0
    pub medical: f32,
    pub piloting: f32,
    pub science: f32,
    pub social: f32,
    pub combat: f32,
}
```

---

## Role Components

### `Crew`
Crew member data (not present on passengers).

```rust
pub struct Crew {
    pub department: Department,
    pub rank: Rank,
    pub shift: Shift,
    pub duty_station: u32,  // Room ID
}
```

**Department:**
- Command, Engineering, Medical, Science, Security, Operations

**Rank:**
- Crewman, Specialist, Petty, Chief, Ensign, Lieutenant, Commander, Captain

**Shift:**
- Alpha (0600-1400), Beta (1400-2200), Gamma (2200-0600)

### `Passenger`
Passenger data (not present on crew).

```rust
pub struct Passenger {
    pub cabin_class: CabinClass,
    pub destination: String,
}
```

**CabinClass:**
- First, Standard, Steerage

### `DutySchedule`
Shift timing for crew.

```rust
pub struct DutySchedule {
    pub shift: Shift,
    pub duty_start: f32,   // Hour of day
    pub duty_end: f32,
    pub duty_station: u32, // Room ID
}
```

### `Faction`
Political/social group affiliation.

```rust
pub enum Faction {
    // Crew factions
    Command,
    Engineering,
    Medical,
    Science,
    Security,
    Operations,
    // Passenger factions
    FirstClass,
    StandardClass,
    Steerage,
}

impl Faction {
    pub fn affinity(&self, other: &Faction) -> f32;  // -1.0 to 1.0
}
```

---

## Social Components

### `InConversation`
Marker for people in a conversation.

```rust
pub struct InConversation {
    pub conversation_id: u32,
    pub partner: Entity,
}
```

### `Relationship`
Stored in RelationshipGraph, not as component.

```rust
pub struct Relationship {
    pub person_a: Entity,
    pub person_b: Entity,
    pub relationship_type: RelationshipType,
    pub strength: f32,        // -1.0 (hostile) to 1.0 (close)
    pub familiarity: f32,     // 0.0 (stranger) to 1.0 (intimate)
    pub last_interaction: f64,
}
```

---

## Room Components

### `Room`
Ship compartment.

```rust
pub struct Room {
    pub id: u32,
    pub name: String,
    pub room_type: RoomType,
    pub bounds: RoomBounds,
    pub deck_level: i32,
    pub world_x: f32,
    pub world_y: f32,
    pub doors: Vec<Door>,
    pub capacity: u32,
}
```

**RoomType:**
- Bridge, Engineering, Quarters, Mess, Medical, Cargo
- Recreation, Corridor, Airlock, Laboratory, Observatory, Cryo

### `RoomConnections`
Adjacency for pathfinding.

```rust
pub struct RoomConnections {
    pub connected_to: Vec<u32>,  // Room IDs
}
```

### `Deck`
Ship level.

```rust
pub struct Deck {
    pub level: i32,     // 0 = main deck
    pub name: String,
}
```

---

## Ship System Components

### `ShipSystem`
Major ship system.

```rust
pub struct ShipSystem {
    pub name: String,
    pub system_type: SystemType,
    pub health: f32,          // 0.0 to 1.0
    pub status: SystemStatus,
}
```

**SystemType:**
- Power, LifeSupport, Propulsion, Navigation
- Communications, Medical, FoodProduction, WaterRecycling

**SystemStatus:**
- Nominal, Degraded, Critical, Offline, Destroyed

### `ResourceFlow`
Resource production/consumption.

```rust
pub struct ResourceFlow {
    pub consumes: Vec<(ResourceType, f32)>,
    pub produces: Vec<(ResourceType, f32)>,
}
```

### `MaintenanceTask`
Repair work item.

```rust
pub struct MaintenanceTask {
    pub id: u32,
    pub system_id: u32,
    pub task_type: MaintenanceType,
    pub priority: u8,
    pub estimated_hours: f32,
    pub assigned_to: Option<Entity>,
    pub progress: f32,
}
```

---

## Event Components

### `Event` (in EventManager)
Random events are not components but managed in EventManager.

```rust
pub struct Event {
    pub id: u32,
    pub event_type: EventType,
    pub room_id: u32,
    pub severity: u8,         // 1-5
    pub state: EventState,
    pub created_at: f64,
    pub responders_needed: u8,
    pub responders_assigned: u8,
}
```

**EventType:**
- SystemFailure, MedicalEmergency, Fire, HullBreach
- Discovery, Celebration, Altercation, ResourceShortage

**EventState:**
- Active, BeingHandled, Resolved, Escalated

---

## Serialization

All components derive `Serialize` and `Deserialize` for save/load:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleComponent {
    pub field: T,
}
```

Entity references (`Entity`) are converted to u64 IDs during serialization and remapped on load.
