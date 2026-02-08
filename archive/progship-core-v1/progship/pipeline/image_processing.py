"""
Image post-processing utilities.
Handles cropping, resizing, thumbnail generation, and quality validation.
"""

from pathlib import Path
from typing import Optional, Tuple, List, Dict, Any
from PIL import Image, ImageStat, ImageOps
import numpy as np
import json


class ImageProcessor:
    """Utility class for image post-processing operations."""
    
    @staticmethod
    def auto_crop(image: Image.Image, threshold: int = 10) -> Image.Image:
        """
        Auto-crop whitespace from image borders.
        
        Args:
            image: PIL Image to crop
            threshold: Pixel value threshold for detecting content (0-255)
            
        Returns:
            Cropped PIL Image
        """
        # Convert to grayscale for content detection
        gray = image.convert('L')
        np_image = np.array(gray)
        
        # Find rows and columns with content (below threshold means dark = content)
        # Invert logic: we want to keep areas with significant variation from white
        mask = np_image < (255 - threshold)
        
        # Find bounding box of content
        rows = np.any(mask, axis=1)
        cols = np.any(mask, axis=0)
        
        if not rows.any() or not cols.any():
            # No content detected, return original
            return image
        
        row_start, row_end = np.where(rows)[0][[0, -1]]
        col_start, col_end = np.where(cols)[0][[0, -1]]
        
        # Add small padding (5% of dimension)
        height, width = np_image.shape
        pad_h = max(int(height * 0.05), 10)
        pad_w = max(int(width * 0.05), 10)
        
        row_start = max(0, row_start - pad_h)
        row_end = min(height, row_end + pad_h)
        col_start = max(0, col_start - pad_w)
        col_end = min(width, col_end + pad_w)
        
        # Crop image
        return image.crop((col_start, row_start, col_end, row_end))
    
    @staticmethod
    def center_and_pad(
        image: Image.Image, 
        target_size: Tuple[int, int],
        background_color: Tuple[int, int, int, int] = (255, 255, 255, 255)
    ) -> Image.Image:
        """
        Center image in a frame of target size, padding with background color.
        
        Args:
            image: PIL Image to center
            target_size: Target (width, height)
            background_color: RGBA background color (default: white)
            
        Returns:
            Centered and padded PIL Image
        """
        # Resize image to fit within target size while maintaining aspect ratio
        image.thumbnail(target_size, Image.Resampling.LANCZOS)
        
        # Create new image with background color
        new_image = Image.new('RGBA', target_size, background_color)
        
        # Calculate position to paste (center)
        paste_x = (target_size[0] - image.width) // 2
        paste_y = (target_size[1] - image.height) // 2
        
        # Paste image onto background
        if image.mode == 'RGBA':
            new_image.paste(image, (paste_x, paste_y), image)
        else:
            new_image.paste(image, (paste_x, paste_y))
        
        return new_image
    
    @staticmethod
    def resize(
        image: Image.Image, 
        size: Tuple[int, int],
        maintain_aspect: bool = True
    ) -> Image.Image:
        """
        Resize image to target size.
        
        Args:
            image: PIL Image to resize
            size: Target (width, height)
            maintain_aspect: If True, maintains aspect ratio and fits within size
            
        Returns:
            Resized PIL Image
        """
        if maintain_aspect:
            image_copy = image.copy()
            image_copy.thumbnail(size, Image.Resampling.LANCZOS)
            return image_copy
        else:
            return image.resize(size, Image.Resampling.LANCZOS)
    
    @staticmethod
    def generate_thumbnail(
        image: Image.Image,
        size: Tuple[int, int] = (256, 256)
    ) -> Image.Image:
        """
        Generate thumbnail of image.
        
        Args:
            image: PIL Image to thumbnail
            size: Thumbnail size (default: 256x256)
            
        Returns:
            Thumbnail PIL Image
        """
        thumb = image.copy()
        thumb.thumbnail(size, Image.Resampling.LANCZOS)
        return thumb
    
    @staticmethod
    def validate_quality(image: Image.Image) -> Dict[str, Any]:
        """
        Validate image quality and detect common issues.
        
        Args:
            image: PIL Image to validate
            
        Returns:
            Dict with validation results and issues
        """
        issues = []
        metrics = {}
        
        # Convert to RGB for analysis
        rgb_image = image.convert('RGB')
        
        # Check dimensions
        width, height = image.size
        metrics['width'] = width
        metrics['height'] = height
        
        if width < 256 or height < 256:
            issues.append(f"Image too small: {width}x{height} (min 256x256)")
        
        # Check if image is completely black or white
        stat = ImageStat.Stat(rgb_image)
        mean_values = stat.mean
        metrics['mean_rgb'] = mean_values
        
        # Check for near-black image (all channels < 10)
        if all(v < 10 for v in mean_values):
            issues.append("Image is nearly black (possible generation failure)")
        
        # Check for near-white image (all channels > 245)
        if all(v > 245 for v in mean_values):
            issues.append("Image is nearly white (possible generation failure)")
        
        # Check contrast (standard deviation)
        stddev_values = stat.stddev
        metrics['stddev_rgb'] = stddev_values
        avg_stddev = sum(stddev_values) / len(stddev_values)
        
        if avg_stddev < 10:
            issues.append(f"Very low contrast (stddev: {avg_stddev:.1f})")
        
        # Check for excessive noise/artifacts (high variance in small regions)
        # Sample 10x10 patches and check variance
        np_image = np.array(rgb_image)
        if width > 100 and height > 100:
            # Sample center patch
            center_patch = np_image[
                height//2-50:height//2+50,
                width//2-50:width//2+50
            ]
            patch_variance = np.var(center_patch)
            metrics['center_patch_variance'] = float(patch_variance)
            
            # Very high variance might indicate noise
            if patch_variance > 5000:
                issues.append(f"High noise detected (variance: {patch_variance:.0f})")
        
        return {
            'valid': len(issues) == 0,
            'issues': issues,
            'metrics': metrics
        }


