//! Room packing algorithm using squarified treemap.
//!
//! Implements the classic squarified treemap algorithm to pack weighted rectangles
//! into available grid zones, minimizing aspect ratio distortion.

/// Room request for treemap placement.
#[derive(Clone)]
pub(super) struct RoomRequest {
    pub node_id: u64,
    pub name: String,
    pub room_type: u8,
    pub target_area: f32,
    pub capacity: u32,
    pub group: u8,
}

/// Placed room result from treemap.
#[allow(dead_code)]
pub(super) struct PlacedRoom {
    pub room_id: u32,
    pub node_id: u64,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub room_type: u8,
}

/// Squarified treemap: packs weighted rectangles into a zone.
/// Returns (original_index, x, y, w, h) for each room.
pub(super) fn squarified_treemap(
    rooms: &[(f32, usize)], // (area_weight, original_index)
    zone_x: usize,
    zone_y: usize,
    zone_w: usize,
    zone_h: usize,
) -> Vec<(usize, usize, usize, usize, usize)> {
    if rooms.is_empty() || zone_w == 0 || zone_h == 0 {
        return Vec::new();
    }
    if rooms.len() == 1 {
        return vec![(rooms[0].1, zone_x, zone_y, zone_w, zone_h)];
    }

    let total_weight: f32 = rooms.iter().map(|(w, _)| *w).sum();
    let zone_area = (zone_w * zone_h) as f32;
    if total_weight <= 0.0 || zone_area <= 0.0 {
        return Vec::new();
    }

    // Normalize weights to sum to zone_area
    let scale = zone_area / total_weight;
    let normalized: Vec<(f32, usize)> = rooms.iter().map(|(w, idx)| (w * scale, *idx)).collect();

    let mut result = Vec::new();
    let mut remaining = &normalized[..];
    let mut cx = zone_x;
    let mut cy = zone_y;
    let mut cw = zone_w;
    let mut ch = zone_h;

    while !remaining.is_empty() && cw > 0 && ch > 0 {
        // Lay out along the shorter dimension
        let layout_vertical = cw <= ch; // strip runs along y if vertical, along x if horizontal
        let strip_len = if layout_vertical { ch } else { cw };
        let strip_breadth = if layout_vertical { cw } else { ch };

        // Greedily add rooms to the current strip, maximizing worst aspect ratio
        let _remaining_area: f32 = remaining.iter().map(|(a, _)| *a).sum();
        let mut best_count = 1;
        let mut best_worst_ratio = f32::MAX;

        for count in 1..=remaining.len() {
            let strip_area: f32 = remaining[..count].iter().map(|(a, _)| *a).sum();
            let strip_thickness = (strip_area / strip_len as f32).ceil() as usize;
            let strip_thickness = strip_thickness.max(1).min(strip_breadth);

            // Compute aspect ratios for rooms in this strip
            let mut worst_ratio: f32 = 0.0;
            let mut _pos = 0.0_f32;
            for (area, _) in &remaining[..count] {
                let room_len = if strip_thickness > 0 {
                    *area / strip_thickness as f32
                } else {
                    *area
                };
                let room_len = room_len.max(1.0);
                let r = if room_len > strip_thickness as f32 {
                    room_len / strip_thickness as f32
                } else {
                    strip_thickness as f32 / room_len
                };
                if r > worst_ratio {
                    worst_ratio = r;
                }
                _pos += room_len;
            }

            if count == 1 || worst_ratio <= best_worst_ratio {
                best_worst_ratio = worst_ratio;
                best_count = count;
            } else {
                break; // Adding more rooms makes aspect ratio worse
            }
        }

        // Lay out best_count rooms in the strip
        let strip_rooms = &remaining[..best_count];
        let strip_area: f32 = strip_rooms.iter().map(|(a, _)| *a).sum();
        let strip_thickness = if strip_len > 0 {
            (strip_area / strip_len as f32).ceil() as usize
        } else {
            1
        };
        let strip_thickness = strip_thickness.max(1).min(strip_breadth);

        let mut pos = 0usize;
        for (i, (area, idx)) in strip_rooms.iter().enumerate() {
            let room_len = if i == best_count - 1 {
                // Last room takes remaining space
                strip_len.saturating_sub(pos)
            } else if strip_thickness > 0 {
                (*area / strip_thickness as f32).round() as usize
            } else {
                1
            };
            let room_len = room_len.max(1).min(strip_len.saturating_sub(pos));

            if room_len == 0 {
                continue;
            }

            let (rx, ry, rw, rh) = if layout_vertical {
                (cx, cy + pos, strip_thickness, room_len)
            } else {
                (cx + pos, cy, room_len, strip_thickness)
            };

            if rw > 0 && rh > 0 {
                result.push((*idx, rx, ry, rw, rh));
            }
            pos += room_len;
        }

        // Advance past this strip
        if layout_vertical {
            cx += strip_thickness;
            cw = cw.saturating_sub(strip_thickness);
        } else {
            cy += strip_thickness;
            ch = ch.saturating_sub(strip_thickness);
        }

        remaining = &remaining[best_count..];
    }

    result
}
