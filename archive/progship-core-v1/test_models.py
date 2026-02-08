"""Test different image models and compare results."""

import sys
from pathlib import Path
import time

# Add parent to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from progship.pipeline.model_registry import ModelType, create_generator, get_model_info


def test_model(model_type: str, prompt: str):
    """Test a specific model with a prompt."""
    print(f"\n{'='*80}")
    
    # Get model info
    info = get_model_info(model_type)
    print(f"Testing: {info.name}")
    print(f"License: {info.license}")
    print(f"Quality Tier: {info.quality_tier}")
    print(f"Native Resolution: {info.native_resolution}px")
    print(f"{'='*80}")
    
    # Create generator
    try:
        if model_type == ModelType.JANUS_PRO:
            generator = create_generator(model_type, resolution=384)
        elif model_type == ModelType.FLUX_SCHNELL:
            generator = create_generator(model_type, num_inference_steps=4)
        else:
            generator = create_generator(model_type)
        
        # Test generation
        output_path = Path(f"test_output/{model_type}_test.png")
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        print(f"\nGenerating image...")
        print(f"Prompt: {prompt}")
        
        start_time = time.time()
        result = generator.generate(
            prompt=prompt,
            seed=42,
            output_path=output_path
        )
        elapsed = time.time() - start_time
        
        print(f"\nGenerated in {elapsed:.1f}s")
        print(f"  Output: {output_path}")
        print(f"  Resolution: {result['resolution']}")
        print(f"  Seed: {result['seed']}")
        
        return True
        
    except Exception as e:
        print(f"\nError: {e}")
        print(f"   Model may not be installed yet")
        return False


def main():
    """Test all available models."""
    
    test_prompt = (
        "A futuristic spacecraft bridge with ceramic white materials, "
        "minimalist design, clean modern aesthetic, command console with "
        "holographic displays, ambient blue lighting, professional concept art, "
        "highly detailed, 8k resolution"
    )
    
    print("Testing Image Generation Models")
    print("="*80)
    print(f"\nTest prompt: {test_prompt}")
    
    models_to_test = [
        (ModelType.SEGMIND_VEGA, "Current default"),
        (ModelType.FLUX_SCHNELL, "Fast, Apache 2.0, 12B params - RECOMMENDED"),
        (ModelType.JANUS_PRO, "Best quality, requires setup"),
    ]
    
    results = {}
    for model_type, description in models_to_test:
        print(f"\n\n{'='*80}")
        print(f"{description}")
        results[model_type] = test_model(model_type, test_prompt)
    
    # Summary
    print(f"\n\n{'='*80}")
    print("SUMMARY")
    print(f"{'='*80}")
    for model_type, success in results.items():
        status = "[OK] Success" if success else "[FAIL] Failed/Not installed"
        print(f"{model_type}: {status}")
    
    print(f"\n\nNext Steps:")
    print(f"1. Check test_output/ folder for generated images")
    print(f"2. Compare quality and decide which model to use")
    print(f"3. Use --model flag: progship generate-images --model flux_schnell ...")


if __name__ == "__main__":
    main()
