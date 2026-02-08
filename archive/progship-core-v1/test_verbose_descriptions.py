"""Regenerate descriptions with updated facility database"""
import sys
sys.path.insert(0, '.')
from progship.pipeline.description_generator import DescriptionGenerator
from progship.data.loader import DatabaseLoader
import json
from pathlib import Path

print("=" * 70)
print("REGENERATING DESCRIPTIONS - Updated Facilities Database")
print("=" * 70)

# Load updated databases
loader = DatabaseLoader()
facilities = loader.load_facilities()
ship_types = loader.load_ship_types()
styles = loader.load_styles()

print(f"\nFacilities loaded: {len(facilities.facilities)}")
for fac in facilities.facilities:
    print(f"  - {fac.id}: {len(fac.base_description)} char description")

# Load structure
structure_file = Path("output/structure.json")
if not structure_file.exists():
    print(f"ERROR: {structure_file} not found")
    sys.exit(1)
    
# Load as ShipStructure model
from progship.data.models import ShipStructure
with open(structure_file) as f:
    structure_data = json.load(f)
structure = ShipStructure(**structure_data)

# Initialize generator
gen = DescriptionGenerator()

# Get ship type and style
ship_type = loader.get_ship_type("colony_stacked")
style = loader.get_style("ceramic_white")

print(f"\nGenerating descriptions...")
print(f"Ship Type: {structure.ship_type_id}")
print(f"Style: {structure.style_id}")

# Generate with updated facility descriptions
manifest = gen.generate_descriptions(
    structure=structure,
    include_structural=False,  # Skip structural for now, test facility first
    include_lights=False
)

# Save
output_file = Path("output/test_descriptions_verbose.json")
with open(output_file, 'w') as f:
    json.dump(manifest.model_dump(), f, indent=2)

print(f"\n[OK] Saved: {output_file}")

# Show the command_console description
for comp in manifest['components']:
    if comp['component_id'] == 'command_console':
        print(f"\n{'=' * 70}")
        print("COMMAND CONSOLE DESCRIPTION (NEW)")
        print("=" * 70)
        print(f"Length: {len(comp['generated_description'])} chars")
        print(f"\n{comp['generated_description']}")
        break

print(f"\n{'=' * 70}")
print("Next: Regenerate images with this verbose description")
