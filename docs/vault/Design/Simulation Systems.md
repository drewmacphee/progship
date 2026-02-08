# Simulation Systems

## Architecture
All simulation runs server-side in SpacetimeDB reducers. Clients are pure observers + input senders.

### Tiered Update Frequencies
| Tier | Frequency | Systems |
|------|-----------|---------|
| T0 | 60 Hz | Movement interpolation |
| T1 | 1 Hz | Activity state machines, social interactions |
| T2 | 0.1 Hz | Needs decay, duty scheduling |
| T3 | 0.01 Hz | Ship systems, maintenance, atmosphere, events |

---

## Implemented Systems ✅

### Needs System
Every person has: **hunger, fatigue, social, comfort, hygiene, health, morale**
- Decay over time at different rates
- Activities satisfy specific needs (eating → hunger, sleeping → fatigue)
- Low health = danger (atmosphere, starvation, events)
- Morale affects work quality and social behavior

### Activity System
State machine per person:
- **Idle** → scan needs → pick highest-priority activity
- **Moving** → pathfinding to target room
- **Performing** → duration timer, need satisfaction
- Activities: eat, sleep, work, exercise, socialize, repair, hygiene, relax

### Social & Conversations
- NPCs initiate conversations when social need is high + someone nearby
- 9 topic types: greeting, work, gossip, personal, complaint, request, flirtation, argument, farewell
- Topic selection based on: relationship familiarity, personality, crew status, morale
- Effects: social need recovery, morale changes, relationship strength updates

### Relationships
- `Connection` table: (person_a, person_b, strength, familiarity)
- Strength evolves through interactions
- Familiarity increases with time spent together

### Duty & Scheduling
- Three shifts: Alpha, Beta, Gamma
- Crew assigned to departments with duty stations
- Off-duty → free to satisfy personal needs
- On-duty → move to workstation, perform job activities

### Atmosphere (Per-Deck)
- O2, CO2, humidity, temperature tracked per deck
- People generate CO2, consume O2 (metabolic output)
- Life support counteracts (CO2 scrubbing, O2 generation)
- Effects: low O2 → health damage, high CO2 → fatigue

### Ship Systems & Maintenance
- Power, life support, engines — health degradation over time
- Maintenance tasks auto-generated for damaged systems
- Cascading failures (power loss → life support fails → atmosphere degrades)

### Events
8 types: fire, hull breach, medical emergency, system failure, resource shortage, altercation, discovery, celebration

### Movement
- Grid-based, distance-based door detection, 20Hz input batching
- Elevator (number keys) and ladder (Up/Down arrows) shaft traversal
- BFS pathfinding through door graph for NPCs

---

## Planned / Ideas 💡

### Environmental Grid (from Gemini)
Per-room atmosphere instead of per-deck. O2 flows between adjacent rooms based on pressure differentials. Sealed rooms deplete fast during fire. Ventilation routing becomes critical.

### Enhanced Utility AI (from Gemini)
- Overcrowding stress (room capacity matters)
- Food quality affecting morale
- Noise modeling (engineering = noisy = stress)
- Oxygen saturation as a metabolic need

### Economy System (from Gemini)
Production vs consumption loop: hydroponics output, water recycling, oxygen generation. Scarcity → rationing → morale effects.

### Flip Maneuver / Zero-G
Mid-voyage ship rotation for deceleration. Zero-G transition, unsecured objects float, system malfunctions.

### LOD for 10,000 Agents
Tiered processing by distance from player. Only render current deck. Reduced tick frequency for distant agents.

## See Also
- [[Architecture Overview]]
- [[Ship Overview]]
- [[Open Problems]]
