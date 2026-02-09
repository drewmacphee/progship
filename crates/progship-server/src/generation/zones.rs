//! Empty zone identification for room placement.
//!
//! Scans the stamped ship grid to find large rectangular regions where rooms
//! can be packed using the treemap algorithm.

/// Rectangular zone on the grid where rooms can be placed.
pub(super) struct GridZone {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

/// Finds large empty rectangular zones in the grid for room placement.
/// Uses a row-run scanning approach with vertical merging.
pub(super) fn find_empty_zones(
    grid: &[Vec<u8>],
    width: usize,
    height: usize,
    cell_empty: u8,
) -> Vec<GridZone> {
    // Simple row-run based approach: scan rows, find horizontal runs of empty,
    // then merge vertically adjacent runs with matching x-ranges.
    let mut zones: Vec<GridZone> = Vec::new();

    // Track which cells are already claimed by a zone
    let mut claimed = vec![vec![false; height]; width];

    for x in 0..width {
        for y in 0..height {
            if grid[x][y] != cell_empty || claimed[x][y] {
                continue;
            }

            // Find the widest run starting at (x, y)
            let mut run_w = 0;
            while x + run_w < width && grid[x + run_w][y] == cell_empty && !claimed[x + run_w][y] {
                run_w += 1;
            }
            if run_w < 3 {
                continue;
            } // too narrow for a room

            // Extend downward while the same x-range is all empty
            let mut run_h = 1;
            'outer: while y + run_h < height {
                for xx in x..(x + run_w) {
                    if grid[xx][y + run_h] != cell_empty || claimed[xx][y + run_h] {
                        break 'outer;
                    }
                }
                run_h += 1;
            }

            if run_h < 3 {
                continue;
            } // too short

            // Claim these cells
            for xx in x..(x + run_w) {
                for yy in y..(y + run_h) {
                    claimed[xx][yy] = true;
                }
            }

            zones.push(GridZone {
                x,
                y,
                w: run_w,
                h: run_h,
            });
        }
    }

    // Sort largest-first
    zones.sort_by(|a, b| (b.w * b.h).cmp(&(a.w * a.h)));
    zones
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_zones_on_simple_grid() {
        // Create a 10x10 grid with a rectangular empty region
        let width = 10;
        let height = 10;
        let mut grid = vec![vec![1u8; height]; width];

        // Create a 6x5 empty zone at (2, 2)
        for x in 2..8 {
            for y in 2..7 {
                grid[x][y] = 0;
            }
        }

        let zones = find_empty_zones(&grid, width, height, 0);

        assert!(!zones.is_empty(), "Should find at least one zone");
        assert_eq!(zones[0].x, 2, "Zone should start at x=2");
        assert_eq!(zones[0].y, 2, "Zone should start at y=2");
        assert_eq!(zones[0].w, 6, "Zone width should be 6");
        assert_eq!(zones[0].h, 5, "Zone height should be 5");
    }

    #[test]
    fn test_zones_no_overlap_with_infrastructure() {
        // Create a grid with infrastructure (1) and empty space (0)
        let width = 15;
        let height = 15;
        let mut grid = vec![vec![0u8; height]; width];

        // Add some infrastructure "walls"
        for y in 0..height {
            grid[5][y] = 1; // Vertical wall at x=5
            grid[10][y] = 1; // Vertical wall at x=10
        }

        let zones = find_empty_zones(&grid, width, height, 0);

        // Verify no zone overlaps with walls
        for zone in &zones {
            for x in zone.x..(zone.x + zone.w) {
                for y in zone.y..(zone.y + zone.h) {
                    assert_eq!(
                        grid[x][y], 0,
                        "Zone at ({}, {}) overlaps with infrastructure",
                        x, y
                    );
                }
            }
        }
    }

    #[test]
    fn test_zones_cover_empty_space() {
        // Create a grid with a large empty region
        let width = 20;
        let height = 20;
        let mut grid = vec![vec![0u8; height]; width];

        // Add border infrastructure
        for x in 0..width {
            grid[x][0] = 1;
            grid[x][height - 1] = 1;
        }
        for y in 0..height {
            grid[0][y] = 1;
            grid[width - 1][y] = 1;
        }

        let zones = find_empty_zones(&grid, width, height, 0);

        // Count empty cells that should be covered
        let mut covered = vec![vec![false; height]; width];
        for zone in &zones {
            for x in zone.x..(zone.x + zone.w) {
                for y in zone.y..(zone.y + zone.h) {
                    covered[x][y] = true;
                }
            }
        }

        // Check that all sizeable empty regions are covered
        let mut uncovered_empty = 0;
        for x in 1..width - 1 {
            for y in 1..height - 1 {
                if grid[x][y] == 0 && !covered[x][y] {
                    uncovered_empty += 1;
                }
            }
        }

        // Some cells might be uncovered if they're too small for the 3x3 minimum
        assert!(
            uncovered_empty < 50,
            "Too many empty cells uncovered: {}",
            uncovered_empty
        );
    }

    #[test]
    fn test_zones_reject_small_areas() {
        // Create a grid with tiny empty regions (smaller than 3x3 minimum)
        let width = 10;
        let height = 10;
        let mut grid = vec![vec![1u8; height]; width];

        // Create a 2x2 empty region (too small)
        grid[5][5] = 0;
        grid[6][5] = 0;
        grid[5][6] = 0;
        grid[6][6] = 0;

        let zones = find_empty_zones(&grid, width, height, 0);

        // Should not find any zones (all too small)
        assert!(zones.is_empty(), "Should not find zones smaller than 3x3");
    }

    #[test]
    fn test_zones_sorted_largest_first() {
        // Create multiple empty regions of different sizes
        let width = 30;
        let height = 30;
        let mut grid = vec![vec![1u8; height]; width];

        // Small zone 3x3 = 9
        for x in 2..5 {
            for y in 2..5 {
                grid[x][y] = 0;
            }
        }

        // Large zone 10x8 = 80
        for x in 10..20 {
            for y in 10..18 {
                grid[x][y] = 0;
            }
        }

        // Medium zone 5x4 = 20
        for x in 22..27 {
            for y in 2..6 {
                grid[x][y] = 0;
            }
        }

        let zones = find_empty_zones(&grid, width, height, 0);

        // Zones should be sorted largest to smallest
        if zones.len() > 1 {
            for i in 0..zones.len() - 1 {
                let area1 = zones[i].w * zones[i].h;
                let area2 = zones[i + 1].w * zones[i + 1].h;
                assert!(
                    area1 >= area2,
                    "Zones should be sorted by area (descending)"
                );
            }
        }
    }
}
