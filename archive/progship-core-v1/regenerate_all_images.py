"""Regenerate ALL concept art with refined isolated asset prompts"""
import sys
sys.path.insert(0, '.')
from progship.pipeline.image_generator import FluxImageGenerator
from progship.data.models import ComponentDescription
from pathlib import Path
import json

print("=" * 70)
print("REGENERATING ALL CONCEPT ART - Refined Isolated Asset Prompts")
print("=" * 70)
print("\nUsing Segmind-Vega (3-4s per image)")
print("Total time: ~30 seconds for 8 images\n")

# Load the existing descriptions
desc_file = Path("output/test_descriptions_with_structural.json")
if not desc_file.exists():
    print(f"ERROR: {desc_file} not found")
    print("Run description generation first")
    sys.exit(1)

with open(desc_file) as f:
    manifest_data = json.load(f)

print(f"Loaded {len(manifest_data['components'])} component descriptions")
print("Components:")
for comp in manifest_data['components']:
    print(f"  - {comp['component_id']} ({comp['component_type']})")

# Initialize generator and output
gen = FluxImageGenerator()
output_dir = Path("output/images_regenerated")
output_dir.mkdir(parents=True, exist_ok=True)

# Build prompts using the ImagePipeline's _build_image_prompt logic
from progship.pipeline.image_pipeline import ImagePipeline
pipeline = ImagePipeline(output_dir=output_dir, model_type="segmind_vega")

results = []
print("\n" + "=" * 70)
print("GENERATING IMAGES")
print("=" * 70)

for i, comp_data in enumerate(manifest_data['components'], 1):
    # Reconstruct ComponentDescription
    comp_desc = ComponentDescription(**comp_data)
    
    print(f"\n[{i}/{len(manifest_data['components'])}] {comp_desc.component_id}")
    print(f"Type: {comp_desc.component_type}")
    
    # Build the prompt using updated pipeline
    full_prompt = pipeline._build_image_prompt(comp_desc)
    print(f"Prompt preview: {full_prompt[:120]}...")
    
    # Generate image
    result = gen.generate(full_prompt)
    
    # Save
    comp_dir = output_dir / comp_desc.component_id
    comp_dir.mkdir(exist_ok=True)
    output_path = comp_dir / f"{comp_desc.component_id}_main.png"
    result['image'].save(output_path, 'PNG')
    
    size_kb = output_path.stat().st_size // 1024
    print(f"[OK] {output_path} ({size_kb}KB)")
    
    results.append({
        'component_id': comp_desc.component_id,
        'component_type': comp_desc.component_type,
        'path': str(output_path),
        'size_kb': size_kb
    })

# Summary
print("\n" + "=" * 70)
print("GENERATION COMPLETE")
print("=" * 70)

print(f"\nGenerated {len(results)} images in {output_dir}/")
print("\nBreakdown by type:")
types = {}
for r in results:
    t = r['component_type']
    types[t] = types.get(t, 0) + 1
print(f"  Rooms: {types.get('room', 0)}")
print(f"  Facilities: {types.get('facility', 0)}")
print(f"  Structural: {types.get('structural', 0)}")
print(f"  Lights: {types.get('light', 0)}")

print(f"\nTotal size: {sum(r['size_kb'] for r in results) // 1024} MB")

print("\n" + "=" * 70)
print("NEXT STEPS")
print("=" * 70)
print("1. Review images in output/images_regenerated/")
print("2. Check light fixtures and facilities (should be clean isolated objects)")
print("3. Feed best images to TRELLIS for 3D conversion")
print("4. Structural elements may need manual geometry + texture mapping")
