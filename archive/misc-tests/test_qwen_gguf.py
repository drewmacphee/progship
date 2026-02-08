#!/usr/bin/env python3
"""Test Qwen-Image-2512 GGUF 8-bit with stable-diffusion-cpp-python"""
from stable_diffusion_cpp import StableDiffusion
from huggingface_hub import hf_hub_download
import time
import os

print("=" * 80)
print("Downloading Qwen-Image-2512 GGUF Components (3 files)")
print("=" * 80)

# Create models directory structure
os.makedirs("models/diffusion", exist_ok=True)
os.makedirs("models/text_encoders", exist_ok=True)
os.makedirs("models/vae", exist_ok=True)

print("\n[1/3] Downloading diffusion model (Q8_0, ~21GB)...")
diffusion_path = hf_hub_download(
    repo_id="Civitai/Qwen-Image-2512-GGUF",
    filename="qwen_image_2512_q8_0.gguf",
    cache_dir="models"
)
print(f"[OK] Diffusion model: {diffusion_path}")

print("\n[2/3] Downloading text encoder (Q4_K_XL, ~4GB)...")
text_encoder_path = hf_hub_download(
    repo_id="unsloth/Qwen2.5-VL-7B-Instruct-GGUF",
    filename="Qwen2.5-VL-7B-Instruct-UD-Q4_K_XL.gguf",
    cache_dir="models"
)
print(f"[OK] Text encoder: {text_encoder_path}")

print("\n[3/3] Downloading VAE (safetensors, ~335MB)...")
vae_path = hf_hub_download(
    repo_id="Comfy-Org/Qwen-Image_ComfyUI",
    filename="split_files/vae/qwen_image_vae.safetensors",
    cache_dir="models"
)
print(f"[OK] VAE: {vae_path}")

print(f"\n{'=' * 80}")
print("Loading model into stable-diffusion.cpp...")
print(f"{'=' * 80}")

# Initialize SD with all 3 components
sd = StableDiffusion(
    model_path=diffusion_path,
    clip_l_path=text_encoder_path,  # Text encoder
    vae_path=vae_path,                # VAE decoder
    wtype="q8_0",                     # 8-bit quantization
)

print("\n[OK] Model loaded with all components!")

# Test prompt
prompt = "A futuristic command console with ceramic white surfaces, glowing blue holographic displays, smooth rounded edges, isolated on white background, product photograph, studio lighting, 8K resolution."

negative_prompt = "blurry, unfocused, low quality, distorted, text errors, people, humans"

print(f"\n{'=' * 80}")
print("Generating image (20 steps, 1024x1024)...")
print(f"{'=' * 80}")

start = time.time()

# Generate with all parameters
output = sd.generate_image(
    prompt=prompt,
    negative_prompt=negative_prompt,
    sample_steps=20,
    width=1024,
    height=1024,
    seed=42,
    cfg_scale=7.5,
)

elapsed = time.time() - start

# Save
os.makedirs("progship-core/output/images_qwen_test", exist_ok=True)
output_path = "progship-core/output/images_qwen_test/command_console_gguf_q8.png"
output.save(output_path)

print(f"\n[OK] Generated in {elapsed:.1f}s ({elapsed/20:.1f}s per step)")
print(f"[OK] Saved to: {output_path}")
print(f"\n{'=' * 80}")
print("✅ SUCCESS! GGUF Q8_0 test complete")
print(f"{'=' * 80}")
