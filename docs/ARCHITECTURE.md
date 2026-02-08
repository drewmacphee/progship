# ProgShip Architecture

## Overview

ProgShip is a real-time simulation engine for deep space colony ships with thousands of simulated individuals. It uses an Entity Component System (ECS) architecture for performance and flexibility.

## Design Principles

1. **True Simulation**: Every person is individually simulated, not statistically sampled
2. **Engine Agnostic**: Core simulation has no rendering dependencies
3. **Data-Oriented**: Components are pure data; systems contain logic
4. **Tiered Updates**: Different frequencies for different systems based on fidelity needs

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        Game Engine                               │
│  (Godot / Unity / Bevy / Custom)                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ progship-ffi │  │progship-godot│  │  Direct Rust Link    │  │
│  │   (C API)    │  │ (GDExtension)│  │                      │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘  │
│         │                  │                     │              │
│         └──────────────────┼─────────────────────┘              │
│                            │                                     │
├────────────────────────────┼────────────────────────────────────┤
│                            ▼                                     │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   progship-core                           │   │
│  │                                                           │   │
│  │  ┌─────────────────────────────────────────────────────┐ │   │
│  │  │                  SimulationEngine                    │ │   │
│  │  │  - world: hecs::World                               │ │   │
│  │  │  - sim_time: f64                                    │ │   │
│  │  │  - resources, events, relationships, conversations  │ │   │
│  │  └─────────────────────────────────────────────────────┘ │   │
│  │                                                           │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │   │
│  │  │ components/ │  │  systems/   │  │   generation/   │   │   │
│  │  │  (Data)     │  │  (Logic)    │  │  (Procedural)   │   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## ECS (Entity Component System)

### Entities

Entities are just IDs. ProgShip has these entity types:
- **Person**: Crew members and passengers (~5,000)
- **Room**: Ship compartments (~200)
- **ShipSystem**: Power, life support, etc. (~20)
- **Conversation**: Temporary dialogue entities

### Components

Components are pure data structs attached to entities:

```rust
// Identity
Person          // Marker component
Name            // Given and family name

// Spatial
Position        // room_id + local coordinates
Movement        // Destination, speed, path

// Physical needs
Needs           // hunger, fatigue, social, comfort, hygiene

// Behavior
Activity        // Current activity type and duration
Personality     // Big Five traits

// Role-specific
Crew            // department, rank, shift, duty_station
Passenger       // cabin_class, destination
DutySchedule    // shift timings and duty station

// Social
InConversation  // Currently in a conversation
Faction         // Political/social group
```

### Systems

Systems are functions that operate on components:

| System | Frequency | Purpose |
|--------|-----------|---------|
| `movement_system` | 60Hz | Interpolate positions |
| `activity_system` | 60Hz | Manage activity state machines |
| `wandering_system` | 10Hz | Give idle people destinations |
| `needs_system` | 0.1Hz | Decay needs over time |
| `social_system` | 0.1Hz | Start/update conversations |
| `duty_system` | 0.1Hz | Manage crew shift changes |
| `ship_systems_system` | 0.01Hz | Resource production/consumption |
| `maintenance_system` | 0.01Hz | Generate and progress repairs |
| `events_system` | 0.01Hz | Generate random events |

## Tiered Update System

The simulation runs systems at different frequencies to balance fidelity and performance:

```
┌─────────────────────────────────────────────────────────────┐
│  Frame (16.6ms @ 60fps)                                      │
│                                                              │
│  T0 (Every Frame):                                           │
│  ├── movement_system     ← Position interpolation            │
│  └── activity_system     ← Activity state checks             │
│                                                              │
│  T1 (Every 100ms):                                           │
│  └── wandering_system    ← Assign destinations to idle       │
│                                                              │
│  T2 (Every 10 seconds):                                      │
│  ├── needs_system        ← Decay hunger, fatigue, etc.       │
│  ├── social_system       ← Conversation management           │
│  └── duty_system         ← Shift changes                     │
│                                                              │
│  T3 (Every 100 seconds):                                     │
│  ├── ship_systems_system ← Power, life support               │
│  ├── maintenance_system  ← Repairs                           │
│  └── events_system       ← Random emergencies                │
└─────────────────────────────────────────────────────────────┘
```

## Ship Layout

Ships are generated procedurally with hull-fitting algorithms:

