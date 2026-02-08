"""
Interior generator - places facilities within rooms.

Ported from C# InteriorGenerator.cs with constraint-based placement.
"""

import random
from typing import List, Optional
from progship.data.models import (
    Room, Facility, PlacedFacility, Transform3D, StyleDescriptor
)
from progship.data.loader import DatabaseLoader


class InteriorGenerator:
    """Generates facility placement within rooms."""
    
    def __init__(self, loader: DatabaseLoader):
        self.loader = loader
    
    def place_facilities(
        self,
        room: Room,
        style: StyleDescriptor,
        seed: Optional[int] = None
    ) -> List[PlacedFacility]:
        """
        Place facilities in a room based on constraints.
        
        Args:
            room: Room template
            style: Visual style for variant selection
            seed: Random seed
            
        Returns:
            List of placed facilities with transforms
        """
        if seed is not None:
            random.seed(seed)
        
        facilities = []
        facility_db = self.loader.load_facilities()
        
        # Get facilities for this room type
        facility_ids = room.characteristic_facilities
        if not facility_ids:
            return facilities
        
        # Simple rule-based placement for now
        # TODO: Implement constraint solver (simulated annealing) - Phase 2
        
        room_width = room.dimensions.get("width", 10.0)
        room_depth = room.dimensions.get("depth", 10.0)
        
        for i, facility_id in enumerate(facility_ids):
            facility = self.loader.get_facility(facility_id)
            if not facility:
                continue
            
            # Find matching variant
            variant = facility.get_variant(style.id)
            variant_id = variant.id if variant else None
            
            # Simple grid placement (temporary)
            x = (i % 3) * (room_width / 3) - room_width/2 + 1.0
            z = (i // 3) * (room_depth / 3) - room_depth/2 + 1.0
            
            transform = Transform3D(
                position=[x, 0.0, z],
                rotation=[0.0, 0.0, 0.0, 1.0],
                scale=[1.0, 1.0, 1.0]
            )
            
            placed = PlacedFacility(
                facility_id=facility_id,
                variant_id=variant_id,
                transform=transform
            )
            facilities.append(placed)
        
        return facilities
    
    def populate_room(
        self,
        room_id: str,
        style_id: str,
        seed: Optional[int] = None
    ) -> List[PlacedFacility]:
        """
        Populate a room with facilities.
        
        Args:
            room_id: Room template ID
            style_id: Style descriptor ID
            seed: Random seed
            
        Returns:
            List of placed facilities
        """
        room = self.loader.get_room(room_id)
        if not room:
            raise ValueError(f"Unknown room: {room_id}")
        
        style = self.loader.get_style(style_id)
        if not style:
            raise ValueError(f"Unknown style: {style_id}")
        
        return self.place_facilities(room, style, seed)
