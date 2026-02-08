# Colony Ship Types

Research on real engineering concepts and fictional archetypes for generation/colony ships. Each has distinct geometry that affects interior layout generation.

---

## 1. Elongated Cylinder (Current: ISV Prometheus)

**Shape**: Long horizontal cylinder/hull, like a naval vessel or submarine.
**Gravity**: Thrust gravity (acceleration = floor) or no spin.
**Cross-section**: Rectangular or oval decks stacked vertically.

### Examples
- Barotrauma submarines (rectangular rooms in elongated hull)
- Alien's USCSS Nostromo (industrial freighter)
- The Expanse's Nauvoo/Behemoth (converted to generation ship)

### Interior Layout
- **Decks**: Horizontal slabs stacked along vertical axis
- **Corridors**: Fore-aft spines with cross-passages
- **Rooms**: Rectangular, packed via grid/treemap
- **Vertical transport**: Elevators, ladders through shaft columns
- **Hull taper**: Narrower at bow/stern, wider amidships

### Generation Approach
Grid-stamp on rectangular deck. This is what we have now.
- ✅ Simple, well-understood
- ✅ Rectangular rooms pack efficiently
- ❌ No artificial gravity without constant thrust
- ❌ Hull taper wastes space at extremes

---

## 2. O'Neill Cylinder (Island Three)

**Shape**: Two counter-rotating cylinders, each ~32km long × 8km diameter.
**Gravity**: Centrifugal (spin). Walk on the INSIDE of the cylinder wall.
**Cross-section**: Circular. "Ground" is the inner surface.

### Examples
- Rendezvous with Rama (50km × 20km alien cylinder)
- Interstellar's Cooper Station (smaller O'Neill at film's end)
- Gundam's space colonies
- Project Hyperion's "Chrysalis" (58km long!)

### Interior Layout
- **Decks**: Concentric rings inside the cylinder wall (like layers of an onion)
- **"Ground" level**: Inner surface — parks, farms, buildings
- **Sub-levels**: Rooms built INTO the cylinder wall (like apartments in a building's walls)
- **Central axis**: Zero-G zone for transport, industry, docking
- **Alternating strips**: Land panels alternate with window panels for sunlight

### Generation Approach
**Radial slice + concentric rings:**
1. Define cylinder cross-section as a circle
2. Central shaft (zero-G hub, 50-100m radius)
3. Ring corridors at fixed radii
4. Radial walls divide annulus into sectors ("pie slices")
5. BSP subdivision within sectors for rooms
6. Rooms are trapezoidal (wider at outer wall, narrower at inner)

Key difference: rooms are NOT rectangular — they're wedge-shaped. Door positions are along arcs, not straight lines.

---

## 3. Stanford Torus

**Shape**: Donut/ring, ~1.8km diameter, tube ~130m across.
**Gravity**: Centrifugal (spin on the ring's axis).
**Cross-section**: Circular tube. Walk on the outer edge of the tube interior.

### Examples
- Elysium (2013 film)
- 2001: A Space Odyssey (Discovery One centrifuge section)
- Halo (game series, massively scaled up)

### Interior Layout
- **Decks**: Nested inside the torus tube, curved floors following the tube cross-section
- **"Ground"**: Outer edge of tube (highest gravity)
- **Upper levels**: Toward tube center (lower gravity)
- **Spokes**: Connecting tubes from hub to ring (transport, utilities)
- **Hub**: Central zero-G station for docking, industry

### Generation Approach
**Segment-based ring layout:**
1. Divide torus into N segments (like slices of a bagel)
2. Each segment is a "deck section" with rectangular-ish rooms
3. Segments connected by ring corridors
4. Spokes connect to central hub
5. Within each segment: standard rectangular packing works (small curvature)

For gameplay purposes, each segment can be treated as a slightly curved rectangular zone — simplifying to our existing grid system with minor adjustments.

---

## 4. Bernal Sphere (Island One)

**Shape**: Sphere, ~500m–2km diameter, rotating.
**Gravity**: Centrifugal at equator, diminishing toward poles.
**Cross-section**: Circular, but gravity varies by latitude.

### Examples
- O'Neill's "Island One" concept
- Various hard-SF novels

### Interior Layout
- **Equatorial band**: Habitation zone (full gravity)
- **Mid-latitudes**: Reduced gravity zones (agriculture, recreation)
- **Polar regions**: Near zero-G (docking, industry, transport)
- **Decks**: Concentric shells within the sphere wall

### Generation Approach
**Latitude-banded zones:**
1. Equatorial belt: standard rectangular rooms (small curvature)
2. Mid-latitude: larger open spaces (farms, parks)
3. Poles: special-purpose (docking, zero-G facilities)
4. Within each band: segment into arcs, pack rooms per arc

---

## 5. Modular / Backbone Ship

**Shape**: Central structural spine with modules attached along its length.
**Gravity**: Spin section (rotating ring/drum) attached to non-rotating spine.

### Examples
- ISS (modular, no spin)
- The Expanse's various ships
- Most "realistic" near-future designs

### Interior Layout
- **Spine**: Non-rotating corridor/utility trunk
- **Modules**: Self-contained pods bolted to spine (habitat, lab, cargo)
- **Drum section**: Rotating cylinder segment for gravity (if any)
- **Each module**: Small, independent layout (like a studio apartment)

### Generation Approach
**Module template + spine assembly:**
1. Define module templates (hab pod, lab pod, cargo pod, etc.)
2. Generate spine with attachment points
3. Assign modules to attachment points based on ship role
4. Within each module: small-scale room packing (4-8 rooms max)
5. Connecting airlocks between modules and spine

Simplest to generate — each module is an independent mini-layout.

---

## 6. Hybrid: Rotating Drum + Elongated Hull

**Shape**: Long hull with one or more rotating drum sections for gravity.
**Most realistic** for actual interstellar travel.

### Examples
- Project Hyperion designs
- The Expanse's Nauvoo (spin drum)
- Pandorum (film)

### Interior Layout
- **Non-rotating sections**: Engineering, propulsion, cargo (zero-G or thrust gravity)
- **Rotating drum(s)**: Habitation, farms (centrifugal gravity)
- **Transition zone**: Bearing assembly connecting rotating/non-rotating
- **Drum interior**: O'Neill-style radial layout (inner surface = ground)
- **Hull sections**: Standard rectangular deck layout

### Generation Approach
**Multi-section composite:**
1. Define ship as sequence of sections along main axis
2. Each section has a type: `Rectangular`, `Drum`, `Spine`, `Shield`
3. Rectangular sections: existing grid-stamp algorithm
4. Drum sections: radial slice algorithm
5. Spine sections: module template assembly
6. Connect sections via transition zones (airlocks, bearing corridors)

This is the most flexible and realistic option.

---

## Comparison Matrix

| Type | Gravity | Room Shape | Layout Complexity | Realism | Fun Factor |
|------|---------|------------|-------------------|---------|------------|
| Elongated Cylinder | Thrust | Rectangular | Low | Medium | Medium |
| O'Neill Cylinder | Spin | Trapezoidal/wedge | High | High | High |
| Stanford Torus | Spin | Rectangular-ish | Medium | High | High |
| Bernal Sphere | Spin (varies) | Curved bands | High | Medium | Medium |
| Modular/Backbone | Mixed | Template pods | Low | High | Medium |
| **Hybrid Drum+Hull** | **Mixed** | **Mixed** | **Medium** | **Highest** | **Highest** |

---

## See Also
- [[Modular Generation Architecture]] — how to support all these with shared code
- [[Procedural Generation]] — current implementation
- [[ISV Prometheus]] — our current ship
