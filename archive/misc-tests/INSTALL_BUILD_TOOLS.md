# Hunyuan3D-2 Build Tools Installation Guide

## Prerequisites
- ✅ Python 3.13
- ✅ PyTorch 2.6.0+cu124
- ✅ NVIDIA RTX 4090 with Driver 581.80
- ⏳ Visual Studio Build Tools (Installing)
- ⏳ CUDA Toolkit 12.4 (Installing)

## Step 1: Install Visual Studio Build Tools 2022

**Download**: https://visualstudio.microsoft.com/downloads/

1. Scroll to **"Tools for Visual Studio"**
2. Download **"Build Tools for Visual Studio 2022"** (~2 MB installer)
3. Run `vs_BuildTools.exe`
4. In the installer, select **"Desktop development with C++"** workload
5. Click **Install** (~7 GB download)
6. Installation time: ~15-20 minutes

**What this installs**:
- MSVC C++ compiler (cl.exe)
- Windows SDK
- CMake
- Build tools for compiling native extensions

## Step 2: Install NVIDIA CUDA Toolkit 12.4 (REQUIRED VERSION)

**⚠️ IMPORTANT**: You **must** use CUDA Toolkit 12.4, not 13.0!

**Why**: Your PyTorch is compiled with CUDA 12.4 (`PyTorch 2.6.0+cu124`). Using CUDA 13.0 will cause compilation errors when building custom extensions due to version mismatch.

**Download**: https://developer.nvidia.com/cuda-12-4-0-download-archive

**Direct link**: https://developer.nvidia.com/compute/cuda/12.4.0/local_installers/cuda_12.4.0_windows.exe

1. Download `cuda_12.4.0_windows.exe` (~3 GB)
2. Run installer
3. Choose **"Express (Recommended)"** installation
4. Installation time: ~10-15 minutes

**Your NVIDIA Driver**: 581.80 (supports CUDA 13.0) is **backward compatible** with CUDA Toolkit 12.4 ✅

**What this installs**:
- CUDA compiler (nvcc) version 12.4
- CUDA libraries matching PyTorch
- CUDA samples
- Sets CUDA_HOME environment variable to 12.4 path

**CUDA Version Compatibility**:
- Driver 581.80: ✅ Supports CUDA 13.0 (forward compatible)
- PyTorch 2.6.0+cu124: ✅ Requires CUDA Toolkit 12.4 for extensions
- Major version mismatch (12.x vs 13.x): ❌ Will break custom CUDA extension compilation

## Step 3: Verify Installation

After installation, **restart PowerShell** and verify:

```powershell
# Check MSVC compiler
where cl
# Should show: C:\Program Files\Microsoft Visual Studio\...\cl.exe

# Check CUDA compiler
nvcc --version
# Should show: release 12.4, V12.4.x

# Check CUDA_HOME
echo $env:CUDA_HOME
# Should show: C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.4
```

## Step 4: Compile Hunyuan3D-2 Custom Modules

```powershell
cd C:\GIT\progship\Hunyuan3D-2

# Compile custom_rasterizer
cd hy3dgen\texgen\custom_rasterizer
python setup.py install

# Compile differentiable_renderer
cd ..\differentiable_renderer
python setup.py install
```

Expected output:
```
Building wheel for custom_rasterizer...
Successfully installed custom_rasterizer
```

## Step 5: Test Hunyuan3D-2

```powershell
cd C:\GIT\progship
python test_hunyuan3d.py
```

Expected: Generate textured GLB with PBR materials in ~25-30 seconds.

## Troubleshooting

**Error: "cl.exe not found"**
- Open "Developer Command Prompt for VS 2022" instead of regular PowerShell
- Or add to PATH: `C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\...\bin\Hostx64\x64`

**Error: "nvcc not found"**
- Restart PowerShell after CUDA installation
- Manually set: `$env:CUDA_HOME="C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.4"`

**Error: "CUDA driver version insufficient"**
- Your driver (581.80) supports CUDA 13.0, which is compatible with CUDA 12.4 ✅

## Total Time Estimate

- VS Build Tools download + install: 20 min
- CUDA Toolkit download + install: 15 min
- Custom module compilation: 5 min
- **Total**: 40 minutes

## Why This Matters

Once installed, you'll be able to:
- ✅ Generate 3D models with **full PBR textures** (albedo, metallic, roughness)
- ✅ Use any CUDA-accelerated 3D model (StableFast3D, InstantMesh, etc.)
- ✅ Compile other CUDA-based Python packages
- ✅ Build custom CUDA kernels for optimization

This is a **one-time setup** that enables the entire ecosystem of modern AI 3D generation tools.
