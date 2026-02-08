"""
Model registry for text-to-image generation.
Makes it easy to add and swap image generation models.
"""

from dataclasses import dataclass
from enum import Enum
from typing import Optional, Protocol, List, Dict, Any
from pathlib import Path
from PIL import Image


@dataclass
class ModelInfo:
    """Metadata about an image generation model."""
    model_id: str
    name: str
    license: str
    native_resolution: int
    max_resolution: int
    supports_negative_prompts: bool
    quality_tier: str  # "fast", "balanced", "quality"
    vram_required: int  # GB
    description: str


class ImageGeneratorProtocol(Protocol):
    """Protocol that all image generators must implement."""
    
    def generate(
        self,
        prompt: str,
        negative_prompt: Optional[str] = None,
        seed: Optional[int] = None,
        output_path: Optional[Path] = None,
        **kwargs
    ) -> dict:
        """Generate an image from text prompt."""
        ...
    
    def batch_generate(
        self,
        prompts: List[str],
        output_dir: Path,
        **kwargs
    ) -> List[dict]:
        """Generate multiple images from list of prompts."""
        ...


class ModelType(str, Enum):
    """Available image generation models."""
    OLLAMA_FLUX2 = "ollama_flux2"
    OLLAMA_JANUS = "ollama_janus"  # FLUX.2 klein via Ollama (RECOMMENDED)
    SEGMIND_VEGA = "segmind_vega"  # Current default
    JANUS_PRO = "janus_pro"  # Better prompt fidelity
    FLUX_SCHNELL = "flux_schnell"  # Fast, Apache 2.0


# Model registry with metadata
MODEL_REGISTRY: Dict[str, ModelInfo] = {
    ModelType.OLLAMA_JANUS: ModelInfo(
        model_id="erwan2/DeepSeek-Janus-Pro-7B",
        name="DeepSeek Janus-Pro-7B (Ollama)",
        license="MIT",
        native_resolution=384,
        max_resolution=2048,
        supports_negative_prompts=False,
        quality_tier="premium",
        vram_required=16,
        description="DeepSeek Janus-Pro via Ollama, best prompt fidelity (80% GenEval), MIT license"
    ),
    
    ModelType.OLLAMA_FLUX2: ModelInfo(
        model_id="x/flux2-klein:4b",
        name="FLUX.2 klein (Ollama)",
        license="Apache 2.0",
        native_resolution=1024,  # FLUX.2 klein native resolution
        max_resolution=2048,  # Via upscaling
        supports_negative_prompts=False,
        quality_tier="quality",
        vram_required=13,
        description="FLUX.2 klein 4B via Ollama, excellent text rendering, <1s generation"
    ),
    
    ModelType.SEGMIND_VEGA: ModelInfo(
        model_id="segmind/Segmind-Vega",
        name="Segmind-Vega",
        license="Apache 2.0",
        native_resolution=1024,
        max_resolution=1024,
        supports_negative_prompts=True,
        quality_tier="fast",
        vram_required=4,
        description="SDXL-based, 1.5B params, fast generation (~1.5s/image)"
    ),
    
    ModelType.JANUS_PRO: ModelInfo(
        model_id="deepseek-ai/Janus-Pro-7B",
        name="Janus-Pro-7B",
        license="MIT (code) + DeepSeek Model License (weights)",
        native_resolution=384,
        max_resolution=2048,  # Via upscaling
        supports_negative_prompts=False,
        quality_tier="quality",
        vram_required=16,
        description="DeepSeek multimodal, 7B params, excellent prompt fidelity"
    ),
    
    ModelType.FLUX_SCHNELL: ModelInfo(
        model_id="black-forest-labs/FLUX.1-schnell",
        name="FLUX.1-schnell",
        license="Apache 2.0",
        native_resolution=1024,
        max_resolution=1024,
        supports_negative_prompts=True,
        quality_tier="balanced",
        vram_required=8,
        description="Fast variant of FLUX.1, 12B params, 4-step generation"
    ),
}


def get_model_info(model_type: str) -> ModelInfo:
    """Get metadata for a model type."""
    if model_type not in MODEL_REGISTRY:
        raise ValueError(
            f"Unknown model type: {model_type}. "
            f"Available: {', '.join(MODEL_REGISTRY.keys())}"
        )
    return MODEL_REGISTRY[model_type]


def list_available_models() -> List[tuple[str, ModelInfo]]:
    """List all available models with their metadata."""
    return [(k, v) for k, v in MODEL_REGISTRY.items()]


def create_generator(
    model_type: str = ModelType.SEGMIND_VEGA,
    **config_overrides
) -> ImageGeneratorProtocol:
    """
    Factory function to create an image generator.
    
    Args:
        model_type: Type of model to use (see ModelType enum)
        **config_overrides: Override default config parameters
        
    Returns:
        ImageGenerator instance
        
    Examples:
        # Use default (Segmind-Vega)
        gen = create_generator()
        
        # Use Janus for better quality
        gen = create_generator(ModelType.JANUS_PRO, resolution=384)
        
        # Use FLUX.1-schnell
        gen = create_generator(ModelType.FLUX_SCHNELL, num_inference_steps=4)
    """
    model_info = get_model_info(model_type)
    
    if model_type == ModelType.OLLAMA_JANUS:
        from .ollama_image_generator import OllamaImageGenerator, OllamaImageConfig
        config = OllamaImageConfig(
            model=model_info.model_id,
            **config_overrides
        )
        return OllamaImageGenerator(config)
    
    elif model_type == ModelType.OLLAMA_FLUX2:
        from .ollama_image_generator import OllamaImageGenerator, OllamaImageConfig
        config = OllamaImageConfig(
            model=model_info.model_id,
            **config_overrides
        )
        return OllamaImageGenerator(config)
    
    elif model_type == ModelType.SEGMIND_VEGA:
        from .image_generator import FluxImageGenerator, ImageConfig
        config = ImageConfig(
            model_id=model_info.model_id,
            **config_overrides
        )
        return FluxImageGenerator(config)
    
    elif model_type == ModelType.JANUS_PRO:
        from .janus_generator import JanusImageGenerator, JanusConfig
        config = JanusConfig(
            model_id=model_info.model_id,
            **config_overrides
        )
        return JanusImageGenerator(config)
    
    elif model_type == ModelType.FLUX_SCHNELL:
        from .flux_schnell_generator import FluxSchnellGenerator, FluxSchnellConfig
        config = FluxSchnellConfig(
            model_id=model_info.model_id,
            **config_overrides
        )
        return FluxSchnellGenerator(config)
    
    else:
        raise ValueError(f"Model type not implemented: {model_type}")
