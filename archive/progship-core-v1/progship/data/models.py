"""
Data models for ProgShip using Pydantic for validation.

Represents ship types, styles, facilities, rooms, and generated structures.
"""

from typing import List, Optional, Dict, Any
from pydantic import BaseModel, Field, field_validator


# === Ship Types & Styles ===

class ShipType(BaseModel):
    """Ship archetype definition."""
    id: str = Field(pattern=r"^[a-z_]+$")
    name: str
    construction_type: str
    default_style: str
    size_class: str
    typical_rooms: List[str] = []
    description: Optional[str] = None


class StyleDescriptor(BaseModel):
    """Visual style palette."""
    id: str = Field(pattern=r"^[a-z_]+$")
    name: str
    material_palette: List[str]
    color_palette: List[str]
    wear_level: str
    aesthetic_tags: List[str]
    description: Optional[str] = None


# === Facilities (Components) ===

class BoundingBox(BaseModel):
    """3D bounding box for collision and placement."""
    width: float = Field(gt=0)
    height: float = Field(gt=0)
    depth: float = Field(gt=0)
    origin: str = "bottom_center"


class AttachmentPoint(BaseModel):
    """Connection point for pipes, wires, etc."""
    id: str
    position: List[float] = Field(min_length=3, max_length=3)
    type: str


# === Structural Elements (Walls, Floors, Doors) ===

class StructuralVariant(BaseModel):
    """Style-specific variation of a structural element."""
    id: str = Field(pattern=r"^[a-z_]+$")
    style_tags: List[str]
    description_override: Optional[str] = None
    model_hints: Optional[Dict[str, Any]] = None


class StructuralElement(BaseModel):
    """Physical structure components (walls, floors, ceilings, doors)."""
    id: str = Field(pattern=r"^[a-z_]+$")
    name: str
    type: str  # "wall", "floor", "ceiling", "door", "corridor"
    category: str  # "structural", "passage"
    base_description: str
    dimensions: Dict[str, float]  # e.g., {"length": 4.0, "height": 3.0, "thickness": 0.2}
    material_hints: List[str] = []  # ["ceramic", "composite", "glass"]
    variants: List[StructuralVariant] = []
    
    # Door-specific properties
    door_type: Optional[str] = None  # "sliding", "airlock", "pressure_door"
    opening_direction: Optional[str] = None  # "inward", "outward", "sliding"
    
    # Corridor-specific properties
    corridor_width: Optional[float] = None
    
    def get_variant(self, style_id: str) -> Optional[StructuralVariant]:
        """Get variant matching a style ID."""
        for variant in self.variants:
            if style_id in variant.style_tags or variant.id == style_id:
                return variant
        return None


# === Lighting ===

class LightVariant(BaseModel):
    """Style-specific variation of a light fixture."""
    id: str = Field(pattern=r"^[a-z_]+$")
    style_tags: List[str]
    description_override: Optional[str] = None
    intensity_multiplier: float = 1.0
    color_override: Optional[List[float]] = None  # RGB


class LightFixture(BaseModel):
    """Light fixture definition (both geometry and illumination)."""
    id: str = Field(pattern=r"^[a-z_]+$")
    name: str
    type: str  # "ambient", "point", "spot", "area", "panel"
    base_description: str
    intensity: float = 1.0
    color: List[float] = Field(default=[1.0, 1.0, 1.0], min_length=3, max_length=3)  # RGB
    range: float = 10.0
    bounding_box: Optional[BoundingBox] = None  # for fixtures with geometry
    variants: List[LightVariant] = []
    
    def get_variant(self, style_id: str) -> Optional[LightVariant]:
        """Get variant matching a style ID."""
        for variant in self.variants:
            if style_id in variant.style_tags or variant.id == style_id:
                return variant
        return None


class FacilityVariant(BaseModel):
    """Style-specific variation of a facility."""
    id: str = Field(pattern=r"^[a-z_]+$")
    style_tags: List[str]
    description_override: Optional[str] = None
    model_hints: Optional[Dict[str, Any]] = None


class Facility(BaseModel):
    """Component definition with variants."""
    id: str = Field(pattern=r"^[a-z_]+$")
    name: str
    category: str
    base_description: str
    bounding_box: BoundingBox
    attachment_points: List[AttachmentPoint] = []
    spatial_constraints: List[str] = []
    variants: List[FacilityVariant] = []
    
    def get_variant(self, style_id: str) -> Optional[FacilityVariant]:
        """Get variant matching a style ID."""
        for variant in self.variants:
            if style_id in variant.style_tags or variant.id == style_id:
                return variant
        return None


# === Rooms ===

class DoorPlacement(BaseModel):
    """Door location on a room wall."""
    wall_side: str  # "north", "south", "east", "west"
    position: float  # 0.0-1.0, normalized position along wall
    structural_element_id: str  # e.g., "door_airlock"


