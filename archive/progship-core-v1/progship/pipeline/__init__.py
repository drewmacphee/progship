"""Pipeline modules for AI-powered content generation."""

from .llm import OllamaClient, LLMConfig, create_client
from .cache import DescriptionCache, CachedDescription
from .prompts import PromptBuilder, get_camera_angles
from .description_generator import DescriptionGenerator
from .image_generator import QwenImageGenerator, ImageConfig
from .image_pipeline import ImagePipeline, ImageManifest
from .image_processing import ImageProcessor, process_image, batch_process_images
from .model_registry import (
    ModelType,
    ModelInfo,
    create_generator,
    get_model_info,
    list_available_models
)

__all__ = [
    'OllamaClient',
    'LLMConfig',
    'create_client',
    'DescriptionCache',
    'CachedDescription',
    'PromptBuilder',
    'get_camera_angles',
    'DescriptionGenerator',
    'FluxImageGenerator',
    'ImageConfig',
    'ImagePipeline',
    'ImageManifest',
    'ImageProcessor',
    'process_image',
    'batch_process_images',
    'ModelType',
    'ModelInfo',
    'create_generator',
    'get_model_info',
    'list_available_models',
]
