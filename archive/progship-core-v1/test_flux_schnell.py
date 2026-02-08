"""Test FLUX.1-schnell with HuggingFace token."""

import sys
import os
from pathlib import Path
import time

# Set token
os.environ['HF_TOKEN'] = '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'

sys.path.insert(0, str(Path(__file__).parent.parent))

from progship.pipeline.model_registry import create_generator, ModelType

def test_flux_schnell():
    """Test FLUX.1-schnell image generation."""
    
    prompt = (
        "A futuristic spacecraft bridge with ceramic white materials, "
        "minimalist design, clean modern aesthetic, command console with "
        "holographic displays, ambient blue lighting, professional concept art, "
        "highly detailed, 8k resolution"
    )
    
    print("Testing FLUX.1-schnell")
    print("="*80)
    print(f"Prompt: {prompt}\n")
    print("Loading model (first run may take a few minutes to download)...")
    
    try:
        generator = create_generator(ModelType.FLUX_SCHNELL)
        
        output_path = Path("test_output/flux_schnell_test.png")
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        print("Generating image...")
        start = time.time()
        
        result = generator.generate(
            prompt=prompt,
            seed=42,
            output_path=output_path
        )
        
        elapsed = time.time() - start
        
        print(f"\n[SUCCESS] Generated in {elapsed:.1f}s")
        print(f"  Output: {output_path}")
        print(f"  Resolution: {result['resolution']}")
        print(f"  Model: {result.get('model', 'FLUX.1-schnell')}")
        
    except Exception as e:
        print(f"\n[ERROR] {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    test_flux_schnell()
