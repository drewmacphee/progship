"""Description caching system for reproducible generation."""

import hashlib
import json
from datetime import datetime
from pathlib import Path
from typing import Optional
from dataclasses import dataclass, asdict


@dataclass
class CachedDescription:
    """A cached description entry."""
    
    prompt: str
    response: str
    timestamp: str
    model_name: str
    cache_key: str


class DescriptionCache:
    """Cache for LLM-generated descriptions."""
    
    def __init__(self, cache_dir: str = ".cache/descriptions", enabled: bool = True):
        """Initialize description cache.
        
        Args:
            cache_dir: Directory to store cache files
            enabled: Whether caching is enabled
        """
        self.cache_dir = Path(cache_dir)
        self.enabled = enabled
        
        if self.enabled:
            self.cache_dir.mkdir(parents=True, exist_ok=True)
    
    def _compute_cache_key(
        self,
        ship_type_id: str,
        style_id: str,
        seed: int,
        component_id: str,
        component_type: str,
    ) -> str:
        """Compute SHA256 cache key from component parameters.
        
        Args:
            ship_type_id: Ship type identifier
            style_id: Style descriptor identifier
            seed: Random seed
            component_id: Component identifier (facility_id or room_id)
            component_type: "facility" or "room"
            
        Returns:
            Hex-encoded SHA256 hash
        """
        key_parts = [
            ship_type_id,
            style_id,
            str(seed),
            component_id,
            component_type,
        ]
        key_string = "|".join(key_parts)
        return hashlib.sha256(key_string.encode()).hexdigest()
    
    def get(
        self,
        ship_type_id: str,
        style_id: str,
        seed: int,
        component_id: str,
        component_type: str,
    ) -> Optional[CachedDescription]:
        """Retrieve cached description if available.
        
        Args:
            ship_type_id: Ship type identifier
            style_id: Style descriptor identifier
            seed: Random seed
            component_id: Component identifier
            component_type: "facility" or "room"
            
        Returns:
            CachedDescription if found, None otherwise
        """
        if not self.enabled:
            return None
        
        cache_key = self._compute_cache_key(
            ship_type_id, style_id, seed, component_id, component_type
        )
        cache_file = self.cache_dir / f"{cache_key}.json"
        
        if not cache_file.exists():
            return None
        
        try:
            with open(cache_file, "r", encoding="utf-8") as f:
                data = json.load(f)
                return CachedDescription(
                    prompt=data["prompt"],
                    response=data["response"],
                    timestamp=data["timestamp"],
                    model_name=data["model_name"],
                    cache_key=cache_key,
                )
        except (json.JSONDecodeError, KeyError) as e:
            print(f"⚠ Warning: Corrupted cache file {cache_file}, ignoring: {e}")
            return None
    
    def set(
        self,
        ship_type_id: str,
        style_id: str,
        seed: int,
        component_id: str,
        component_type: str,
        prompt: str,
        response: str,
        model_name: str,
    ):
        """Store description in cache.
        
        Args:
            ship_type_id: Ship type identifier
            style_id: Style descriptor identifier
            seed: Random seed
            component_id: Component identifier
            component_type: "facility" or "room"
            prompt: Original prompt sent to LLM
            response: Generated description from LLM
            model_name: Model used for generation
        """
        if not self.enabled:
            return
        
        cache_key = self._compute_cache_key(
            ship_type_id, style_id, seed, component_id, component_type
        )
        cache_file = self.cache_dir / f"{cache_key}.json"
        
        data = {
            "prompt": prompt,
            "response": response,
            "timestamp": datetime.utcnow().isoformat(),
            "model_name": model_name,
            "cache_key": cache_key,
            "metadata": {
                "ship_type_id": ship_type_id,
                "style_id": style_id,
                "seed": seed,
                "component_id": component_id,
                "component_type": component_type,
            },
        }
        
        try:
            with open(cache_file, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2)
        except IOError as e:
            print(f"⚠ Warning: Failed to write cache file {cache_file}: {e}")
    
    def clear(self):
        """Clear all cached descriptions."""
        if not self.enabled or not self.cache_dir.exists():
            return
        
        count = 0
        for cache_file in self.cache_dir.glob("*.json"):
            try:
                cache_file.unlink()
                count += 1
            except OSError as e:
                print(f"⚠ Warning: Failed to delete {cache_file}: {e}")
        
        print(f"✓ Cleared {count} cached descriptions")
    
    def stats(self) -> dict:
        """Get cache statistics.
        
        Returns:
            Dictionary with cache stats (total_entries, total_size_bytes)
        """
        if not self.enabled or not self.cache_dir.exists():
            return {"total_entries": 0, "total_size_bytes": 0}
        
        cache_files = list(self.cache_dir.glob("*.json"))
        total_size = sum(f.stat().st_size for f in cache_files)
        
        return {
            "total_entries": len(cache_files),
            "total_size_bytes": total_size,
            "total_size_mb": round(total_size / (1024 * 1024), 2),
        }
