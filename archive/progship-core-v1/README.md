# ProgShip Core - Engine-Agnostic Ship Generation Pipeline

A Python-based procedural ship generation system that uses local AI models to create structures, descriptions, concept art, and 3D models.

## Features (Planned)

- **Stage 1**: Structure generation (rooms, decks, components)
- **Stage 2**: AI-powered description generation (local LLM)
- **Stage 3**: Concept art generation (Flux.2 9b)
- **Stage 4**: 3D model generation (TRELLIS.2-4B)
- **Stage 5**: Animation generation (optional, awaiting model support)

## Installation

```bash
cd progship-core

# Using uv (recommended - much faster than pip)
uv venv
.venv\Scripts\activate  # Windows
source .venv/bin/activate  # Linux/Mac
uv pip install -r requirements.txt

# Or using standard pip
python -m venv .venv
.venv\Scripts\activate  # Windows
pip install -r requirements.txt
```

## Usage

```bash
# Initialize new project
python -m progship init

# Generate ship structure
python -m progship generate --ship-type "Colony Ship, rotating" --style "ceramic_white"

# Run specific stage only
python -m progship generate --stage structure

# Enable optional animation generation
python -m progship generate --enable-animations
```

## Project Status

**Phase 0: Project Restructuring** - In Progress
- [x] Archive Godot prototype
- [x] Create Python project structure
- [x] Set up virtual environment
- [x] Create initial requirements.txt
- [ ] Initialize package structure

## Architecture

See `docs/ARCHITECTURE.md` for detailed pipeline documentation.

## License

TBD
