"""TRELLIS image-to-3D model wrapper for converting concept art to 3D meshes."""

from dataclasses import dataclass
from pathlib import Path
from typing import Optional, Dict, Any
from PIL import Image
import subprocess
import sys


@dataclass
class TrellisConfig:
    """Configuration for TRELLIS 3D generation."""
    model_id: str = "microsoft/TRELLIS-image-large"  # 1.2B params, best quality
    seed: int = 42
    sparse_structure_steps: int = 12
    sparse_structure_cfg: float = 7.5
    slat_steps: int = 12
    slat_cfg: float = 3.0
    simplify_ratio: float = 0.95  # Mesh simplification (0-1, higher = more simplification)
    texture_size: int = 1024  # Texture resolution for GLB export


class TrellisGenerator:
    """
    Wrapper for Microsoft TRELLIS image-to-3D generation.
    
    Converts concept art images into 3D meshes (GLB format).
    Supports dimension constraints and quality validation.
    """
    
    def __init__(self, config: Optional[TrellisConfig] = None):
        """Initialize TRELLIS generator with config."""
        self.config = config or TrellisConfig()
        self.pipeline = None
        self._model_loaded = False
        
    def check_installation(self) -> bool:
        """Check if TRELLIS is installed and accessible."""
        try:
            import trellis
            return True
        except ImportError:
            return False
    
    def install_instructions(self) -> str:
        """Return installation instructions for TRELLIS."""
        return """
TRELLIS Installation Instructions:
==================================

CRITICAL: TRELLIS requires Linux OS. Windows support is experimental.

Prerequisites:
  - Linux OS (tested on Ubuntu)
  - NVIDIA GPU with 16GB+ VRAM (tested on A100, A6000, RTX 4090)
  - CUDA Toolkit 11.8 or 12.2
  - conda (Miniconda recommended)
  - Python 3.8+

Installation Steps:
  1. Clone repository:
     git clone --recurse-submodules https://github.com/microsoft/TRELLIS.git
     cd TRELLIS
  
  2. Install dependencies:
     . ./setup.sh --new-env --basic --xformers --flash-attn --diffoctreerast --spconv --mipgaussian --kaolin --nvdiffrast
  
  3. Activate environment:
     conda activate trellis
  
  4. Verify installation:
     python -c "import trellis; print('TRELLIS installed successfully')"

Model Download:
  Models are auto-downloaded from HuggingFace on first run.
  TRELLIS-image-large: ~5GB download

Alternative (Windows - Experimental):
  See https://github.com/microsoft/TRELLIS/issues/3
  Not officially supported, may have issues.

For more details: https://github.com/microsoft/TRELLIS
"""
    
    def _load_model(self):
        """Lazy load the TRELLIS model."""
        if self._model_loaded:
            return
        
        if not self.check_installation():
            print("[ERROR] TRELLIS not installed!")
            print(self.install_instructions())
            raise ImportError("TRELLIS is not installed. See installation instructions above.")
        
        print(f"Loading TRELLIS model: {self.config.model_id}")
        print("This may take a while on first run (downloading ~5GB model)...")
        
        import os
        os.environ['SPCONV_ALGO'] = 'native'  # Faster for single runs
        
        from trellis.pipelines import TrellisImageTo3DPipeline
        
        # Load pipeline
        self.pipeline = TrellisImageTo3DPipeline.from_pretrained(self.config.model_id)
        self.pipeline.cuda()
        
        self._model_loaded = True
        print("[OK] TRELLIS model loaded")
    
    def generate(
        self,
        image_path: str | Path,
        seed: Optional[int] = None
    ) -> Dict[str, Any]:
        """
        Generate 3D mesh from image.
        
        Args:
            image_path: Path to input concept art image
            seed: Random seed for generation (uses config.seed if None)
            
        Returns:
            Dictionary with 'gaussian', 'radiance_field', and 'mesh' outputs
        """
        self._load_model()
        
        # Load image
        image = Image.open(image_path)
        
        # Use provided seed or config seed
        generation_seed = seed if seed is not None else self.config.seed
        
        print(f"Generating 3D mesh from: {image_path}")
        
        # Run pipeline
        outputs = self.pipeline.run(
            image,
            seed=generation_seed,
            sparse_structure_sampler_params={
                "steps": self.config.sparse_structure_steps,
                "cfg_strength": self.config.sparse_structure_cfg,
            },
            slat_sampler_params={
                "steps": self.config.slat_steps,
                "cfg_strength": self.config.slat_cfg,
            },
        )
        
        return outputs
    
    def export_glb(
        self,
        outputs: Dict[str, Any],
        output_path: str | Path,
        simplify: Optional[float] = None,
        texture_size: Optional[int] = None
    ):
        """
        Export 3D outputs to GLB file.
        
        Args:
            outputs: Output from generate() method
            output_path: Path to save GLB file
            simplify: Mesh simplification ratio (0-1, uses config if None)
            texture_size: Texture resolution (uses config if None)
        """
        from trellis.utils import postprocessing_utils
        
        simplify = simplify if simplify is not None else self.config.simplify_ratio
        texture_size = texture_size if texture_size is not None else self.config.texture_size
        
        # Convert to GLB
        glb = postprocessing_utils.to_glb(
            outputs['gaussian'][0],
            outputs['mesh'][0],
            simplify=simplify,
            texture_size=texture_size,
        )
        
        # Save GLB
        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        glb.export(str(output_path))
        
        print(f"[OK] Saved GLB: {output_path}")
    
    def save_ply(self, outputs: Dict[str, Any], output_path: str | Path):
        """
        Save 3D Gaussian as PLY file.
        
        Args:
            outputs: Output from generate() method
            output_path: Path to save PLY file
        """
        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        outputs['gaussian'][0].save_ply(str(output_path))
        print(f"[OK] Saved PLY: {output_path}")


