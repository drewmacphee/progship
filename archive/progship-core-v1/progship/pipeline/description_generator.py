"""Description generation pipeline using LLM and caching."""

from datetime import datetime
from typing import Optional
from pathlib import Path
import json

from ..data.models import (
    ShipStructure,
    ComponentDescription,
    DescriptionManifest,
)
from ..data.loader import get_loader
from .llm import OllamaClient, LLMConfig
from .cache import DescriptionCache
from .prompts import PromptBuilder, get_camera_angles


class DescriptionGenerator:
    """Generate visual descriptions for ship components."""
    
    def __init__(
        self,
        llm_client: Optional[OllamaClient] = None,
        cache: Optional[DescriptionCache] = None,
        prompt_builder: Optional[PromptBuilder] = None,
    ):
        """Initialize description generator.
        
        Args:
            llm_client: Ollama client (creates default if None)
            cache: Description cache (creates default if None)
            prompt_builder: Prompt builder (creates default if None)
        """
        self.llm = llm_client or OllamaClient()
        self.cache = cache or DescriptionCache()
        self.prompts = prompt_builder or PromptBuilder()
        self.loader = get_loader()
        
    def generate_descriptions(
        self,
        structure: ShipStructure,
        use_cache: bool = True,
        include_structural: bool = True,
        include_lights: bool = True,
    ) -> DescriptionManifest:
        """Generate descriptions for all components in a ship structure.
        
        Args:
            structure: Ship structure from Phase 2
            use_cache: Whether to use cached descriptions
            include_structural: Whether to generate descriptions for structural elements
            include_lights: Whether to generate descriptions for light fixtures
            
        Returns:
            DescriptionManifest with all component descriptions
        """
        print(f"[DESC] Generating descriptions for {structure.ship_type_id} ship...")
        
        # Load database for context
        ship_type = self.loader.get_ship_type(structure.ship_type_id)
        style = self.loader.get_style(structure.style_id)
        
        components = []
        cache_misses = []
        
        # Process rooms
        unique_rooms = set(placed_room.room_id for placed_room in structure.rooms)
        for room_id in unique_rooms:
            room = self.loader.get_room(room_id)
            
            # Check cache first
            cached = None
            if use_cache:
                cached = self.cache.get(
                    ship_type_id=structure.ship_type_id,
                    style_id=structure.style_id,
                    seed=structure.seed or 0,
                    component_id=room_id,
                    component_type="room",
                )
            
            if cached:
                print(f"  [OK] Room '{room_id}' (cached)")
                components.append(
                    ComponentDescription(
                        component_id=room_id,
                        component_type="room",
                        base_description=",".join(room.characteristic_facilities),  # Join list to string
                        generated_description=cached.response,
                        style_tags=style.aesthetic_tags,
                        camera_angles=get_camera_angles("room"),
                    )
                )
            else:
                # Build prompt and queue for generation
                prompt = self.prompts.build_room_prompt(room, style, ship_type)
                cache_misses.append({
                    "component_id": room_id,
                    "component_type": "room",
                    "prompt": prompt,
                    "base_description": ", ".join(room.characteristic_facilities),
                })
        
        # Process facilities
        for placed_room in structure.rooms:
            for placed_facility in placed_room.facilities:
                facility_id = placed_facility.facility_id
                variant_id = placed_facility.variant_id
                
                # Create unique component ID (facility_id + variant)
                component_id = f"{facility_id}_{variant_id}" if variant_id else facility_id
                
                # Check cache
                cached = None
                if use_cache:
                    cached = self.cache.get(
                        ship_type_id=structure.ship_type_id,
                        style_id=structure.style_id,
                        seed=structure.seed or 0,
                        component_id=component_id,
                        component_type="facility",
                    )
                
                if cached:
                    print(f"  [OK] Facility '{facility_id}' (cached)")
                    facility = self.loader.get_facility(facility_id)
                    components.append(
                        ComponentDescription(
                            component_id=component_id,
                            component_type="facility",
                            base_description=facility.base_description,
                            generated_description=cached.response,
                            style_tags=style.aesthetic_tags,
                            camera_angles=get_camera_angles("facility"),
                        )
                    )
                else:
                    # Build prompt and queue for generation
                    facility = self.loader.get_facility(facility_id)
                    prompt = self.prompts.build_facility_prompt(
                        facility, style, ship_type, variant_id
                    )
                    cache_misses.append({
                        "component_id": component_id,
                        "component_type": "facility",
                        "prompt": prompt,
                        "base_description": facility.base_description,
                    })
        
        # Process structural elements (if requested)
        if include_structural:
            # Collect unique structural elements from room geometries
            unique_structural = set()
            for placed_room in structure.rooms:
                room = self.loader.get_room(placed_room.room_id)
                if room and room.geometry:
                    unique_structural.add(room.geometry.floor_element_id)
                    unique_structural.add(room.geometry.ceiling_element_id)
                    unique_structural.add(room.geometry.wall_element_id)
                    # Add doors from door placements
                    for door_placement in room.geometry.door_placements:
                        unique_structural.add(door_placement.structural_element_id)
            
            # Also add connection elements (doors/corridors)
            for connection in structure.connections:
                unique_structural.add(connection.connection_element_id)
            
            # Generate descriptions for each unique structural element
            for element_id in unique_structural:
                element = self.loader.get_structural_element(element_id)
                if not element:
                    continue
                
                # Check cache
                cached = None
                if use_cache:
                    cached = self.cache.get(
                        ship_type_id=structure.ship_type_id,
                        style_id=structure.style_id,
                        seed=structure.seed or 0,
                        component_id=element_id,
                        component_type="structural",
                    )
                
                if cached:
                    print(f"  [OK] Structural '{element_id}' (cached)")
                    components.append(
                        ComponentDescription(
                            component_id=element_id,
                            component_type="structural",
                            base_description=element.base_description,
                            generated_description=cached.response,
                            style_tags=style.aesthetic_tags,
                            camera_angles=get_camera_angles("structural"),
                        )
                    )
                else:
                    # Build prompt and queue for generation
                    prompt = self.prompts.build_structural_prompt(element, style, ship_type)
                    cache_misses.append({
                        "component_id": element_id,
                        "component_type": "structural",
                        "prompt": prompt,
                        "base_description": element.base_description,
                    })
        
        # Process light fixtures (if requested)
        if include_lights:
            # Collect unique lights from rooms
            unique_lights = set()
            for placed_room in structure.rooms:
                for placed_light in placed_room.lights:
                    light_id = placed_light.light_id
                    variant_id = placed_light.variant_id
                    component_id = f"{light_id}_{variant_id}" if variant_id else light_id
                    unique_lights.add((light_id, variant_id, component_id))
                
                # Also add characteristic lights from room templates
                room = self.loader.get_room(placed_room.room_id)
                if room:
                    for light_id in room.characteristic_lights:
                        if (light_id, None, light_id) not in unique_lights:
                            unique_lights.add((light_id, None, light_id))
            
            # Generate descriptions for each unique light
            for light_id, variant_id, component_id in unique_lights:
                light = self.loader.get_light_fixture(light_id)
                if not light:
                    continue
                
                # Check cache
                cached = None
                if use_cache:
                    cached = self.cache.get(
                        ship_type_id=structure.ship_type_id,
                        style_id=structure.style_id,
                        seed=structure.seed or 0,
                        component_id=component_id,
                        component_type="light",
                    )
                
                if cached:
                    print(f"  [OK] Light '{light_id}' (cached)")
                    components.append(
                        ComponentDescription(
                            component_id=component_id,
                            component_type="light",
                            base_description=light.base_description,
                            generated_description=cached.response,
                            style_tags=style.aesthetic_tags,
                            camera_angles=get_camera_angles("light"),
                        )
                    )
                else:
                    # Build prompt and queue for generation
                    prompt = self.prompts.build_light_prompt(light, style, ship_type, variant_id)
                    cache_misses.append({
                        "component_id": component_id,
                        "component_type": "light",
                        "prompt": prompt,
                        "base_description": light.base_description,
                    })
        
        # Batch generate all cache misses
        if cache_misses:
            print(f"  [GEN] Generating {len(cache_misses)} new descriptions...")
            prompts = [item["prompt"] for item in cache_misses]
            responses = self.llm.batch_generate(prompts)
            
            # Store in cache and add to components
            for item, response in zip(cache_misses, responses):
                print(f"  [OK] {item['component_type'].title()} '{item['component_id']}' generated")
                
                # Cache the result
                if use_cache:
                    self.cache.set(
                        ship_type_id=structure.ship_type_id,
                        style_id=structure.style_id,
                        seed=structure.seed or 0,
                        component_id=item["component_id"],
                        component_type=item["component_type"],
                        prompt=item["prompt"],
                        response=response,
                        model_name=self.llm.config.model,
                    )
                
                # Add to manifest
                components.append(
                    ComponentDescription(
                        component_id=item["component_id"],
                        component_type=item["component_type"],
                        base_description=item["base_description"],
                        generated_description=response,
                        style_tags=style.aesthetic_tags,
                        camera_angles=get_camera_angles(item["component_type"]),
                    )
                )
        
        # Create manifest
        manifest = DescriptionManifest(
            ship_type_id=structure.ship_type_id,
            style_id=structure.style_id,
            seed=structure.seed or 0,
            generation_timestamp=datetime.utcnow().isoformat(),
            model_name=self.llm.config.model,
            components=components,
        )
        
        print(f"[OK] Generated {len(components)} descriptions")
        return manifest
    
    def save_manifest(self, manifest: DescriptionManifest, output_path: str | Path):
        """Save description manifest to JSON file.
        
        Args:
            manifest: Description manifest to save
            output_path: Output file path
        """
        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        with open(output_path, "w", encoding="utf-8") as f:
            json.dump(manifest.model_dump(), f, indent=2)
        
        print(f"[OK] Saved manifest to: {output_path}")

