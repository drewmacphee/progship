"""
Asset Bundle Manifest Generator

Creates a comprehensive manifest.json that references all generated assets
(structure, descriptions, images, 3D models) for consumption by any game engine.
"""

from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Dict, List, Optional, Any
import json
import hashlib
from datetime import datetime


@dataclass
class AssetReference:
    """Reference to a single asset with metadata."""
    component_id: str
    component_type: str  # room, facility, structural, light
    
    # File references
    model_path: Optional[str] = None          # GLB model
    concept_art_path: Optional[str] = None    # Main concept art PNG
    icon_path: Optional[str] = None           # Thumbnail/icon
    description: Optional[str] = None          # AI-generated text
    
    # Transform (from structure.json)
    position: Optional[List[float]] = None    # [x, y, z]
    rotation: Optional[List[float]] = None    # [qx, qy, qz, qw] quaternion
    scale: Optional[List[float]] = None       # [sx, sy, sz]
    
    # Metadata
    dimensions: Optional[Dict[str, float]] = None  # width, height, depth
    bounding_box: Optional[Dict[str, float]] = None
    material_hints: Optional[List[str]] = None
    
    # File validation
    model_exists: bool = False
    model_size_mb: float = 0.0
    concept_art_exists: bool = False
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary, excluding validation fields."""
        d = asdict(self)
        # Remove validation fields (internal only)
        d.pop('model_exists', None)
        d.pop('concept_art_exists', None)
        return d


@dataclass
class AssetBundleManifest:
    """Complete manifest for generated ship assets."""
    # Ship metadata
    ship_id: str
    ship_type_id: str
    style_id: str
    seed: int
    generation_timestamp: str
    
    # File references
    structure_path: str           # structure.json
    descriptions_path: str        # descriptions.json
    
    # Assets organized by component_id
    assets: Dict[str, AssetReference]
    
    # Summary stats
    total_assets: int
    total_models: int
    total_images: int
    total_size_mb: float
    
    # Pipeline metadata
    pipeline_version: str = "1.0"
    llm_model: str = "Qwen/Qwen2.5-7B-Instruct"
    image_model: str = "segmind/Segmind-Vega"
    model_3d_generator: str = "microsoft/TRELLIS-image-large"
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        d = asdict(self)
        # Convert nested AssetReference objects
        d['assets'] = {k: v.to_dict() if isinstance(v, AssetReference) else v 
                       for k, v in self.assets.items()}
        return d
    
    def to_json(self, output_path: Path, indent: int = 2) -> None:
        """Save manifest to JSON file."""
        with open(output_path, 'w') as f:
            json.dump(self.to_dict(), f, indent=indent)
    
    @classmethod
    def from_json(cls, input_path: Path) -> 'AssetBundleManifest':
        """Load manifest from JSON file."""
        with open(input_path, 'r') as f:
            data = json.load(f)
        
        # Convert asset dicts back to AssetReference objects
        assets = {k: AssetReference(**v) for k, v in data['assets'].items()}
        data['assets'] = assets
        
        return cls(**data)


class AssetBundleBuilder:
    """Builds asset bundle manifest from generated files."""
    
    def __init__(self, output_dir: Path):
        self.output_dir = Path(output_dir)
        
    def build_manifest(
        self,
        ship_id: str,
        ship_type_id: str,
        style_id: str,
        seed: int,
        structure_path: Path,
        descriptions_path: Path,
        images_dir: Path,
        models_dir: Path
    ) -> AssetBundleManifest:
        """
        Build complete asset bundle manifest.
        
        Args:
            ship_id: Unique ship identifier
            ship_type_id: Ship type (e.g., 'colony_rotating_solar')
            style_id: Visual style (e.g., 'ceramic_white')
            seed: Random seed used for generation
            structure_path: Path to structure.json
            descriptions_path: Path to descriptions.json
            images_dir: Directory containing concept art
            models_dir: Directory containing GLB models
            
        Returns:
            Complete AssetBundleManifest
        """
        print("=" * 80)
        print("BUILDING ASSET BUNDLE MANIFEST")
        print("=" * 80)
        
        # Load descriptions
        with open(descriptions_path, 'r') as f:
            descriptions_data = json.load(f)
        
        components = descriptions_data.get('components', [])
        print(f"\nFound {len(components)} components in descriptions")
        
        # Build asset references
        assets = {}
        total_size = 0.0
        models_count = 0
        images_count = 0
        
        for comp in components:
            comp_id = comp['component_id']
            comp_type = comp['component_type']
            
            print(f"\n[{comp_id}] ({comp_type})")
            
            # Find model
            model_path = models_dir / f"{comp_id}.glb"
            model_exists = model_path.exists()
            model_size_mb = 0.0
            
            if model_exists:
                model_size_mb = model_path.stat().st_size / (1024 * 1024)
                total_size += model_size_mb
                models_count += 1
                print(f"  ✓ Model: {model_path.name} ({model_size_mb:.2f} MB)")
            else:
                print(f"  ✗ Model: {model_path.name} NOT FOUND")
            
            # Find concept art
            concept_art_path = images_dir / comp_id / f"{comp_id}_main.png"
            concept_exists = concept_art_path.exists()
            
            if concept_exists:
                images_count += 1
                print(f"  ✓ Image: {concept_art_path.name}")
            else:
                print(f"  ✗ Image: NOT FOUND")
            
            # Create asset reference
            asset = AssetReference(
                component_id=comp_id,
                component_type=comp_type,
                model_path=str(model_path.relative_to(self.output_dir)) if model_exists else None,
                concept_art_path=str(concept_art_path.relative_to(self.output_dir)) if concept_exists else None,
                description=comp.get('generated_description'),
                dimensions=comp.get('dimensions'),
                material_hints=comp.get('material_hints'),
                model_exists=model_exists,
                model_size_mb=model_size_mb,
                concept_art_exists=concept_exists
            )
            
            assets[comp_id] = asset
        
        # Create manifest
        manifest = AssetBundleManifest(
            ship_id=ship_id,
            ship_type_id=ship_type_id,
            style_id=style_id,
            seed=seed,
            generation_timestamp=datetime.utcnow().isoformat(),
            structure_path=str(structure_path.relative_to(self.output_dir)) if structure_path.exists() else "",
            descriptions_path=str(descriptions_path.relative_to(self.output_dir)),
            assets=assets,
            total_assets=len(assets),
            total_models=models_count,
            total_images=images_count,
            total_size_mb=total_size
        )
        
        return manifest
    
    def export_bundle(
        self,
        manifest: AssetBundleManifest,
        bundle_dir: Path,
        create_archive: bool = False
    ) -> Path:
        """
        Export asset bundle to organized directory structure.
        
        Args:
            manifest: Asset bundle manifest
            bundle_dir: Output directory for bundle
            create_archive: Whether to create .zip archive
            
        Returns:
            Path to exported bundle directory
        """
        print("\n" + "=" * 80)
        print("EXPORTING ASSET BUNDLE")
        print("=" * 80)
        
        bundle_dir = Path(bundle_dir)
        bundle_dir.mkdir(parents=True, exist_ok=True)
        
        # Save manifest
        manifest_path = bundle_dir / "manifest.json"
        manifest.to_json(manifest_path)
        print(f"\n✓ Manifest: {manifest_path}")
        
        # TODO: Copy all referenced files to bundle directory
        # (For now, manifest just references files in place)
        
        print(f"\n✓ Bundle exported to: {bundle_dir}")
        
        if create_archive:
            # TODO: Create .zip archive
            print(f"\nℹ Archive creation not yet implemented")
        
        return bundle_dir


def validate_bundle(manifest: AssetBundleManifest) -> Dict[str, Any]:
    """
    Validate asset bundle for completeness.
    
    Returns:
        Validation report with issues found
    """
    print("\n" + "=" * 80)
    print("VALIDATING ASSET BUNDLE")
    print("=" * 80)
    
    issues = []
    warnings = []
    
    # Check for missing models
    missing_models = [a for a in manifest.assets.values() if not a.model_exists]
    if missing_models:
        issues.append(f"Missing {len(missing_models)} models: {[a.component_id for a in missing_models]}")
    
    # Check for missing images
    missing_images = [a for a in manifest.assets.values() if not a.concept_art_exists]
    if missing_images:
        warnings.append(f"Missing {len(missing_images)} images: {[a.component_id for a in missing_images]}")
    
    # Check for suspiciously small models
    small_models = [a for a in manifest.assets.values() if a.model_exists and a.model_size_mb < 1.0]
    if small_models:
        warnings.append(f"Suspiciously small models (<1 MB): {[(a.component_id, f'{a.model_size_mb:.2f} MB') for a in small_models]}")
    
    report = {
        'valid': len(issues) == 0,
        'total_assets': manifest.total_assets,
        'total_models': manifest.total_models,
        'total_images': manifest.total_images,
        'issues': issues,
        'warnings': warnings
    }
    
    # Print report
    print(f"\nAssets: {manifest.total_assets}")
    print(f"Models: {manifest.total_models}/{manifest.total_assets}")
    print(f"Images: {manifest.total_images}/{manifest.total_assets}")
    print(f"Total size: {manifest.total_size_mb:.1f} MB")
    
    if issues:
        print(f"\n❌ ISSUES FOUND ({len(issues)}):")
        for issue in issues:
            print(f"  - {issue}")
    
    if warnings:
        print(f"\n⚠️  WARNINGS ({len(warnings)}):")
        for warning in warnings:
            print(f"  - {warning}")
    
    if not issues and not warnings:
        print(f"\n✓ Bundle is valid!")
    
    return report
