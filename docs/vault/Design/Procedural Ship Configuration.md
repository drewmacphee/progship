# Procedural Ship Configuration System

> Design document for the multi-stage procedural generation pipeline that selects ship systems,  
> sizes crew and infrastructure, and fills out the supply manifest based on mission parameters.

## Overview

Ship generation currently takes 3 inputs (`deck_count`, `crew_count`, `passenger_count`) and produces a hardcoded rectangular ship with fixed systems. This document designs a **mission-driven generation pipeline** where the destination, propulsion choice, and system selections cascade into a coherent, internally-consistent ship.

The system serves two modes:
1. **Random default** — generate a coherent mission brief with reasonable defaults
2. **Player customization** — allow the player to tweak destination, ship type, systems, and colony size before generation

---

## Generation Pipeline (6 Stages)

```
Stage 1: Mission Parameters
    ↓
Stage 2: Propulsion & Voyage Profile
    ↓
Stage 3: System Selection (7 categories)
    ↓
Stage 4: Population & Crew Sizing
    ↓
Stage 5: Ship Layout & Facility Manifest
    ↓
Stage 6: Supply Manifest & Stockpile Calculation
```

Each stage's outputs feed the next stage's inputs. The pipeline can be run fully procedurally or paused at any stage for player customization.

---

## Stage 1: Mission Parameters

The seed inputs that drive everything else.

### MissionConfig

| Field | Type | Range | Default | Notes |
|-------|------|-------|---------|-------|
| `destination` | enum | See table below | Random weighted | Drives voyage duration |
| `colony_target_pop` | u32 | 500–10,000 | 2,500 | How many colonists at arrival |
| `mission_priority` | enum | Exploration / Colonization / Evacuation | Colonization | Affects risk tolerance |
| `tech_level` | enum | Conservative / Standard / Advanced | Standard | Unlocks system variants |
| `budget_class` | enum | Austerity / Standard / Abundant | Standard | Affects redundancy, reserves |
| `seed` | u64 | — | Random | Reproducible generation |

### Destination Table

| Destination | Distance (ly) | Habitability Score | Risk Factor | Notes |
|-------------|--------------|-------------------|-------------|-------|
| Proxima Centauri b | 4.24 | 0.6 | Medium | Stellar flares, uncertain atmosphere |
| Alpha Centauri Bb | 4.37 | 0.7 | Low | Sun-like star, unconfirmed planets |
| Barnard's Star b | 5.96 | 0.4 | Medium | Cold super-Earth |
| Luyten b (GJ 273b) | 12.36 | 0.7 | Low | Habitable zone super-Earth |
| Ross 128 b | 10.89 | 0.8 | Low | Quiet red dwarf, temperate zone |
| TRAPPIST-1 e | 39.46 | 0.9 | High | Best habitability, very long voyage |
| Tau Ceti e | 11.91 | 0.6 | Medium | Debris disk hazard |
| Kepler-442b | 1,206 | 0.95 | Extreme | Multi-generation, theoretical only |

---

## Stage 2: Propulsion & Voyage Profile

Propulsion choice determines voyage duration, which determines everything downstream.

### Propulsion Variants

| Variant | Cruise Speed | Accel Time | Tech Level | Fuel Type | Fuel Mass Factor | Notes |
|---------|-------------|-----------|-----------|-----------|-----------------|-------|
| **Fission Pulse (Orion)** | 0.005c | 5 years | Conservative | Fission pellets | 3.0× payload | Proven physics, heavy |
| **Fusion Torch** | 0.03c | 2 years | Standard | D-He3 pellets | 1.5× payload | Baseline ISV Prometheus |
| **Advanced Fusion** | 0.05c | 1.5 years | Advanced | D-He3 + antimatter catalyst | 1.2× payload | Higher efficiency |
| **Ion Drive (Nuclear Electric)** | 0.01c | 10 years | Conservative | Xenon / Hydrogen | 0.8× payload | Slow but efficient |
| **Laser Sail (Departure Boost)** | 0.1c | 0 (external) | Advanced | None (beamed) | 0.5× payload | Can't decelerate without magsail |
| **Hybrid Sail + Fusion** | 0.08c | 0 + 2 years decel | Advanced | D-He3 (decel only) | 0.7× payload | Sail accel, fusion decel |

### Derived Voyage Profile

