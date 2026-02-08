"""Fast FLUX.1-schnell configuration for RTX 4090."""

import os
os.environ['HF_TOKEN'] = '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'

import torch
from diffusers import FluxPipeline
from pathlib import Path
import time

print("FLUX.1-schnell - Fast Configuration")
print("="*80)

prompt = "A futuristic spacecraft bridge, ceramic white, minimalist design"

print("Loading model...")
start = time.time()

# Load with float16 for speed
pipe = FluxPipeline.from_pretrained(
    "black-forest-labs/FLUX.1-schnell",
    torch_dtype=torch.float16
).to("cuda")

load_time = time.time() - start
print(f"Loaded in {load_time:.1f}s")
print(f"VRAM: {torch.cuda.memory_allocated() / 1024**3:.1f} GB")

print("\nGenerating 1024x1024 image...")
gen_start = time.time()

image = pipe(
    prompt,
    num_inference_steps=4,
    guidance_scale=0.0,
    height=1024,
    width=1024,
    generator=torch.Generator("cuda").manual_seed(42)
).images[0]

gen_time = time.time() - gen_start

output_path = Path("test_output/flux_fast.png")
image.save(output_path)

print(f"\n[SUCCESS] Generated in {gen_time:.1f}s")
print(f"  Total: {time.time() - start:.1f}s")
print(f"  Peak VRAM: {torch.cuda.max_memory_allocated() / 1024**3:.1f} GB")
print(f"  Output: {output_path}")
