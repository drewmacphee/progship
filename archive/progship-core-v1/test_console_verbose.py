"""Test console image with NEW verbose description"""
import sys
sys.path.insert(0, '.')
from progship.pipeline.image_generator import FluxImageGenerator
from progship.pipeline.image_pipeline import ImagePipeline
from progship.data.models import ComponentDescription
from pathlib import Path
import json

print("=" * 70)
print("COMMAND CONSOLE - NEW Verbose Description Test")
print("=" * 70)

# Load the NEW verbose description
with open('output/test_descriptions_verbose.json') as f:
    data = json.load(f)

console_data = [c for c in data['components'] if c['component_id'] == 'command_console'][0]

print(f"\nOLD description ({len('The Colony Stacked command console exudes a pristine, otherworldly elegance...')} chars):")
print("'The Colony Stacked command console exudes a pristine, otherworldly elegance typical of Ceramic White...'")
print("\nNEW description ({} chars):".format(len(console_data['generated_description'])))
print(console_data['generated_description'][:300] + "...")

# Create ComponentDescription
console_desc = ComponentDescription(**console_data)

# Build image prompt
pipeline = ImagePipeline(output_dir="output", model_type="segmind_vega")
full_prompt = pipeline._build_image_prompt(console_desc)

print(f"\n{'=' * 70}")
print("IMAGE PROMPT (will be truncated to 77 tokens by CLIP)")
print("=" * 70)
print(full_prompt)

# Generate image
print(f"\n{'=' * 70}")
print("GENERATING...")
print("=" * 70)

gen = FluxImageGenerator()
result = gen.generate(full_prompt)

# Save
output_path = Path("output/console_verbose.png")
result['image'].save(output_path, 'PNG')

print(f"\n[OK] Generated: {output_path} ({output_path.stat().st_size // 1024}KB)")
print("\n" + "=" * 70)
print("Compare to: output/images_regenerated/command_console/command_console_main.png")
print("Should now clearly show:")
print("  ✓ Curved console desk/workstation")
print("  ✓ Multiple display screens")
print("  ✓ Control buttons/panels")
print("  ✓ Star Trek/Expanse bridge aesthetic")
