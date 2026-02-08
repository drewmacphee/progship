"""Direct test - verify file actually saves"""
import sys
sys.path.insert(0, '.')
from progship.pipeline.flux_schnell_generator import FluxSchnellGenerator
from pathlib import Path

print("=" * 70)
print("DIRECT TEST: Isolated Wall Panel Asset")
print("=" * 70)

gen = FluxSchnellGenerator()
prompt = "White ceramic wall panel, single modular piece. 3D game asset render, isolated object, no background, white studio lighting, professional 3D render"
path = Path('output/test_isolated_assets/wall_direct.png')

print(f"\nPrompt: {prompt}")
print(f"Output: {path}")
print(f"Generating... (7-8 minutes)\n")

result = gen.generate(prompt, output_path=str(path))

print(f"\n[RESULT]")
print(f"  Path from generator: {result['path']}")
print(f"  File actually exists: {path.exists()}")
print(f"  File size: {path.stat().st_size if path.exists() else 'N/A'} bytes")

if path.exists():
    print(f"\n✓ SUCCESS! Generated: {path}")
else:
    print(f"\n✗ FAILED! File not created at {path}")
