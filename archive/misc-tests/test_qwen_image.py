#!/usr/bin/env python3
"""Test Qwen-Image-2512 with command console description"""
import sys
from pathlib import Path

# Add progship-core to path
sys.path.insert(0, str(Path(__file__).parent / "progship-core"))

from progship.pipeline.image_generator import QwenImageGenerator, ImageConfig
from PIL import Image

print("=" * 80)
print("Qwen-Image-2512 Test: Command Console")
print("=" * 80)

# Load the verbose description from Phase 2
desc_path = Path("progship-core/output/structure_descriptions.json")
import json
with open(desc_path) as f:
    manifest = json.load(f)

# Find command console description
console_desc = None
for comp in manifest["components"]:
    if comp["component_id"] == "command_console":
        console_desc = comp["generated_description"]
        break

if not console_desc:
    print("[ERROR] Could not find command_console description")
    sys.exit(1)

print(f"\nDescription ({len(console_desc)} chars):")
print(console_desc[:300] + "...")

# Build enhanced prompt for product catalog style
prompt = f"{console_desc}\n\nProduct photograph, studio lighting, isolated on white background, 8K resolution, ultra detailed, professional rendering."

negative_prompt = "人物, 人, 低分辨率, 低画质, 模糊, 扭曲, 变形, AI感, 画面过饱和, 蜡像感, 构图混乱, 文字模糊. No people, no humans, no characters."

print(f"\n{'=' * 80}")
print("Generating with Qwen-Image-2512 (full model, 50 steps)...")
print(f"{'=' * 80}")

# Configure for maximum quality
config = ImageConfig(
    model_id="Qwen/Qwen-Image-2512",
    aspect_ratio="1:1",  # 1328x1328 for 3D conversion
    num_inference_steps=50,  # Full quality
    guidance_scale=7.5,
    true_cfg_scale=4.0,
    seed=42,
    dtype="bfloat16"
)

generator = QwenImageGenerator(config)

# Generate
result = generator.generate(
    prompt=prompt,
    negative_prompt=negative_prompt,
    seed=42
)

# Save
output_dir = Path("progship-core/output/images_qwen_test")
output_dir.mkdir(parents=True, exist_ok=True)
output_path = output_dir / "command_console_qwen.png"

result["image"].save(output_path)

print(f"\n[OK] Saved to: {output_path}")
print(f"Resolution: {result['image'].size}")
print(f"Model: {result['metadata']['model']}")
print(f"Steps: {result['metadata']['steps']}")
print(f"Aspect ratio: {result['metadata']['aspect_ratio']}")

print("\n" + "=" * 80)
print("✅ SUCCESS! Qwen-Image-2512 test complete")
print("=" * 80)
print("\nNext: Compare quality to FLUX version:")
print(f"  FLUX: progship-core/output/images_regenerated/command_console/command_console_main.png")
print(f"  Qwen: {output_path}")
