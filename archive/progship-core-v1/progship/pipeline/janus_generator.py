"""Janus-Pro-7B image generator wrapper."""

from dataclasses import dataclass
from pathlib import Path
from typing import Optional, List
import torch
from PIL import Image
from datetime import datetime


@dataclass
class JanusConfig:
    """Configuration for Janus-Pro-7B image generation."""
    model_id: str = "deepseek-ai/Janus-Pro-7B"
    resolution: int = 384  # Max native resolution
    temperature: float = 1.0
    cfg_weight: float = 5.0  # Classifier-free guidance
    seed: Optional[int] = None


class JanusImageGenerator:
    """Wrapper for DeepSeek Janus-Pro-7B text-to-image generation."""
    
    def __init__(self, config: Optional[JanusConfig] = None):
        """Initialize Janus generator with config."""
        self.config = config or JanusConfig()
        self.model = None
        self.processor = None
        self.device = "cuda" if torch.cuda.is_available() else "cpu"
        
    def _load_model(self):
        """Lazy load the Janus model."""
        if self.model is not None:
            return
            
        print(f"Loading Janus-Pro-7B model (first time may take a while)...")
        
        try:
            from transformers import AutoModelForCausalLM
            from janus.models import VLChatProcessor, MultiModalityCausalLM
            
            # Load processor and model
            self.processor = VLChatProcessor.from_pretrained(
                self.config.model_id,
                trust_remote_code=True
            )
            
            self.model = AutoModelForCausalLM.from_pretrained(
                self.config.model_id,
                trust_remote_code=True,
                torch_dtype=torch.bfloat16 if torch.cuda.is_available() else torch.float32
            )
            
            self.model.to(self.device).eval()
            print(f"✓ Janus-Pro-7B loaded on {self.device}")
            
        except ImportError:
            raise RuntimeError(
                "Janus model not installed. Please install:\n"
                "  git clone https://github.com/deepseek-ai/Janus.git\n"
                "  cd Janus\n"
                "  pip install -e .\n"
            )
    
    def generate(
        self,
        prompt: str,
        negative_prompt: Optional[str] = None,
        seed: Optional[int] = None,
        output_path: Optional[Path] = None,
        upscale: bool = True,
        target_resolution: int = 1024
    ) -> dict:
        """
        Generate an image from text prompt.
        
        Args:
            prompt: Text description of desired image
            negative_prompt: Things to avoid (ignored by Janus, kept for compatibility)
            seed: Random seed for reproducibility
            output_path: Where to save the image
            upscale: Whether to upscale from 384px to target_resolution
            target_resolution: Target size if upscaling (default 1024)
            
        Returns:
            Dict with image path, seed, and generation metadata
        """
        self._load_model()
        
        # Set seed
        if seed is None:
            seed = self.config.seed or torch.randint(0, 2**32, (1,)).item()
        
        torch.manual_seed(seed)
        if torch.cuda.is_available():
            torch.cuda.manual_seed_all(seed)
        
        # Generate image using Janus
        conversation = [
            {
                "role": "User",
                "content": prompt,
            },
            {"role": "Assistant", "content": ""},
        ]
        
        # Prepare inputs
        prepare_inputs = self.processor(
            conversations=conversation,
            images=None,
            force_batchify=True
        ).to(self.device, dtype=torch.bfloat16 if torch.cuda.is_available() else torch.float32)
        
        # Generate with model
        with torch.no_grad():
            generation_output = self.model.generate(
                **prepare_inputs,
                do_sample=True,
                temperature=self.config.temperature,
                max_new_tokens=576,  # For 384x384 image tokens
            )
        
        # Decode image
        images = self.processor.decode_image_tokens(
            generation_output,
            img_size=self.config.resolution
        )
        
        if not images:
            raise RuntimeError("Image generation failed")
        
        image = images[0]  # Get first generated image
        
        # Upscale if requested
        if upscale and target_resolution > self.config.resolution:
            from PIL import Image as PILImage
            image = image.resize(
                (target_resolution, target_resolution),
                PILImage.Resampling.LANCZOS
            )
        
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
            'resolution': image.size,
            'native_resolution': self.config.resolution,
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
