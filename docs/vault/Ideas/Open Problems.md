# Open Problems

Active issues and design questions that need resolution.

---

## 🔴 Critical

### Room Size Inflation
The squarified treemap packer expands rooms to fill available zone space. A 14m² cabin target becomes 450m². Only ~76 rooms per deck instead of hundreds.

**Root cause**: Treemap algorithm divides ALL available space among rooms. With too few rooms per zone, each gets a huge slice.

**Possible fixes**:
1. Cap placed room dimensions (e.g., `min(treemap_area, target_area * 1.5)`) and leave remaining space as "empty/utility"
2. Dramatically increase room counts to fill zones (3,500 cabins across 10 decks = 350/deck)
3. Different packing algorithm that respects target sizes (grid-pack small rooms, treemap only large ones)
4. Hybrid: treemap for large rooms (mess halls, reactors), grid-pack for small rooms (cabins, bathrooms)

**Impact**: Layout looks unrealistic, rooms don't feel right. This is the #1 visual problem.

---

## 🟡 Important

### Population Scale
Currently generating only ~100 crew + ~50 passengers. Target is 5,000–10,000. Need to:
- Scale up person generation
- Verify simulation performance at 5K+ agents
- Implement LOD for distant agents (reduced tick frequency)
- Client-side: only render current deck's people

### Interstitial Service Decks
Planned but not implemented. Between every 2–3 habitable decks:
- Same hull footprint, no rooms — crawlspace grid
- Contains HVAC, power conduits, water pipes, data cables
- Access via ladder shafts and maintenance hatches
- Height: 1m (traversable for maintenance, not standing)

Questions:
- Are these walkable in gameplay? Or just data for simulation?
- How to render them? Wireframe overlay? Separate deck view?

### Room Type Visual Differentiation
Client `room_color()` function only handles old room types. Many rooms render default gray. Need:
- Color per room category (habitation=blue, medical=white, engineering=orange)
- Room name labels visible on floor
- Icons for key facilities

---

## 🟢 Future Considerations

### Per-Room Atmosphere
Currently per-deck. Per-room would enable:
- Sealed rooms depleting O2 during fire
- Pressure doors isolating hull breaches
- Ventilation system as gameplay mechanic
- Much more interesting emergencies

### Economy Loop
Basic resource tracking exists but no full production/consumption loop:
- Hydroponics → food production rate
- Water recycling → water availability
- Power generation vs draw
- Scarcity → rationing → morale cascade

### Ship Shape: Rectangular vs Cylindrical
Current: horizontal rectangular hull (like a naval vessel)
Alternative: vertical cylinder with spin gravity (Gemini's "vertical spine")
- Radial layout for circular decks
- More realistic for artificial gravity
- Would require fundamental layout rewrite
- Could be a "ship class" option

### Flip Maneuver
Mid-voyage rotation for deceleration:
- Zero-G transition period
- Gameplay event with consequences
- Unsecured items float, systems strain

### First-Person Camera
Current: top-down. Future: walk through the ship in first person.
- Requires detailed interior geometry
- NPC models and animations
- Much higher visual fidelity needed
- Phase F goal

---

## Resolved ✅

- ~~NORTH/SOUTH convention swap~~ — Fixed in Round 2
- ~~Doors to outside hull~~ — Fixed with hull boundary checks
- ~~Room overlap~~ — Eliminated by grid system
- ~~Teleporting through doors~~ — Fixed with distance-based detection
- ~~Choppy movement~~ — Fixed with 20Hz batching + lerp interpolation
- ~~Camera tilt~~ — Fixed by removing look_at, position-only updates
- ~~Random room-to-room doors~~ — Fixed with type-based filtering
- ~~Door verification: 0 errors~~ — Verified with Python pipeline

## See Also
- [[Procedural Generation]] — layout algorithm details
- [[Simulation Systems]] — what's implemented
- [[Phase Roadmap]] — what's next
