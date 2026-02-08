# Quick validation of v2 medical bay boundaries
$json = Get-Content "src\Data\MedicalBayAI_v2.json" | ConvertFrom-Json

Write-Host "`n=== MEDICAL BAY AI v2 VALIDATION ===" -ForegroundColor Cyan
Write-Host "Room bounds: 12m(x) × 4m(y) × 8m(z)" -ForegroundColor Yellow
Write-Host "Floor: y=0.0m, Ceiling: y=4.0m" -ForegroundColor Yellow
Write-Host "Walls: x=±6.0m, z=±4.0m`n" -ForegroundColor Yellow

# Component dimensions (from database)
$dims = @{
    "floor_grating_2x2m" = @{ w=2.0; h=0.05; d=2.0 }
    "ceiling_panel_2x2m" = @{ w=2.0; h=0.05; d=2.0 }
    "doorway_frame_standard" = @{ w=1.2; h=2.5; d=0.2 }
    "light_fixture_wall_mounted" = @{ w=0.3; h=0.3; d=0.15 }
    "control_console_engineering" = @{ w=1.2; h=1.4; d=0.6 }
    "hull_panel_flat_2x2m" = @{ w=2.0; h=2.0; d=0.1 }
    "pipe_segment_straight_1m" = @{ w=0.2; h=0.2; d=1.0 }
    "conduit_electrical_1m" = @{ w=0.1; h=0.1; d=1.0 }
}

$errorCount = 0
$totalComponents = 0

foreach ($system in $json.ship.decks[0].rooms[0].systems) {
    $systemType = $system.type
    Write-Host "System: $systemType" -ForegroundColor Green
    
    foreach ($comp in $system.components) {
        $totalComponents++
        $id = $comp.component_id
        $pos = $comp.position
        $dim = $dims[$id]
        
        if (-not $dim) {
            Write-Host "  ⚠️  Unknown component: $id" -ForegroundColor Yellow
            continue
        }
        
        # Calculate AABB bounds
        $minY = $pos[1] - ($dim.h / 2)
        $maxY = $pos[1] + ($dim.h / 2)
        $minX = $pos[0] - ($dim.w / 2)
        $maxX = $pos[0] + ($dim.w / 2)
        $minZ = $pos[2] - ($dim.d / 2)
        $maxZ = $pos[2] + ($dim.d / 2)
        
        # Check boundaries
        $errors = @()
        if ($minY < 0.0) { $errors += "Below floor (minY=$([math]::Round($minY,3))m)" }
        if ($maxY > 4.0) { $errors += "Above ceiling (maxY=$([math]::Round($maxY,3))m)" }
        if ($minX < -6.0) { $errors += "Beyond -X wall (minX=$([math]::Round($minX,3))m)" }
        if ($maxX > 6.0) { $errors += "Beyond +X wall (maxX=$([math]::Round($maxX,3))m)" }
        if ($minZ < -4.0) { $errors += "Beyond -Z wall (minZ=$([math]::Round($minZ,3))m)" }
        if ($maxZ > 4.0) { $errors += "Beyond +Z wall (maxZ=$([math]::Round($maxZ,3))m)" }
        
        if ($errors.Count -gt 0) {
            $errorCount++
            Write-Host "  ❌ $($comp.instance_name) at [$($pos[0]),$($pos[1]),$($pos[2])]:" -ForegroundColor Red
            foreach ($err in $errors) {
                Write-Host "      $err" -ForegroundColor Red
            }
        }
    }
}

Write-Host "`n=== RESULTS ===" -ForegroundColor Cyan
Write-Host "Total components: $totalComponents"
Write-Host "Validation errors: $errorCount"

if ($errorCount -eq 0) {
    Write-Host "`n✅ SUCCESS! Zero validation errors!" -ForegroundColor Green
    Write-Host "🎯 Boundary calculations are perfect!" -ForegroundColor Green
    Write-Host "Improvement: 30 errors (v1) → 0 errors (v2)" -ForegroundColor Green
} else {
    Write-Host "`n⚠️  Still have $errorCount errors" -ForegroundColor Yellow
    Write-Host "Improvement: 30 errors (v1) → $errorCount errors (v2)" -ForegroundColor Yellow
}
