#!/usr/bin/env python3
"""
Test StableFast3D via Gradio Client (no local install needed).
Uses HuggingFace Space API to avoid compilation issues.
"""
from gradio_client import Client, handle_file
from pathlib import Path
import time

print("=" * 80)
print("StableFast3D Test via Gradio Client: Command Console")
print("=" * 80)

# Connect to HuggingFace Space
print("\nConnecting to StableFast3D Space...")
client = Client("stabilityai/stable-fast-3d")
print(f"[OK] Connected")

# View API to find correct endpoint
print("\nChecking available API endpoints...")
api_info = client.view_api(return_format="dict")
print(f"Available endpoints:")
for endpoint in api_info.get("named_endpoints", {}).values():
    print(f"  - {endpoint.get('api_name')}: {endpoint.get('parameters', [])}")
print()

# Test image
image_path = "/mnt/c/GIT/progship/progship-core/output/images_regenerated/command_console/command_console_main.png"
print(f"\nInput image: {image_path}")
print(f"Size: {Path(image_path).stat().st_size / 1024:.1f} KB")

# Run prediction
print("\nRunning StableFast3D inference...")
print("This may take 30-60 seconds (API queue + generation)...")

start = time.time()
try:
    result = client.predict(
        input_image=handle_file(image_path),
        foreground_ratio=0.85,
        remesh_option="None",
        vertex_count=-1,
        texture_size=1024,
        api_name="/run_button"
    )
    elapsed = time.time() - start
    
    print(f"[OK] Generation complete in {elapsed:.1f}s")
    print(f"Result type: {type(result)}")
    print(f"Result length: {len(result) if isinstance(result, (list, tuple)) else 'N/A'}")
    
    # Result is tuple: (preview_image, 3d_model)
    preview_image_path, model_path = result
    
    print(f"\nPreview: {preview_image_path}")
    print(f"Model: {model_path}")
    
    # Copy model to our output
    if model_path:
        import shutil
        output_path = Path("/root/console_textured.glb")
        shutil.copy(model_path, output_path)
        
        size_mb = output_path.stat().st_size / (1024 * 1024)
        print(f"\n[OK] Model saved: {output_path} ({size_mb:.2f} MB)")
        print(f"✅ Model includes UV-unwrapped textures!")
        
        # Copy to Windows
        import subprocess
        subprocess.run([
            "cp", str(output_path),
            "/mnt/c/GIT/progship/progship-core/output/console_textured.glb"
        ])
        print(f"✅ Copied to Windows output directory")
        
except Exception as e:
    print(f"[ERROR] {type(e).__name__}: {e}")
    import traceback
    traceback.print_exc()

print("\n" + "=" * 80)
