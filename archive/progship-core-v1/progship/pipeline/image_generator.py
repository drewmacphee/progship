"""
Image generation using Flux.2 [klein] 9B model.
Converts text descriptions into concept art images.
"""

from dataclasses import dataclass
from pathlib import Path
from typing import Optional, List, Dict, Any
import json
from datetime import datetime
import torch


@dataclass
class ImageConfig:
    """Configuration for image generation."""
    model_id: str = "unsloth/Qwen-Image-2512-unsloth-bnb-4bit"  # Quantized for 24GB VRAM
    resolution: int = 1024
    aspect_ratio: str = "1:1"  # 1:1, 16:9, 9:16, 4:3, 3:4, 3:2, 2:3
    num_inference_steps: int = 20  # Balanced quality/speed (vs 50 for full precision)
    guidance_scale: float = 7.5  # Standard CFG guidance
    true_cfg_scale: float = 4.0  # Qwen-specific true CFG
    seed: Optional[int] = None
    device: str = "cuda" if torch.cuda.is_available() else "cpu"
    dtype: str = "float16"  # Use fp16 for quantized model


class QwenImageGenerator:
    """Qwen-Image-2512: State-of-the-art text-to-image (Dec 2025). 4-bit quantized for RTX 4090."""
    
    # Qwen aspect ratio presets (resolution optimized)
    ASPECT_RATIOS = {
        "1:1": (1024, 1024),    # Practical middle ground (vs 1328 native +50% slower)
        "16:9": (1664, 928),
        "9:16": (928, 1664),
        "4:3": (1472, 1104),
        "3:4": (1104, 1472),
        "3:2": (1584, 1056),
        "2:3": (1056, 1584),
    }
    
    def __init__(self, config: Optional[ImageConfig] = None):
        self.config = config or ImageConfig()
        self.pipeline = None
        self._model_loaded = False
        
    def _load_model(self):
        """Lazy load the model to avoid startup delays."""
        if self._model_loaded:
            return
            
        print(f"Loading Qwen-Image-2512-4bit (state-of-the-art Dec 2025, 4-bit quantized) from {self.config.model_id}...")
        print("This may take a few minutes on first run (downloading ~4-6GB model)...")
        
        from diffusers import DiffusionPipeline
        
        # Convert dtype string to torch dtype
        dtype = torch.bfloat16 if self.config.dtype == "bfloat16" else torch.float16
        
        # Load 4-bit quantized pipeline (no CPU offload needed!)
        self.pipeline = DiffusionPipeline.from_pretrained(
            self.config.model_id,
            torch_dtype=dtype,
        )
        self.pipeline = self.pipeline.to(self.config.device)
        
        self._model_loaded = True
        print(f"[OK] Qwen-Image-2512-4bit loaded on {self.config.device}")
        print(f"[OK] Using 4-bit quantization for 24GB VRAM compatibility")
        print(f"[OK] Using {dtype} precision for optimal quality")
    
    def generate(
        self, 
        prompt: str, 
        negative_prompt: Optional[str] = None,
        seed: Optional[int] = None,
        aspect_ratio: Optional[str] = None,
        **kwargs
    ) -> Dict[str, Any]:
        """
        Generate a single image from a text prompt.
        
        Args:
            prompt: Text description to generate image from
            negative_prompt: Things to avoid in the image
            seed: Random seed for reproducibility
            aspect_ratio: Aspect ratio preset (1:1, 16:9, etc.) - overrides resolution
            **kwargs: Additional parameters to pass to pipeline
            
        Returns:
            Dict with 'image' (PIL Image), 'prompt', 'seed', 'metadata'
        """
        self._load_model()
        
        # Resolve dimensions (aspect ratio overrides resolution)
        ar = aspect_ratio or self.config.aspect_ratio
        if ar in self.ASPECT_RATIOS:
            width, height = self.ASPECT_RATIOS[ar]
        else:
            width = height = self.config.resolution
        
        # Use provided seed or config seed
        actual_seed = seed if seed is not None else self.config.seed
        generator = None
        if actual_seed is not None:
            generator = torch.Generator(device=self.config.device).manual_seed(actual_seed)
        
        # Generate image with Qwen-specific parameters
        result = self.pipeline(
            prompt=prompt,
            negative_prompt=negative_prompt,
            width=width,
            height=height,
            num_inference_steps=self.config.num_inference_steps,
            guidance_scale=self.config.guidance_scale,
            true_cfg_scale=self.config.true_cfg_scale,  # Qwen-specific
            generator=generator,
            **kwargs
        )
        
        return {
            "image": result.images[0],
            "prompt": prompt,
            "negative_prompt": negative_prompt,
            "seed": actual_seed,
            "metadata": {
                "model": self.config.model_id,
                "resolution": f"{width}x{height}",
                "aspect_ratio": ar,
                "steps": self.config.num_inference_steps,
                "guidance_scale": self.config.guidance_scale,
                "true_cfg_scale": self.config.true_cfg_scale,
                "timestamp": datetime.now().isoformat(),
            }
        }
    
    def batch_generate(
        self, 
        prompts: List[str], 
        negative_prompt: Optional[str] = None,
        seed: Optional[int] = None,
        **kwargs
    ) -> List[Dict[str, Any]]:
        """
        Generate multiple images from a list of prompts.
        Note: This processes prompts sequentially, not in parallel batches.
        
        Args:
            prompts: List of text descriptions
            negative_prompt: Things to avoid (applied to all)
            seed: Base random seed (incremented for each prompt)
            **kwargs: Additional parameters
            
        Returns:
            List of generation results
        """
        results = []
        for i, prompt in enumerate(prompts):
            # Increment seed for variation
            prompt_seed = (seed + i) if seed is not None else None
            
            print(f"Generating image {i+1}/{len(prompts)}: {prompt[:60]}...")
            result = self.generate(prompt, negative_prompt, prompt_seed, **kwargs)
            results.append(result)
            
        return results
    
    def save_image(
        self, 
        result: Dict[str, Any], 
        output_path: Path,
        save_metadata: bool = True
    ):
        """
        Save generated image to disk with optional metadata.
        
        Args:
            result: Generation result from generate()
            output_path: Path to save image (should end in .png)
            save_metadata: Whether to save metadata JSON alongside image
        """
        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        # Save image
        result["image"].save(output_path)
        
        # Save metadata if requested
        if save_metadata:
            metadata_path = output_path.with_suffix(".json")
            metadata = {
                "prompt": result["prompt"],
                "negative_prompt": result["negative_prompt"],
                "seed": result["seed"],
                **result["metadata"]
            }
            with open(metadata_path, "w") as f:
                json.dump(metadata, f, indent=2)
        
        print(f"[OK] Saved image: {output_path}")

