"""Simple standalone test of Z-Image Turbo via Ollama API."""

import requests
import base64
from pathlib import Path
import json
import time

def test_zimage_generation():
    """Test Z-Image Turbo image generation via Ollama HTTP API."""
    
    prompt = (
        "A futuristic spacecraft bridge with ceramic white materials, "
        "minimalist design, clean modern aesthetic, command console with "
        "holographic displays, ambient blue lighting, professional concept art, "
        "highly detailed, 8k resolution"
    )
    
    print("Testing Z-Image Turbo via Ollama")
    print("="*80)
    print(f"Prompt: {prompt}\n")
    print("Generating image...")
    
    # Call Ollama API
    api_url = "http://localhost:11434/api/generate"
    payload = {
        "model": "x/z-image-turbo",
        "prompt": prompt,
        "stream": False,
        "options": {
            "seed": 42
        }
    }
    
    start = time.time()
    
    try:
        response = requests.post(api_url, json=payload, timeout=300)
        response.raise_for_status()
        
        elapsed = time.time() - start
        data = response.json()
        
        print(f"\n[OK] API call completed in {elapsed:.1f}s")
        print(f"Response keys: {list(data.keys())}")
        
        # Try to extract image
        if "images" in data and data["images"]:
            print(f"\nFound 'images' field with {len(data['images'])} image(s)")
            image_b64 = data["images"][0]
        elif "response" in data:
            print(f"\nFound 'response' field:")
            print(f"  Length: {len(data['response'])} chars")
            
            # Check if response looks like base64
            response_text = data["response"]
            if len(response_text) > 1000 and all(c in 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=' for c in response_text[:100]):
                image_b64 = response_text
                print("  Looks like base64 image data")
            else:
                print(f"  First 200 chars: {response_text[:200]}")
                print("\n[ERROR] Response doesn't look like image data")
                return
        else:
            print(f"\n[WARN] No 'images' or 'response' field found")
            print(f"Full response: {json.dumps(data, indent=2)[:500]}")
            return
        
        # Save image
        output_path = Path("test_output/zimage_turbo_test.png")
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        image_data = base64.b64decode(image_b64)
        output_path.write_bytes(image_data)
        
        print(f"\n[SUCCESS] Image saved to: {output_path}")
        print(f"  Size: {len(image_data):,} bytes")
        
    except requests.exceptions.Timeout:
        print(f"\n[ERROR] Request timed out after 300s")
    except requests.exceptions.HTTPError as e:
        print(f"\n[ERROR] HTTP {e.response.status_code}: {e}")
        print(f"Response: {e.response.text[:500]}")
    except Exception as e:
        print(f"\n[ERROR] {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    test_zimage_generation()