```rust
struct VoyageProfile {
    propulsion: PropulsionType,
    cruise_speed_c: f64,          // fraction of c
    accel_duration_years: f64,
    cruise_duration_years: f64,
    decel_duration_years: f64,
    total_duration_years: f64,
    fuel_mass_tons: f64,          // drives propellant tankage
    reaction_mass_tons: f64,
}
```

**Calculation**: `total_duration = accel + (distance / speed) + decel`

Example: Proxima Centauri b + Fusion Torch:
- Accel: 2 years, Cruise: 139 years at 0.03c, Decel: 2 years → **143 years total**

---

## Stage 3: System Selection

Each of the 7 major system categories has 2–4 variants with different tradeoffs. The selection is driven by `tech_level`, `budget_class`, and `mission_priority`, but can be overridden by the player.

### 3.1 Power Systems

| Variant | Output (MW) | Fuel Consumption | MTBF | Mass (tons) | Tech Level | Tradeoff |
|---------|------------|-----------------|------|-------------|-----------|---------|
| **Fission Reactor** | 10 | 2 kg/day U-235 | 5 yr | 500 | Conservative | Reliable, heavy, finite fuel |
| **Fusion Reactor (D-He3)** | 20 | 0.5 kg/day | 10 yr | 300 | Standard | Balanced — current baseline |
| **Advanced Fusion (D-T + breeding)** | 30 | 0.3 kg/day | 8 yr | 250 | Advanced | High output, complex maintenance |
| **Antimatter-Catalyzed** | 50 | 0.01 kg/day AM | 3 yr | 150 | Advanced | Incredible output, fragile, catastrophic failure mode |

### 3.2 Life Support

| Variant | Recovery Rate | Power Draw | MTBF | Crew Needed | Tech Level | Tradeoff |
|---------|-------------|-----------|------|-------------|-----------|---------|
| **Open-Loop + Reserves** | 70% water, 50% O₂ | Low | 15 yr | 10 | Conservative | Simple, massive water/O₂ stockpile needed |
| **Standard ECLSS** | 96% water, 99% O₂ | 120 kW | 2 yr | 35 | Standard | ISS-derived, proven — current baseline |
| **Bioregenerative (BLSS)** | 99% water, 99.5% O₂ | 80 kW | 1 yr | 50 | Advanced | Plants do the work, but fragile ecosystem |
| **Hybrid ECLSS + BLSS** | 98% water, 99.5% O₂ | 100 kW | 3 yr | 40 | Standard | Best of both, moderate complexity |

### 3.3 Food Production

| Variant | Output Capacity | Growing Area | Power Draw | Crew Needed | Tech Level | Tradeoff |
|---------|----------------|-------------|-----------|-------------|-----------|---------|
| **Stored Rations Only** | 0 (all from stock) | 0 m² | 0 kW | 0 | Conservative | Simple, finite — only viable for short voyages |
| **Hydroponics + Algae** | 80% of demand | 6,000 m² | 96 kW | 80 | Standard | Current baseline |
| **Full Bioloop** | 100%+ of demand | 10,000 m² | 150 kW | 120 | Advanced | Self-sustaining but space-hungry |
| **Cultured + Hydroponics** | 95% of demand | 4,000 m² | 120 kW | 105 | Standard | More protein variety, less space |

### 3.4 Propulsion (from Stage 2)

Already selected — drives this stage's integration. Determines:
- Engine room count and size
- Fuel storage requirements
- Engineering crew allocation

### 3.5 Water Systems

| Variant | Recovery Rate | Capacity (m³/day) | MTBF | Tech Level | Tradeoff |
|---------|-------------|-------------------|------|-----------|---------|
| **Basic Filtration** | 85% | 120 | 3 yr | Conservative | Simple, high water loss |
| **RO + UV Sterilization** | 96% | 140 | 1 yr | Standard | Current baseline |
| **Membrane Bioreactor** | 99% | 160 | 2 yr | Advanced | Near-perfect recovery, complex biology |

### 3.6 Harvesting & Supplementation (Optional)

| System | Available At | Power Draw | Crew Needed | Effect |
|--------|-------------|-----------|-------------|--------|
| **Ramscoop** | Standard+ | 500 kW | 8 | +0.5–2 kg H/day → water supplement |
| **Magsail** | Standard+ | Passive | 4 | +50–800 kW supplemental power |
| **Mining Drones (×6)** | Advanced | 0 (idle) / 200 kW (active) | 12 (surge) | Comet mining opportunity |
| **Synthesis Lab** | Standard+ | 50 kW | 6 | Polymer, fertilizer, chemical synthesis |

