"""
Image generation pipeline for ship components.
Converts text descriptions into concept art images.
"""

from pathlib import Path
from typing import Optional, List, Dict, Any
import json
from datetime import datetime
from tqdm import tqdm

from progship.data.models import DescriptionManifest, ComponentDescription
from progship.pipeline.model_registry import create_generator, ModelType
from progship.pipeline.image_generator import ImageConfig


class ImageManifest:
    """Manifest tracking generated images for ship components."""
    
    def __init__(
        self,
        ship_type_id: str,
        style_id: str,
        seed: int,
        components: List[Dict[str, Any]],
        generation_timestamp: str,
        model_name: str,
        config: Dict[str, Any]
    ):
        self.ship_type_id = ship_type_id
        self.style_id = style_id
        self.seed = seed
        self.components = components
        self.generation_timestamp = generation_timestamp
        self.model_name = model_name
        self.config = config
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "ship_type_id": self.ship_type_id,
            "style_id": self.style_id,
            "seed": self.seed,
            "components": self.components,
            "generation_timestamp": self.generation_timestamp,
            "model_name": self.model_name,
            "config": self.config
        }
    
    def save(self, path: Path):
        """Save manifest to JSON file."""
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w") as f:
            json.dump(self.to_dict(), f, indent=2)


class ImagePipeline:
    """Pipeline for generating concept art from component descriptions."""
    
    def __init__(
        self,
        image_config: Optional[ImageConfig] = None,
        output_dir: Path = Path("output/images"),
        model_type: ModelType = ModelType.FLUX_SCHNELL
    ):
        self.config = image_config or ImageConfig()
        self.generator = create_generator(model_type)
        self.output_dir = Path(output_dir)
    
    def generate_from_manifest(
        self,
        manifest_path: Path,
        negative_prompt: Optional[str] = None,
        generate_angles: bool = False
    ) -> ImageManifest:
        """
        Generate images for all components in a description manifest.
        
        Args:
            manifest_path: Path to descriptions manifest JSON
            negative_prompt: Optional negative prompt for all generations
            generate_angles: Whether to generate multiple camera angles per component
            
        Returns:
            ImageManifest with paths to generated images
        """
        # Load description manifest
        with open(manifest_path, "r") as f:
            manifest_data = json.load(f)
        
        # Create manifest object
        desc_manifest = DescriptionManifest(**manifest_data)
        
        print(f"\n{'='*60}")
        print(f"Image Generation Pipeline")
        print(f"{'='*60}")
        print(f"Ship Type: {desc_manifest.ship_type_id}")
        print(f"Style: {desc_manifest.style_id}")
        print(f"Seed: {desc_manifest.seed}")
        print(f"Components: {len(desc_manifest.components)}")
        print(f"Model: {self.config.model_id}")
        print(f"Resolution: {self.config.resolution}x{self.config.resolution}")
        print(f"{'='*60}\n")
        
        # Generate images for each component
        components_with_images = []
        
        for comp_desc in tqdm(desc_manifest.components, desc="Generating images"):
            component_images = self._generate_component_images(
                comp_desc,
                desc_manifest.seed,
                negative_prompt,
                generate_angles
            )
            components_with_images.append(component_images)
        
        # Create image manifest
        image_manifest = ImageManifest(
            ship_type_id=desc_manifest.ship_type_id,
            style_id=desc_manifest.style_id,
            seed=desc_manifest.seed,
            components=components_with_images,
            generation_timestamp=datetime.now().isoformat(),
            model_name=self.config.model_id,
            config={
                "resolution": self.config.resolution,
                "steps": self.config.num_inference_steps,
                "guidance_scale": self.config.guidance_scale
            }
        )
        
        print(f"\n[OK] Generated images for {len(components_with_images)} components")
        
        return image_manifest
    
    def _generate_component_images(
        self,
        comp_desc: ComponentDescription,
        base_seed: int,
        negative_prompt: Optional[str],
        generate_angles: bool
    ) -> Dict[str, Any]:
        """Generate images for a single component."""
        
        component_dir = self.output_dir / comp_desc.component_id
        component_dir.mkdir(parents=True, exist_ok=True)
        
        # Build enhanced prompt
        prompt = self._build_image_prompt(comp_desc)
        
        # Default negative prompt if not provided
        if negative_prompt is None:
            negative_prompt = "blurry, low quality, distorted, text, watermark"
        
        images = []
        
        # Generate main image
        seed = base_seed
        result = self.generator.generate(
            prompt=prompt,
            negative_prompt=negative_prompt,
            seed=seed
        )
        
        # Save main image
        image_path = component_dir / f"{comp_desc.component_id}_main.png"
        self.generator.save_image(result, image_path, save_metadata=True)
        
        images.append({
            "view": "main",
            "path": str(image_path.relative_to(self.output_dir.parent)),
            "seed": seed,
            "prompt": prompt
        })
        
        # Generate additional angles if requested
        if generate_angles and comp_desc.camera_angles:
            for i, angle in enumerate(comp_desc.camera_angles[:3], start=1):  # Max 3 angles
                angle_prompt = f"{prompt}, {angle} view"
                angle_seed = seed + i
                
                result = self.generator.generate(
                    prompt=angle_prompt,
                    negative_prompt=negative_prompt,
                    seed=angle_seed
                )
                
                angle_name = angle.lower().replace(" ", "_").replace("/", "_")
                image_path = component_dir / f"{comp_desc.component_id}_{angle_name}.png"
                self.generator.save_image(result, image_path, save_metadata=True)
                
                images.append({
                    "view": angle,
                    "path": str(image_path.relative_to(self.output_dir.parent)),
                    "seed": angle_seed,
                    "prompt": angle_prompt
                })
        
        return {
            "component_id": comp_desc.component_id,
            "component_type": comp_desc.component_type,
            "images": images
        }
    
    def _build_image_prompt(self, comp_desc: ComponentDescription) -> str:
        """Build enhanced prompt for image generation."""
        
        # Start with generated description
        prompt_parts = [comp_desc.generated_description]
        
        # Add style tags if available
        if comp_desc.style_tags:
            prompt_parts.append(", ".join(comp_desc.style_tags))
        
        # Add component-type-specific quality boosters for ISOLATED 3D ASSETS
        if comp_desc.component_type == "structural":
            # Emphasize single isolated modular asset for 3D conversion
            quality_tags = [
                "3D game asset render",
                "single piece",
                "centered composition",
                "isolated object",
                "pure white background",
                "product photography style",
                "orthographic view",
                "white studio lighting",
                "no shadows",
                "modular design",
                "clean surfaces",
                "professional 3D render",
                "8k resolution"
            ]
        elif comp_desc.component_type == "light":
            # Emphasize single isolated light fixture asset
            quality_tags = [
                "3D game asset render",
                "single fixture",
                "centered composition",
                "isolated light fixture",
                "pure white background",
                "product photography style",
                "white studio lighting",
                "illumination visible",
                "glowing elements",
                "professional 3D render",
                "highly detailed",
                "8k resolution"
            ]
        elif comp_desc.component_type == "room":
            # For rooms, we still want scenes but from asset perspective
            quality_tags = [
                "interior architecture",
                "atmospheric lighting",
                "cinematic composition",
                "professional concept art",
                "8k resolution",
                "wide angle perspective"
            ]
        else:  # facility or other
            # Emphasize single isolated console/equipment asset
            quality_tags = [
                "3D game asset render",
                "single object",
                "centered composition",
                "isolated object",
                "pure white background",
                "product photography style",
                "white studio lighting",
                "highly detailed",
                "professional 3D render",
                "8k resolution",
                "dramatic lighting"
            ]
        
        prompt_parts.extend(quality_tags)
        
        return ", ".join(prompt_parts)

