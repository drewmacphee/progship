# ProgShip

A real-time simulation engine for deep space colony ships with 5,000+ crew and passengers. Built with Rust and Bevy.

## Features

- **ECS Architecture**: Entity Component System using `hecs` for scalable simulation
- **Real-time Simulation**: True simulation of all individuals (not statistical sampling)
- **Tiered Updates**: Movement at 60Hz, activities at 1Hz, needs at 0.1Hz for efficiency
- **Procedural Generation**: Ships, crew, passengers generated from configurable templates
- **Social Simulation**: Conversations, relationships, faction dynamics
- **Ship Systems**: Power, life support, maintenance, random events
- **Crew Duties**: Shift-based duty schedules (Alpha/Beta/Gamma)
- **Save/Load**: Binary serialization of complete simulation state
- **Bevy Visualization**: Top-down 2D viewer for development and gameplay

## Requirements

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- Windows/macOS/Linux

## Quick Start

```bash
# Clone and build
git clone https://github.com/yourusername/progship.git
cd progship

# Build the simulation core
cargo build --package progship-core --release

# Run tests (51 tests)
cargo test

# Run the visualization
cargo run --package progship-viewer --release
```

## Viewer Controls

| Key | Action |
|-----|--------|
| 1-5 | Switch to deck 1-5 |
| PageUp/Down | Navigate decks |
| +/- | Speed up/slow down time |
| Space | Pause/resume |
| Click | Select person |
| Scroll | Zoom in/out |
| Drag | Pan camera |
| Ctrl+S | Save simulation |
| Ctrl+L | Load simulation |

## Project Structure

```
crates/
├── progship-core/       # Simulation engine (ECS-based)
│   ├── components/      # Data: Person, Room, Needs, Activity, etc.
│   ├── systems/         # Logic: movement, needs, social, events, duty
│   ├── generation/      # Procedural ship and crew generation
│   └── persistence.rs   # Save/load system
├── progship-viewer/     # Bevy-based 2D visualization
└── progship-ffi/        # C FFI bindings for external integration
```

## Architecture

The simulation uses an **Entity Component System** where:
- **Entities** are IDs (person, room, ship system)
- **Components** are pure data (Position, Needs, Crew, Activity)
- **Systems** are logic that operate on components (movement_system, needs_system)

### Key Components

| Component | Description |
|-----------|-------------|
| `Person` | Marker for human entities |
| `Position` | Location (room_id + local coordinates) |
| `Needs` | Hunger, fatigue, social, comfort, hygiene |
| `Activity` | Current activity (Working, Eating, Sleeping, etc.) |
| `Crew` | Department, rank, shift, duty station |
| `Passenger` | Cabin class, destination |
| `Personality` | Big Five traits |
| `Faction` | Political/social group affiliation |

### Systems (Update Order)

| Tier | Frequency | Systems |
|------|-----------|---------|
| T0 | 60Hz | Movement interpolation |
| T1 | 10Hz | Wandering, activity updates |
| T2 | 0.1Hz | Needs decay, social, duty |
| T3 | 0.01Hz | Ship systems, maintenance, events |

## Performance

Benchmarked with tiered update system:
- 1,000 agents: 0.15ms/frame
- 5,000 agents: 0.8ms/frame  
- 10,000 agents: 1.6ms/frame

All well under 16ms budget for 60 FPS.

## License

TBD