Each is independently selectable. `budget_class: Austerity` may exclude all; `Abundant` includes all.

### 3.7 Defensive & Safety Systems

| System | Effect | Mass (tons) | Crew Needed | Tech Level |
|--------|--------|-------------|-------------|-----------|
| **Whipple Shield (Passive)** | Micrometeorite protection | 200 | 0 | Conservative |
| **Magnetic Deflector** | Charged particle + small debris | 100 | 4 | Standard |
| **Point Defense Laser** | Large debris ablation | 50 | 8 | Advanced |
| **Radiation Storm Shelter** | Solar/cosmic ray protection | 150 | 0 | Conservative |

### Selection Algorithm

```
for each system_category:
    available_variants = filter by tech_level
    
    if mission_priority == Evacuation:
        prefer lowest_mass, fastest_deployment
    elif mission_priority == Colonization:
        prefer highest_reliability, best_recovery_rates
    elif mission_priority == Exploration:
        prefer most_advanced, highest_output
    
    if budget_class == Austerity:
        exclude optional systems; prefer cheapest
    elif budget_class == Abundant:
        include all optional systems; prefer best
    
    selected = weighted_random(available_variants, priority_weights)
```

---

## Stage 4: Population & Crew Sizing

Population is derived from mission parameters + selected systems, not input directly.

### Colony Population Calculation

```
base_colonists = colony_target_pop                    // from MissionConfig

// Colonists need to arrive healthy after N years
birth_rate = 0.015/year    // 1.5% — controlled, not exponential
death_rate = 0.008/year    // 0.8% — good medical care
net_growth = 0.007/year

// Work backwards from target arrival population
departure_colonists = colony_target_pop / (1 + net_growth)^voyage_years
// Clamp: departure_colonists >= 500 (genetic diversity minimum)
```

### Crew Sizing (Derived from Selected Systems)

Each selected system declares its crew requirement. Total crew = sum of all system crews + overhead.

```
crew_requirement = Σ (system.crew_needed for each selected system)
                 + ship_officers(deck_count)        // ~2 per deck
                 + security(total_population * 0.02) // 2% of population
                 + medical(total_population * 0.01)  // 1% of population
                 + education(children_estimate * 0.05) // teachers
                 + administration(total_population * 0.005)
                 + reserve_factor(1.15)              // 15% slack for shifts/leave
```

### Department Breakdown Template

| Department | Sizing Rule | Example (7,500 pop) |
|-----------|------------|-------------------|
| Engineering | Sum of all system crew requirements | ~215 |
| Medical | 1% of total population | ~75 |
| Security | 2% of total population | ~150 |
| Science | 20 + (harvesting crew if selected) | ~40 |
| Operations | 2 per deck + administration | ~80 |
| Command | 15 + 2 per 1,000 population | ~30 |
| Agriculture | From food production system crew | ~105 |
| Education | 5% of estimated children | ~40 |
| **Total Crew** | | **~735** |

> Note: The remaining ~4,265 crew (of 5,000) serve rotating backup, training, and cross-functional roles. This provides the 3-shift coverage and leave rotation.

### Population Summary Output

```rust
struct PopulationProfile {
    total_population: u32,
    crew_count: u32,
    passenger_count: u32,     // = departure_colonists
    department_counts: HashMap<Department, u32>,
    children_estimate: u32,   // 15% of passengers for long voyages
    genetic_diversity_ok: bool, // >= 500 unrelated individuals
}
```

---

## Stage 5: Ship Layout & Facility Manifest

With population, systems, and crew sizes known, compute the physical ship.

### Deck Count Calculation

```
// Minimum habitable area per person
area_per_person = 40 m²  // cabin + shared spaces + corridors

// System-specific space requirements
system_area = Σ (system.room_count × system.avg_room_area)
habitation_area = total_population × area_per_person
total_usable_area = system_area + habitation_area

// Deck dimensions (from ship type)
deck_usable_area = deck_width × deck_length × usable_fraction(0.7)

deck_count = ceil(total_usable_area / deck_usable_area)
           + infrastructure_decks(ceil(deck_count / 3))  // service decks
           + 2  // command + engineering minimum
```

### Facility Manifest Generation

