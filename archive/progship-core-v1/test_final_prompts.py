"""Test FINAL refined prompts for isolated 3D assets"""
import sys
sys.path.insert(0, '.')
from progship.pipeline.image_pipeline import ImagePipeline
from progship.data.models import ComponentDescription
from pathlib import Path

print("=" * 70)
print("FINAL PROMPT TEST - Refined for Single Isolated Objects")
print("=" * 70)

# Test structural, light, and facility with updated prompts
wall_desc = ComponentDescription(
    component_id="wall_ceramic_white",
    component_type="structural",
    base_description="Single modular ceramic white panel",
    generated_description="A single pristine white ceramic wall panel, 4m x 3.5m modular piece. Clean surface with subtle hexagonal pattern. Designed for modular assembly with seamless edges.",
    style_tags=["ceramic", "white", "modular"],
    camera_angles=["front"]
)

light_desc = ComponentDescription(
    component_id="console_spot",
    component_type="light",
    base_description="Single adjustable spotlight fixture",
    generated_description="A single compact cylindrical spotlight fixture with white ceramic housing. Adjustable arm design. Emits focused warm white beam with visible LED element.",
    style_tags=["spotlight", "ceramic", "LED"],
    camera_angles=["3_4_view"]
)

console_desc = ComponentDescription(
    component_id="navigation_console",
    component_type="facility",
    base_description="Single curved console station",
    generated_description="A single curved white ceramic console with integrated holographic display panel. Smooth touchscreen surface with minimalist interface. Clean futuristic design.",
    style_tags=["console", "holographic", "ceramic"],
    camera_angles=["front"]
)

# Initialize pipeline with Segmind-Vega
pipeline = ImagePipeline(
    output_dir="output/test_final_isolated",
    model_type="segmind_vega"
)

print("\nGenerating with REFINED prompts:")
print("  + 'single piece, centered composition'")
print("  + 'pure white background'")
print("  + 'product photography style'")
print("  + 'orthographic view' (structural)")
print("  + 'no shadows'")
print()

tests = [
    ("wall", wall_desc),
    ("light", light_desc),
    ("console", console_desc)
]

for name, desc in tests:
    print(f"\n{name.upper()}")
    print("-" * 70)
    
    # Show the full prompt
    full_prompt = pipeline._build_image_prompt(desc)
    print(f"Full prompt: {full_prompt[:150]}...\n")
    
    # Generate directly
    from progship.pipeline.image_generator import FluxImageGenerator
    gen = FluxImageGenerator()
    result = gen.generate(full_prompt)
    
    # Save
    output_path = Path(f"output/test_final_isolated/{name}.png")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    result['image'].save(output_path, 'PNG')
    
    print(f"[OK] Saved: {output_path} ({output_path.stat().st_size // 1024}KB)")

print("\n" + "=" * 70)
print("Complete! Check output/test_final_isolated/")
print("\nCompare to output/test_isolated_vega/ to see improvements:")
print("  - Wall should be SINGLE panel (not multiple tiles)")
print("  - Light should be SINGLE fixture (centered)")
print("  - Console should be cleaner with pure white background")
