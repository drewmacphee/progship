"""Optimized FLUX.1-schnell test with diagnostics."""

import os
os.environ['HF_TOKEN'] = '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'

import torch
from diffusers import FluxPipeline
from pathlib import Path
import time

print("FLUX.1-schnell Diagnostics")
print("="*80)
print(f"PyTorch: {torch.__version__}")
print(f"CUDA available: {torch.cuda.is_available()}")
if torch.cuda.is_available():
    print(f"Device: {torch.cuda.get_device_name(0)}")
    print(f"VRAM total: {torch.cuda.get_device_properties(0).total_memory / 1024**3:.1f} GB")

prompt = "A futuristic spacecraft bridge, ceramic white, minimalist"

print("\nLoading model...")
start = time.time()

pipe = FluxPipeline.from_pretrained(
    "black-forest-labs/FLUX.1-schnell",
    torch_dtype=torch.float16  # Changed from bfloat16
)

# Enable optimizations
pipe.enable_model_cpu_offload()  # Offload unused parts to CPU
pipe.enable_vae_tiling()  # Reduce VRAM usage
pipe.to("cuda")

load_time = time.time() - start
print(f"Loaded in {load_time:.1f}s")

# Check VRAM usage
if torch.cuda.is_available():
    print(f"VRAM allocated: {torch.cuda.memory_allocated() / 1024**3:.1f} GB")
    print(f"VRAM reserved: {torch.cuda.memory_reserved() / 1024**3:.1f} GB")

print("\nGenerating...")
gen_start = time.time()

image = pipe(
    prompt,
    num_inference_steps=4,
    guidance_scale=0.0,
    height=512,  # Smaller size for speed test
    width=512,
    generator=torch.Generator("cuda").manual_seed(42)
).images[0]

gen_time = time.time() - gen_start

output_path = Path("test_output/flux_optimized.png")
image.save(output_path)

print(f"\n[SUCCESS] Generated in {gen_time:.1f}s")
print(f"  Output: {output_path}")

if torch.cuda.is_available():
    print(f"  Peak VRAM: {torch.cuda.max_memory_allocated() / 1024**3:.1f} GB")