Each selected system declares the rooms it needs:

```rust
struct SystemRoomRequirement {
    room_type: RoomType,
    count: u32,             // how many of this room
    min_area: f32,          // m²
    max_area: f32,          // m²
    deck_zone: DeckZone,    // where it should be placed
    requires_adjacency: Option<RoomType>,  // must be near this type
    scales_with: ScalingFactor, // what drives count/size
}

enum ScalingFactor {
    Fixed,                           // always this many (e.g., 1 bridge)
    PerPopulation(f32),              // 1 per N people (e.g., mess halls)
    PerSystem(SystemCategory),       // 1 per instance of a system
    PerDeck,                         // 1 per habitable deck
}
```

### Scaling Examples

| Room Type | Scaling Rule | For 7,500 pop |
|-----------|-------------|---------------|
| Bridge | Fixed(1) | 1 |
| Mess Hall | PerPopulation(500) | 15 |
| Single Cabin | PerPopulation(1) × cabin_fraction | ~4,000 |
| Double Cabin | PerPopulation(2) × couple_fraction | ~800 |
| Family Cabin | PerPopulation(4) × family_fraction | ~200 |
| Medical Bay | PerPopulation(1500) | 5 |
| Hydroponics Bay | PerSystem(FoodProduction) | 12 (if Hydro+Algae selected) |
| Reactor Room | PerSystem(Power) × redundancy | 2 |
| Water Treatment | PerSystem(Water) | 4 |
| Machine Shop | Fixed(2) | 2 |
| School | PerPopulation(2000) for long voyages | 4 |
| Gym | PerPopulation(1000) | 8 |
| Chapel/Meditation | PerPopulation(2000) | 4 |

### Deck Zone Assignment

```
Deck allocation priority (bow to stern):
  1. Command (bridge, comms, nav) — always forward
  2. Habitation (cabins, mess, recreation) — largest zone
  3. Medical (sickbay, pharmacy, quarantine) — mid-ship
  4. Agriculture (hydroponics, bioreactors) — mid-ship (needs light/power)
  5. Science (labs, synthesis, observatory) — variable
  6. Cargo & Storage (supplies, spare parts) — aft
  7. Engineering (reactor, engines, fabrication) — always aft
  8. Service decks — interstitial (every 3rd deck)
```

---

## Stage 6: Supply Manifest & Stockpile Calculation

The final stage fills out the supply manifest based on population, systems, and voyage duration.

### Per-Person Consumption (from Personal Supplies Manifest)

```rust
struct PersonalConsumption {
    food_kg_per_day: f32,       // 1.81
    water_l_per_day: f32,       // 9.0 gross
    oxygen_kg_per_day: f32,     // 0.90
    hygiene_kg_per_month: f32,  // 0.62
    medical_kg_per_year: f32,   // 0.50
    clothing_kg_per_year: f32,  // 2.40
}
```

### Supply Calculation Algorithm

```
for each resource:
    daily_demand = population × per_person_rate
    daily_production = Σ (system.output for selected systems)
    daily_net_loss = daily_demand - daily_production
    
    // Reserves sized to voyage + safety margin
    emergency_reserve = daily_demand × emergency_days(resource)
    buffer_reserve = daily_net_loss × voyage_years × 365
    
    total_stockpile = emergency_reserve + buffer_reserve
    
    // Apply budget class modifier
    if budget_class == Austerity: total_stockpile *= 0.8
    if budget_class == Abundant:  total_stockpile *= 1.3
```

### Emergency Reserve Days (by resource)

| Resource | Emergency Days | Rationale |
|----------|---------------|-----------|
| Food (stored) | 90 | Tier 3 deep emergency |
| Water | 30 | Primary tank capacity |
| Oxygen | 6 | Life-critical, fast production |
| Power (backup fuel) | 30 | Emergency generator capacity |
| Spare parts | 365 | 1-year fabrication buffer |
| Medical | 180 | 6-month pharmaceutical reserve |

### Stockpile Validation

After calculation, validate that the ship can actually carry its required stockpiles:

```
cargo_mass = Σ all stockpile masses
propellant_mass = voyage_profile.reaction_mass_tons
fuel_mass = voyage_profile.fuel_mass_tons

total_ship_mass = structural_mass(deck_count, ship_type)
               + population_mass(total_population × 80 kg avg)
               + cargo_mass + propellant_mass + fuel_mass

// Validate propulsion can move this mass
thrust_to_weight = propulsion.thrust / (total_ship_mass × g)
if thrust_to_weight < min_threshold:
    WARN: "Ship too heavy for selected propulsion"
    → Reduce population, increase engine count, or change propulsion
```

