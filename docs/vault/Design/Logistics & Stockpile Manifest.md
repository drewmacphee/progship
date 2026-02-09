# Logistics & Stockpile Manifest

> Total ship inventory, depletion curves, and sustainability analysis for the ISV Prometheus.  
> Designed for variable voyage lengths (100–500+ years) with near-complete closed-loop systems.

## Design Constraints

| Parameter | Value | Justification |
|-----------|-------|---------------|
| Population | 7,500 nominal | 5,000 crew + 2,500 passengers |
| Max population (growth) | 10,000 | Biological growth buffer |
| Recycling target | ≥95% mass closure | Only trace losses acceptable |
| Net mass loss rate | ~3,750 kg/day (~1,369 t/year) | From all combined processes |
| Fabrication capability | Must exceed wear rate | Ship must be self-sustaining |
| Destination range | 4.24–12.36 light-years | Proxima Centauri to Luyten's Star |

### Voyage Duration by Destination and Speed

| Destination | Distance (ly) | At 0.01c | At 0.03c | At 0.05c |
|-------------|--------------|----------|----------|----------|
| Proxima Centauri b | 4.24 | 424 years | 141 years | 85 years |
| Alpha Centauri A/B | 4.37 | 437 years | 146 years | 87 years |
| Barnard's Star | 5.96 | 596 years | 199 years | 119 years |
| Luyten's Star (GJ 273) | 12.36 | 1,236 years | 412 years | 247 years |

> All stockpiles below are baselined for **150-year voyage** (Proxima at 0.03c) with reserves for 200+ years.

---

## Category 1: Renewable Resources (Closed-Loop)

These resources are produced onboard at rates matching or exceeding consumption. Stockpiles serve as buffer for system failures, not as primary supply.

### Food

| Metric | Value |
|--------|-------|
| Daily production capacity | 13,575 kg/day |
| Daily consumption | 13,575 kg/day |
| Buffer stock | 1,725 metric tons (127 days) |
| Production sources | Hydroponics (40%), algae (25%), cultured protein (20%), fungiculture (10%), reserve (5%) |
| Closure rate | ~99% (composting returns nutrients to growing systems) |
| Risk | Single-point: seed stock genetic diversity must be maintained |

**Depletion scenario (production failure):**

| Duration of failure | Impact |
|---------------------|--------|
| 1 week | Buffer absorbs; no rationing needed |
| 1 month | Tier 2 reserves activated; reduced portions |
| 3 months | Tier 3 emergency rations; strict rationing; health impacts begin |
| 6+ months | Critical — population cannot be sustained at 7,500 |

### Water

| Metric | Value |
|--------|-------|
| Daily demand | 140 m³/day |
| Recycling recovery | 96% (136.7 m³/day recovered) |
| Net daily loss | 3.3 m³/day |
| Total reserves | 9,200 m³ (primary + emergency + ice) |
| Reserve duration (no recycling) | 66 days |
| Reserve duration (at 96% recovery) | 2,788 days (7.6 years) |

**Long-term water balance:**

| Voyage Length | Net Water Lost | % of Reserves | Sustainable? |
|--------------|---------------|--------------|-------------|
| 100 years | 120,450 m³ | 1,309% of reserves | ❌ Without makeup |
| 150 years | 180,675 m³ | 1,964% | ❌ Without makeup |

> Water is technically non-renewable over centuries. The 4% loss rate means the ship must either: (a) improve recycling to >99%, (b) harvest ice from encountered bodies, or (c) carry vastly more initial water/ice. The 5,000 m³ hull ice reserve is the primary long-term buffer, supplemented by trace hydrogen capture from the interstellar medium.

### Oxygen

| Metric | Value |
|--------|-------|
| Daily production | 7,500 kg/day (electrolysis) + 500 kg/day (photosynthesis) |
| Daily consumption | 6,750 kg/day |
| Surplus | +1,250 kg/day (116%) |
| Emergency reserve | 45,000 kg (6 days at full population) |
| Closure rate | ~99.5% (water electrolysis → O₂, Sabatier recapture) |

---

## Category 2: Slowly Depleting Resources

These are consumed faster than they can be regenerated but last the full voyage with proper management.

### Reactor Fuel (Deuterium + He-3)

| Metric | Value |
|--------|-------|
| Consumption rate | 0.5 kg/day per reactor (1 active) |
| Annual consumption | 182.5 kg/year |
| Initial stockpile | 180,000 kg (180 metric tons) |
| Duration at 1 reactor | 986 years |
| Duration at 2 reactors | 493 years |
| Replenishment | None (finite — but sufficient for any target) |
| Risk level | 🟢 Very Low |

