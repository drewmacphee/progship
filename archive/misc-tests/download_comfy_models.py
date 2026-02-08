#!/usr/bin/env python3
"""Download Qwen-Image-2512 GGUF files to ComfyUI directories"""
from huggingface_hub import hf_hub_download
import shutil
from pathlib import Path

comfy_root = Path(r"C:\Users\drewj\Documents\ComfyUI")

print("=" * 80)
print("Downloading Qwen-Image-2512 GGUF Components for ComfyUI")
print("=" * 80)

# Download smaller Q4_K_M for faster setup (6GB vs 21GB)
print("\n[1/3] Downloading diffusion model (Q4_K_M, ~6GB)...")
diffusion_file = hf_hub_download(
    repo_id="unsloth/Qwen-Image-2512-GGUF",
    filename="qwen-image-2512-Q4_K_M.gguf",
)
diffusion_dest = comfy_root / "models" / "unet" / "qwen-image-2512-Q4_K_M.gguf"
if not diffusion_dest.exists():
    shutil.copy2(diffusion_file, diffusion_dest)
print(f"[OK] {diffusion_dest}")

print("\n[2/3] Downloading text encoder (Q4_K_XL, ~4GB)...")
text_encoder_file = hf_hub_download(
    repo_id="unsloth/Qwen2.5-VL-7B-Instruct-GGUF",
    filename="Qwen2.5-VL-7B-Instruct-UD-Q4_K_XL.gguf",
)
text_encoder_dest = comfy_root / "models" / "text_encoders" / "Qwen2.5-VL-7B-Instruct-UD-Q4_K_XL.gguf"
if not text_encoder_dest.exists():
    shutil.copy2(text_encoder_file, text_encoder_dest)
print(f"[OK] {text_encoder_dest}")

print("\n[3/3] Downloading VAE (safetensors, ~335MB)...")
vae_file = hf_hub_download(
    repo_id="Comfy-Org/Qwen-Image_ComfyUI",
    filename="split_files/vae/qwen_image_vae.safetensors",
)
vae_dest = comfy_root / "models" / "vae" / "qwen_image_vae.safetensors"
if not vae_dest.exists():
    shutil.copy2(vae_file, vae_dest)
print(f"[OK] {vae_dest}")

print(f"\n{'=' * 80}")
print("✅ All files downloaded to ComfyUI!")
print(f"{'=' * 80}")
print("\nNext: Load ComfyUI workflow")
print("  1. Start ComfyUI")
print("  2. Drag qwen_workflow.json into browser")
print("  3. Click 'Queue Prompt'")
