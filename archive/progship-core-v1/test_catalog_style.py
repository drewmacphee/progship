"""Final attempt - Product catalog style prompt"""
import sys
sys.path.insert(0, '.')
from progship.pipeline.image_generator import FluxImageGenerator
from pathlib import Path

print("=" * 70)
print("PRODUCT CATALOG APPROACH - Simpler, More Direct")
print("=" * 70)

# Completely rewrite the prompt in product catalog style
catalog_prompt = """White futuristic spaceship command console, product photograph. 
Curved desk workstation with touchscreen displays and control panel. 
Semi-circular design, 3 meters wide. 
Multiple glowing blue screens showing navigation interface.
Physical buttons and controls below screens.
Clean white ceramic surface.
Floating on pure white seamless background.
Studio lighting, no shadows.
Professional product photography.
Single object, centered, isometric view."""

negative_prompt = "room, floor, walls, ceiling, windows, people, environment, scene, context, multiple items, furniture, interior design"

print("CATALOG PROMPT:")
print(catalog_prompt)
print(f"\nNEGATIVE: {negative_prompt}")

# Generate
print(f"\n{'=' * 70}")
print("GENERATING")
print("=" * 70)

gen = FluxImageGenerator()
result = gen.generate(
    prompt=catalog_prompt,
    negative_prompt=negative_prompt
)

output_path = Path("output/console_catalog.png")
result['image'].save(output_path, 'PNG')

print(f"\n[OK] {output_path} ({output_path.stat().st_size // 1024}KB)")

print("\n" + "=" * 70)
print("3 VERSIONS TO COMPARE:")
print("=" * 70)
print("1. console_verbose.png - verbose LLM description")
print("2. console_with_negative.png - verbose + negative prompt")
print("3. console_catalog.png - product catalog style (THIS ONE)")
print()
print("Next: Test BEST version with TRELLIS image-to-3D")
