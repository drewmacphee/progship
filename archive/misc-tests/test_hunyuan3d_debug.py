#!/usr/bin/env python3
"""Test Hunyuan3D-2 with extensive debugging"""
import sys
import os

# Add Hunyuan3D-2 to path
hunyuan_path = r"C:\GIT\progship\Hunyuan3D-2"
if hunyuan_path not in sys.path:
    sys.path.insert(0, hunyuan_path)

# Change to Hunyuan directory for relative imports
os.chdir(hunyuan_path)

import torch
from PIL import Image
from pathlib import Path
import time

print("=" * 80)
print("Hunyuan3D-2 DEBUG Test")
print("=" * 80)

# Check CUDA memory before starting
if torch.cuda.is_available():
    print(f"\nGPU: {torch.cuda.get_device_name(0)}")
    print(f"Total VRAM: {torch.cuda.get_device_properties(0).total_memory / 1024**3:.2f} GB")
    print(f"CUDA Version: {torch.version.cuda}")
    print(f"PyTorch Version: {torch.__version__}")
else:
    print("\n⚠ WARNING: CUDA not available!")

# Import pipelines
from hy3dgen.shapegen import Hunyuan3DDiTFlowMatchingPipeline
from hy3dgen.texgen import Hunyuan3DPaintPipeline

print("\n" + "-" * 80)
print("PHASE 1: Model Loading")
print("-" * 80)

model_path = 'tencent/Hunyuan3D-2'

print("\n1. Loading shape pipeline...")
start = time.time()
pipeline_shapegen = Hunyuan3DDiTFlowMatchingPipeline.from_pretrained(model_path)
print(f"   ✓ Loaded in {time.time() - start:.1f}s")
print(f"   Device: {pipeline_shapegen.device if hasattr(pipeline_shapegen, 'device') else 'unknown'}")

print("\n2. Loading texture pipeline...")
start = time.time()
pipeline_texgen = Hunyuan3DPaintPipeline.from_pretrained(model_path)
print(f"   ✓ Loaded in {time.time() - start:.1f}s")

# Check memory after loading
if torch.cuda.is_available():
    torch.cuda.synchronize()
    allocated = torch.cuda.memory_allocated(0) / 1024**3
    reserved = torch.cuda.memory_reserved(0) / 1024**3
    print(f"\n   GPU Memory after loading:")
    print(f"     Allocated: {allocated:.2f} GB")
    print(f"     Reserved: {reserved:.2f} GB")

print("\n" + "-" * 80)
print("PHASE 2: Image Loading")
print("-" * 80)

image_path = r"C:\GIT\progship\progship-core\output\images_regenerated\command_console\command_console_main.png"
print(f"\nLoading: {image_path}")
image = Image.open(image_path).convert("RGBA")
print(f"✓ Image size: {image.size}")
print(f"✓ Image mode: {image.mode}")

print("\n" + "-" * 80)
print("PHASE 3: Shape Generation")
print("-" * 80)

print("\nGenerating shape...")
start = time.time()
mesh = pipeline_shapegen(image=image)[0]
shape_time = time.time() - start
print(f"✓ Shape generated in {shape_time:.1f}s")
print(f"  Vertices: {len(mesh.vertices) if hasattr(mesh, 'vertices') else 'unknown'}")
print(f"  Faces: {len(mesh.faces) if hasattr(mesh, 'faces') else 'unknown'}")

# Check memory after shape generation
if torch.cuda.is_available():
    torch.cuda.synchronize()
    allocated = torch.cuda.memory_allocated(0) / 1024**3
    reserved = torch.cuda.memory_reserved(0) / 1024**3
    print(f"\n  GPU Memory after shape generation:")
    print(f"    Allocated: {allocated:.2f} GB")
    print(f"    Reserved: {reserved:.2f} GB")

print("\n" + "-" * 80)
print("PHASE 4: Texture Generation (DEBUG)")
print("-" * 80)

print("\n⚠ POTENTIAL HANG POINT - Monitoring closely...")
print(f"  Calling: pipeline_texgen(mesh, image=image)")
print(f"  Time: {time.strftime('%H:%M:%S')}")

# Try with explicit parameters
start = time.time()
try:
    print("\n  Starting texture generation...")
    mesh_textured = pipeline_texgen(mesh, image=image)
    texture_time = time.time() - start
    print(f"\n✓ Textures generated in {texture_time:.1f}s")
    
    # Check final memory
    if torch.cuda.is_available():
        torch.cuda.synchronize()
        allocated = torch.cuda.memory_allocated(0) / 1024**3
        reserved = torch.cuda.memory_reserved(0) / 1024**3
        print(f"\n  GPU Memory after texture generation:")
        print(f"    Allocated: {allocated:.2f} GB")
        print(f"    Reserved: {reserved:.2f} GB")
    
    # Export
    output_path = Path(r"C:\GIT\progship\progship-core\output\console_hunyuan_debug.glb")
    print(f"\n5. Exporting to {output_path}...")
    mesh_textured.export(str(output_path))
    
    if output_path.exists():
        size_mb = output_path.stat().st_size / (1024 * 1024)
        print(f"✓ Exported: {size_mb:.2f} MB")
        print(f"\nTotal time: {shape_time + texture_time:.1f}s")
        
        print("\n" + "=" * 80)
        print("✅ SUCCESS!")
        print("=" * 80)
    
except KeyboardInterrupt:
    print("\n\n⚠ INTERRUPTED by user")
    raise
except Exception as e:
    print(f"\n\n❌ ERROR at {time.time() - start:.1f}s into texture generation")
    print(f"   {type(e).__name__}: {e}")
    import traceback
    traceback.print_exc()
    
    # Check if OOM
    if torch.cuda.is_available():
        try:
            allocated = torch.cuda.memory_allocated(0) / 1024**3
            reserved = torch.cuda.memory_reserved(0) / 1024**3
            print(f"\n   GPU Memory at error:")
            print(f"     Allocated: {allocated:.2f} GB")
            print(f"     Reserved: {reserved:.2f} GB")
        except:
            pass
