"""Test updated description generator with structural elements and lights."""

from progship.data.models import (
    ShipStructure, PlacedRoom, PlacedFacility, PlacedLight,
    Transform3D, RoomGeometry, DoorPlacement, RoomConnection
)
from progship.pipeline.description_generator import DescriptionGenerator
from progship.data.loader import get_loader
import json

# Create a test structure with all element types
structure = ShipStructure(
    ship_type_id="colony_stacked",
    style_id="ceramic_white",
    seed=42,
    rooms=[
        PlacedRoom(
            room_id="bridge",
            transform=Transform3D(
                position=[0.0, 0.0, 0.0],
                rotation=[0.0, 0.0, 0.0, 1.0]
            ),
            facilities=[
                PlacedFacility(
                    facility_id="command_console",
                    variant_id=None,
                    transform=Transform3D(
                        position=[-5.0, 0.0, -4.0],
                        rotation=[0.0, 0.0, 0.0, 1.0]
                    )
                )
            ],
            lights=[
                PlacedLight(
                    light_id="ceiling_panel_ambient",
                    variant_id=None,
                    transform=Transform3D(
                        position=[0.0, 3.0, 0.0],
                        rotation=[0.0, 0.0, 0.0, 1.0]
                    )
                ),
                PlacedLight(
                    light_id="console_spot",
                    variant_id=None,
                    transform=Transform3D(
                        position=[-5.0, 2.5, -4.0],
                        rotation=[0.0, 0.0, 0.0, 1.0]
                    )
                )
            ]
        )
    ],
    connections=[
        RoomConnection(
            from_room_id="bridge",
            to_room_id="corridor_01",
            connection_type="door",
            connection_element_id="door_sliding_standard",
            from_anchor=[5.0, 0.0, 0.0],
            to_anchor=[-5.0, 0.0, 0.0]
        )
    ]
)

# Update bridge room to have geometry
loader = get_loader()
bridge_room = loader.get_room("bridge")
if bridge_room:
    from progship.data.models import Room, RoomGeometry, DoorPlacement
    # Create updated room with geometry
    bridge_with_geometry = Room(
        id="bridge",
        name="Bridge",
        dimensions={"width": 10.0, "height": 3.5, "depth": 8.0},
        geometry=RoomGeometry(
            width=10.0,
            height=3.5,
            depth=8.0,
            floor_element_id="floor_metal_grate",
            ceiling_element_id="ceiling_panel_lit",
            wall_element_id="wall_ceramic_white",
            door_placements=[
                DoorPlacement(
                    wall_side="east",
                    position=0.5,
                    structural_element_id="door_sliding_standard"
                )
            ]
        ),
        characteristic_facilities=["command_console"],
        characteristic_lights=["ceiling_panel_ambient", "console_spot"]
    )
    
    # Temporarily inject it into loader cache
    loader._rooms.rooms = [bridge_with_geometry]

print("Testing Description Generator with Structural Elements and Lights")
print("="*80)

# Generate descriptions
generator = DescriptionGenerator()
manifest = generator.generate_descriptions(
    structure,
    use_cache=False,  # Force fresh generation
    include_structural=True,
    include_lights=True
)

print("\n" + "="*80)
print("Generated Descriptions Summary")
print("="*80)

# Count by type
counts = {}
for component in manifest.components:
    comp_type = component.component_type
    counts[comp_type] = counts.get(comp_type, 0) + 1

print(f"\nTotal components: {len(manifest.components)}")
for comp_type, count in sorted(counts.items()):
    print(f"  - {comp_type}: {count}")

# Show sample descriptions
print("\n" + "="*80)
print("Sample Descriptions")
print("="*80)

for comp_type in ["structural", "light", "facility", "room"]:
    matching = [c for c in manifest.components if c.component_type == comp_type]
    if matching:
        sample = matching[0]
        print(f"\n[{sample.component_type.upper()}] {sample.component_id}")
        print(f"Base: {sample.base_description[:80]}...")
        print(f"Generated: {sample.generated_description[:150]}...")
        print(f"Camera angles: {', '.join(sample.camera_angles)}")

# Save manifest
output_path = "output/test_descriptions_with_structural.json"
generator.save_manifest(manifest, output_path)

print("\n" + "="*80)
print("[SUCCESS] Description generator updated and tested!")
print("="*80)
