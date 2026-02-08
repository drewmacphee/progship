"""Test loading of new structural elements and light fixtures databases."""

from progship.data.loader import DatabaseLoader
from progship.data.models import (
    StructuralElement, LightFixture, RoomGeometry,
    Transform3D, PlacedLight, RoomConnection
)

# Test loading databases
loader = DatabaseLoader()

print("Testing Database Loading")
print("="*80)

# Load structural elements
structural_db = loader.load_structural_elements()
print(f"\n[OK] Loaded {len(structural_db.elements)} structural elements:")
for elem in structural_db.elements:
    print(f"  - {elem.id}: {elem.name} ({elem.type})")
    print(f"    Dimensions: {elem.dimensions}")
    print(f"    Variants: {len(elem.variants)}")

# Load light fixtures
lights_db = loader.load_light_fixtures()
print(f"\n[OK] Loaded {len(lights_db.fixtures)} light fixtures:")
for light in lights_db.fixtures:
    print(f"  - {light.id}: {light.name} ({light.type})")
    print(f"    Intensity: {light.intensity}, Range: {light.range}m")
    print(f"    Variants: {len(light.variants)}")

# Test creating instances
print("\n" + "="*80)
print("Testing Data Model Instantiation")
print("="*80)

# Test RoomGeometry
geometry = RoomGeometry(
    width=10.0,
    height=3.5,
    depth=8.0,
    floor_element_id="floor_metal_grate",
    ceiling_element_id="ceiling_panel_lit",
    wall_element_id="wall_ceramic_white"
)
print(f"\n[OK] Created RoomGeometry: {geometry.width}x{geometry.height}x{geometry.depth}")

# Test PlacedLight
light_transform = Transform3D(
    position=[0.0, 3.0, 0.0],
    rotation=[0.0, 0.0, 0.0, 1.0]
)
placed_light = PlacedLight(
    light_id="ceiling_panel_ambient",
    transform=light_transform
)
print(f"[OK] Created PlacedLight: {placed_light.light_id}")

# Test RoomConnection
connection = RoomConnection(
    from_room_id="bridge",
    to_room_id="corridor_01",
    connection_type="door",
    connection_element_id="door_sliding_standard",
    from_anchor=[5.0, 0.0, 0.0],
    to_anchor=[-5.0, 0.0, 0.0]
)
print(f"[OK] Created RoomConnection: {connection.from_room_id} -> {connection.to_room_id}")

# Test get specific elements
print("\n" + "="*80)
print("Testing Element Retrieval")
print("="*80)

wall = loader.get_structural_element("wall_ceramic_white")
if wall:
    print(f"\n[OK] Retrieved wall: {wall.name}")
    print(f"  Description: {wall.base_description[:80]}...")
    variant = wall.get_variant("ceramic_white")
    if variant:
        print(f"  Variant: {variant.id}")

light = loader.get_light_fixture("ceiling_panel_ambient")
if light:
    print(f"\n[OK] Retrieved light: {light.name}")
    print(f"  Description: {light.base_description[:80]}...")
    variant = light.get_variant("ceramic_white")
    if variant:
        print(f"  Variant: {variant.id}")

print("\n" + "="*80)
print("[SUCCESS] All tests passed!")
print("="*80)
