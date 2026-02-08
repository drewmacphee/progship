"""
Database loader for ProgShip JSON data files.

Loads ship types, styles, facilities, and rooms with validation.
"""

import json
from pathlib import Path
from typing import Optional
from progship.data.models import (
    ShipTypeDatabase, StyleDatabase, FacilityDatabase, RoomDatabase,
    StructuralDatabase, LightDatabase,
    ShipType, StyleDescriptor, Facility, Room, StructuralElement, LightFixture
)


class DatabaseLoader:
    """Loads and caches JSON databases."""
    
    def __init__(self, data_dir: Optional[Path] = None):
        """
        Initialize loader.
        
        Args:
            data_dir: Path to data directory. Defaults to progship-core/data/
        """
        if data_dir is None:
            data_dir = Path(__file__).parent.parent.parent / "data"
        self.data_dir = Path(data_dir)
        
        # Caches
        self._ship_types: Optional[ShipTypeDatabase] = None
        self._styles: Optional[StyleDatabase] = None
        self._facilities: Optional[FacilityDatabase] = None
        self._rooms: Optional[RoomDatabase] = None
        self._structural: Optional[StructuralDatabase] = None
        self._lights: Optional[LightDatabase] = None
    
    def load_ship_types(self, force_reload: bool = False) -> ShipTypeDatabase:
        """Load ship types database."""
        if self._ship_types is None or force_reload:
            path = self.data_dir / "ship_types.json"
            with open(path, 'r', encoding='utf-8') as f:
                data = json.load(f)
            self._ship_types = ShipTypeDatabase(**data)
        return self._ship_types
    
    def load_styles(self, force_reload: bool = False) -> StyleDatabase:
        """Load style descriptors database."""
        if self._styles is None or force_reload:
            path = self.data_dir / "style_descriptors.json"
            with open(path, 'r', encoding='utf-8') as f:
                data = json.load(f)
            self._styles = StyleDatabase(**data)
        return self._styles
    
    def load_facilities(self, force_reload: bool = False) -> FacilityDatabase:
        """Load facilities database."""
        if self._facilities is None or force_reload:
            path = self.data_dir / "facilities.json"
            with open(path, 'r', encoding='utf-8') as f:
                data = json.load(f)
            self._facilities = FacilityDatabase(**data)
        return self._facilities
    
    def load_rooms(self, force_reload: bool = False) -> RoomDatabase:
        """Load rooms database (if exists)."""
        if self._rooms is None or force_reload:
            path = self.data_dir / "rooms.json"
            if not path.exists():
                # Return empty database if file doesn't exist yet
                self._rooms = RoomDatabase(rooms=[])
            else:
                with open(path, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                self._rooms = RoomDatabase(**data)
        return self._rooms
    
    def get_ship_type(self, ship_type_id: str) -> Optional[ShipType]:
        """Get a specific ship type by ID."""
        db = self.load_ship_types()
        for ship_type in db.ship_types:
            if ship_type.id == ship_type_id:
                return ship_type
        return None
    
    def get_style(self, style_id: str) -> Optional[StyleDescriptor]:
        """Get a specific style by ID."""
        db = self.load_styles()
        for style in db.style_descriptors:
            if style.id == style_id:
                return style
        return None
    
    def get_facility(self, facility_id: str) -> Optional[Facility]:
        """Get a specific facility by ID."""
        db = self.load_facilities()
        for facility in db.facilities:
            if facility.id == facility_id:
                return facility
        return None
    
    def get_room(self, room_id: str) -> Optional[Room]:
        """Get a specific room by ID."""
        db = self.load_rooms()
        for room in db.rooms:
            if room.id == room_id:
                return room
        return None
    
    def load_structural_elements(self, force_reload: bool = False) -> StructuralDatabase:
        """Load structural elements database."""
        if self._structural is None or force_reload:
            path = self.data_dir / "structural_elements.json"
            if not path.exists():
                self._structural = StructuralDatabase(elements=[])
            else:
                with open(path, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                self._structural = StructuralDatabase(**data)
        return self._structural
    
    def load_light_fixtures(self, force_reload: bool = False) -> LightDatabase:
        """Load light fixtures database."""
        if self._lights is None or force_reload:
            path = self.data_dir / "light_fixtures.json"
            if not path.exists():
                self._lights = LightDatabase(fixtures=[])
            else:
                with open(path, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                self._lights = LightDatabase(**data)
        return self._lights
    
    def get_structural_element(self, element_id: str) -> Optional[StructuralElement]:
        """Get a specific structural element by ID."""
        db = self.load_structural_elements()
        for element in db.elements:
            if element.id == element_id:
                return element
        return None
    
    def get_light_fixture(self, fixture_id: str) -> Optional[LightFixture]:
        """Get a specific light fixture by ID."""
        db = self.load_light_fixtures()
        for fixture in db.fixtures:
            if fixture.id == fixture_id:
                return fixture
        return None


# Singleton instance
_loader: Optional[DatabaseLoader] = None

def get_loader() -> DatabaseLoader:
    """Get singleton database loader instance."""
    global _loader
    if _loader is None:
        _loader = DatabaseLoader()
    return _loader