class WindowPlacement(BaseModel):
    """Window location on a room wall."""
    wall_side: str  # "north", "south", "east", "west"
    position: float  # 0.0-1.0, normalized position along wall
    width: float
    height: float


class RoomGeometry(BaseModel):
    """Detailed geometry specification for room construction."""
    width: float = Field(gt=0)
    height: float = Field(gt=0)
    depth: float = Field(gt=0)
    wall_thickness: float = Field(default=0.2, gt=0)
    floor_element_id: str  # "floor_metal_grate"
    ceiling_element_id: str  # "ceiling_panel_lit"
    wall_element_id: str  # "wall_ceramic_white"
    door_placements: List[DoorPlacement] = []
    window_placements: List[WindowPlacement] = []


class Room(BaseModel):
    """Room template definition."""
    id: str = Field(pattern=r"^[a-z_]+$")
    name: str
    dimensions: Dict[str, float]  # width, height, depth (legacy, kept for compatibility)
    geometry: Optional[RoomGeometry] = None  # NEW: detailed geometry
    characteristic_facilities: List[str] = []
    characteristic_lights: List[str] = []  # NEW: typical light fixtures for this room
    spatial_constraints: List[str] = []
    description: Optional[str] = None


# === Generated Structure ===

class Transform3D(BaseModel):
    """3D transformation (position, rotation, scale)."""
    position: List[float] = Field(min_length=3, max_length=3)
    rotation: List[float] = Field(min_length=4, max_length=4)  # quaternion [x,y,z,w]
    scale: List[float] = Field(default=[1.0, 1.0, 1.0], min_length=3, max_length=3)


class PlacedLight(BaseModel):
    """Light instance in scene."""
    light_id: str
    variant_id: Optional[str] = None
    transform: Transform3D
    intensity_override: Optional[float] = None
    color_override: Optional[List[float]] = None  # RGB
    room_id: Optional[str] = None


class PlacedStructuralElement(BaseModel):
    """Structural element instance (for special cases like unique doors)."""
    element_id: str
    variant_id: Optional[str] = None
    transform: Transform3D
    room_id: Optional[str] = None


class PlacedFacility(BaseModel):
    """Facility instance with transform."""
    facility_id: str
    variant_id: Optional[str] = None
    transform: Transform3D
    room_id: Optional[str] = None


class RoomConnection(BaseModel):
    """Connection between two rooms via door/corridor."""
    from_room_id: str
    to_room_id: str
    connection_type: str  # "door", "corridor", "hatch"
    connection_element_id: str  # e.g., "door_airlock_01"
    from_anchor: List[float] = Field(min_length=3, max_length=3)  # [x, y, z] on from_room
    to_anchor: List[float] = Field(min_length=3, max_length=3)  # [x, y, z] on to_room
    corridor_length: Optional[float] = None  # if connection_type == "corridor"


class PlacedRoom(BaseModel):
    """Room instance with transform and contained elements."""
    room_id: str
    transform: Transform3D
    facilities: List[PlacedFacility] = []
    lights: List[PlacedLight] = []  # NEW
    structural_elements: List[PlacedStructuralElement] = []  # NEW (for unique elements)


class ShipStructure(BaseModel):
    """Generated ship structure."""
    ship_type_id: str
    style_id: str
    seed: Optional[int] = None
    rooms: List[PlacedRoom] = []
    connections: List[RoomConnection] = []  # NEW
    metadata: Dict[str, Any] = {}


class ComponentDescription(BaseModel):
    """Description of a single component (room or facility)."""
    
    component_id: str
    component_type: str  # "facility" or "room"
    base_description: str  # from database
    generated_description: str  # from LLM
    style_tags: List[str]
    camera_angles: List[str]  # ["front_view", "three_quarter_view", etc.]


class DescriptionManifest(BaseModel):
    """Manifest of all generated descriptions for a ship."""
    
    ship_type_id: str
    style_id: str
    seed: int
    generation_timestamp: str
    model_name: str
    components: List[ComponentDescription]


# === Database Collections ===

class StructuralDatabase(BaseModel):
    """Collection of structural elements."""
    elements: List[StructuralElement]


class LightDatabase(BaseModel):
    """Collection of light fixtures."""
    fixtures: List[LightFixture]


class ShipTypeDatabase(BaseModel):
    """Collection of ship types."""
    ship_types: List[ShipType]


class StyleDatabase(BaseModel):
    """Collection of style descriptors."""
    style_descriptors: List[StyleDescriptor]


class FacilityDatabase(BaseModel):
    """Collection of facilities."""
    facilities: List[Facility]


class RoomDatabase(BaseModel):
    """Collection of room templates."""
    rooms: List[Room]
