# In-Transit Resource Harvesting

> Research-backed analysis of systems that can supplement ship supplies during interstellar voyage.  
> Covers: ISM collection, energy harvesting, Oort Cloud mining, and onboard synthesis.

## Overview

A generation ship cannot rely solely on supplies loaded at departure. Over a 100–500 year voyage, even 95% recycling efficiency means significant cumulative losses. Four categories of in-transit supplementation exist, ranging from well-understood to highly speculative:

| Category | Feasibility | Yield | Impact on Ship Design |
|----------|------------|-------|----------------------|
| 1. ISM Hydrogen Collection | Moderate (proven physics) | Low but continuous | Large magnetic scoop infrastructure |
| 2. Energy Harvesting | Moderate | Supplemental power | Collector arrays, magnetic sails |
| 3. Oort Cloud / Rogue Body Mining | Speculative (opportunistic) | Potentially high per event | Mining drones, trajectory planning |
| 4. Onboard Transmutation & Synthesis | Near-term feasible | Converts energy → materials | Reactor time, feedstock management |

---

## 1. Interstellar Medium (ISM) Hydrogen Collection

### What's Out There

The interstellar medium is extremely diffuse but contains useful materials:

| Component | Density | % of ISM | Harvestable? |
|-----------|---------|----------|-------------|
| Hydrogen (H, H₂, H⁺) | ~1 atom/cm³ | 75% by mass | ✅ Primary target |
| Helium (He, He⁺) | ~0.1 atom/cm³ | 24% by mass | ✅ Fusion fuel (He-3) |
| Heavier elements (C, N, O) | ~0.001 atom/cm³ | ~1% by mass | ⚠️ Trace only |
| Dust grains (silicate, carbon, ice) | ~10⁻¹² g/cm³ | ~1% by mass | ⚠️ Microscopic, hard to collect |

> For reference: Earth's atmosphere is ~10¹⁹ atoms/cm³. The ISM is roughly **10 quintillion times** less dense.

### Bussard Ramscoop (Modified)

The original Bussard Ramjet (1960) proposed using a magnetic scoop to funnel ISM hydrogen into a fusion reactor for propulsion. Pure ramjet propulsion is likely infeasible due to drag exceeding thrust at realistic speeds. However, a **modified ramscoop** optimized for material collection (not propulsion) is more practical:

| Parameter | Value | Notes |
|-----------|-------|-------|
| Scoop diameter | 50–100 km (magnetic field) | Superconducting coil array |
| Collection rate at 0.03c | ~0.5 kg H/day | Depends heavily on local ISM density |
| Collection rate at 0.05c | ~2.0 kg H/day | Higher speed = more throughput |
| Power requirement | 500 kW continuous | To maintain magnetic field |
| Drag penalty | ~1% thrust reduction | Acceptable for collection-only mode |

### What Collected Hydrogen Provides

| Use | Rate | Significance |
|-----|------|-------------|
| Water synthesis (H₂ + O₂ → H₂O) | 0.5–2 kg H → 4.5–18 L water/day | Offsets 3–14% of daily water loss |
| Fusion fuel supplement | 0.5–2 kg D/day (if deuterium extracted) | Negligible vs 0.5 kg/day reactor consumption (D is 0.015% of H) |
| Propellant mass augmentation | Direct mass addition | Extends reaction mass reserves |

### ISV Prometheus Ramscoop Design

| Component | Specification |
|-----------|--------------|
| Forward magnetic scoop array | 6 superconducting coils, 10 km effective radius |
| Ionization laser grid | Pre-ionizes neutral H ahead of scoop |
| Collection chamber | Compresses and stores collected gas |
| Isotope separator | Extracts deuterium, He-3 from bulk hydrogen |
| Integration | Forward hull section, deployable during cruise phase |

**Gameplay implications**: The ramscoop is a major ship system that requires maintenance, can be damaged, and its output varies based on local ISM density (which changes along the route). Passing through denser regions (remnant molecular clouds) could be a positive event; passing through voids reduces collection.

---

## 2. Energy Harvesting

### Sources Available in Deep Interstellar Space

| Source | Energy Density | Harvestable? | Notes |
|--------|---------------|-------------|-------|
| Cosmic microwave background | 0.25 eV/cm³ | ❌ Too diffuse | Cannot extract useful work |
| Galactic cosmic rays | ~1 eV/cm³ | ⚠️ Marginal | High energy per particle, very low flux |
| Interstellar magnetic field | ~0.1 nT | ⚠️ Via motion | Only useful at high velocity |
| Kinetic energy of ISM impacts | Proportional to v² | ✅ At cruise speed | Ship's own velocity provides energy |
| Residual stellar radiation | ~10⁻⁶ W/m² | ❌ Negligible | Too far from any star |

### Magnetic Sail Power Generation

A magnetic sail (magsail) deployed during cruise can harvest energy from ISM plasma interactions:

