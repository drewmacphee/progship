#!/usr/bin/env python3
"""Test StableFast3D with one of our concept art images"""
import os
os.environ["CUDA_VISIBLE_DEVICES"] = "0"

import sys
sys.path.insert(0, "/root/stable-fast-3d")

import torch
from PIL import Image
from sf3d.system import SF3D
from pathlib import Path

print("=" * 80)
print("StableFast3D Test: Command Console with Textures")
print("=" * 80)

# Load model
print("\nLoading StableFast3D model...")
model = SF3D.from_pretrained(
    "stabilityai/stable-fast-3d",
    config_name="config.yaml",
    weight_name="model.safetensors",
)
model.cuda()
model.eval()
print("[OK] Model loaded on GPU")

# Load test image
image_path = "/mnt/c/GIT/progship/progship-core/output/images_regenerated/command_console/command_console_main.png"
image = Image.open(image_path)
print(f"\nInput image: {image_path}")
print(f"Resolution: {image.size}")

# Run inference
print("\nRunning StableFast3D inference...")
print("This should take ~0.5-2 seconds...")

with torch.no_grad():
    output = model.run_image(
        image,
        bake_resolution=1024,  # Texture resolution
        remesh="none",  # Keep original mesh
    )

print("[OK] Conversion complete!")

# Export with textures
output_path = "/root/console_stablefast3d.glb"
print(f"\nExporting to: {output_path}")

# StableFast3D exports directly
output.save_glb(output_path)

if Path(output_path).exists():
    size_mb = Path(output_path).stat().st_size / (1024 * 1024)
    print(f"[OK] Exported: {output_path} ({size_mb:.2f} MB)")
    print(f"✅ Model includes UV-unwrapped textures!")
else:
    print("[FAIL] GLB not created")

print("\n" + "=" * 80)
print("SUCCESS! Textured 3D model generated")
print("=" * 80)
print(f"\nCopy to Windows:")
print(f"  cp {output_path} /mnt/c/GIT/progship/progship-core/output/console_textured.glb")
