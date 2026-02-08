"""Test image generation for all component types (structural, lights, facilities, rooms)."""

import os
os.environ['HF_TOKEN'] = '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'

from pathlib import Path
from progship.pipeline.image_pipeline import ImagePipeline
from progship.pipeline.image_generator import ImageConfig
from progship.pipeline.model_registry import ModelType

print("Testing Image Generation for All Component Types")
print("="*80)
print("This will generate concept art for:")
print("  - 1 room (bridge)")
print("  - 1 facility (command_console)")
print("  - 4 structural elements (wall, floor, ceiling, door)")
print("  - 2 light fixtures (ceiling_panel_ambient, console_spot)")
print("\nUsing FLUX.1-schnell (30-60s per image)")
print("="*80)
print()

# Use the descriptions we just generated
manifest_path = Path("output/test_descriptions_with_structural.json")

if not manifest_path.exists():
    print(f"[ERROR] Manifest not found: {manifest_path}")
    print("Run test_description_generator.py first to create the manifest.")
    exit(1)

# Configure image generation (using lower resolution for testing)
config = ImageConfig(
    resolution=512,  # 512x512 for faster testing (vs 1024x1024)
    num_inference_steps=4,
    guidance_scale=0.0
)

pipeline = ImagePipeline(
    image_config=config,
    output_dir=Path("output/images_all_types"),
    model_type=ModelType.FLUX_SCHNELL
)

# Generate images
try:
    image_manifest = pipeline.generate_from_manifest(
        manifest_path=manifest_path,
        negative_prompt="blurry, low quality, distorted, text, watermark, abstract, unrealistic",
        generate_angles=False  # Just main views for testing
    )
    
    # Save manifest
    manifest_output = Path("output/images_all_types_manifest.json")
    image_manifest.save(manifest_output)
    print(f"\n[OK] Saved image manifest: {manifest_output}")
    
    # Print summary
    print("\n" + "="*80)
    print("Generated Images Summary")
    print("="*80)
    
    by_type = {}
    for component in image_manifest.components:
        comp_type = component["component_type"]
        by_type[comp_type] = by_type.get(comp_type, 0) + 1
    
    for comp_type, count in sorted(by_type.items()):
        print(f"  {comp_type}: {count} images")
    
    print(f"\nTotal: {len(image_manifest.components)} components")
    print(f"Output directory: {pipeline.output_dir}")
    
    print("\n" + "="*80)
    print("[SUCCESS] Image generation complete!")
    print("="*80)
    
except Exception as e:
    print(f"\n[ERROR] Image generation failed: {e}")
    import traceback
    traceback.print_exc()
