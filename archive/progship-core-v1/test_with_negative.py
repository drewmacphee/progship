"""Test console with NEGATIVE PROMPT to remove environment"""
import sys
sys.path.insert(0, '.')
from progship.pipeline.image_generator import FluxImageGenerator
from progship.pipeline.image_pipeline import ImagePipeline
from progship.data.models import ComponentDescription
from pathlib import Path
import json

print("=" * 70)
print("CONSOLE TEST - Adding NEGATIVE PROMPT")
print("=" * 70)

# Load the verbose console description
with open('output/test_descriptions_verbose.json') as f:
    data = json.load(f)

console_data = [c for c in data['components'] if c['component_id'] == 'command_console'][0]
console_desc = ComponentDescription(**console_data)

# Build image prompt
pipeline = ImagePipeline(output_dir="output", model_type="segmind_vega")
positive_prompt = pipeline._build_image_prompt(console_desc)

# Strong negative prompt to remove ALL environmental context
negative_prompt = "room, interior, walls, floor, ceiling, windows, plants, furniture, people, characters, scene, environment, background objects, architectural context, hallway, corridor, doors, multiple objects, pattern, texture fill, tiled, repeated"

print(f"POSITIVE: {positive_prompt[:200]}...")
print(f"\nNEGATIVE: {negative_prompt}")

# Generate with negative prompt
print(f"\n{'=' * 70}")
print("GENERATING WITH NEGATIVE PROMPT")
print("=" * 70)

gen = FluxImageGenerator()
result = gen.generate(
    prompt=positive_prompt,
    negative_prompt=negative_prompt
)

# Save
output_path = Path("output/console_with_negative.png")
result['image'].save(output_path, 'PNG')

print(f"\n[OK] Generated: {output_path} ({output_path.stat().st_size // 1024}KB)")

print("\n" + "=" * 70)
print("COMPARISON")
print("=" * 70)
print("1. output/console_verbose.png (no negative)")
print("   - Has windows, plants, floor")
print()
print("2. output/console_with_negative.png (WITH negative)")
print("   - Should be isolated console only")
print()
print("Review both, then test best one with TRELLIS")
