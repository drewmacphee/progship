#!/usr/bin/env python3
"""
Test asset bundle generation with current generated assets.
"""
import sys
sys.path.insert(0, '.')

from progship.export.bundle import AssetBundleBuilder, validate_bundle
from pathlib import Path

print("=" * 80)
print("TESTING ASSET BUNDLE GENERATION")
print("=" * 80)

# Paths
output_dir = Path("output")
descriptions_path = output_dir / "test_descriptions_with_structural.json"
images_dir = output_dir / "images_regenerated"
models_dir = output_dir / "models"
bundle_dir = output_dir / "asset_bundle"

# Validate paths exist
if not descriptions_path.exists():
    print(f"ERROR: {descriptions_path} not found")
    print("Run test_description_generator.py first")
    sys.exit(1)

if not images_dir.exists():
    print(f"ERROR: {images_dir} not found")
    print("Run regenerate_all_images.py first")
    sys.exit(1)

if not models_dir.exists():
    print(f"ERROR: {models_dir} not found")
    print("Run TRELLIS batch conversion first")
    sys.exit(1)

# Build manifest
builder = AssetBundleBuilder(output_dir)

manifest = builder.build_manifest(
    ship_id="colony_001",
    ship_type_id="colony_stacked",
    style_id="ceramic_white",
    seed=42,
    structure_path=Path("structure.json"),  # Placeholder - not generated yet
    descriptions_path=descriptions_path,
    images_dir=images_dir,
    models_dir=models_dir
)

# Validate bundle
report = validate_bundle(manifest)

# Export bundle
bundle_path = builder.export_bundle(
    manifest=manifest,
    bundle_dir=bundle_dir,
    create_archive=False
)

print("\n" + "=" * 80)
print("SUCCESS! Asset bundle generated")
print("=" * 80)
print(f"\nManifest: {bundle_path / 'manifest.json'}")
print(f"Bundle directory: {bundle_path}")
print(f"\nNext steps:")
print(f"1. View manifest: cat {bundle_path / 'manifest.json'}")
print(f"2. Load in Godot/Unity/Web viewer")
print(f"3. Validate models open in 3D viewer")
