"""Test script for Segmind-Vega image generation."""

import sys
from pathlib import Path

# Add parent to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from progship.pipeline.image_generator import FluxImageGenerator, ImageConfig

def test_flux():
    """Test Segmind-Vega with a simple prompt."""
    
    print("=" * 60)
    print("Testing Segmind-Vega Image Generation (Apache 2.0, ungated)")
    print("=" * 60)
    
    # Create config
    config = ImageConfig(
        resolution=512,  # Start with smaller resolution for faster test
        seed=42
    )
    
    # Create generator
    generator = FluxImageGenerator(config)
    
    # Test prompt
    test_prompt = "A futuristic reactor core in a spaceship, white ceramic materials, clean aesthetic, technical lighting"
    
    print(f"\nPrompt: {test_prompt}")
    print("\nGenerating image...")
    
    # Generate
    result = generator.generate(test_prompt)
    
    # Save
    output_path = Path("test_output/flux_test.png")
    generator.save_image(result, output_path)
    
    print(f"\n✓ Test complete! Check {output_path}")
    print(f"  Seed: {result['seed']}")
    print(f"  Resolution: {config.resolution}x{config.resolution}")

if __name__ == "__main__":
    test_flux()
