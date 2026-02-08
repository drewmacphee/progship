"""Prompt templates and builders for LLM-powered description generation."""

from typing import Dict, List
from ..data.models import (
    Facility, Room, ShipType, StyleDescriptor,
    StructuralElement, LightFixture
)


class PromptBuilder:
    """Build prompts for generating visual descriptions."""
    
    # System prompt for consistent output format
    SYSTEM_PROMPT = """You are a concept art description expert specializing in detailed sci-fi spaceship interiors. 
Your descriptions should focus on visual details suitable for concept art generation: materials, colors, 
lighting, textures, wear patterns, and atmosphere. Be specific and concise."""
    
    # Template for facility descriptions
    FACILITY_TEMPLATE = """Describe a {facility_name} for concept art generation.

Context:
- Ship Type: {ship_type}
- Visual Style: {style_name}
- Materials: {materials}
- Color Palette: {colors}
- Wear Level: {wear_level}
- Base Characteristics: {base_description}

Create a detailed visual description (100-150 words) focusing on:
- Physical appearance and materials
- Lighting and atmosphere
- Notable visual details
- How the style influences the design

Description:"""
    
    # Template for room descriptions
    ROOM_TEMPLATE = """Describe a {room_name} for concept art generation.

Context:
- Ship Type: {ship_type}
- Visual Style: {style_name}
- Materials: {materials}
- Color Palette: {colors}
- Wear Level: {wear_level}
- Room Purpose: {room_purpose}

Create a detailed visual description (100-150 words) focusing on:
- Overall layout and spatial feel
- Dominant features and equipment
- Lighting and atmosphere
- How the style influences the room design

Description:"""
    
    # Template for structural elements (walls, floors, doors, etc.)
    STRUCTURAL_TEMPLATE = """Describe a {element_name} ({element_type}) for concept art generation.

Context:
- Ship Type: {ship_type}
- Visual Style: {style_name}
- Materials: {materials}
- Color Palette: {colors}
- Wear Level: {wear_level}
- Base Characteristics: {base_description}
- Element Type: {element_type}

Create a detailed visual description (80-120 words) focusing on:
- Surface materials and textures
- Modular/tileable design aspects
- Panel details, seams, connections
- Lighting integration (if applicable)
- How the style influences the structural design

Description:"""
    
    # Template for light fixtures
    LIGHT_TEMPLATE = """Describe a {light_name} fixture for concept art generation.

Context:
- Ship Type: {ship_type}
- Visual Style: {style_name}
- Materials: {materials}
- Color Palette: {colors}
- Wear Level: {wear_level}
- Light Type: {light_type}
- Light Characteristics: {base_description}

Create a detailed visual description (80-120 words) focusing on:
- Fixture housing design and materials
- Light emission quality and diffusion
- Mounting mechanism
- Integration with surrounding architecture
- How the style influences the fixture design

Description:"""
    
    def __init__(self):
        """Initialize prompt builder."""
        pass
    
    def build_facility_prompt(
        self,
        facility: Facility,
        style: StyleDescriptor,
        ship_type: ShipType,
        variant_id: str | None = None,
    ) -> str:
        """Build prompt for facility description.
        
        Args:
            facility: Facility definition from database
            style: Style descriptor for visual aesthetic
            ship_type: Ship type for context
            variant_id: Optional variant ID for style-specific description
            
        Returns:
            Formatted prompt string
        """
        # Get variant-specific description override if available
        base_description = facility.base_description
        if variant_id:
            variant = facility.get_variant(variant_id)
            if variant and variant.description_override:
                base_description = variant.description_override
        
        # Format prompt with context
        prompt = self.FACILITY_TEMPLATE.format(
            facility_name=facility.id.replace("_", " ").title(),
            ship_type=ship_type.id.replace("_", " ").title(),
            style_name=style.id.replace("_", " ").title(),
            materials=", ".join(style.material_palette),
            colors=", ".join(style.color_palette),
            wear_level=style.wear_level,
            base_description=base_description,
        )
        
        return prompt
    
    def build_room_prompt(
        self,
        room: Room,
        style: StyleDescriptor,
        ship_type: ShipType,
    ) -> str:
        """Build prompt for room description.
        
        Args:
            room: Room template from database
            style: Style descriptor for visual aesthetic
            ship_type: Ship type for context
            
        Returns:
            Formatted prompt string
        """
        # Infer room purpose from characteristic facilities
        room_purpose = f"Contains: {', '.join(room.characteristic_facilities)}"
        
        # Format prompt with context
        prompt = self.ROOM_TEMPLATE.format(
            room_name=room.id.replace("_", " ").title(),
            ship_type=ship_type.id.replace("_", " ").title(),
            style_name=style.id.replace("_", " ").title(),
            materials=", ".join(style.material_palette),
            colors=", ".join(style.color_palette),
            wear_level=style.wear_level,
            room_purpose=room_purpose,
        )
        
        return prompt
    
    def build_structural_prompt(
        self,
        element: StructuralElement,
        style: StyleDescriptor,
        ship_type: ShipType,
        variant_id: str | None = None,
    ) -> str:
        """Build prompt for structural element description.
        
        Args:
            element: Structural element from database
            style: Style descriptor for visual aesthetic
            ship_type: Ship_type for context
            variant_id: Optional variant ID for style-specific description
            
        Returns:
            Formatted prompt string
        """
        # Get variant-specific description override if available
        base_description = element.base_description
        if variant_id:
            variant = element.get_variant(variant_id)
            if variant and variant.description_override:
                base_description = variant.description_override
        
        # Format prompt with context
        prompt = self.STRUCTURAL_TEMPLATE.format(
            element_name=element.id.replace("_", " ").title(),
            element_type=element.type.title(),
            ship_type=ship_type.id.replace("_", " ").title(),
            style_name=style.id.replace("_", " ").title(),
            materials=", ".join(style.material_palette),
            colors=", ".join(style.color_palette),
            wear_level=style.wear_level,
            base_description=base_description,
        )
        
        return prompt
    
    def build_light_prompt(
        self,
        light: LightFixture,
        style: StyleDescriptor,
        ship_type: ShipType,
        variant_id: str | None = None,
    ) -> str:
        """Build prompt for light fixture description.
        
        Args:
            light: Light fixture from database
            style: Style descriptor for visual aesthetic
            ship_type: Ship type for context
            variant_id: Optional variant ID for style-specific description
            
        Returns:
            Formatted prompt string
        """
        # Get variant-specific description override if available
        base_description = light.base_description
        if variant_id:
            variant = light.get_variant(variant_id)
            if variant and variant.description_override:
                base_description = variant.description_override
        
        # Format prompt with context
        prompt = self.LIGHT_TEMPLATE.format(
            light_name=light.id.replace("_", " ").title(),
            light_type=light.type.title(),
            ship_type=ship_type.id.replace("_", " ").title(),
            style_name=style.id.replace("_", " ").title(),
            materials=", ".join(style.material_palette),
            colors=", ".join(style.color_palette),
            wear_level=style.wear_level,
            base_description=base_description,
        )
        
        return prompt
    
    def build_batch_prompts(
        self,
        items: List[Dict],
        style: StyleDescriptor,
        ship_type: ShipType,
    ) -> List[str]:
        """Build multiple prompts efficiently.
        
        Args:
            items: List of dicts with 'type' (facility/room/structural/light) and 'data' (object)
            style: Style descriptor
            ship_type: Ship type
            
        Returns:
            List of formatted prompts
        """
        prompts = []
        for item in items:
            if item["type"] == "facility":
                prompt = self.build_facility_prompt(
                    item["data"],
                    style,
                    ship_type,
                    item.get("variant_id"),
                )
            elif item["type"] == "room":
                prompt = self.build_room_prompt(
                    item["data"],
                    style,
                    ship_type,
                )
            elif item["type"] == "structural":
                prompt = self.build_structural_prompt(
                    item["data"],
                    style,
                    ship_type,
                    item.get("variant_id"),
                )
            elif item["type"] == "light":
                prompt = self.build_light_prompt(
                    item["data"],
                    style,
                    ship_type,
                    item.get("variant_id"),
                )
            else:
                raise ValueError(f"Unknown item type: {item['type']}")
            
            prompts.append(prompt)
        
        return prompts


def get_camera_angles(component_type: str) -> List[str]:
    """Get recommended camera angles for concept art.
    
    Args:
        component_type: "facility", "room", "structural", or "light"
        
    Returns:
        List of recommended camera angle descriptions
    """
    if component_type == "facility":
        return [
            "front_view",  # Straight-on view
            "three_quarter_view",  # 3/4 angle view
            "detail_closeup",  # Close-up of interesting details
        ]
    elif component_type == "room":
        return [
            "wide_angle",  # Show full room layout
            "hero_shot",  # Dramatic angle highlighting key features
            "detail_corner",  # Corner view with atmospheric details
        ]
    elif component_type == "structural":
        return [
            "front_view",  # Straight-on view of panel/tile
            "detail_texture",  # Close-up of surface texture
            "tiling_context",  # Show how elements tile together
        ]
    elif component_type == "light":
        return [
            "fixture_detail",  # Close-up of fixture housing
            "illumination_view",  # Show light emission and coverage
            "mounting_context",  # Show how it mounts to surface
        ]
    else:
        return ["front_view"]
