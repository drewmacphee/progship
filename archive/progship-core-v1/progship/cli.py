"""
Command-line interface for ProgShip.

Usage:
    progship generate --ship-type colony_stacked --style ceramic_white
    progship describe output/structure.json
    progship validate
"""

import json
import click
from pathlib import Path
from progship.data.loader import get_loader
from progship.data.validator import SchemaValidator
from progship.data.models import ShipStructure
from progship.generation.architect import ShipArchitect
from progship.generation.interior import InteriorGenerator
from progship.pipeline import DescriptionGenerator


@click.group()
@click.version_option(version="0.1.0")
def cli():
    """ProgShip - Engine-agnostic ship generation pipeline."""
    pass


@cli.command()
@click.option('--ship-type', required=True, help='Ship type ID (e.g., colony_stacked)')
@click.option('--style', required=True, help='Style descriptor ID (e.g., ceramic_white)')
@click.option('--rooms', default=10, help='Number of rooms to generate')
@click.option('--seed', default=None, type=int, help='Random seed for reproducibility')
@click.option('--output', default='output/structure.json', help='Output file path')
@click.option('--with-descriptions', is_flag=True, help='Generate AI descriptions (Stage 2)')
@click.option('--with-images', is_flag=True, help='Generate concept art (Stage 3)')
@click.option('--process-images', is_flag=True, help='Post-process images (crop, thumbnails)')
@click.option('--no-cache', is_flag=True, help='Disable description caching')
@click.option('--image-resolution', default=1024, type=int, help='Image resolution (default: 1024)')
@click.option('--image-steps', default=25, type=int, help='Image inference steps (default: 25)')
def generate(ship_type: str, style: str, rooms: int, seed: int, output: str, 
             with_descriptions: bool, with_images: bool, process_images: bool, no_cache: bool,
             image_resolution: int, image_steps: int):
    """Generate ship structure (Stage 1) and optionally descriptions (Stage 2)."""
    click.echo(f"🚀 Generating {ship_type} ship with {style} style...")
    
    loader = get_loader()
    architect = ShipArchitect(loader)
    interior_gen = InteriorGenerator(loader)
    
    # Generate structure (Stage 1)
    structure = architect.generate_structure(
        ship_type_id=ship_type,
        style_id=style,
        room_count=rooms,
        seed=seed
    )
    
    # Populate rooms with facilities
    for placed_room in structure.rooms:
        facilities = interior_gen.populate_room(
            room_id=placed_room.room_id,
            style_id=style,
            seed=seed
        )
        placed_room.facilities = facilities
    
    # Save structure to JSON
    output_path = Path(output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(structure.model_dump(), f, indent=2)
    
    click.echo(f"✓ Generated structure saved to: {output_path}")
    click.echo(f"  Rooms: {len(structure.rooms)}")
    total_facilities = sum(len(r.facilities) for r in structure.rooms)
    click.echo(f"  Facilities: {total_facilities}")
    
    # Generate descriptions (Stage 2) if requested
    descriptions_path = None
    if with_descriptions or with_images:
        click.echo("\n🎨 Generating AI descriptions (Stage 2)...")
        from progship.pipeline import DescriptionGenerator
        desc_generator = DescriptionGenerator()
        
        manifest = desc_generator.generate_descriptions(
            structure,
            use_cache=not no_cache
        )
        
        # Save descriptions
        descriptions_path = output_path.parent / f"{output_path.stem}_descriptions.json"
        desc_generator.save_manifest(manifest, descriptions_path)
        click.echo(f"✓ Descriptions saved to: {descriptions_path}")
    
    # Generate images (Stage 3) if requested
    if with_images:
        if descriptions_path is None:
            click.echo("❌ Error: --with-images requires descriptions. Use --with-descriptions or generate descriptions first.")
            return
        
        click.echo("\n🖼️  Generating concept art (Stage 3)...")
        from progship.pipeline import ImagePipeline, ImageConfig, batch_process_images
        
        config = ImageConfig(
            resolution=image_resolution,
            num_inference_steps=image_steps
        )
        pipeline = ImagePipeline(image_config=config)
        
        image_manifest = pipeline.generate_from_manifest(
            manifest_path=descriptions_path,
            negative_prompt="blurry, low quality, distorted"
        )
        
        # Save image manifest
        images_path = output_path.parent / f"{output_path.stem}_images.json"
        image_manifest.save(images_path)
        click.echo(f"✓ Image manifest saved to: {images_path}")
        total_images = sum(len(c['images']) for c in image_manifest.components)
        click.echo(f"  Total images: {total_images}")
        
        # Post-process images if requested
        if process_images:
            click.echo("\n🔧 Post-processing images...")
            results = batch_process_images(
                images_path,
                auto_crop=True,
                generate_thumbnails=True,
                validate=True
            )
            
            click.echo(f"✓ Processed {results['total_processed']} images")
            if results['issues_found'] > 0:
                click.echo(f"⚠️  {results['issues_found']} images had quality issues")


@cli.command()
@click.argument('structure_file', type=click.Path(exists=True))
@click.option('--output', default=None, help='Output file path (default: <structure>_descriptions.json)')
@click.option('--no-cache', is_flag=True, help='Disable description caching')
def describe(structure_file: str, output: str, no_cache: bool):
    """Generate AI descriptions for an existing structure file (Stage 2)."""
    click.echo(f"🎨 Generating descriptions for {structure_file}...")
    
    # Load structure
    with open(structure_file, 'r', encoding='utf-8') as f:
        structure_data = json.load(f)
        structure = ShipStructure(**structure_data)
    
    # Generate descriptions
    desc_generator = DescriptionGenerator()
    manifest = desc_generator.generate_descriptions(
        structure,
        use_cache=not no_cache
    )
    
    # Determine output path
    if output is None:
        structure_path = Path(structure_file)
        output = structure_path.parent / f"{structure_path.stem}_descriptions.json"
    else:
        output = Path(output)
    
    # Save manifest
    desc_generator.save_manifest(manifest, output)
    click.echo(f"✓ Descriptions saved to: {output}")


@cli.command()
def cache_clear():
    """Clear all cached descriptions."""
    from progship.pipeline import DescriptionCache
    
    click.echo("🗑️  Clearing description cache...")
    cache = DescriptionCache()
    cache.clear()


@cli.command()
def cache_stats():
    """Show cache statistics."""
    from progship.pipeline import DescriptionCache
    
    cache = DescriptionCache()
    stats = cache.stats()
    
    click.echo("📊 Cache Statistics:")
    click.echo(f"  Total entries: {stats['total_entries']}")
    click.echo(f"  Total size: {stats['total_size_mb']} MB")


@cli.command()
def validate():
    """Validate all JSON data files against schemas."""
    click.echo("🔍 Validating data files...")
    validator = SchemaValidator()
    all_valid = validator.print_validation_report()
    
    if not all_valid:
        raise click.ClickException("Validation failed")


@cli.command()
def list_ship_types():
    """List available ship types."""
    loader = get_loader()
    db = loader.load_ship_types()
    
    click.echo("Available Ship Types:")
    click.echo("=" * 60)
    for ship_type in db.ship_types:
        click.echo(f"\n{ship_type.id}")
        click.echo(f"  Name: {ship_type.name}")
        click.echo(f"  Construction: {ship_type.construction_type}")
        click.echo(f"  Default Style: {ship_type.default_style}")
        click.echo(f"  Size: {ship_type.size_class}")


@cli.command()
def list_styles():
    """List available style descriptors."""
    loader = get_loader()
    db = loader.load_styles()
    
    click.echo("Available Styles:")
    click.echo("=" * 60)
    for style in db.style_descriptors:
        click.echo(f"\n{style.id}")
        click.echo(f"  Name: {style.name}")
        click.echo(f"  Materials: {', '.join(style.material_palette[:3])}")
        click.echo(f"  Colors: {', '.join(style.color_palette[:3])}")
        click.echo(f"  Wear: {style.wear_level}")


@cli.command()
def list_facilities():
    """List available facilities."""
    loader = get_loader()
    db = loader.load_facilities()
    
    click.echo("Available Facilities:")
    click.echo("=" * 60)
    for facility in db.facilities:
        click.echo(f"\n{facility.id}")
        click.echo(f"  Name: {facility.name}")
        click.echo(f"  Category: {facility.category}")
        click.echo(f"  Variants: {len(facility.variants)}")


@cli.command('generate-images')
@click.argument('descriptions_file', type=click.Path(exists=True))
@click.option('--output', default='output/structure_images.json', help='Output manifest path')
@click.option('--model', default='segmind_vega', help='Model to use: segmind_vega, janus_pro, flux_schnell')
@click.option('--resolution', default=1024, type=int, help='Image resolution (default: 1024)')
@click.option('--steps', default=25, type=int, help='Inference steps (default: 25)')
@click.option('--angles', is_flag=True, help='Generate multiple camera angles')
@click.option('--negative', default=None, help='Negative prompt')
@click.option('--process', is_flag=True, help='Auto-process images (crop, thumbnails)')
def generate_images_cmd(descriptions_file: str, output: str, model: str, resolution: int, 
                        steps: int, angles: bool, negative: str, process: bool):
    """Generate concept art images from descriptions (Stage 3)."""
    from progship.pipeline import ImagePipeline, batch_process_images, get_model_info
    
    # Show model info
    model_info = get_model_info(model)
    click.echo(f"🎨 Generating concept art with {model_info.name}...")
    click.echo(f"   License: {model_info.license}")
    click.echo(f"   Quality: {model_info.quality_tier}")
    
    # Create pipeline with selected model
    pipeline = ImagePipeline(
        model_type=model,
        resolution=resolution,
        num_inference_steps=steps
    )
    
    # Generate images
    manifest = pipeline.generate_from_manifest(
        manifest_path=Path(descriptions_file),
        negative_prompt=negative,
        generate_angles=angles
    )
    
    # Save manifest
    output_path = Path(output)
    manifest.save(output_path)
    
    click.echo(f"\n✓ Image manifest saved to: {output_path}")
    click.echo(f"  Components: {len(manifest.components)}")
    total_images = sum(len(c['images']) for c in manifest.components)
    click.echo(f"  Images: {total_images}")
    
    # Post-process images if requested
    if process:
        click.echo("\n🔧 Post-processing images...")
        results = batch_process_images(
            output_path,
            auto_crop=True,
            generate_thumbnails=True,
            validate=True
        )
        
        click.echo(f"✓ Processed {results['total_processed']} images")
        if results['issues_found'] > 0:
            click.echo(f"⚠️  {results['issues_found']} images had quality issues")
    
    # Post-process images if requested
    if process:
        click.echo("\n🔧 Post-processing images...")
        results = batch_process_images(
            output_path,
            auto_crop=True,
            generate_thumbnails=True,
            validate=True
        )
        
        click.echo(f"✓ Processed {results['total_processed']} images")
        if results['issues_found'] > 0:
            click.echo(f"⚠️  {results['issues_found']} images had quality issues")


@cli.command('list-models')
def list_models_cmd():
    """List available image generation models."""
    from progship.pipeline import list_available_models
    
    click.echo("Available Image Generation Models:")
    click.echo("=" * 80)
    
    for model_type, info in list_available_models():
        click.echo(f"\n{model_type} - {info.name}")
        click.echo(f"  License: {info.license}")
        click.echo(f"  Resolution: {info.native_resolution}px (native), up to {info.max_resolution}px")
        click.echo(f"  Quality: {info.quality_tier}")
        click.echo(f"  VRAM: {info.vram_required}GB required")
        click.echo(f"  Negative prompts: {'Yes' if info.supports_negative_prompts else 'No'}")
        click.echo(f"  Description: {info.description}")


@cli.command('process-images')
@click.argument('image_manifest', type=click.Path(exists=True))
@click.option('--output-dir', default='output/images_processed', help='Output directory')
@click.option('--no-crop', is_flag=True, help='Skip auto-cropping')
@click.option('--no-thumbs', is_flag=True, help='Skip thumbnail generation')
@click.option('--no-validate', is_flag=True, help='Skip quality validation')
def process_images_cmd(image_manifest: str, output_dir: str, no_crop: bool, 
                       no_thumbs: bool, no_validate: bool):
    """Post-process images (crop, resize, thumbnails, validation)."""
    from progship.pipeline import batch_process_images
    
    click.echo(f"🔧 Post-processing images from {image_manifest}...")
    
    results = batch_process_images(
        Path(image_manifest),
        Path(output_dir),
        auto_crop=not no_crop,
        generate_thumbnails=not no_thumbs,
        validate=not no_validate
    )
    
    click.echo(f"\n✓ Batch processing complete!")
    click.echo(f"  Total images: {results['total_images']}")
    click.echo(f"  Processed: {results['total_processed']}")
    
    if results['issues_found'] > 0:
        click.echo(f"  ⚠️  Quality issues: {results['issues_found']}")
    
    click.echo(f"\nProcessed images saved to: {output_dir}")
    click.echo(f"Results manifest: {output_dir}/processing_results.json")


def main():
    """Entry point for CLI."""
    cli()


if __name__ == '__main__':
    main()
