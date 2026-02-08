#!/usr/bin/env python3
"""Test generating descriptions with verbose database entries"""
import sys
sys.path.insert(0, '.')

from progship.generation.ship_generator import ShipGenerator
from progship.pipeline.description_generator import DescriptionGenerator
from pathlib import Path
import json

print("=" * 80)
print("TESTING VERBOSE DESCRIPTIONS")
print("=" * 80)

# Generate a simple ship
print("\n[1/2] Generating ship structure...")
ship_gen = ShipGenerator()
ship = ship_gen.generate(
    ship_type_id="colony_rotating_solar",
    style_id="ceramic_white",
    seed=12345
)
print(f"  Generated ship with {len(ship['rooms'])} rooms")
print(f"  Components: {len(ship['components'])} total")

# Generate descriptions
print("\n[2/2] Generating descriptions with verbose database...")
desc_gen = DescriptionGenerator(cache_dir=".cache/descriptions")
manifest = desc_gen.generate_descriptions(
    ship_data=ship,
    ship_type_id="colony_rotating_solar",
    style_id="ceramic_white",
    seed=12345
)

# Save manifest
output_path = Path("output/test_verbose_manifest.json")
output_path.parent.mkdir(exist_ok=True, parents=True)
with open(output_path, 'w') as f:
    json.dump(manifest, f, indent=2)

print(f"\n[OK] Generated {len(manifest['components'])} descriptions")
print(f"Saved to: {output_path}")

# Show first component description
first = manifest['components'][0]
print(f"\nSample ({first['component_id']}):")
print(f"  Type: {first['component_type']}")
print(f"  Description length: {len(first['generated_description'])} chars")
print(f"  First 200 chars: {first['generated_description'][:200]}...")

print("\n" + "=" * 80)
print("SUCCESS! Descriptions generated with verbose database")
print("=" * 80)
print("Next: Regenerate images with these descriptions")
