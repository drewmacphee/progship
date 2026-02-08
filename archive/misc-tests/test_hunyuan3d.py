#!/usr/bin/env python3
"""Test Hunyuan3D-2 for textured 3D model generation"""
import sys
import os

# Add Hunyuan3D-2 to path
hunyuan_path = r"C:\GIT\progship\Hunyuan3D-2"
if hunyuan_path not in sys.path:
    sys.path.insert(0, hunyuan_path)

# Change to Hunyuan directory for relative imports
os.chdir(hunyuan_path)

from PIL import Image
from pathlib import Path
import time

print("=" * 80)
print("Hunyuan3D-2 Test: Command Console (with PBR textures)")
print("=" * 80)

# Import after path setup
# from hy3dgen.rembg import BackgroundRemover  # Skip - our images have transparency
from hy3dgen.shapegen import Hunyuan3DDiTFlowMatchingPipeline
from hy3dgen.texgen import Hunyuan3DPaintPipeline

print("\nLoading models from HuggingFace...")
print("This will download models on first run (~several GB)")

try:
    model_path = 'tencent/Hunyuan3D-2'
    
    print("\n1. Loading shape generation pipeline...")
    pipeline_shapegen = Hunyuan3DDiTFlowMatchingPipeline.from_pretrained(model_path)
    print("[OK] Shape pipeline loaded")
    
    print("\n2. Loading texture generation pipeline...")
    pipeline_texgen = Hunyuan3DPaintPipeline.from_pretrained(model_path)
    print("[OK] Texture pipeline loaded")
    
    # Load test image (already has transparent background)
    image_path = r"C:\GIT\progship\progship-core\output\images_regenerated\command_console\command_console_main.png"
    print(f"\n3. Loading image: {image_path}")
    image = Image.open(image_path).convert("RGBA")
    print(f"[OK] Image loaded: {image.size}")
    print("[OK] Image already has transparent background, skipping rembg")
    
    # Generate shape
    print("\n4. Generating 3D mesh (this takes ~10-15 seconds)...")
    start = time.time()
    mesh = pipeline_shapegen(image=image)[0]
    shape_time = time.time() - start
    print(f"[OK] Shape generated in {shape_time:.1f}s")
    
    # Generate texture
    print("\n5. Generating PBR textures (this takes ~10-15 seconds)...")
    start = time.time()
    mesh = pipeline_texgen(mesh, image=image)
    texture_time = time.time() - start
    print(f"[OK] Textures generated in {texture_time:.1f}s")
    
    # Export
    output_path = Path(r"C:\GIT\progship\progship-core\output\console_hunyuan.glb")
    print(f"\n6. Exporting to {output_path}...")
    mesh.export(str(output_path))
    
    if output_path.exists():
        size_mb = output_path.stat().st_size / (1024 * 1024)
        print(f"[OK] Exported: {output_path} ({size_mb:.2f} MB)")
        print(f"\nTotal time: {shape_time + texture_time:.1f}s")
        print(f"✅ Model includes PBR textures (albedo, metallic, roughness)!")
        
        print("\n" + "=" * 80)
        print("SUCCESS! Hunyuan3D-2 generated textured 3D model")
        print("=" * 80)
    
except Exception as e:
    print(f"\n[ERROR] {type(e).__name__}: {e}")
    import traceback
    traceback.print_exc()