| Parameter | Value |
|-----------|-------|
| Sail diameter | 50 km magnetic field |
| Power generated at 0.03c | ~50–200 kW |
| Power generated at 0.05c | ~200–800 kW |
| Primary use | Supplemental power, reduces reactor load |
| Secondary use | Deceleration braking at destination |

> At 0.03c, the magsail could provide **~5% of ship's electrical needs** (vs 20 MW reactor output). Not transformative, but extends reactor fuel life and provides redundancy.

### Kinetic Impact Energy Recovery

At 0.03c (9,000 km/s), every gram of ISM material that hits the ship carries enormous kinetic energy:

| Metric | Value |
|--------|-------|
| KE per gram at 0.03c | 40.5 MJ (equivalent to ~10 kg of TNT) |
| ISM mass intercepted (100 m² cross-section) | ~0.003 g/day |
| Power recoverable | ~1.4 W |

> Negligible for power generation, but this energy is real and must be managed as **radiation shielding** — it's a hazard, not a resource at these scales.

---

## 3. Oort Cloud & Rogue Body Mining

### Opportunity Window

The Oort Cloud extends from ~2,000 AU to ~100,000 AU from the Sun. A ship at 0.03c crosses this zone in approximately 1.5–7 years of travel time. Additionally, rogue bodies (ejected asteroids, comets, planetesimals) exist throughout interstellar space.

| Body Type | Estimated Density | Composition | Harvestable Mass |
|-----------|------------------|-------------|-----------------|
| Oort Cloud comets | 1 per ~10³ AU³ | Water ice, CO₂, NH₃, silicates, organics | 10⁹–10¹² kg per comet |
| Rogue asteroids | Unknown (very sparse) | Metals, silicates | Variable |
| Interstellar dust clouds | Rare dense patches | H₂, dust, ices | Diffuse but large |

### Mining Drone Concept

The ship carries a complement of autonomous mining drones that can be deployed to intercept and process bodies detected ahead:

| Parameter | Value |
|-----------|-------|
| Drone complement | 6 mining drones, 2 survey drones |
| Detection range | Long-range telescope/LIDAR, ~500 AU ahead |
| Deployment range | ±50 AU from ship trajectory |
| Delta-v budget per drone | 5 km/s (ion drive) |
| Mining time per target | 30–180 days |
| Return payload capacity | 50–500 metric tons per mission |
| Drone MTBF | 20 years |

### What a Comet Provides

A typical 1 km Oort Cloud comet contains approximately:

| Material | Mass (metric tons) | Ship Use |
|----------|-------------------|----------|
| Water ice | 250,000,000 | Water reserves (centuries of supply) |
| CO₂ ice | 50,000,000 | CO₂ for plant growth, atmosphere supplement |
| Ammonia ice | 25,000,000 | Nitrogen source for agriculture |
| Silicate dust | 100,000,000 | Raw material for ceramics, glass |
| Organic compounds | 10,000,000 | Carbon feedstock for polymer synthesis |
| Metals (Fe, Ni, trace) | 5,000,000 | Metal stock replenishment |

> Even capturing **0.001%** of a single comet's mass would yield 5,000 metric tons — enough to fully replenish the ship's raw material and water reserves.

### Encounter Probability

| Scenario | Probability over 150-year voyage |
|----------|--------------------------------|
| Pass within 100 AU of Oort Cloud body | ~15–30% (multiple possible) |
| Pass within 10 AU (drone-reachable) | ~2–5% |
| Encounter rogue body in interstellar space | <1% per decade |
| Encounter dense ISM patch | ~5–10% (varies by route) |

**Gameplay implications**: Comet encounters are rare, high-value events. Detection triggers a major decision: divert resources to mining (crew, power, drones) vs. stay on course. Successful mining could reset resource anxiety for decades. Failed missions lose drones and crew time.

---

## 4. Onboard Transmutation & Synthesis

### What the Ship Can Convert

Using reactor energy and collected materials, several synthesis pathways exist:

| Process | Input | Output | Energy Cost | Status |
|---------|-------|--------|-------------|--------|
| Water electrolysis | H₂O + energy | H₂ + O₂ | 286 kJ/mol | ✅ Current system |
| Sabatier reaction | CO₂ + 4H₂ | CH₄ + 2H₂O | Exothermic | ✅ Current system |
| Fischer-Tropsch synthesis | CO + H₂ | Hydrocarbons | Moderate | ✅ For polymer feedstock |
| Haber-Bosch (modified) | N₂ + 3H₂ | 2NH₃ | 500 kJ/mol | ✅ Fertilizer production |
| Bioplastic synthesis | Algae biomass + energy | PHA/PLA polymers | Moderate | ✅ Extends polymer reserves |
| Metal recycling/refining | Scrap + energy | Pure metals | Variable | ✅ Current fabrication system |
| Isotope separation | Bulk H | Deuterium (D) | High | ⚠️ Low yield (0.015% of H is D) |

