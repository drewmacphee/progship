"""Fast test with Segmind-Vega (1-2s per image)."""

from pathlib import Path
from progship.pipeline.image_pipeline import ImagePipeline
from progship.pipeline.image_generator import ImageConfig
from progship.pipeline.model_registry import ModelType

print("Fast Image Generation Test (Segmind-Vega)")
print("="*80)
print("Generating 8 images with Segmind-Vega (1-2s each, ~15s total)")
print("="*80)
print()

manifest_path = Path("output/test_descriptions_with_structural.json")

if not manifest_path.exists():
    print(f"[ERROR] Manifest not found: {manifest_path}")
    exit(1)

# Fast config with Segmind-Vega
config = ImageConfig(
    resolution=512,
    num_inference_steps=20,  # Reduced for speed
    guidance_scale=7.5
)

pipeline = ImagePipeline(
    image_config=config,
    output_dir=Path("output/images_fast_test"),
    model_type=ModelType.SEGMIND_VEGA  # Fast model
)

try:
    image_manifest = pipeline.generate_from_manifest(
        manifest_path=manifest_path,
        negative_prompt="blurry, low quality, distorted, text, watermark",
        generate_angles=False
    )
    
    manifest_output = Path("output/images_fast_test_manifest.json")
    image_manifest.save(manifest_output)
    
    print("\n" + "="*80)
    print("Generated Images Summary")
    print("="*80)
    
    by_type = {}
    for component in image_manifest.components:
        comp_type = component["component_type"]
        by_type[comp_type] = by_type.get(comp_type, 0) + 1
        
        # Show first image path for each type
        if comp_type not in locals():
            print(f"\n{comp_type.upper()}:")
            for img in component["images"][:1]:
                print(f"  {component['component_id']}: {img['path']}")
    
    print(f"\n" + "="*80)
    print("SUMMARY")
    print("="*80)
    for comp_type, count in sorted(by_type.items()):
        print(f"  {comp_type}: {count} images")
    
    print(f"\nTotal: {len(image_manifest.components)} components")
    print(f"Output: {pipeline.output_dir}")
    print("\n[SUCCESS] Fast test complete!")
    print("="*80)
    
except Exception as e:
    print(f"\n[ERROR] {e}")
    import traceback
    traceback.print_exc()