---

## Data Structures Summary

### New Tables/Structs for ShipConfig

```rust
// Replaces current minimal ShipConfig
struct MissionConfig {
    destination: Destination,
    colony_target_pop: u32,
    mission_priority: MissionPriority,
    tech_level: TechLevel,
    budget_class: BudgetClass,
    seed: u64,
}

struct VoyageProfile {
    propulsion: PropulsionType,
    cruise_speed_c: f64,
    total_duration_years: f64,
    fuel_mass_tons: f64,
    reaction_mass_tons: f64,
}

struct SystemSelection {
    power: PowerVariant,
    life_support: LifeSupportVariant,
    food_production: FoodProductionVariant,
    water_system: WaterSystemVariant,
    harvesting: Vec<HarvestingSystem>,  // optional, 0–4 selected
    defense: Vec<DefenseSystem>,        // optional
}

struct PopulationProfile {
    total_population: u32,
    crew_count: u32,
    passenger_count: u32,
    department_counts: Vec<(Department, u32)>,
}

struct SupplyManifest {
    food_stockpile_kg: f64,
    water_reserve_m3: f64,
    oxygen_reserve_kg: f64,
    fuel_reserve_kg: f64,
    reaction_mass_tons: f64,
    spare_parts_kg: f64,
    raw_materials_kg: f64,
    medical_supplies_kg: f64,
    // ... per resource type
}
```

---

## Implementation Phases

### Phase 1: Mission Config & Voyage Profile
- [ ] Define `MissionConfig` struct with destination enum
- [ ] Define `VoyageProfile` calculation from propulsion × destination
- [ ] Add `PropulsionType` enum with 6 variants and specs
- [ ] Add destination table with distances and habitability scores
- [ ] Update `init_ship()` to accept `MissionConfig` instead of raw params

### Phase 2: System Variant Selection
- [ ] Define variant enums for all 7 system categories
- [ ] Define `SystemRoomRequirement` struct for each variant
- [ ] Implement selection algorithm (tech_level × budget × priority weighting)
- [ ] Each variant declares: crew_needed, power_draw, rooms, consumables, MTBF
- [ ] Store selected systems in new `ShipSystems` table

### Phase 3: Population Sizing
- [ ] Implement colony population back-calculation from arrival target
- [ ] Implement crew sizing from system crew requirements + overhead
- [ ] Department allocation with scaling rules
- [ ] Genetic diversity validation (≥500 unrelated individuals)

### Phase 4: Dynamic Facility Manifest
- [ ] Replace hardcoded `facility_manifest()` with system-driven generation
- [ ] Each system variant provides its room requirements via `SystemRoomRequirement`
- [ ] Scaling rules (PerPopulation, PerSystem, PerDeck, Fixed)
- [ ] Deck count derivation from total area requirements
- [ ] Deck zone assignment algorithm

### Phase 5: Supply Manifest Calculation
- [ ] Per-resource stockpile calculation from population × duration × production
- [ ] Emergency reserve sizing
- [ ] Budget class modifiers
- [ ] Mass budget validation (can the ship carry its cargo?)
- [ ] Propulsion mass ratio validation

### Phase 6: Player Customization UI
- [ ] Pre-game configuration screen with mission parameters
- [ ] "Randomize" button for full procedural generation
- [ ] System selection cards with tradeoff visualization
- [ ] Mass budget indicator (shows remaining capacity)
- [ ] "Launch" button triggers full pipeline

---

## Related Documents
- [[Personal Supplies Manifest]] — Per-person consumption rates (Stage 6 inputs)
- [[Ship Systems Manifest]] — System details, MTBF, spare parts (Stage 3 data)
- [[Logistics & Stockpile Manifest]] — Depletion curves (Stage 6 validation)
- [[In-Transit Resource Harvesting]] — Optional harvesting systems (Stage 3.6)
- [[Modular Generation Architecture]] — Trait-based ship type system (complements Stage 5)
- [[Colony Ship Types]] — Ship geometry variants (future Stage 5 expansion)
- [[Ship Overview]] — Current hardcoded facility manifest (to be replaced by Stage 4)
