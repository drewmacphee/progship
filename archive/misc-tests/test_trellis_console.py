#!/usr/bin/env python3
"""Test TRELLIS image-to-3D conversion"""
import os
os.environ["ATTN_BACKEND"] = "xformers"
os.environ["SPCONV_ALGO"] = "native"

import sys
sys.path.insert(0, "/root/TRELLIS")

from PIL import Image
from trellis.pipelines import TrellisImageTo3DPipeline
import trimesh
import numpy as np

print("=" * 70)
print("TRELLIS Image-to-3D Test: Command Console")
print("=" * 70)

# Load image
image_path = "/root/console_test.png"
image = Image.open(image_path)
print(f"Input image: {image_path}")
print(f"Resolution: {image.size}")

# Load pipeline
print("\nLoading TRELLIS pipeline...")
pipeline = TrellisImageTo3DPipeline.from_pretrained("microsoft/TRELLIS-image-large")
pipeline.cuda()
print("[OK] Pipeline loaded on GPU")

# Run image-to-3D
print("\nRunning image-to-3D conversion...")
print("This may take 2-5 minutes...")

outputs = pipeline.run(
    image,
    seed=42,
)

print(f"\n[OK] Conversion complete!")
print(f"Output type: {type(outputs)}")

# Export to GLB
print("\nExporting to GLB format...")
print(f"Output keys: {outputs.keys()}")

# Get the mesh representation
glb_path = "/root/console_test.glb"

# TRELLIS returns dict with different representations
# Try to export the mesh
if 'mesh' in outputs:
    meshes = outputs['mesh']
    print(f"Mesh type: {type(meshes)}, length: {len(meshes) if isinstance(meshes, list) else 'N/A'}")
    
    # If it's a list, export each mesh
    if isinstance(meshes, list):
        for i, mesh in enumerate(meshes):
            mesh_path = glb_path.replace('.glb', f'_{i}.glb')
            print(f"Exporting mesh {i}...")
            print(f"  Vertices shape: {mesh.vertices.shape if hasattr(mesh, 'vertices') else 'N/A'}")
            print(f"  Faces shape: {mesh.faces.shape if hasattr(mesh, 'faces') else 'N/A'}")
            
            # Extract vertices and faces
            vertices = mesh.vertices.cpu().numpy()
            faces = mesh.faces.cpu().numpy()
            
            # Create trimesh object
            print(f"  Creating trimesh from {len(vertices)} vertices and {len(faces)} faces...")
            tmesh = trimesh.Trimesh(vertices=vertices, faces=faces)
            
            # Export to GLB
            print(f"  Exporting to {mesh_path}...")
            tmesh.export(mesh_path)
            
            if os.path.exists(mesh_path):
                size_mb = os.path.getsize(mesh_path) / (1024 * 1024)
                print(f"  [OK] {mesh_path} ({size_mb:.2f} MB)")
    else:
        meshes.export(glb_path)
        
        if os.path.exists(glb_path):
            size_mb = os.path.getsize(glb_path) / (1024 * 1024)
            print(f"[OK] Exported: {glb_path} ({size_mb:.2f} MB)")

print("\n" + "=" * 70)
print("SUCCESS! 3D model(s) generated from 2D concept art")
print("=" * 70)
print("Copy back to Windows:")
for i in range(len(outputs.get('mesh', []))):
    print(f"  cp /root/console_test_{i}.glb /mnt/c/GIT/progship/progship-core/output/")