### Synthesis Impact on Depletion Curves

| Resource | Without Synthesis | With Synthesis | Improvement |
|----------|------------------|---------------|-------------|
| Polymer feedstock | 67 years effective | 120+ years | +80% (bio-synthesis from cellulose) |
| Water (net loss offset) | 96% recycling | 97.5% (Sabatier water recovery) | Extends reserves 60% |
| Fertilizer/nutrients | 10 years imported stock | Indefinite (Haber process) | ∞ |
| Metal stock | 222 years (with recycling) | 300+ years (with ISM collection) | +35% |

---

## 5. Combined Supplementation Impact

### Baseline vs. Supplemented Voyage (150 years)

| Resource | Baseline Depletion | With All Supplements | Net Effect |
|----------|-------------------|---------------------|-----------|
| Water | -180,675 m³ cumulative loss | -90,000 m³ (ramscoop + Sabatier) | 50% reduction in loss |
| Polymer feedstock | Exhausted at year 67 | Lasts 120+ years (bio-synth) | Viable for full voyage |
| Metal stock | Lasts 222 years | 300+ years (recycling + ISM) | No concern |
| Reactor fuel | Lasts 986 years | 1,000+ years (trace ISM D) | Negligible improvement |
| Food production inputs | Switch to synthesized at year 10 | Same, but higher quality | Comet nitrogen helps |
| ECLSS consumables | Fabricated after year 25–50 | Same | No change |

### Supplementation System Crew Requirements

| System | Crew Needed | Department |
|--------|------------|-----------|
| Ramscoop operations & maintenance | 8 | Engineering |
| Magsail management | 4 | Engineering |
| Mining drone operations | 12 (when active) | Mining/EVA |
| Synthesis chemistry lab | 6 | Science |
| Isotope separation | 4 | Science |
| **Total** | **34** (+ 12 surge for mining)| |

---

## Simulation Integration

### New Ship Systems to Model

| System | Type | Subsystems |
|--------|------|-----------|
| Ramscoop | ShipSystem | Magnetic coils, ionization grid, compressor, separator |
| Magsail | ShipSystem | Superconducting loops, power converter, deployment mechanism |
| Mining Operations | ShipSystem | Drone bay, survey telescope, processing plant |
| Synthesis Lab | ShipSystem | Fischer-Tropsch reactor, Haber process, bio-synth vats |

### Event Types

| Event | Trigger | Player Impact |
|-------|---------|--------------|
| Dense ISM patch detected | Random (route-dependent) | Ramscoop output doubles for weeks |
| ISM void entered | Random | Ramscoop output drops to near zero |
| Oort Cloud body detected | Distance-based (2,000–100,000 AU from Sol) | Major decision: deploy mining drones? |
| Rogue body encounter | Very rare random | High-risk, high-reward opportunity |
| Ramscoop coil failure | MTBF-based | Must repair or lose collection |
| Mining drone loss | Mission risk roll | Crew morale impact, resource loss |
| Synthesis breakthrough | Research event | Improves a conversion efficiency permanently |

### Resource Flow Diagram

```
ISM Hydrogen ──► Ramscoop ──► H₂ Storage ──┬──► Water Synthesis (H₂ + O₂ → H₂O)
                                            ├──► Deuterium Extraction → Reactor Fuel
                                            ├──► Propellant Mass Reserve
                                            └──► Haber Process (+ N₂ → NH₃ → Fertilizer)

ISM Plasma ────► Magsail ───► Electrical Power ──► Ship Grid (supplemental)

Oort Cloud ────► Mining Drones ──► Raw Ice/Rock ──┬──► Water (melt + purify)
                                                   ├──► CO₂ → Plant Growth
                                                   ├──► NH₃ → Nitrogen → Fertilizer
                                                   ├──► Silicates → Ceramics/Glass
                                                   ├──► Organics → Polymer Feedstock
                                                   └──► Metals → Fabrication Stock

Biomass Waste ─► Synthesis Lab ──┬──► Bioplastic (PHA/PLA)
                                 ├──► Fischer-Tropsch Hydrocarbons
                                 └──► Compost → Agriculture Nutrients
```

---

## References

- Bussard, R. W. (1960). "Galactic Matter and Interstellar Flight." *Astronautica Acta* 6:179–194.
- NASA ECLSS documentation — Sabatier reactor and water recovery systems
- Zubrin, R. (1999). "Magnetic Sails and Interstellar Travel." — Magsail propulsion and braking
- Project Hyperion — British Interplanetary Society generation ship studies
- Stanford/NASA SESS — Energy harvesting from solar wind and galactic cosmic rays

---

## Related Documents
- [[Logistics & Stockpile Manifest]] — Depletion curves (baseline without harvesting)
- [[Ship Systems Manifest]] — Existing ship systems; new systems above extend this
- [[Ship Overview]] — Where ramscoop, drone bay, and synthesis lab are located
- [[Open Problems]] — Economy loop integration
