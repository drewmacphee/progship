"""Test image post-processing utilities."""

import sys
from pathlib import Path

# Add parent to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from progship.pipeline.image_processing import ImageProcessor, process_image
from PIL import Image

def test_processing():
    """Test image processing on generated images."""
    
    print("=" * 60)
    print("Testing Image Post-Processing")
    print("=" * 60)
    
    # Find test image
    test_image = Path("output/images/bridge/bridge_main.png")
    
    if not test_image.exists():
        print(f"❌ Test image not found: {test_image}")
        print("Run image generation first: progship generate --with-images")
        return
    
    print(f"\nProcessing: {test_image}")
    
    # Load image
    image = Image.open(test_image)
    print(f"Original size: {image.size}")
    
    processor = ImageProcessor()
    
    # Test quality validation
    print("\n1. Quality Validation:")
    validation = processor.validate_quality(image)
    print(f"   Valid: {validation['valid']}")
    print(f"   Mean RGB: {[f'{v:.1f}' for v in validation['metrics']['mean_rgb']]}")
    print(f"   Std Dev: {[f'{v:.1f}' for v in validation['metrics']['stddev_rgb']]}")
    if validation['issues']:
        print(f"   Issues: {', '.join(validation['issues'])}")
    
    # Test auto-crop
    print("\n2. Auto-Crop:")
    cropped = processor.auto_crop(image)
    print(f"   Cropped size: {cropped.size}")
    print(f"   Reduction: {image.size[0] - cropped.size[0]}px width, {image.size[1] - cropped.size[1]}px height")
    
    # Test thumbnail generation
    print("\n3. Thumbnail Generation:")
    thumb_256 = processor.generate_thumbnail(cropped, (256, 256))
    print(f"   256x256 thumbnail: {thumb_256.size}")
    thumb_512 = processor.generate_thumbnail(cropped, (512, 512))
    print(f"   512x512 thumbnail: {thumb_512.size}")
    
    # Test full processing pipeline
    print("\n4. Full Processing Pipeline:")
    output_dir = Path("output/images_processed_test")
    results = process_image(
        test_image,
        output_dir,
        auto_crop=True,
        generate_thumbnails=True,
        validate=True
    )
    
    print(f"   Output directory: {output_dir}")
    print(f"   Files created:")
    for file_info in results['processed_files']:
        print(f"     - {file_info['type']}: {Path(file_info['path']).name} ({file_info['size']})")
    
    print(f"\n✓ Test complete! Check {output_dir} for processed images")

if __name__ == "__main__":
    test_processing()
