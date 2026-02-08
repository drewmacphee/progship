#!/usr/bin/env python3
"""
Batch convert all concept art images to 3D models with TRELLIS.
Processes all images from output/images_regenerated/ and exports GLB files.
"""
import os
os.environ["ATTN_BACKEND"] = "xformers"
os.environ["SPCONV_ALGO"] = "native"

import sys
sys.path.insert(0, "/root/TRELLIS")

from PIL import Image
from trellis.pipelines import TrellisImageTo3DPipeline
import trimesh
from pathlib import Path
import time

print("=" * 80)
print("BATCH TRELLIS CONVERSION: All Concept Art → 3D Models")
print("=" * 80)

# Find all generated images
input_dir = Path("/mnt/c/GIT/progship/progship-core/output/images_regenerated")
output_dir = Path("/root/models_batch")
output_dir.mkdir(exist_ok=True)

# Find all *_main.png images
image_files = []
for subdir in input_dir.iterdir():
    if subdir.is_dir():
        main_image = subdir / f"{subdir.name}_main.png"
        if main_image.exists():
            image_files.append((subdir.name, main_image))

print(f"\nFound {len(image_files)} images to convert:")
for component_id, path in image_files:
    size_mb = path.stat().st_size / (1024 * 1024)
    print(f"  - {component_id} ({size_mb:.1f} MB)")

# Load TRELLIS pipeline once
print("\n" + "=" * 80)
print("LOADING TRELLIS PIPELINE")
print("=" * 80)
pipeline = TrellisImageTo3DPipeline.from_pretrained("microsoft/TRELLIS-image-large")
pipeline.cuda()
print("[OK] Pipeline loaded on GPU\n")

# Process each image
results = []
start_time = time.time()

print("=" * 80)
print("CONVERTING IMAGES TO 3D")
print("=" * 80)

for i, (component_id, image_path) in enumerate(image_files, 1):
    print(f"\n[{i}/{len(image_files)}] {component_id}")
    print(f"  Input: {image_path.name}")
    
    try:
        # Load and convert
        image = Image.open(image_path)
        print(f"  Resolution: {image.size}")
        print(f"  Converting... (this takes ~8 seconds)")
        
        convert_start = time.time()
        outputs = pipeline.run(image, seed=42)
        convert_time = time.time() - convert_start
        
        # Extract mesh
        mesh = outputs['mesh'][0]
        vertices = mesh.vertices.cpu().numpy()
        faces = mesh.faces.cpu().numpy()
        
        # Create trimesh and export
        tmesh = trimesh.Trimesh(vertices=vertices, faces=faces)
        glb_path = output_dir / f"{component_id}.glb"
        tmesh.export(glb_path)
        
        # Check file
        if glb_path.exists():
            size_mb = glb_path.stat().st_size / (1024 * 1024)
            print(f"  [OK] {glb_path.name} ({size_mb:.2f} MB, {len(vertices):,} verts, {convert_time:.1f}s)")
            
            results.append({
                'component_id': component_id,
                'success': True,
                'glb_path': str(glb_path),
                'size_mb': size_mb,
                'vertices': len(vertices),
                'faces': len(faces),
                'time': convert_time
            })
        else:
            print(f"  [FAIL] GLB not created")
            results.append({'component_id': component_id, 'success': False})
            
    except Exception as e:
        print(f"  [ERROR] {type(e).__name__}: {e}")
        results.append({'component_id': component_id, 'success': False, 'error': str(e)})

total_time = time.time() - start_time

# Summary
print("\n" + "=" * 80)
print("BATCH CONVERSION COMPLETE")
print("=" * 80)

successful = [r for r in results if r.get('success')]
failed = [r for r in results if not r.get('success')]

print(f"\nTotal time: {total_time:.1f} seconds ({total_time/60:.1f} minutes)")
print(f"Processed: {len(image_files)} images")
print(f"Successful: {len(successful)}")
print(f"Failed: {len(failed)}")

if successful:
    print(f"\nSuccessful conversions:")
    for r in successful:
        print(f"  ✓ {r['component_id']}: {r['size_mb']:.1f} MB, {r['vertices']:,} verts, {r['time']:.1f}s")
    
    total_size = sum(r['size_mb'] for r in successful)
    avg_time = sum(r['time'] for r in successful) / len(successful)
    print(f"\nTotal size: {total_size:.1f} MB")
    print(f"Average conversion time: {avg_time:.1f}s per model")

if failed:
    print(f"\nFailed conversions:")
    for r in failed:
        error = r.get('error', 'Unknown error')
        print(f"  ✗ {r['component_id']}: {error}")

print("\n" + "=" * 80)
print("NEXT STEPS")
print("=" * 80)
print(f"1. Copy models to Windows:")
print(f"   wsl -d Ubuntu -- bash -c \"cp /root/models_batch/*.glb /mnt/c/GIT/progship/progship-core/output/models/\"")
print(f"2. View models in Godot/Blender/online GLB viewer")
print(f"3. Validate geometry and scale")
print(f"4. Build Phase 6: Asset bundle manifest")
