# Modular Generation Architecture

How to support multiple ship types (rectangular hull, O'Neill cylinder, torus, hybrid) with reusable, composable generation code.

---

## Core Insight

Every ship type shares the same fundamental generation steps — they just do each step differently based on geometry:

1. **Define the boundary** (hull shape per deck/section)
2. **Lay infrastructure** (corridors, shafts, utilities)
3. **Identify fillable zones** (empty space between infrastructure)
4. **Pack rooms into zones** (respecting room specs)
5. **Place doors** (between adjacent rooms/corridors)
6. **Connect vertically** (shafts, elevators, ladders)
7. **Emit tables** (Room, Door, Corridor → SpacetimeDB)

The OUTPUT is always the same: Room/Door/Corridor tables. Only the geometry math changes.

---

## Proposed Trait System

```rust
/// Defines the shape of one "level" (deck, ring segment, etc.)
trait DeckShape {
    /// Returns the boundary polygon for this level
    fn boundary(&self) -> Vec<(i32, i32)>;
    
    /// Is this grid cell inside the boundary?
    fn contains(&self, x: i32, y: i32) -> bool;
    
    /// Width and height of bounding box
    fn bounds(&self) -> (i32, i32);
}

/// Generates infrastructure (corridors, shafts) for a level
trait InfrastructureLayout {
    /// Stamp corridors and shafts onto the grid
    fn stamp_infrastructure(&self, grid: &mut Grid, deck: &DeckShape);
    
    /// Returns shaft positions (must be consistent across levels)
    fn shaft_positions(&self) -> Vec<ShaftAnchor>;
}

/// Fills empty zones with rooms
trait RoomPacker {
    /// Find empty rectangular zones in grid
    fn find_zones(&self, grid: &Grid) -> Vec<Zone>;
    
    /// Pack rooms into a zone
    fn pack_rooms(&self, zone: &Zone, specs: &[RoomSpec]) -> Vec<PlacedRoom>;
}

/// Places doors between adjacent rooms/corridors
trait DoorPlacer {
    /// Scan grid for adjacencies, emit doors
    fn place_doors(&self, grid: &Grid, rooms: &[PlacedRoom]) -> Vec<DoorPlacement>;
}
```

---

## Concrete Implementations

### Rectangular Hull (current)
```
DeckShape:       RectangularDeck { width, length, taper }
Infrastructure:  SpineAndCross { spine_width, cross_interval, svc_corridor }
RoomPacker:      SquarifiedTreemap
DoorPlacer:      GridAdjacency
```

### O'Neill Cylinder
```
DeckShape:       AnnularDeck { inner_radius, outer_radius, sectors }
Infrastructure:  RadialSpokes { ring_corridors, radial_count }
RoomPacker:      SectorBSP (subdivide pie slices)
DoorPlacer:      ArcAdjacency (doors along curved walls)
```

### Stanford Torus
```
DeckShape:       TorusSegment { segment_angle, tube_radius, ring_radius }
Infrastructure:  SegmentSpine { spoke_connections }
RoomPacker:      SquarifiedTreemap (segments are ~rectangular)
DoorPlacer:      GridAdjacency (same as rectangular, segments are flat enough)
```

### Modular/Backbone
```
DeckShape:       ModulePod { pod_type, length, width }
Infrastructure:  SpineAttach { spine_width, attach_points }
RoomPacker:      TemplateStamp (predefined room layouts per pod type)
DoorPlacer:      AirlockConnect (module-to-spine connections)
```

### Hybrid Drum+Hull
```
// Composes multiple section types along ship axis:
sections: [
    ShieldSection { shape: Conical },
    HabitatDrum { shape: Annular, rotation: true },
    TransitionZone { shape: Rectangular },
    EngineeringHull { shape: Rectangular },
    PropulsionSection { shape: Conical },
]
// Each section uses its own DeckShape + Infrastructure + Packer
```

---

## What's Shared (Reusable Across ALL Ship Types)

| Component | Why It's Universal |
|-----------|-------------------|
| **Room/Door/Corridor tables** | Output format is the same regardless of geometry |
| **Facility manifest** | Same rooms needed (cabins, mess, medical) on any ship |
| **Room spec system** | Same data: room_type, target_area, quantity, deck_zone |
| **Door verification** | Mathematical checks work on any adjacency data |
| **Pathfinding (BFS)** | Door graph is shape-agnostic |
| **Simulation systems** | Needs, activities, atmosphere — don't care about geometry |
| **Client rendering** | Rooms are rectangles in DB regardless of hull shape* |

*For curved ships, the DB stores "flattened" room positions. The client can optionally curve-project them for visual fidelity, but the simulation doesn't need to know.

---

## What's Ship-Type-Specific

| Component | Varies By |
|-----------|-----------|
| **Boundary function** | `contains(x,y)` depends on hull shape |
| **Corridor topology** | Spine vs radial vs ring vs module-attach |
| **Room packing** | Treemap vs sector-BSP vs template |
| **Vertical transport** | Elevators vs spokes vs radial shafts |
| **Gravity model** | Thrust vs spin (affects which wall is "floor") |

---

## Implementation Strategy

### Phase 1: Extract Traits (Refactor Current Code)
Take existing `generation.rs` and split into:
- `RectangularDeck` implementing `DeckShape`
- `SpineInfrastructure` implementing `InfrastructureLayout`
- `TreemapPacker` implementing `RoomPacker`
- `GridDoorPlacer` implementing `DoorPlacer`

No new functionality — just reorganization. Everything still works the same.

### Phase 2: Add Second Ship Type
Implement `AnnularDeck` + `RadialInfrastructure` for O'Neill cylinder.
Proves the trait system works with genuinely different geometry.

### Phase 3: Hybrid Composition
Build a `CompositeShip` that chains sections:
```rust
struct CompositeShip {
    sections: Vec<Box<dyn ShipSection>>,
}
```
Each section generates independently, then transition zones connect them.

---

## SpacetimeDB Compatibility

All of this must work in WASM:
- No external crates (no `nalgebra`, `cgmath`, etc.)
- Geometry math implemented from scratch (sin/cos/sqrt via `core::f32`)
- Grid and room data are intermediate — only Room/Door/Corridor tables are stored
- Trait objects may need to be enums instead (`ShipType::Rectangular | ShipType::Cylinder`)

---

## Open Questions

1. **Curved room rendering**: Should the client render curved walls for O'Neill/torus ships, or approximate with short straight segments?
2. **Gravity direction**: In spin ships, "down" is outward. How does this affect movement code? (Player walks on inner surface of cylinder)
3. **Scale differences**: O'Neill cylinders are 32km long vs our 400m hull. How to handle vastly different scales?
4. **When to build this**: Should we finish polishing the rectangular ship first, or refactor to traits now?

---

## See Also
- [[Colony Ship Types]] — the ship designs this architecture supports
- [[Procedural Generation]] — current rectangular implementation
- [[Technical Constraints]] — WASM limitations
- [[Open Problems]] — room size inflation affects all ship types
