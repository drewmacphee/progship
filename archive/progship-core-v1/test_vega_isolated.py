"""Test isolated assets with Segmind-Vega (faster, less VRAM)"""
import sys
sys.path.insert(0, '.')
from progship.pipeline.image_generator import FluxImageGenerator
from pathlib import Path

print("=" * 70)
print("ISOLATED ASSET TEST - Segmind-Vega")
print("=" * 70)

gen = FluxImageGenerator()
output_dir = Path('output/test_isolated_vega')
output_dir.mkdir(parents=True, exist_ok=True)

# Test 3 different asset types with ISOLATED keywords
tests = [
    ("wall", "White ceramic wall panel, single modular piece, clean surface. 3D game asset render, isolated object, no background, white studio lighting, professional 3D render, 8k"),
    ("light", "Small spotlight fixture, cylindrical white ceramic housing. 3D game asset render, isolated light fixture, no background, white studio lighting, glowing LED element, professional 3D render, 8k"),
    ("console", "Curved white console with flat display panel. 3D game asset render, isolated object, no background, white studio lighting, clean futuristic design, professional 3D render, 8k")
]

for name, prompt in tests:
    print(f"\n{name.upper()}")
    print("-" * 70)
    print(f"Prompt: {prompt[:100]}...")
    
    path = output_dir / f"{name}.png"
    result = gen.generate(prompt)
    
    # Save the image from result
    result['image'].save(path, 'PNG')
    
    if path.exists():
        print(f"[OK] {path} ({path.stat().st_size // 1024}KB)")
    else:
        print(f"[FAIL] Not created")

print("\n" + "=" * 70)
print(f"Complete! Check {output_dir}/")
print("\nSegmind-Vega: 3-4s per image (Apache 2.0, 1.5B params)")
