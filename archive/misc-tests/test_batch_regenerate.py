#!/usr/bin/env python3
"""
Regenerate ALL concept art with verbose descriptions and product catalog prompts.
This script will:
1. Generate descriptions for all components using NEW verbose database entries
2. Generate concept art for all components using product catalog style prompts
3. Save all images to output/batch_regenerated/
"""

import sys
from pathlib import Path

# Add progship-core to path
core_dir = Path(__file__).parent / "progship-core"
sys.path.insert(0, str(core_dir))

from progship.structure.structure_generator import ShipStructureGenerator
from progship.pipeline.description_generator import DescriptionGenerator
from progship.pipeline.image_pipeline import ImagePipeline
from PIL import Image
import json
import time

print("=" * 80)
print("BATCH REGENERATION: All Concept Art with Verbose Descriptions")
print("=" * 80)

# Create output directory
output_dir = Path("progship-core/output/batch_regenerated")
output_dir.mkdir(exist_ok=True, parents=True)

# Generate ship structure
print("\n[1/3] Generating ship structure...")
generator = ShipStructureGenerator()
ship = generator.generate_ship(
    ship_type_id="colony_rotating_solar",
    style_id="ceramic_white",
    seed=12345
)
print(f"  Generated ship with {len(ship.rooms)} rooms, {len(ship.systems)} systems")

# Generate descriptions for all components
print("\n[2/3] Generating verbose descriptions...")
desc_gen = DescriptionGenerator()
descriptions = desc_gen.generate_descriptions(
    ship=ship,
    ship_type_id="colony_rotating_solar",
    style_id="ceramic_white"
)

# Save descriptions
desc_path = output_dir / "descriptions_verbose.json"
with open(desc_path, 'w') as f:
    json.dump(descriptions, f, indent=2)
print(f"  Generated {len(descriptions)} descriptions")
print(f"  Saved to: {desc_path}")

# Generate concept art for all components
print("\n[3/3] Generating concept art (product catalog style)...")
print("  This will take 2-5 minutes depending on model speed...")

image_pipeline = ImagePipeline()
results = image_pipeline.generate_batch(
    descriptions=descriptions,
    output_dir=output_dir,
    ship_type_id="colony_rotating_solar",
    style_id="ceramic_white"
)

print("\n" + "=" * 80)
print("BATCH REGENERATION COMPLETE!")
print("=" * 80)
print(f"Total images generated: {len(results)}")
print(f"Output directory: {output_dir}")
print("\nGenerated images:")
for component_id, image_path in results.items():
    size = Path(image_path).stat().st_size / 1024
    print(f"  {component_id}: {Path(image_path).name} ({size:.1f} KB)")

print("\nNext step: Convert images to 3D models with TRELLIS")
print("Command: python test_batch_trellis.py")
