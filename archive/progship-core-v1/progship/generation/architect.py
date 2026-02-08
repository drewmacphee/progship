"""
Ship architect - generates room layouts for ships.

Ported from C# ShipArchitect.cs with improvements for engine-agnostic output.
"""

import random
from typing import List, Optional, Tuple
from progship.data.models import (
    ShipType, Room, PlacedRoom, Transform3D, ShipStructure
)
from progship.data.loader import DatabaseLoader


class ShipArchitect:
    """Generates spatial layout of rooms in a ship."""
    
    def __init__(self, loader: DatabaseLoader):
        self.loader = loader
    
    def generate_layout(
        self,
        ship_type: ShipType,
        target_room_count: int = 10,
        seed: Optional[int] = None
    ) -> List[PlacedRoom]:
        """
        Generate room layout for a ship.
        
        Args:
            ship_type: Ship archetype
            target_room_count: Desired number of rooms
            seed: Random seed for reproducibility
            
        Returns:
            List of placed rooms with transforms
        """
        if seed is not None:
            random.seed(seed)
        
        rooms = []
        room_db = self.loader.load_rooms()
        
        # For now, use simple linear stacking
        # TODO: Implement greedy DFS with constraint checking (Phase 2)
        position_y = 0.0
        
        for i in range(target_room_count):
            # Pick room from typical rooms for this ship type
            if ship_type.typical_rooms and room_db.rooms:
                available_rooms = [r for r in room_db.rooms if r.id in ship_type.typical_rooms]
                if not available_rooms:
                    available_rooms = room_db.rooms
                room = random.choice(available_rooms)
            else:
                # Fallback: create placeholder room
                room = Room(
                    id=f"room_{i}",
                    name=f"Room {i}",
                    dimensions={"width": 10.0, "height": 3.0, "depth": 10.0}
                )
            
            # Stack rooms vertically for now
            transform = Transform3D(
                position=[0.0, position_y, 0.0],
                rotation=[0.0, 0.0, 0.0, 1.0],  # No rotation (quaternion identity)
                scale=[1.0, 1.0, 1.0]
            )
            
            placed_room = PlacedRoom(
                room_id=room.id,
                transform=transform,
                facilities=[]
            )
            rooms.append(placed_room)
            
            # Advance position for next room
            room_height = room.dimensions.get("height", 3.0)
            position_y += room_height
        
        return rooms
    
    def generate_structure(
        self,
        ship_type_id: str,
        style_id: str,
        room_count: int = 10,
        seed: Optional[int] = None
    ) -> ShipStructure:
        """
        Generate complete ship structure.
        
        Args:
            ship_type_id: Ship type ID
            style_id: Style descriptor ID
            room_count: Number of rooms
            seed: Random seed
            
        Returns:
            ShipStructure with rooms and metadata
        """
        ship_type = self.loader.get_ship_type(ship_type_id)
        if not ship_type:
            raise ValueError(f"Unknown ship type: {ship_type_id}")
        
        style = self.loader.get_style(style_id)
        if not style:
            raise ValueError(f"Unknown style: {style_id}")
        
        rooms = self.generate_layout(ship_type, room_count, seed)
        
        structure = ShipStructure(
            ship_type_id=ship_type_id,
            style_id=style_id,
            seed=seed,
            rooms=rooms,
            metadata={
                "ship_type_name": ship_type.name,
                "style_name": style.name,
                "room_count": len(rooms)
            }
        )
        
        return structure