def check_trellis_requirements() -> Dict[str, bool]:
    """
    Check if system meets TRELLIS requirements.
    
    Returns:
        Dictionary with requirement check results
    """
    import platform
    import torch
    
    checks = {}
    
    # OS check
    checks['linux'] = platform.system() == 'Linux'
    checks['windows'] = platform.system() == 'Windows'
    
    # CUDA check
    checks['cuda_available'] = torch.cuda.is_available()
    if checks['cuda_available']:
        checks['cuda_version'] = torch.version.cuda
        checks['gpu_name'] = torch.cuda.get_device_name(0)
        checks['gpu_memory_gb'] = torch.cuda.get_device_properties(0).total_memory / 1024**3
        checks['sufficient_vram'] = checks['gpu_memory_gb'] >= 16
    
    # TRELLIS installation
    checks['trellis_installed'] = False
    try:
        import trellis
        checks['trellis_installed'] = True
    except ImportError:
        pass
    
    return checks


if __name__ == "__main__":
    """Check TRELLIS installation status."""
    print("TRELLIS Requirements Check")
    print("="*80)
    
    checks = check_trellis_requirements()
    
    print(f"OS: {'Linux' if checks.get('linux') else 'Windows (experimental)'}")
    print(f"CUDA Available: {checks.get('cuda_available', False)}")
    
    if checks.get('cuda_available'):
        print(f"CUDA Version: {checks.get('cuda_version', 'N/A')}")
        print(f"GPU: {checks.get('gpu_name', 'N/A')}")
        print(f"GPU Memory: {checks.get('gpu_memory_gb', 0):.1f} GB")
        print(f"Sufficient VRAM (16GB+): {checks.get('sufficient_vram', False)}")
    
    print(f"\nTRELLIS Installed: {checks.get('trellis_installed', False)}")
    
    if not checks.get('trellis_installed'):
        print("\n" + "="*80)
        print("TRELLIS NOT INSTALLED")
        print("="*80)
        generator = TrellisGenerator()
        print(generator.install_instructions())
    else:
        print("\n[OK] TRELLIS is installed and ready to use!")
