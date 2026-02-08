"""FLUX.1-schnell image generator wrapper (Apache 2.0, fast variant)."""

from dataclasses import dataclass
from pathlib import Path
from typing import Optional, List
import torch
from PIL import Image
from datetime import datetime


@dataclass
class FluxSchnellConfig:
    """Configuration for FLUX.1-schnell image generation."""
    model_id: str = "black-forest-labs/FLUX.1-schnell"
    resolution: int = 1024
    num_inference_steps: int = 4  # Schnell is optimized for 1-4 steps
    guidance_scale: float = 0.0  # Schnell doesn't use CFG
    seed: Optional[int] = None


class FluxSchnellGenerator:
    """Wrapper for FLUX.1-schnell (Apache 2.0, fast variant)."""
    
    def __init__(self, config: Optional[FluxSchnellConfig] = None):
        """Initialize FLUX.1-schnell generator with config."""
        self.config = config or FluxSchnellConfig()
        self.pipeline = None
        self._model_loaded = False
        
    def _load_model(self):
        """Lazy load the FLUX.1-schnell model."""
        if self._model_loaded:
            return
            
        print(f"Loading FLUX.1-schnell model (Apache 2.0, 12B params)...")
        print("This may take a while on first run (downloading ~24GB model)...")
        
        from diffusers import FluxPipeline
        
        # Load pipeline
        self.pipeline = FluxPipeline.from_pretrained(
            self.config.model_id,
            torch_dtype=torch.bfloat16
        )
        
        # Enable CPU offload to save VRAM
        self.pipeline.enable_model_cpu_offload()
        
        # Enable memory-efficient attention
        self.pipeline.enable_attention_slicing()
        
        self._model_loaded = True
        print(f"[OK] FLUX.1-schnell loaded (4-step fast generation)")
    
    def generate(
        self,
        prompt: str,
        negative_prompt: Optional[str] = None,
        seed: Optional[int] = None,
        output_path: Optional[Path] = None
    ) -> dict:
        """
        Generate an image from text prompt.
        
        Args:
            prompt: Text description of desired image
            negative_prompt: Things to avoid (not used by schnell, kept for compatibility)
            seed: Random seed for reproducibility
            output_path: Where to save the image
            
        Returns:
            Dict with image path, seed, and generation metadata
        """
        self._load_model()
        
        # Set seed
        if seed is None:
            seed = self.config.seed or torch.randint(0, 2**32, (1,)).item()
        
        generator = torch.Generator("cuda" if torch.cuda.is_available() else "cpu")
        generator.manual_seed(seed)
        
        # Generate image
        result = self.pipeline(
            prompt=prompt,
            num_inference_steps=self.config.num_inference_steps,
            guidance_scale=self.config.guidance_scale,
            height=self.config.resolution,
            width=self.config.resolution,
            generator=generator,
        )
        
        image = result.images[0]
        
        # Save image
        if output_path:
            output_path = Path(output_path)
            output_path.parent.mkdir(parents=True, exist_ok=True)
            image.save(output_path, 'PNG')
        
        return {
            'image': image,
            'path': str(output_path) if output_path else None,
            'seed': seed,
            'prompt': prompt,
            'resolution': (self.config.resolution, self.config.resolution),
            'timestamp': datetime.now().isoformat()
        }
    
    def batch_generate(
        self,
        prompts: List[str],
        output_dir: Path,
        **kwargs
    ) -> List[dict]:
        """Generate multiple images from list of prompts."""
        results = []
        
        for i, prompt in enumerate(prompts):
            output_path = output_dir / f"image_{i:03d}.png"
            result = self.generate(prompt, output_path=output_path, **kwargs)
            results.append(result)
        
        return results

