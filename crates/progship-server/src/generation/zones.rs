//! Empty zone identification for room placement.
//!
//! Scan grid for contiguous empty regions where rooms can be placed.

/// Grid cell values
pub const CELL_EMPTY: u8 = 0;
#[allow(dead_code)]
pub const CELL_MAIN_CORRIDOR: u8 = 1;
#[allow(dead_code)]
pub const CELL_SERVICE_CORRIDOR: u8 = 2;
#[allow(dead_code)]
pub const CELL_SHAFT: u8 = 3;
#[allow(dead_code)]
pub const CELL_ROOM_BASE: u8 = 10; // room N = CELL_ROOM_BASE + N (wraps at 246)

/// Rectangular zone on the grid where rooms can be placed.
pub struct GridZone {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

/// Scan the grid for contiguous rectangular zones of CELL_EMPTY.
pub fn find_empty_zones(grid: &[Vec<u8>], width: usize, height: usize) -> Vec<GridZone> {
    // Simple row-run based approach: scan rows, find horizontal runs of empty,
    // then merge vertically adjacent runs with matching x-ranges.
    let mut zones: Vec<GridZone> = Vec::new();

    // Track which cells are already claimed by a zone
    let mut claimed = vec![vec![false; height]; width];

    for x in 0..width {
        for y in 0..height {
            if grid[x][y] != CELL_EMPTY || claimed[x][y] {
                continue;
            }

            // Find the widest run starting at (x, y)
            let mut run_w = 0;
            while x + run_w < width && grid[x + run_w][y] == CELL_EMPTY && !claimed[x + run_w][y] {
                run_w += 1;
            }
            if run_w < 3 {
                continue;
            } // too narrow for a room

            // Extend downward while the same x-range is all empty
            let mut run_h = 1;
            'outer: while y + run_h < height {
                for xx in x..(x + run_w) {
                    if grid[xx][y + run_h] != CELL_EMPTY || claimed[xx][y + run_h] {
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
