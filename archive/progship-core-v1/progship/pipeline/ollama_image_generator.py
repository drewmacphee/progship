"""Ollama image generator for FLUX.2 klein models."""

from dataclasses import dataclass
from pathlib import Path
from typing import Optional, List
import base64
import io
from PIL import Image
from datetime import datetime
import requests
import json


@dataclass
class OllamaImageConfig:
    """Configuration for Ollama image generation."""
    model: str = "erwan2/DeepSeek-Janus-Pro-7B"  # Changed to Janus
    api_base: str = "http://localhost:11434"
    seed: Optional[int] = None


class OllamaImageGenerator:
    """
    Wrapper for Ollama-hosted image generation models.
    Uses FLUX.2 klein via Ollama's unified API.
    """
    
    def __init__(self, config: Optional[OllamaImageConfig] = None):
        """Initialize Ollama image generator with config."""
        self.config = config or OllamaImageConfig()
        self._model_checked = False
        
    def _check_model_available(self):
        """Check if the model is available in Ollama."""
        if self._model_checked:
            return
            
        try:
            response = requests.get(f"{self.config.api_base}/api/tags")
            response.raise_for_status()
            models = response.json()
            
            available = any(
                m['name'] == self.config.model 
                for m in models.get('models', [])
            )
            
            if not available:
                raise RuntimeError(
                    f"Model {self.config.model} not found in Ollama.\n"
                    f"Please pull it first:\n"
                    f"  ollama pull {self.config.model}\n\n"
                    f"Available models: {[m['name'] for m in models.get('models', [])]}"
                )
            
            self._model_checked = True
            print(f"[OK] Ollama model {self.config.model} ready")
            
        except requests.RequestException as e:
            raise RuntimeError(
                f"Cannot connect to Ollama API at {self.config.api_base}.\n"
                f"Make sure Ollama is running: ollama serve\n"
                f"Error: {e}"
            )
    
    def generate(
        self,
        prompt: str,
        negative_prompt: Optional[str] = None,
        seed: Optional[int] = None,
        output_path: Optional[Path] = None,
        upscale: bool = False,
        target_resolution: int = 1024
    ) -> dict:
        """
        Generate an image from text prompt using Ollama.
        
        Args:
            prompt: Text description of desired image
            negative_prompt: Not supported by FLUX.2 (kept for compatibility)
            seed: Random seed for reproducibility (if model supports it)
            output_path: Where to save the image
            upscale: Whether to upscale (FLUX.2 klein native res varies)
            target_resolution: Target size if upscaling
            
        Returns:
            Dict with image path, seed, and generation metadata
        """
        self._check_model_available()
        
        # Build the generation prompt
        # For Ollama image gen models, we send text and get back image data
        generation_prompt = prompt
        if self.config.seed or seed:
            # Some models may support seed via prompt engineering
            generation_prompt = f"[seed:{seed or self.config.seed}] {prompt}"
        
        # Call Ollama API for image generation
        try:
            response = requests.post(
                f"{self.config.api_base}/api/generate",
                json={
                    "model": self.config.model,
                    "prompt": generation_prompt,
                    "stream": False
                },
                timeout=300  # 5 minutes max
            )
            response.raise_for_status()
            result = response.json()
            
            # Extract image from response
            # Ollama returns images as base64 in the response
            if 'images' in result and result['images']:
                image_b64 = result['images'][0]
                image_data = base64.b64decode(image_b64)
                image = Image.open(io.BytesIO(image_data))
            elif 'response' in result:
                # Some models may return base64 in the text response
                # Try to extract it
                response_text = result['response']
                if response_text.startswith('data:image'):
                    # Data URL format
                    image_b64 = response_text.split(',')[1]
                    image_data = base64.b64decode(image_b64)
                    image = Image.open(io.BytesIO(image_data))
                else:
                    raise RuntimeError(f"No image data in response. Got: {response_text[:200]}")
            else:
                raise RuntimeError(f"Unexpected response format: {result}")
            
            # Upscale if requested
            original_size = image.size
            if upscale and max(image.size) < target_resolution:
                image = image.resize(
                    (target_resolution, target_resolution),
                    Image.Resampling.LANCZOS
                )
            
            # Save image
            if output_path:
                output_path = Path(output_path)
                output_path.parent.mkdir(parents=True, exist_ok=True)
                image.save(output_path, 'PNG')
            
            return {
                'image': image,
                'path': str(output_path) if output_path else None,
                'seed': seed or self.config.seed,
                'prompt': prompt,
                'resolution': image.size,
                'native_resolution': original_size,
                'timestamp': datetime.now().isoformat(),
                'model': self.config.model
            }
            
        except requests.RequestException as e:
            raise RuntimeError(f"Ollama API error: {e}")
        except Exception as e:
            raise RuntimeError(f"Image generation failed: {e}")
    
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
