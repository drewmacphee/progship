"""Quick test of Janus-Pro via Ollama."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from progship.pipeline.ollama_image_generator import OllamaImageGenerator, OllamaImageConfig

def test_ollama_janus():
    """Test Janus-Pro image generation via Ollama."""
    
    prompt = (
        "A futuristic spacecraft bridge with ceramic white materials, "
        "minimalist design, clean modern aesthetic, command console with "
        "holographic displays, ambient blue lighting, professional concept art, "
        "highly detailed, 8k resolution"
    )
    
    print("Testing DeepSeek Janus-Pro-7B via Ollama")
    print("="*80)
    print(f"Prompt: {prompt}\n")
    
    generator = OllamaImageGenerator(
        OllamaImageConfig(model="erwan2/DeepSeek-Janus-Pro-7B")
    )
    
    output_path = Path("test_output/ollama_janus_test.png")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    print("Generating image...")
    
    import time
    start = time.time()
    
    try:
        result = generator.generate(
            prompt=prompt,
            seed=42,
            output_path=output_path
        )
        
        elapsed = time.time() - start
        
        print(f"\n[OK] Generated in {elapsed:.1f}s")
        print(f"  Output: {output_path}")
        print(f"  Resolution: {result['resolution']}")
        print(f"  Model: {result['model']}")
        
    except Exception as e:
        print(f"\n[ERROR] {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    test_ollama_janus()
