"""Test updated prompts for isolated 3D assets with FLUX"""
import sys
sys.path.insert(0, 'C:/GIT/progship/progship-core')

from progship.pipeline.flux_schnell_generator import FluxSchnellGenerator
from pathlib import Path

# NEW prompts with isolated asset keywords
wall_prompt = "A pristine white ceramic wall panel. 3D game asset render, isolated object, no background, white studio lighting, modular design, tileable texture, architectural detail, professional 3D render, 8k resolution"

light_prompt = "Compact spotlight fixture with ceramic housing. 3D game asset render, isolated light fixture, no background, white studio lighting, illumination visible, professional 3D render, 8k resolution"

console_prompt = "Curved white console with holographic displays. 3D game asset render, isolated object, no background, white studio lighting, professional 3D render, 8k resolution"

print("FLUX.1-schnell - Testing NEW Isolated Asset Prompts")
print("=" * 70)

generator = FluxSchnellGenerator()
output_dir = Path("output/test_isolated_assets")
output_dir.mkdir(parents=True, exist_ok=True)

print("\n1. WALL PANEL")
print(f"Prompt: {wall_prompt}\n")
generator.generate(wall_prompt, str(output_dir / "wall.png"))

print("\n2. LIGHT FIXTURE")  
print(f"Prompt: {light_prompt}\n")
generator.generate(light_prompt, str(output_dir / "light.png"))

print("\n3. CONSOLE")
print(f"Prompt: {console_prompt}\n")
generator.generate(console_prompt, str(output_dir / "console.png"))

print("\n" + "=" * 70)
print(f"[OK] Complete! Images in {output_dir}/")
print("\nKey changes: '3D game asset', 'isolated object', 'no background'")
