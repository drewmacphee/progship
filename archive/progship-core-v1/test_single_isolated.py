"""Quick test: Single wall panel with isolated asset prompt"""
import sys
sys.path.insert(0, 'C:/GIT/progship/progship-core')

from progship.pipeline.flux_schnell_generator import FluxSchnellGenerator
from pathlib import Path

# NEW isolated asset prompt
wall_prompt = "White ceramic wall panel, modular design, clean surface. 3D game asset render, isolated object, no background, white studio lighting, professional 3D render, 8k"

print("Testing FLUX with isolated asset prompt")
print("=" * 70)
print(f"Prompt: {wall_prompt}\n")

generator = FluxSchnellGenerator()
output_dir = Path("output/test_isolated_assets")
output_dir.mkdir(parents=True, exist_ok=True)

output_path = output_dir / "wall_panel_isolated.png"
print(f"Generating to: {output_path}")
print("This will take ~7 minutes with FLUX.1-schnell...\n")

generator.generate(wall_prompt, str(output_path))

print("\n" + "=" * 70)
print(f"[OK] Complete! Check {output_path}")
print("\nCompare to previous images - should be isolated asset, not scene")
