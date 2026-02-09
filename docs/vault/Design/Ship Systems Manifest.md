# Ship Systems Manifest

> Detailed maintenance, spare parts, failure modes, and consumable requirements for every major ship system.  
> Based on NASA ECLSS data, industrial MTBF standards, and generation-ship extrapolations.

## Design Philosophy

Generation ships cannot be resupplied. Every system must be:
1. **Repairable** — no "replace and discard" culture; every component rebuildable
2. **Redundant** — critical systems have N+1 or N+2 backup capacity
3. **Predictive** — condition monitoring prevents surprise failures
4. **Fabricable** — onboard workshops can manufacture most replacement parts

### Maintenance Tiers

| Tier | Scope | Performed By | Frequency |
|------|-------|-------------|-----------|
| T1: Inspection | Visual/sensor checks, cleaning | Any trained crew | Daily–Weekly |
| T2: Preventive | Filter changes, lubrication, calibration | Maintenance crew | Weekly–Monthly |
| T3: Corrective | Component replacement, minor repair | Engineering specialists | As needed |
| T4: Overhaul | Full system teardown and rebuild | Engineering team + workshop | 5–25 year cycles |
| T5: Fabrication | Manufacture replacement parts from raw stock | Machine shop crew | As needed |

---

## 1. Power Systems

### 1.1 Reactor Core (×2, one standby)

| Parameter | Value |
|-----------|-------|
| Output | 50 MW thermal → 20 MW electrical each |
| Fuel | Deuterium-Helium-3 fusion pellets |
| Fuel consumption | 0.5 kg/day per reactor |
| Fuel reserve | 180 metric tons (enough for 500+ years at 1 reactor) |
| MTBF (major failure) | 87,600 hrs (10 years) |
| MTBF (minor fault) | 8,760 hrs (1 year) |
| Planned overhaul cycle | Every 15 years |

#### Consumables & Spare Parts

| Item | Quantity Stocked | Lifespan | Replacement Rate |
|------|-----------------|----------|-----------------|
| Magnetic confinement coils | 8 sets | 12 years | 0.67/year |
| Plasma facing tiles | 200 tiles | 5 years | 40/year |
| Superconductor coolant (He-4) | 2,000 L | Recycled 99.5% | 10 L/year loss |
| Control rod assemblies | 12 | 20 years | 0.6/year |
| Sensor arrays (temp/flux) | 40 | 3 years | 13/year |
| Power coupling insulators | 60 | 8 years | 7.5/year |

#### Failure Modes

| Mode | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Confinement coil degradation | High (expected) | Output reduction 20–50% | Swap to standby coil set |
| Coolant leak | Medium | Emergency shutdown required | Automated isolation valves |
| Control system fault | Medium | Power oscillation | Triple-redundant controllers |
| Fuel pellet contamination | Low | Reduced efficiency | Pellet QC at fabrication |
| Catastrophic containment breach | Very low | Reactor loss | Blast containment + standby reactor |

### 1.2 Emergency Generators (×4)

| Parameter | Value |
|-----------|-------|
| Output | 2 MW each (8 MW total emergency) |
| Fuel | Chemical (hydrazine reserve) |
| Reserve fuel | 20 metric tons (30 days at full load) |
| MTBF | 43,800 hrs (5 years) |
| Maintenance cycle | Annual inspection, 5-year overhaul |
| Startup time | 30 seconds (automated) |

### 1.3 Power Distribution

| Component | Count | MTBF | Spares Stocked |
|-----------|-------|------|----------------|
| Main bus bars | 4 | 25 years | 2 |
| Distribution transformers | 24 | 15 years | 6 |
| Circuit breakers (high voltage) | 120 | 10 years | 24 |
| Cable runs (km total) | 85 km | 50 years | 5 km reserve |
| Junction boxes | 400 | 20 years | 40 |

---

## 2. Life Support (ECLSS)

### 2.1 Oxygen Generation (×6 units)

| Parameter | Value |
|-----------|-------|
| Technology | Water electrolysis (PEM) |
| Output per unit | 1,250 kg O₂/day |
| Total capacity | 7,500 kg O₂/day (100% demand) |
| Power draw | 20 kW per unit |
| MTBF | 17,520 hrs (2 years) |
| Overhaul cycle | Every 3 years |

#### Consumables

| Item | Quantity Stocked | Lifespan | Rate |
|------|-----------------|----------|------|
| PEM membrane stacks | 24 (4 per unit) | 2 years | 12/year |
| Electrode assemblies | 36 | 3 years | 12/year |
| Deionization filters | 72 | 6 months | 144/year |
| Pressure regulators | 18 | 5 years | 3.6/year |
| Flow sensors | 36 | 3 years | 12/year |

### 2.2 CO₂ Scrubbing (×8 units)

| Parameter | Value |
|-----------|-------|
| Technology | Molecular sieve + amine swing |
| Capacity per unit | 1,062 kg CO₂/day |
| Total capacity | 8,500 kg CO₂/day (113% of peak) |
| Power draw | 12 kW per unit |
| MTBF | 8,760 hrs (1 year) |
| Overhaul cycle | Every 2 years |