def process_image(
    input_path: Path,
    output_dir: Path,
    auto_crop: bool = True,
    generate_thumbnails: bool = True,
    validate: bool = True,
    sizes: Optional[List[Tuple[int, int]]] = None
) -> Dict[str, Any]:
    """
    Process a single image with all post-processing steps.
    
    Args:
        input_path: Path to input image
        output_dir: Directory for processed images
        auto_crop: Whether to auto-crop whitespace
        generate_thumbnails: Whether to generate thumbnails
        validate: Whether to validate quality
        sizes: List of (width, height) tuples for additional sizes
        
    Returns:
        Dict with processing results and output paths
    """
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Load image
    image = Image.open(input_path)
    original_size = image.size
    
    processor = ImageProcessor()
    results = {
        'input_path': str(input_path),
        'original_size': original_size,
        'processed_files': []
    }
    
    # Validate quality
    if validate:
        validation = processor.validate_quality(image)
        results['validation'] = validation
        
        if not validation['valid']:
            print(f"⚠️  Quality issues in {input_path.name}: {', '.join(validation['issues'])}")
    
    # Auto-crop
    if auto_crop:
        image = processor.auto_crop(image)
        results['cropped_size'] = image.size
    
    # Save main processed image
    main_output = output_dir / f"{input_path.stem}_processed.png"
    image.save(main_output, 'PNG', optimize=True)
    results['processed_files'].append({
        'type': 'main',
        'path': str(main_output),
        'size': image.size
    })
    
    # Generate thumbnails
    if generate_thumbnails:
        thumb_sizes = [(256, 256), (512, 512)] if sizes is None else sizes
        
        for thumb_size in thumb_sizes:
            thumb = processor.generate_thumbnail(image, thumb_size)
            thumb_path = output_dir / f"{input_path.stem}_thumb_{thumb_size[0]}x{thumb_size[1]}.png"
            thumb.save(thumb_path, 'PNG', optimize=True)
            results['processed_files'].append({
                'type': 'thumbnail',
                'path': str(thumb_path),
                'size': thumb.size
            })
    
    # Save processing metadata
    metadata_path = output_dir / f"{input_path.stem}_processing.json"
    with open(metadata_path, 'w') as f:
        json.dump(results, f, indent=2)
    
    return results


def batch_process_images(
    image_manifest_path: Path,
    output_base_dir: Path = Path("output/images_processed"),
    **process_kwargs
) -> Dict[str, Any]:
    """
    Batch process all images from an image manifest.
    
    Args:
        image_manifest_path: Path to image manifest JSON
        output_base_dir: Base directory for processed images
        **process_kwargs: Additional arguments for process_image()
        
    Returns:
        Dict with batch processing results
    """
    # Load manifest
    with open(image_manifest_path, 'r') as f:
        manifest = json.load(f)
    
    # Get manifest directory for resolving relative paths
    manifest_dir = image_manifest_path.parent
    
    results = {
        'manifest_path': str(image_manifest_path),
        'components': [],
        'total_images': 0,
        'total_processed': 0,
        'issues_found': 0,
        'skipped': 0
    }
    
    # Process each component's images
    for component in manifest['components']:
        component_id = component['component_id']
        component_results = {
            'component_id': component_id,
            'images': []
        }
        
        for image_info in component['images']:
            results['total_images'] += 1
            image_path_str = image_info['path']
            
            # Resolve path relative to manifest directory
            image_path = manifest_dir / image_path_str
            
            if not image_path.exists():
                print(f"⚠️  Image not found: {image_path}")
                results['skipped'] += 1
                continue
            
            # Create output directory for this component
            output_dir = output_base_dir / component_id
            
            # Process image
            try:
                process_result = process_image(
                    image_path,
                    output_dir,
                    **process_kwargs
                )
                
                component_results['images'].append(process_result)
                results['total_images'] += 1
                results['total_processed'] += 1
                
                # Check for issues
                if 'validation' in process_result and not process_result['validation']['valid']:
                    results['issues_found'] += 1
                
            except Exception as e:
                print(f"❌ Error processing {image_path}: {e}")
                component_results['images'].append({
                    'input_path': str(image_path),
                    'error': str(e)
                })
        
        results['components'].append(component_results)
    
    # Save batch results
    results_path = output_base_dir / "processing_results.json"
    results_path.parent.mkdir(parents=True, exist_ok=True)
    with open(results_path, 'w') as f:
        json.dump(results, f, indent=2)
    
    return results