### Propulsion Reaction Mass (Hydrogen)

| Metric | Value |
|--------|-------|
| Initial stockpile | 50,000 metric tons |
| Consumption profile | High during accel/decel phases, zero during cruise |
| Accel phase | ~20% of stock (10,000 t) |
| Cruise phase | Negligible (attitude control only) |
| Decel phase | ~20% of stock (10,000 t) |
| Reserve after arrival | ~60% (30,000 t) |
| Risk level | 🟢 Low (well-margined) |

### Emergency Generator Fuel (Hydrazine)

| Metric | Value |
|--------|-------|
| Stockpile | 20 metric tons |
| Consumption (emergency use) | 667 kg/day at full 8 MW load |
| Duration at full load | 30 days |
| Expected use over voyage | <5 metric tons (intermittent emergencies) |
| Risk level | 🟢 Low |

---

## Category 3: Consumable Parts & Materials

These are fabricated onboard from raw stock. The constraint is raw material reserves.

### Raw Materials Stockpile

| Material | Initial Stock (metric tons) | Annual Demand (t/year) | Years of Supply | Renewable? |
|----------|---------------------------|----------------------|----------------|-----------|
| Iron/steel stock | 80 | 1.2 | 67 | Recycled from worn parts (~70%) |
| Aluminum stock | 50 | 0.6 | 83 | Recycled (~80%) |
| Titanium stock | 30 | 0.3 | 100 | Recycled (~75%) |
| Copper/wire stock | 25 | 0.2 | 125 | Recycled (~85%) |
| Polymer feedstock | 50 | 1.5 | 33 | Partially synthesized from biomass |
| Electronic components | 15 (by mass) | 0.15 | 100 | Some fabricated onboard |
| Ceramic/glass raw | 20 | 0.2 | 100 | Recycled (~60%) |
| Lubricants & oils | 10 | 0.3 | 33 | Partially bio-synthesized |
| Sealants & adhesives | 8 | 0.2 | 40 | Partially synthesized |
| Fasteners (bolts, screws) | 5 | 0.1 | 50 | Fabricated from metal stock |

**Effective supply with recycling:**

| Material | Effective Annual Net Loss | Effective Years |
|----------|--------------------------|----------------|
| Iron/steel | 0.36 t/year (70% recycled) | 222 |
| Aluminum | 0.12 t/year (80% recycled) | 417 |
| Titanium | 0.075 t/year (75% recycled) | 400 |
| Copper | 0.03 t/year (85% recycled) | 833 |
| Polymer | 0.75 t/year (50% recycled) | 67 |

> **Polymer feedstock is the tightest constraint** for long voyages. Bio-synthesis from cellulose waste extends this significantly, but polymer recycling technology improvements would be a high-priority research area during the voyage.

### ECLSS Consumable Parts

| Category | Annual Demand | Stock (units) | Years of Supply |
|----------|-------------|--------------|----------------|
| PEM membranes (O₂ gen) | 12/year | 600 | 50 |
| CO₂ sieve beds | 21/year | 1,050 | 50 |
| RO membranes (water) | 32/year | 1,600 | 50 |
| HEPA filters | 640/year | 16,000 | 25 |
| UV sterilization bulbs | 96/year | 4,800 | 50 |
| Activated carbon | 384 kg/year | 10,000 kg | 26 |

> After initial stock depletion, ECLSS parts must be fabricated onboard. The fabrication lab can produce all of these from raw materials, but at reduced quality. Research into extending filter and membrane lifespans is ongoing.

### Hydroponics & Food Production

| Category | Annual Demand | Stock | Years |
|----------|-------------|-------|-------|
| Grow substrate (perlite/clay) | 4,000 kg/year | 40,000 kg | 10 (then recycled) |
| Nutrient concentrates | 6,000 L/year | 60,000 L | 10 (then synthesized) |
| LED grow panels | 48/year | 2,400 | 50 |
| Seed stock vault | Self-replenishing | 500 kg | ∞ |
| Bioreactor tubing | 400 m/year | 20,000 m | 50 |

---

## Category 4: Irreplaceable Assets

Items that cannot be fabricated onboard and must last the entire voyage.

