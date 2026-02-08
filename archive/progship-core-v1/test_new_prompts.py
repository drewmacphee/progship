"""Test improved image prompts for isolated 3D assets"""
import sys
sys.path.insert(0, 'C:/GIT/progship/progship-core')

from progship.pipeline.image_pipeline import ImagePipeline
from progship.data.models import ComponentDescription

# Test with wall panel (structural element)
wall_desc = ComponentDescription(
    component_id="wall_ceramic_white",
    component_type="structural",
    base_description="4m x 3.5m modular ceramic white panel",
    generated_description="A pristine white ceramic wall panel with subtle hexagonal surface pattern. Clean, seamless edges designed for modular assembly. Smooth matte finish with faint panel seams every meter. Minimal wear, institutional cleanliness.",
    style_tags=["ceramic", "white", "modular", "clean"],
    camera_angles=["front"]
)

# Test with light fixture
light_desc = ComponentDescription(
    component_id="console_spot",
    component_type="light",
    base_description="Adjustable spotlight mounted above workstation",
    generated_description="Compact cylindrical spotlight fixture with white ceramic housing. Adjustable arm allows 180-degree rotation. Emits focused warm white beam (3500K). Recessed LED element visible when active. Modern minimalist design.",
    style_tags=["spotlight", "adjustable", "ceramic", "LED"],
    camera_angles=["3_4_view"]
)

# Test with facility (console)
console_desc = ComponentDescription(
    component_id="navigation_console",
    component_type="facility",
    base_description="Central navigation control station",
    generated_description="Curved white ceramic console with integrated holographic display panel. Three-seat configuration with ergonomic operator stations. Smooth touchscreen surfaces embedded flush with panel. Minimalist interface with subtle LED status indicators. Clean lines, institutional aesthetic.",
    style_tags=["console", "holographic", "ceramic", "ergonomic"],
    camera_angles=["front", "3_4_view"]
)

print("Testing new isolated asset prompts with FLUX.1-schnell...")
print("=" * 70)

# Initialize pipeline with FLUX
pipeline = ImagePipeline(
    output_dir="output/test_isolated_assets",
    model_type="flux_schnell"  # Use FLUX.1-schnell for quality
)

# Test structural element
print("\n1. STRUCTURAL ELEMENT: Wall Panel")
print("-" * 70)
print(f"Full prompt:\n{pipeline._build_image_prompt(wall_desc)}\n")
wall_result = pipeline.generate_from_manifest({
    "ship_id": "test",
    "components": [wall_desc]
})
print(f"[OK] Generated: {wall_result['images'][0]['path']}")

# Test light fixture
print("\n2. LIGHT FIXTURE: Console Spot")
print("-" * 70)
print(f"Full prompt:\n{pipeline._build_image_prompt(light_desc)}\n")
light_result = pipeline.generate_from_manifest({
    "ship_id": "test",
    "components": [light_desc]
})
print(f"[OK] Generated: {light_result['images'][0]['path']}")

# Test facility
print("\n3. FACILITY: Navigation Console")
print("-" * 70)
print(f"Full prompt:\n{pipeline._build_image_prompt(console_desc)}\n")
console_result = pipeline.generate_from_manifest({
    "ship_id": "test",
    "components": [console_desc]
})
print(f"[OK] Generated: {console_result['images'][0]['path']}")

print("\n" + "=" * 70)
print("Test complete! Check output/test_isolated_assets/ for results")
print("\nKey prompt improvements:")
print("  - Added '3D game asset render', 'isolated object', 'no background'")
print("  - Using FLUX.1-schnell for higher quality")
print("  - White studio lighting emphasis")