```
┌─────────────────────────────────────────────────────────────┐
│                         Deck 1 (Bridge)                      │
│  ┌─────────┐ ┌────────────────────────┐ ┌─────────┐        │
│  │ Bridge  │ │       Corridor         │ │Officers │        │
│  └─────────┘ └────────────────────────┘ └─────────┘        │
├─────────────────────────────────────────────────────────────┤
│                        Deck 2 (Crew)                         │
│  ┌─────────┐ ┌────────────────────────┐ ┌─────────┐        │
│  │ Quarters│ │       Corridor         │ │  Mess   │        │
│  └─────────┘ └────────────────────────┘ └─────────┘        │
├─────────────────────────────────────────────────────────────┤
│                     Deck 3 (Passenger)                       │
│  ┌─────────┐ ┌────────────────────────┐ ┌─────────┐        │
│  │ Cabins  │ │       Corridor         │ │Recreation│       │
│  └─────────┘ └────────────────────────┘ └─────────┘        │
└─────────────────────────────────────────────────────────────┘
```

Rooms are connected via:
- **Corridors**: Central spine on each deck
- **Doors**: Between adjacent rooms
- **Elevators**: Between decks (at fore/aft corridor ends)

## Pathfinding

Movement uses door-based pathfinding:

1. Find path of rooms from current to destination
2. For each room transition, find the connecting door
3. Move to door position, enter next room, repeat

```rust
struct Movement {
    destination: Vec3,       // Final target position
    final_destination: Vec3, // Ultimate goal
    speed: f32,              // Movement speed (m/s)
    path: Vec<u32>,          // Room IDs to traverse
    path_index: usize,       // Current room in path
    next_door_position: Option<Vec3>,
    entry_door_positions: Vec<Vec3>,
    exit_door_positions: Vec<Vec3>,
}
```

## Social System

### Conversations

Conversations are entities with participant tracking:

```rust
struct Conversation {
    participants: Vec<Entity>,
    topic: ConversationTopic,
    state: ConversationState,
    start_time: f64,
}
```

People within proximity may start conversations based on:
- Social need level
- Personality (extraversion)
- Existing relationship strength
- Both being idle or in compatible activities

### Relationships

Relationships form a graph between people:

```rust
struct RelationshipGraph {
    relationships: HashMap<(Entity, Entity), Relationship>,
}

struct Relationship {
    strength: f32,      // -1.0 hostile to 1.0 close
    familiarity: f32,   // 0.0 stranger to 1.0 intimate
    last_interaction: f64,
}
```

### Factions

People belong to factions that affect social dynamics:

- **Crew Factions**: Command, Engineering, Medical, Science, Security, Operations
- **Passenger Factions**: FirstClass, StandardClass, Steerage

Faction affinity modifies relationship formation and conversation topics.

## Events System

Random events add drama and emergencies:

```rust
enum EventType {
    SystemFailure,    // Ship system breaks down
    MedicalEmergency, // Someone needs medical attention
    Fire,             // Fire in a compartment
    HullBreach,       // Pressure loss (severe)
    Discovery,        // Positive: science discovery
    Celebration,      // Positive: morale boost
    Altercation,      // Conflict between people
    ResourceShortage, // Running low on supplies
}
```

Events have:
- **Severity**: 1-5 scale
- **Location**: Room where event occurs
- **Responders**: Required crew to handle
- **State**: Active → BeingHandled → Resolved/Escalated

## Save/Load System

Complete simulation state is serialized via bincode:

```rust
struct SaveData {
    version: u32,
    sim_time: f64,
    time_scale: f32,
    ship_layout: Option<SerializableShipLayout>,
    resources: ShipResources,
    maintenance_queue: MaintenanceQueue,
    relationships: RelationshipGraph,
    conversations: ConversationManager,
    events: EventManager,
    entities: Vec<SerializableEntity>,
}
```

Entity serialization captures all attached components dynamically.

## Integration Points

### C FFI (`progship-ffi`)

```c
// Create simulation
SimHandle* sim = sim_create();
sim_generate(sim, "ISV Prometheus", 5, 200, 800);

// Game loop
sim_update(sim, delta_seconds);
int count = sim_get_person_count(sim);

// Query state
float x, y, z;
sim_get_person_position(sim, person_id, &x, &y, &z);

// Cleanup
sim_destroy(sim);
```

### Godot GDExtension (`progship-godot`)

```gdscript
var sim = ProgShipSimulation.new()
sim.generate("ISV Prometheus", 5, 200, 800)

func _process(delta):
    sim.update(delta)
    for person in sim.get_all_people():
        var pos = person["position"]  # {x, y, z}
        var needs = person["needs"]   # {hunger, fatigue, ...}
```

## Performance

The tiered update system enables scaling to 10,000+ agents:

| Agents | Frame Time | FPS |
|--------|------------|-----|
| 1,000  | 0.15ms     | 60+ |
| 5,000  | 0.8ms      | 60+ |
| 10,000 | 1.6ms      | 60+ |

Key optimizations:
- Movement at 60Hz operates only on Position + Movement components
- Needs decay at 0.1Hz reduces per-frame work
- Ship systems at 0.01Hz (every ~100 seconds sim time)
- Parallel-safe queries (hecs supports rayon)
