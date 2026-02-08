# ProgShip Full Rebuild Script
# Builds server, publishes, initializes ship, verifies doors, builds client

param(
    [string]$ServerUrl = "http://localhost:3000",
    [string]$ShipName = "Test Ship",
    [int]$Decks = 21,
    [int]$Length = 100,
    [int]$Radius = 50
)

$ErrorActionPreference = "Stop"

Write-Host "=== ProgShip Full Rebuild ===" -ForegroundColor Cyan

# Step 1: Build server
Write-Host "`n[1/6] Building server..." -ForegroundColor Yellow
spacetime build --project-path crates/progship-server
if ($LASTEXITCODE -ne 0) { Write-Host "Server build FAILED" -ForegroundColor Red; exit 1 }

# Step 2: Publish
Write-Host "`n[2/6] Publishing to SpacetimeDB..." -ForegroundColor Yellow
spacetime publish --clear-database -y --project-path crates/progship-server progship -s $ServerUrl
if ($LASTEXITCODE -ne 0) { Write-Host "Publish FAILED" -ForegroundColor Red; exit 1 }

# Step 3: Init ship
Write-Host "`n[3/6] Initializing ship..." -ForegroundColor Yellow
spacetime call progship init_ship "`"$ShipName`"" $Decks $Length $Radius -s $ServerUrl
if ($LASTEXITCODE -ne 0) { Write-Host "Init FAILED" -ForegroundColor Red; exit 1 }

# Step 4: Dump and verify
Write-Host "`n[4/6] Dumping rooms and doors..." -ForegroundColor Yellow
spacetime sql progship "SELECT id, room_type, deck, x, y, width, height FROM room" -s $ServerUrl > rooms_dump.txt
spacetime sql progship "SELECT id, room_a, room_b, wall_a, wall_b, door_x, door_y, width FROM door" -s $ServerUrl > doors_dump.txt

Write-Host "`n[5/6] Verifying doors..." -ForegroundColor Yellow
python verify_doors.py
if ($LASTEXITCODE -ne 0) { Write-Host "Door verification FAILED" -ForegroundColor Red; exit 1 }

# Step 5: Build client
Write-Host "`n[6/6] Building client..." -ForegroundColor Yellow
cargo build --package progship-client
if ($LASTEXITCODE -ne 0) { Write-Host "Client build FAILED" -ForegroundColor Red; exit 1 }

Write-Host "`n=== Rebuild Complete ===" -ForegroundColor Green
Write-Host "Run the client with: cargo run --package progship-client"
