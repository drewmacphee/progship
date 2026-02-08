#!/usr/bin/env python3
"""Test TripoSR - simpler alternative with texture support"""
import sys
sys.path.insert(0, "/root/TripoSR")

import torch
from PIL import Image
from pathlib import Path
import time
from tsr.system import TSR
from tsr.utils import remove_background, resize_foreground

print("=" * 80)
print("TripoSR Test: Command Console (with textures)")
print("=" * 80)

# Load model
print("\nLoading TripoSR model...")
model = TSR.from_pretrained(
    "stabilityai/TripoSR",
    config_name="config.yaml",
    weight_name="model.ckpt",
)
model.renderer.set_chunk_size(8192)
model.to("cuda")
print("[OK] Model loaded on GPU")

# Load and preprocess image
image_path = "/mnt/c/GIT/progship/progship-core/output/images_regenerated/command_console/command_console_main.png"
image = Image.open(image_path)
print(f"\nInput image: {image_path}")
print(f"Resolution: {image.size}")

# Remove background and resize
print("\nPreprocessing image...")
image = remove_background(image)
image = resize_foreground(image, 0.85)
print("[OK] Preprocessed")

# Run inference
print("\nRunning TripoSR inference...")
print("This should take 3-5 seconds...")

start = time.time()
with torch.no_grad():
    scene_codes = model([image], device="cuda")
    meshes = model.extract_mesh(scene_codes)
elapsed = time.time() - start

print(f"[OK] Generation complete in {elapsed:.1f}s")

# Export
output_path = Path("/root/console_triposr.glb")
meshes[0].export(output_path)

if output_path.exists():
    size_mb = output_path.stat().st_size / (1024 * 1024)
    print(f"\n[OK] Exported: {output_path} ({size_mb:.2f} MB)")
    
    # Check for textures
    import trimesh
    mesh = trimesh.load(output_path)
    has_textures = hasattr(mesh.visual, 'material') and mesh.visual.material is not None
    has_vertex_colors = hasattr(mesh.visual, 'vertex_colors') and mesh.visual.vertex_colors is not None
    
    print(f"Textures: {'✅ Yes' if has_textures else '❌ No'}")
    print(f"Vertex colors: {'✅ Yes' if has_vertex_colors else '❌ No'}")
    
    # Copy to Windows
    import subprocess
    subprocess.run([
        "cp", str(output_path),
        "/mnt/c/GIT/progship/progship-core/output/console_triposr.glb"
    ])
    print(f"\n✅ Copied to Windows: output/console_triposr.glb")

print("\n" + "=" * 80)
print("SUCCESS!")
print("=" * 80)
