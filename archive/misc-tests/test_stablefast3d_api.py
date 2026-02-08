#!/usr/bin/env python3
"""
Test StableFast3D via Hugging Face Inference API (no local install needed).
Alternative to local compilation issues.
"""
from huggingface_hub import InferenceClient
from PIL import Image
from pathlib import Path
import time

print("=" * 80)
print("StableFast3D Test via HuggingFace API: Command Console")
print("=" * 80)

# Initialize client
client = InferenceClient(token="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")

# Load test image
image_path = Path("/mnt/c/GIT/progship/progship-core/output/images_regenerated/command_console/command_console_main.png")
image = Image.open(image_path)
print(f"\nInput image: {image_path.name}")
print(f"Resolution: {image.size}")

# Call API
print("\nCalling StableFast3D API...")
print("This may take 30-60 seconds...")

start = time.time()
try:
    # Use the image-to-3d task
    result = client.image_to_3d(
        image=image,
        model="stabilityai/stable-fast-3d"
    )
    elapsed = time.time() - start
    
    print(f"[OK] Conversion complete in {elapsed:.1f}s")
    
    # Save result
    output_path = Path("/root/console_api_test.glb")
    with open(output_path, 'wb') as f:
        f.write(result)
    
    size_mb = output_path.stat().st_size / (1024 * 1024)
    print(f"[OK] Saved: {output_path} ({size_mb:.2f} MB)")
    print(f"\nCopy to Windows:")
    print(f"  cp {output_path} /mnt/c/GIT/progship/progship-core/output/console_textured.glb")
    
except Exception as e:
    print(f"[ERROR] {type(e).__name__}: {e}")
    print("\nNote: API may not support image-to-3d task yet.")
    print("Trying local installation approach...")
