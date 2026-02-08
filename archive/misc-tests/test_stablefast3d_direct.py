#!/usr/bin/env python3
"""
Direct HF inference without local compilation
Uses huggingface_hub to download and cache the model
"""
from huggingface_hub import hf_hub_download, snapshot_download
import torch
from PIL import Image
import trimesh
import numpy as np
from pathlib import Path
import time

print("=" * 80)
print("StableFast3D via HF Hub: Command Console")
print("=" * 80)

# Download model files
print("\nDownloading model from HuggingFace...")
model_path = snapshot_download(
    repo_id="stabilityai/stable-fast-3d",
    allow_patterns=["*.safetensors", "*.json", "*.yaml"],
    cache_dir="/root/.cache/huggingface"
)
print(f"[OK] Model cached at: {model_path}")

# This won't work without the custom modules, but let's see what we get
print("\n⚠️  WARNING: This requires custom CUDA modules to run inference")
print("Attempting to load model structure...")

try:
    # Try importing the model code
    import sys
    sys.path.insert(0, "/root/stable-fast-3d")
    from sf3d.system import SF3D
    
    print("[OK] Module imported, loading weights...")
    model = SF3D.from_pretrained(model_path)
    print("[OK] Model loaded!")
    
except Exception as e:
    print(f"[ERROR] Cannot load model: {e}")
    print("\nThis confirms we need either:")
    print("1. Docker container with pre-built extensions")
    print("2. Manual compilation of CUDA modules")
    print("3. Alternative model (TripoSR, InstantMesh)")
    
print("\n" + "=" * 80)