| Asset | Quantity | Mitigation |
|-------|---------|-----------|
| Hull structural members | 1 set | Weldable, patchable; no replacement |
| Reactor pressure vessel | 2 (one standby) | Designed for 500-year fatigue life |
| Main engine bell nozzles | 4 | Liner replaceable; shell permanent |
| Cryo seed vault | 1 | Redundant samples; genetic diversity |
| Cultural archives (digital) | Triple-redundant storage | Error correction; periodic migration |
| Navigation star catalogs | Hardened ROM + live updates | Cannot be regenerated if lost |

---

## Depletion Timeline (150-Year Voyage at 0.03c)

| Year | Event | Action Required |
|------|-------|----------------|
| 0 | Launch — all systems nominal | — |
| 10 | First grow substrate cycle exhausted | Switch to recycled substrate |
| 15 | First reactor overhaul | 2-week reduced power on standby reactor |
| 25 | Initial HEPA filter stock depleted | Begin onboard filter fabrication |
| 30 | First emergency generator overhaul | Routine |
| 33 | Polymer feedstock at 50% | Increase bio-synthesis, reduce polymer use |
| 50 | Initial ECLSS membrane stocks at 50% | Begin membrane fabrication program |
| 50 | First LED grow panel stock exhausted | Begin LED fabrication |
| 67 | Raw iron stock depleted without recycling | Must maintain >70% metal recycling |
| 75 | Mid-voyage — all systems must be self-sustaining | No original consumables should be relied upon |
| 100 | Polymer feedstock exhausted (without bio-synth) | Bio-synthesis must be mature |
| 120 | Begin deceleration phase preparations | Engine overhaul, fuel reallocation |
| 135 | Deceleration begins | 20% fuel consumed, high engine stress |
| 148 | Orbital insertion at destination | — |
| 150 | Arrival — colony establishment begins | Remaining stockpiles become colony seed |

---

## Critical Supply Thresholds (Simulation Triggers)

These thresholds trigger gameplay events, rationing, and crew behavior changes.

| Resource | Green (Normal) | Yellow (Caution) | Orange (Rationing) | Red (Critical) |
|----------|---------------|-----------------|-------------------|---------------|
| Food buffer | >30 days | 14–30 days | 7–14 days | <7 days |
| Water reserves | >60 days | 30–60 days | 14–30 days | <14 days |
| O₂ reserve | >4 days | 2–4 days | 1–2 days | <1 day |
| Power (reactor health) | >0.7 | 0.5–0.7 | 0.3–0.5 | <0.3 |
| Spare parts stock | >5 years supply | 2–5 years | 1–2 years | <1 year |
| Raw materials | >50 years supply | 20–50 years | 10–20 years | <10 years |
| Reactor fuel | >200 years | 100–200 years | 50–100 years | <50 years |
| Propellant mass | >150% needed | 100–150% | 80–100% | <80% (cannot decelerate fully) |

### Behavioral Effects of Scarcity

| Level | Crew Effects | Passenger Effects |
|-------|-------------|------------------|
| Green | Normal operations | Normal life |
| Yellow | Increased maintenance priority; efficiency warnings | Mild anxiety; reduced recreation |
| Orange | Mandatory rationing; extended shifts; research priority | Visible stress; hoarding behavior; complaints |
| Red | Emergency protocols; martial authority; triage | Panic; unrest; potential mutiny events |

---

## Mass Budget Summary

| Category | Initial Mass (metric tons) | % of Cargo Mass |
|----------|---------------------------|----------------|
| Reactor fuel (D + He-3) | 180 | 0.3% |
| Propulsion reaction mass | 50,000 | 83.3% |
| Water (all reserves) | 9,200 | 15.3% |
| Food reserves | 1,725 | 2.9% |
| Raw materials (metals) | 185 | 0.3% |
| Polymer + chemical feedstock | 68 | 0.1% |
| ECLSS spares & consumables | 35 | 0.06% |
| Medical supplies | 1 | <0.01% |
| Emergency generator fuel | 20 | 0.03% |
| Electronics & components | 15 | 0.03% |
| Misc (textiles, tools, etc.) | 50 | 0.08% |
| **Total cargo/consumables** | **~61,479** | **100%** |

> Propulsion reaction mass dominates at 83% of total cargo. This is consistent with real spacecraft design — the tyranny of the rocket equation means most mass is propellant. The actual "living supplies" are only ~11,300 metric tons (~17% of cargo).

---

## Related Documents
- [[Personal Supplies Manifest]] — Per-person consumption breakdown
- [[Ship Systems Manifest]] — System-by-system maintenance and spares
- [[Ship Overview]] — Physical layout and facility manifest
- [[Simulation Systems]] — How resource levels affect simulation
- [[Open Problems]] — Economy loop and scarcity mechanics