#### Consumables

| Item | Quantity Stocked | Lifespan | Rate |
|------|-----------------|----------|------|
| Molecular sieve beds | 32 | 1.5 years | 21/year |
| Amine solution (L) | 4,000 | 1 year (refreshed) | 4,000 L/year |
| Activated carbon filters | 96 | 3 months | 384/year |
| Blower assemblies | 16 | 4 years | 4/year |
| Humidity pre-filters | 48 | 2 months | 288/year |

### 2.3 Air Circulation & HVAC

| Component | Count | MTBF | Spares |
|-----------|-------|------|--------|
| Main circulation fans | 32 | 5 years | 8 |
| Duct damper actuators | 200 | 8 years | 30 |
| HEPA filters | 160 | 6 months | 640 (2-year stock) |
| Temperature sensors | 400 | 5 years | 80 |
| Humidity controllers | 80 | 4 years | 20 |
| Heat exchangers | 16 | 10 years | 4 |

### 2.4 Pressure Management

| Component | Count | MTBF | Spares |
|-----------|-------|------|--------|
| Pressure sensors | 200 | 5 years | 40 |
| Emergency seal actuators | 120 | 15 years | 12 |
| Bulkhead door seals (gaskets) | 120 | 5 years | 48 |
| Pressure relief valves | 40 | 10 years | 8 |

---

## 3. Water Systems

### 3.1 Water Recovery & Purification (×4 units)

| Parameter | Value |
|-----------|-------|
| Capacity per unit | 35 m³/day processed |
| Total capacity | 140 m³/day (100% demand) |
| Recovery rate | 96% overall |
| Power draw | 15 kW per unit |
| MTBF | 8,760 hrs (1 year) |

#### Consumables

| Item | Quantity Stocked | Lifespan | Rate |
|------|-----------------|----------|------|
| Reverse osmosis membranes | 32 | 1 year | 32/year |
| UV sterilization bulbs | 48 | 6 months | 96/year |
| Ion exchange resin (kg) | 800 | 1 year (regenerated) | 200 kg/year new |
| Multifiltration cartridges | 64 | 3 months | 256/year |
| pH adjustment chemicals (kg) | 200 | 1 year | 200 kg/year |
| Pump impellers | 16 | 3 years | 5/year |
| Pressure seals/gaskets | 80 | 2 years | 40/year |

### 3.2 Wastewater Processing

| Component | Count | MTBF | Spares |
|-----------|-------|------|--------|
| Urine processors | 8 | 1.5 years | 4 |
| Bioreactor chambers | 4 | 5 years | 1 |
| Sludge centrifuges | 4 | 3 years | 2 |
| Chemical dosing pumps | 16 | 2 years | 8 |

---

## 4. Food Production Systems

### 4.1 Hydroponics Bays (×12)

| Parameter | Value |
|-----------|-------|
| Total growing area | 6,000 m² |
| Output | 5,500 kg food/day |
| Power draw | 8 kW per bay (lighting) |
| Water use | 45 m³/day (95% recirculated) |
| MTBF (bay-level) | 4,380 hrs (6 months — minor issues) |

#### Consumables

| Item | Quantity Stocked | Lifespan | Rate |
|------|-----------------|----------|------|
| Growth substrate (perlite/clay) | 12,000 kg | 3 years | 4,000 kg/year |
| Nutrient solution concentrates (L) | 6,000 | 1 year | 6,000 L/year |
| LED grow panels | 240 | 5 years | 48/year |
| Irrigation pumps | 24 | 3 years | 8/year |
| Seed stock (genetic diversity vault) | 500 kg | Renewable | Self-replenishing |
| Pollination drones | 48 | 2 years | 24/year |
| pH/EC sensors | 72 | 1 year | 72/year |

### 4.2 Algae Bioreactors (×8)

| Parameter | Value |
|-----------|-------|
| Output | 3,400 kg biomass/day |
| CO₂ input | Feeds from scrubber output |
| Harvest cycle | Continuous (daily skim) |
| MTBF | 17,520 hrs (2 years) |

#### Consumables

| Item | Quantity Stocked | Lifespan | Rate |
|------|-----------------|----------|------|
| Bioreactor tubing | 400 m | 1 year | 400 m/year |
| Culture medium minerals (kg) | 2,000 | 1 year | 2,000 kg/year |
| Sterilization agents (L) | 500 | 1 year | 500 L/year |
| Optical density sensors | 16 | 2 years | 8/year |

### 4.3 Cultured Protein Vats (×4)

| Parameter | Value |
|-----------|-------|
| Output | 2,700 kg protein/day |
| Growth medium | Amino acid + sugar solution |
| MTBF | 8,760 hrs (1 year) |

#### Consumables

