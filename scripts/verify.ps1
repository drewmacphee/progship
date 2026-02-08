# ProgShip Quick Verify Script
# Dumps rooms/doors from running SpacetimeDB and verifies door placement

param(
    [string]$ServerUrl = "http://localhost:3000"
)

$ErrorActionPreference = "Stop"

Write-Host "=== ProgShip Door Verification ===" -ForegroundColor Cyan

Write-Host "`n[1/2] Dumping rooms and doors..." -ForegroundColor Yellow
spacetime sql progship "SELECT id, room_type, deck, x, y, width, height FROM room" -s $ServerUrl > rooms_dump.txt
spacetime sql progship "SELECT id, room_a, room_b, wall_a, wall_b, door_x, door_y, width FROM door" -s $ServerUrl > doors_dump.txt

Write-Host "`n[2/2] Running verification..." -ForegroundColor Yellow
python verify_doors.py

Write-Host "`n=== Done ===" -ForegroundColor Green