| Item | Quantity Stocked | Lifespan | Rate |
|------|-----------------|----------|------|
| Cell culture growth medium (L) | 8,000 | 6 months | 16,000 L/year |
| Bioreactor bags/vessels | 48 | 3 months | 192/year |
| Starter culture stocks (cryo) | 200 vials | Renewable | ~20/year used |
| Sterile filtration units | 96 | 1 month | 1,152/year |

---

## 5. Propulsion

### 5.1 Main Engines (×4, 2 active + 2 standby)

| Parameter | Value |
|-----------|-------|
| Type | Fusion torch / Ion drive hybrid |
| Thrust (cruise) | Continuous low-thrust acceleration |
| Fuel | Deuterium (shared with reactor supply) |
| Propellant | Reaction mass (hydrogen ice) |
| Propellant reserve | 50,000 metric tons |
| MTBF (major) | 43,800 hrs (5 years) |
| Overhaul cycle | Every 10 years |

#### Consumables

| Item | Quantity Stocked | Lifespan | Rate |
|------|-----------------|----------|------|
| Ion grid assemblies | 16 | 5 years | 3.2/year |
| Thruster nozzle liners | 8 | 10 years | 0.8/year |
| Magnetic nozzle coils | 8 | 12 years | 0.67/year |
| Gimbal actuators | 16 | 8 years | 2/year |
| Propellant feed valves | 24 | 5 years | 4.8/year |

### 5.2 Attitude Control (RCS)

| Component | Count | MTBF | Spares |
|-----------|-------|------|--------|
| RCS thruster pods | 24 | 10 years | 6 |
| Propellant tanks (RCS) | 8 | 30 years | 1 |
| Gyroscopes (CMG units) | 6 | 8 years | 3 |
| Star trackers | 4 | 10 years | 2 |

---

## 6. Navigation & Sensors

| Component | Count | MTBF | Spares |
|-----------|-------|------|--------|
| Primary nav computer | 3 (triple redundant) | 10 years | 1 |
| Long-range telescope | 2 | 15 years | Lens/mirror spares |
| LIDAR/radar arrays | 4 | 8 years | 2 |
| Communication laser array | 2 | 10 years | 1 |
| Inertial measurement units | 6 | 5 years | 3 |

---

## 7. Medical Systems

| Component | Count | MTBF | Spares |
|-----------|-------|------|--------|
| Surgical suites | 4 | 15 years (overhaul) | Instrument sets: 8 |
| Diagnostic imagers (MRI/CT) | 2 | 10 years | Coil/tube replacements: 4 |
| Pharmaceutical synthesizers | 2 | 5 years | Cartridge sets: 20 |
| Emergency defibrillators | 20 | 8 years | 5 |
| Ventilators | 12 | 5 years | 4 |
| Biosafety cabinets | 4 | 10 years | Filter sets: 40 |

---

## 8. Fabrication & Workshop

> The ship's ability to manufacture replacement parts is a critical system in itself.

| Facility | Count | Capability |
|----------|-------|-----------|
| CNC machine shop | 2 | Metal parts, precision to 0.01mm |
| 3D metal printers | 4 | Titanium, steel, aluminum sintering |
| 3D polymer printers | 6 | Structural plastic, seals, housings |
| Electronics fabrication lab | 1 | PCB printing, component soldering |
| Glass/ceramic kiln | 1 | Reactor tiles, optical components |
| Welding stations | 8 | Hull repair, structural work |
| Raw metal stock | 200 metric tons | Iron, aluminum, titanium, copper |
| Polymer feedstock | 50 metric tons | ABS, PEEK, PTFE, silicone |
| Electronic components | 100,000+ units | ICs, resistors, capacitors, connectors |

### Fabrication Capacity vs Demand

| Category | Annual Demand (kg) | Fabrication Capacity (kg/year) | Surplus? |
|----------|-------------------|-------------------------------|----------|
| Metal parts | 2,500 | 8,000 | ✅ 3.2× |
| Polymer parts | 1,200 | 4,000 | ✅ 3.3× |
| Electronics | 150 | 300 | ✅ 2× |
| Ceramic/glass | 200 | 500 | ✅ 2.5× |

---

## Maintenance Crew Requirements

| Department | Crew Count | Coverage |
|-----------|-----------|----------|
| Power engineering | 40 | Reactor, generators, distribution |
| Life support technicians | 35 | ECLSS, atmosphere, pressure |
| Water systems | 20 | Purification, distribution, waste |
| Propulsion engineering | 25 | Engines, RCS, fuel systems |
| General maintenance | 45 | HVAC, structural, plumbing |
| Electronics/computer | 20 | Sensors, control systems, networking |
| Fabrication/machine shop | 30 | Part manufacturing, 3D printing |
| **Total maintenance crew** | **215** | ~4.3% of population |

---

## Related Documents
- [[Personal Supplies Manifest]] — Per-person consumption rates
- [[Logistics & Stockpile Manifest]] — Total ship inventory and depletion curves
- [[Simulation Systems]] — How maintenance integrates with simulation
- [[Ship Overview]] — Facility layout and room assignments
