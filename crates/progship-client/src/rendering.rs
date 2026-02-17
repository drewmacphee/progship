//! 3D rendering for the ProgShip client.
//!
//! Handles room mesh generation, people capsules, door frames, and floor colors.

use bevy::prelude::*;
use progship_client_sdk::*;
use progship_logic::constants::room_types;
use spacetimedb_sdk::Table;

use crate::state::{
    ConnectionState, DoorMarker, IndicatorEntity, PersonEntity, PlayerState, RoomEntity, RoomLabel,
    UiState, ViewState,
};

pub fn sync_rooms(
    state: Res<ConnectionState>,
    mut view: ResMut<ViewState>,
    mut commands: Commands,
    existing: Query<Entity, With<RoomEntity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };

    // Rebuild when deck changes or subscription data arrives
    let room_count = conn.db.room().iter().count();
    if view.current_deck != view.prev_deck {
        view.rooms_dirty = true;
        view.minimap_dirty = true;
        view.prev_deck = view.current_deck;
    }
    if room_count != view.prev_room_count && room_count > 0 {
        view.rooms_dirty = true;
        view.minimap_dirty = true;
        view.prev_room_count = room_count;
    }

    if !view.rooms_dirty {
        return;
    }
    view.rooms_dirty = false;

    // Despawn existing room entities (flat hierarchy, no children)
    for entity in existing.iter() {
        if let Some(mut cmd) = commands.get_entity(entity) {
            cmd.despawn();
        }
    }

    // Collect doors and rooms for this deck
    let doors: Vec<_> = conn.db.door().iter().collect();
    let all_rooms: Vec<_> = conn.db.room().iter().collect();
    let deck_rooms: Vec<&Room> = all_rooms
        .iter()
        .filter(|r| r.deck == view.current_deck)
        .collect();

    let wall_height = 3.0;
    let wall_thickness = 0.3;

    // --- Phase 1: Spawn floors, labels, furniture (per-room) ---
    for room in &deck_rooms {
        let color = room_color(room.room_type);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(room.width, 0.2, room.height))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                ..default()
            })),
            Transform::from_xyz(room.x, 0.0, room.y),
            RoomEntity {
                room_id: room.id,
                deck: room.deck,
            },
        ));
        if !room_types::is_corridor(room.room_type) {
            let font_size = (room.width.min(room.height) * 2.5).clamp(8.0, 28.0);
            commands.spawn((
                Text2d::new(&room.name),
                TextFont {
                    font_size,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
                Transform::from_xyz(room.x, 0.2, room.y)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                RoomLabel,
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
            spawn_furniture(&mut commands, &mut meshes, &mut materials, room);
        }
    }

    // --- Phase 2: Build unique wall edges ---
    // Each edge is a horizontal or vertical wall segment on a room boundary.
    // Shared boundaries between two rooms produce ONE edge (lower ID owns it).
    // Partial overlaps split the larger room's edge into covered and uncovered segments.
    struct WallEdge {
        x: f32,
        z: f32,
        length: f32,
        horizontal: bool, // true = along X (N/S walls), false = along Z (E/W walls)
        room_id: u32,     // room that "owns" this edge for color
        deck: i32,
    }

    let mut edges: Vec<WallEdge> = Vec::new();
    let eps = 0.05;

    for room in &deck_rooms {
        let rx = room.x;
        let ry = room.y;
        let rw = room.width;
        let rh = room.height;
        let r_left = rx - rw / 2.0;
        let r_right = rx + rw / 2.0;
        let r_top = ry - rh / 2.0;
        let r_bot = ry + rh / 2.0;

        // For each side, find neighbor coverage and split into segments
        for side in 0u8..4 {
            // edge_pos: the coordinate perpendicular to the wall
            // edge_start..edge_end: the range along the wall axis
            let (edge_pos, edge_start, edge_end, horizontal) = match side {
                0 => (r_top, r_left, r_right, true), // N
                1 => (r_bot, r_left, r_right, true), // S
                2 => (r_right, r_top, r_bot, false), // E
                3 => (r_left, r_top, r_bot, false),  // W
                _ => unreachable!(),
            };

            // Find all neighbors that share this edge
            let mut covered: Vec<(f32, f32)> = Vec::new();
            for other in &deck_rooms {
                if other.id == room.id {
                    continue;
                }
                let o_left = other.x - other.width / 2.0;
                let o_right = other.x + other.width / 2.0;
                let o_top = other.y - other.height / 2.0;
                let o_bot = other.y + other.height / 2.0;

                let (neighbor_edge, n_start, n_end) = match side {
                    0 => (o_bot, o_left, o_right), // N: neighbor's south edge
                    1 => (o_top, o_left, o_right), // S: neighbor's north edge
                    2 => (o_left, o_top, o_bot),   // E: neighbor's west edge
                    3 => (o_right, o_top, o_bot),  // W: neighbor's east edge
                    _ => unreachable!(),
                };

                // Check if edges are flush
                if (edge_pos - neighbor_edge).abs() < eps {
                    // Compute overlap range
                    let ov_start = edge_start.max(n_start);
                    let ov_end = edge_end.min(n_end);
                    if ov_end - ov_start > eps {
                        if room.id < other.id {
                            // We own this shared segment — mark as covered
                            // (we'll draw it, neighbor won't)
                            covered.push((ov_start, ov_end));
                        } else {
                            // Neighbor owns it — mark as covered so we skip it
                            covered.push((ov_start, ov_end));
                        }
                    }
                }
            }

            // Sort covered segments
            covered.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            // Split edge into uncovered (exterior) and owned-shared segments
            let mut cursor = edge_start;
            for (cov_start, cov_end) in &covered {
                // Uncovered segment before this coverage: always draw (exterior wall)
                if *cov_start - cursor > eps {
                    let seg_start = cursor;
                    let seg_end = *cov_start;
                    let length = seg_end - seg_start;
                    let center = seg_start + length / 2.0;
                    let (wx, wz) = if horizontal {
                        (center, edge_pos)
                    } else {
                        (edge_pos, center)
                    };
                    edges.push(WallEdge {
                        x: wx,
                        z: wz,
                        length,
                        horizontal,
                        room_id: room.id,
                        deck: room.deck,
                    });
                }

                // Shared segment: only draw if we're the lower ID
                // Find which neighbor covers this segment
                let we_own = deck_rooms.iter().any(|other| {
                    if other.id >= room.id {
                        return false;
                    }
                    let o_left = other.x - other.width / 2.0;
                    let o_right = other.x + other.width / 2.0;
                    let o_top = other.y - other.height / 2.0;
                    let o_bot = other.y + other.height / 2.0;
                    let neighbor_edge = match side {
                        0 => o_bot,
                        1 => o_top,
                        2 => o_left,
                        3 => o_right,
                        _ => return false,
                    };
                    if (edge_pos - neighbor_edge).abs() >= eps {
                        return false;
                    }
                    let (n_start, n_end) = match side {
                        0 | 1 => (o_left, o_right),
                        2 | 3 => (o_top, o_bot),
                        _ => return false,
                    };
                    n_start < *cov_end - eps && n_end > *cov_start + eps
                });

                // If a lower-ID neighbor covers this, they will draw it. Skip.
                if !we_own {
                    let length = cov_end - cov_start;
                    let center = cov_start + length / 2.0;
                    let (wx, wz) = if horizontal {
                        (center, edge_pos)
                    } else {
                        (edge_pos, center)
                    };
                    edges.push(WallEdge {
                        x: wx,
                        z: wz,
                        length,
                        horizontal,
                        room_id: room.id,
                        deck: room.deck,
                    });
                }

                cursor = *cov_end;
            }

            // Remaining uncovered segment after last coverage
            if edge_end - cursor > eps {
                let length = edge_end - cursor;
                let center = cursor + length / 2.0;
                let (wx, wz) = if horizontal {
                    (center, edge_pos)
                } else {
                    (edge_pos, center)
                };
                edges.push(WallEdge {
                    x: wx,
                    z: wz,
                    length,
                    horizontal,
                    room_id: room.id,
                    deck: room.deck,
                });
            }
        }
    }

    // --- Phase 3: Assign door gaps to edges ---
    // For each door, find which edge it sits on and record the gap.
    let mut edge_gaps: Vec<Vec<(f32, f32)>> = vec![Vec::new(); edges.len()];

    for door in &doors {
        let room_a = all_rooms.iter().find(|r| r.id == door.room_a);
        let room_b = all_rooms.iter().find(|r| r.id == door.room_b);
        if room_a.is_none() || room_b.is_none() {
            continue;
        }
        let ra = room_a.unwrap();
        let rb = room_b.unwrap();
        if ra.deck != view.current_deck && rb.deck != view.current_deck {
            continue;
        }

        // The door sits on a boundary between rooms.
        // Find the matching edge by position.
        let horizontal = door.wall_a == 0 || door.wall_a == 1;
        let boundary_pos = if horizontal { door.door_y } else { door.door_x };
        let door_axis_pos = if horizontal { door.door_x } else { door.door_y };

        // Determine the actual boundary coordinate from the room geometry
        let edge_coord = match door.wall_a {
            0 => ra.y - ra.height / 2.0,
            1 => ra.y + ra.height / 2.0,
            2 => ra.x + ra.width / 2.0,
            3 => ra.x - ra.width / 2.0,
            _ => continue,
        };

        for (i, edge) in edges.iter().enumerate() {
            if edge.horizontal != horizontal {
                continue;
            }
            let edge_coord_match = if horizontal {
                (edge.z - edge_coord).abs() < eps
            } else {
                (edge.x - edge_coord).abs() < eps
            };
            if !edge_coord_match {
                continue;
            }
            // Check if door falls within this edge segment
            let half = edge.length / 2.0;
            let edge_center = if horizontal { edge.x } else { edge.z };
            let seg_start = edge_center - half;
            let seg_end = edge_center + half;
            if door_axis_pos > seg_start - eps && door_axis_pos < seg_end + eps {
                edge_gaps[i].push((door_axis_pos, door.width));
                break;
            }
        }
    }

    // --- Phase 4: Draw walls from edges ---
    for (i, edge) in edges.iter().enumerate() {
        let room = all_rooms.iter().find(|r| r.id == edge.room_id).unwrap();
        let wall_color = room_color(room.room_type).with_luminance(0.3);
        let edge_center = if edge.horizontal { edge.x } else { edge.z };

        let positions: Vec<f32> = edge_gaps[i].iter().map(|g| g.0).collect();
        let widths: Vec<f32> = edge_gaps[i].iter().map(|g| g.1).collect();

        spawn_wall_with_gaps(
            &mut commands,
            &mut meshes,
            &mut materials,
            wall_color,
            edge.x,
            edge.z,
            edge.length,
            wall_height,
            wall_thickness,
            edge.horizontal,
            &positions,
            edge_center,
            &widths,
            edge.room_id,
            edge.deck,
        );
    }

    // --- Phase 5: Door frames ---
    let frame_color = Color::srgb(0.55, 0.55, 0.6);
    let frame_mat = materials.add(StandardMaterial {
        base_color: frame_color,
        ..default()
    });
    let frame_depth = wall_thickness + 0.1;
    let post_w = 0.2;
    let lintel_height = 0.3;

    for door in &doors {
        let room_a = all_rooms.iter().find(|r| r.id == door.room_a);
        let room_b = all_rooms.iter().find(|r| r.id == door.room_b);
        if room_a.is_none() || room_b.is_none() {
            continue;
        }
        let ra = room_a.unwrap();
        let rb = room_b.unwrap();
        if ra.deck != view.current_deck || rb.deck != view.current_deck {
            continue;
        }
        // Skip frames for plain corridor-to-corridor
        if room_types::is_plain_corridor(ra.room_type)
            && room_types::is_plain_corridor(rb.room_type)
        {
            continue;
        }
        // Only spawn once per door (lower room_a)
        if door.room_a > door.room_b {
            continue;
        }

        let horizontal = door.wall_a == 0 || door.wall_a == 1;
        let boundary = match door.wall_a {
            0 => ra.y - ra.height / 2.0,
            1 => ra.y + ra.height / 2.0,
            2 => ra.x + ra.width / 2.0,
            3 => ra.x - ra.width / 2.0,
            _ => continue,
        };

        if horizontal {
            spawn_door_frame(
                &mut commands,
                &mut meshes,
                &frame_mat,
                door.door_x,
                boundary,
                door.width,
                wall_height,
                frame_depth,
                post_w,
                lintel_height,
                true,
                door.room_a,
                ra.deck,
            );
        } else {
            spawn_door_frame(
                &mut commands,
                &mut meshes,
                &frame_mat,
                boundary,
                door.door_y,
                door.width,
                wall_height,
                frame_depth,
                post_w,
                lintel_height,
                false,
                door.room_a,
                ra.deck,
            );
        }
    }

    // Corridor floors already rendered by their Room entries (type 17/24)
    // The Corridor table is for data only (carries flags, connectivity), not rendering.
    // Shaft rooms (110/111) are also rendered via their Room table entries per-deck.
}

/// Spawn simple furniture props inside rooms based on room type.
fn spawn_furniture(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    room: &Room,
) {
    let rt = room.room_type;
    let cx = room.x;
    let cz = room.y;
    let hw = room.width / 2.0 - 0.5; // half-width with margin
    let hh = room.height / 2.0 - 0.5;
    let re = RoomEntity {
        room_id: room.id,
        deck: room.deck,
    };

    match rt {
        // Bridge / CIC — console desks in a semicircle
        0 | 2 => {
            let desk_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.15, 0.15, 0.25),
                ..default()
            });
            let desk = meshes.add(Cuboid::new(1.5, 0.8, 0.6));
            for i in 0..3 {
                let offset = (i as f32 - 1.0) * 2.0;
                commands.spawn((
                    Mesh3d(desk.clone()),
                    MeshMaterial3d(desk_mat.clone()),
                    Transform::from_xyz(cx + offset, 0.4, cz - hh * 0.5),
                    re.clone(),
                ));
            }
        }
        // Cabins / Quarters — beds
        10 | 14 | 15 | 16 => {
            let bed_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.25, 0.30, 0.45),
                ..default()
            });
            let bed = meshes.add(Cuboid::new(1.0, 0.4, 2.0));
            let count = if hw > 2.0 { 2 } else { 1 };
            for i in 0..count {
                let offset = if count == 1 {
                    0.0
                } else {
                    (i as f32 - 0.5) * 2.5
                };
                commands.spawn((
                    Mesh3d(bed.clone()),
                    MeshMaterial3d(bed_mat.clone()),
                    Transform::from_xyz(cx + offset, 0.2, cz + hh * 0.4),
                    re.clone(),
                ));
            }
        }
        // Cabin Double / Family Suite / VIP — bigger bed
        11..=13 => {
            let bed_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.30, 0.28, 0.42),
                ..default()
            });
            let bed = meshes.add(Cuboid::new(1.6, 0.4, 2.0));
            commands.spawn((
                Mesh3d(bed),
                MeshMaterial3d(bed_mat),
                Transform::from_xyz(cx, 0.2, cz + hh * 0.4),
                re.clone(),
            ));
        }
        // Mess Hall / Wardroom / Cafe — tables with chairs
        20 | 21 | 25 => {
            let table_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.32, 0.22),
                ..default()
            });
            let table = meshes.add(Cuboid::new(1.8, 0.75, 0.9));
            let cols = ((hw * 2.0) / 3.0).floor().max(1.0) as i32;
            let rows = ((hh * 2.0) / 3.0).floor().max(1.0) as i32;
            for r in 0..rows.min(4) {
                for c in 0..cols.min(6) {
                    let x = cx - hw + 1.5 + c as f32 * 3.0;
                    let z = cz - hh + 1.5 + r as f32 * 3.0;
                    commands.spawn((
                        Mesh3d(table.clone()),
                        MeshMaterial3d(table_mat.clone()),
                        Transform::from_xyz(x, 0.375, z),
                        re.clone(),
                    ));
                }
            }
        }
        // Hospital / Surgery / Medbay — beds in rows
        30 | 31 | 37 => {
            let bed_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.70, 0.72, 0.75),
                ..default()
            });
            let bed = meshes.add(Cuboid::new(0.9, 0.5, 1.8));
            let count = ((hw * 2.0) / 2.5).floor().max(1.0) as i32;
            for i in 0..count.min(6) {
                let x = cx - hw + 1.2 + i as f32 * 2.5;
                commands.spawn((
                    Mesh3d(bed.clone()),
                    MeshMaterial3d(bed_mat.clone()),
                    Transform::from_xyz(x, 0.25, cz),
                    re.clone(),
                ));
            }
        }
        // Gym — equipment blocks
        40 => {
            let equip_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.3, 0.35),
                ..default()
            });
            let equip = meshes.add(Cuboid::new(1.0, 1.2, 0.8));
            let count = ((hw * 2.0) / 2.0).floor().max(1.0) as i32;
            for i in 0..count.min(5) {
                let x = cx - hw + 1.0 + i as f32 * 2.0;
                commands.spawn((
                    Mesh3d(equip.clone()),
                    MeshMaterial3d(equip_mat.clone()),
                    Transform::from_xyz(x, 0.6, cz + hh * 0.3),
                    re.clone(),
                ));
            }
        }
        // Engineering / Reactor — large machinery blocks
        60..=63 => {
            let mach_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.35, 0.25, 0.15),
                ..default()
            });
            let machine = meshes.add(Cuboid::new(2.0, 2.0, 2.0));
            commands.spawn((
                Mesh3d(machine),
                MeshMaterial3d(mach_mat),
                Transform::from_xyz(cx, 1.0, cz),
                re.clone(),
            ));
        }
        // Hydroponics — planter rows
        80 => {
            let plant_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.15, 0.45, 0.15),
                ..default()
            });
            let planter = meshes.add(Cuboid::new(0.8, 0.6, room.height - 1.0));
            let count = ((hw * 2.0) / 1.5).floor().max(1.0) as i32;
            for i in 0..count.min(8) {
                let x = cx - hw + 0.6 + i as f32 * 1.5;
                commands.spawn((
                    Mesh3d(planter.clone()),
                    MeshMaterial3d(plant_mat.clone()),
                    Transform::from_xyz(x, 0.3, cz),
                    re.clone(),
                ));
            }
        }
        // Cargo Bay — stacked crates
        90 | 91 => {
            let crate_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.35, 0.30, 0.22),
                ..default()
            });
            let crate_mesh = meshes.add(Cuboid::new(1.2, 1.2, 1.2));
            let count = ((hw * 2.0) / 2.0).floor().max(1.0) as i32;
            for i in 0..count.min(4) {
                let x = cx - hw + 1.0 + i as f32 * 2.0;
                commands.spawn((
                    Mesh3d(crate_mesh.clone()),
                    MeshMaterial3d(crate_mat.clone()),
                    Transform::from_xyz(x, 0.6, cz - hh * 0.3),
                    re.clone(),
                ));
            }
        }
        _ => {} // No furniture for unlisted types
    }
}

fn spawn_wall_with_gaps(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    color: Color,
    wall_x: f32,
    wall_z: f32, // 3D position of wall center
    wall_length: f32,
    wall_height: f32,
    wall_thickness: f32,
    horizontal: bool,       // true = runs along X, false = runs along Z
    door_positions: &[f32], // door world positions along the wall axis
    room_center: f32,       // room center along the wall axis (for converting door pos)
    door_widths: &[f32],    // per-door widths
    room_id: u32,
    deck: i32,
) {
    let mat = materials.add(StandardMaterial {
        base_color: color,
        ..default()
    });

    if door_positions.is_empty() {
        // No doors — solid wall (corner posts handle corner fill)
        if horizontal {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(wall_length, wall_height, wall_thickness))),
                MeshMaterial3d(mat),
                Transform::from_xyz(wall_x, wall_height / 2.0, wall_z),
                RoomEntity { room_id, deck },
            ));
        } else {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(wall_thickness, wall_height, wall_length))),
                MeshMaterial3d(mat),
                Transform::from_xyz(wall_x, wall_height / 2.0, wall_z),
                RoomEntity { room_id, deck },
            ));
        }
        return;
    }

    // Build wall segments around door gaps
    let mut gaps: Vec<(f32, f32)> = door_positions
        .iter()
        .zip(door_widths.iter())
        .map(|(&dp, &dw)| {
            let offset = dp - room_center;
            (offset - dw / 2.0, offset + dw / 2.0)
        })
        .collect();
    gaps.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let half_len = wall_length / 2.0;
    let mut cursor = -half_len;
    let wall_end = half_len;

    for (gap_start, gap_end) in &gaps {
        let seg_len = gap_start - cursor;
        if seg_len > 0.01 {
            let seg_center = cursor + seg_len / 2.0;
            if horizontal {
                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(seg_len, wall_height, wall_thickness))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(wall_x + seg_center, wall_height / 2.0, wall_z),
                    RoomEntity { room_id, deck },
                ));
            } else {
                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(wall_thickness, wall_height, seg_len))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(wall_x, wall_height / 2.0, wall_z + seg_center),
                    RoomEntity { room_id, deck },
                ));
            }
        }
        cursor = *gap_end;
    }

    // Final segment after last gap
    let seg_len = wall_end - cursor;
    if seg_len > 0.01 {
        let seg_center = cursor + seg_len / 2.0;
        if horizontal {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(seg_len, wall_height, wall_thickness))),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(wall_x + seg_center, wall_height / 2.0, wall_z),
                RoomEntity { room_id, deck },
            ));
        } else {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(wall_thickness, wall_height, seg_len))),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(wall_x, wall_height / 2.0, wall_z + seg_center),
                RoomEntity { room_id, deck },
            ));
        }
    }
}

/// Spawn a door frame (two posts + lintel) at the given position.
/// `horizontal`: true if the wall runs along X (N/S walls), false for Z (E/W walls).
#[allow(clippy::too_many_arguments)]
fn spawn_door_frame(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    frame_mat: &Handle<StandardMaterial>,
    x: f32,
    z: f32,
    door_width: f32,
    wall_height: f32,
    frame_depth: f32,
    post_w: f32,
    lintel_height: f32,
    horizontal: bool,
    room_id: u32,
    deck: i32,
) {
    let re = RoomEntity { room_id, deck };
    if horizontal {
        // Wall along X: posts offset in X, frame depth in Z
        let post_mesh = meshes.add(Cuboid::new(post_w, wall_height, frame_depth));
        for sign in [-1.0_f32, 1.0] {
            commands.spawn((
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(x + sign * door_width / 2.0, wall_height / 2.0, z),
                DoorMarker,
                re.clone(),
            ));
        }
        let lintel = meshes.add(Cuboid::new(
            door_width + post_w * 2.0,
            lintel_height,
            frame_depth,
        ));
        commands.spawn((
            Mesh3d(lintel),
            MeshMaterial3d(frame_mat.clone()),
            Transform::from_xyz(x, wall_height - lintel_height / 2.0, z),
            DoorMarker,
            re,
        ));
    } else {
        // Wall along Z: posts offset in Z, frame depth in X
        let post_mesh = meshes.add(Cuboid::new(frame_depth, wall_height, post_w));
        for sign in [-1.0_f32, 1.0] {
            commands.spawn((
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(x, wall_height / 2.0, z + sign * door_width / 2.0),
                DoorMarker,
                re.clone(),
            ));
        }
        let lintel = meshes.add(Cuboid::new(
            frame_depth,
            lintel_height,
            door_width + post_w * 2.0,
        ));
        commands.spawn((
            Mesh3d(lintel),
            MeshMaterial3d(frame_mat.clone()),
            Transform::from_xyz(x, wall_height - lintel_height / 2.0, z),
            DoorMarker,
            re,
        ));
    }
}

pub fn sync_people(
    state: Res<ConnectionState>,
    mut view: ResMut<ViewState>,
    player: Res<PlayerState>,
    ui: Res<UiState>,
    mut commands: Commands,
    mut existing: Query<(Entity, &PersonEntity, &mut Transform)>,
    indicators: Query<Entity, With<IndicatorEntity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };

    let dt = time.delta_secs();

    // Incremental sync at 2Hz (was 5Hz full despawn/respawn)
    view.people_sync_timer += dt;
    let do_sync = view.people_sync_timer >= 0.5;
    if do_sync {
        view.people_sync_timer = 0.0;
    }

    // Build set of person_ids that should be visible on current deck
    // Only do full scan during sync ticks
    if do_sync {
        // Collect who SHOULD be on this deck
        let mut wanted: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for pos in conn.db.position().iter() {
            if let Some(room) = conn.db.room().id().find(&pos.room_id) {
                if room.deck == view.current_deck {
                    wanted.insert(pos.person_id);
                }
            }
        }

        // Despawn entities no longer on this deck
        let mut have: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut despawned: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for (entity, pe, _) in existing.iter() {
            if wanted.contains(&pe.person_id) {
                have.insert(pe.person_id);
            } else {
                despawned.insert(pe.person_id);
                if let Some(mut cmd) = commands.get_entity(entity) {
                    cmd.despawn(); // recursive: also despawns indicator children
                }
            }
        }

        // Despawn indicators on surviving entities (will be recreated below)
        for entity in indicators.iter() {
            if let Some(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
        }

        // Spawn only NEW people (not already in scene)
        let capsule_mesh = meshes.add(Capsule3d::new(0.4, 1.2));

        for &pid in &wanted {
            if have.contains(&pid) {
                continue; // already spawned
            }
            let Some(pos) = conn.db.position().person_id().find(&pid) else {
                continue;
            };

            let is_player = Some(pid) == player.person_id;
            let person = conn.db.person().id().find(&pid);
            let is_crew = person.as_ref().map(|p| p.is_crew).unwrap_or(false);
            let is_selected = ui.selected_person == Some(pid);

            let base_color = if is_player {
                Color::srgb(0.0, 1.0, 0.2)
            } else if is_crew {
                Color::srgb(0.3, 0.5, 1.0)
            } else {
                Color::srgb(0.9, 0.8, 0.3)
            };

            let needs = conn.db.needs().person_id().find(&pid);
            let health = needs.as_ref().map(|n| n.health).unwrap_or(1.0);
            let final_color = if health < 0.5 {
                Color::srgb(1.0, 0.2, 0.2)
            } else if is_selected {
                Color::srgb(1.0, 1.0, 1.0)
            } else {
                base_color
            };

            let person_height = if is_player { 1.0 } else { 0.8 };

            commands.spawn((
                Mesh3d(capsule_mesh.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: final_color,
                    ..default()
                })),
                Transform::from_xyz(pos.x, person_height, pos.y).with_scale(Vec3::new(
                    1.0,
                    if is_player { 1.2 } else { 1.0 },
                    1.0,
                )),
                PersonEntity { person_id: pid },
            ));
        }

        // Spawn indicators as children of surviving person entities
        let indicator_mesh = meshes.add(Sphere::new(0.2));
        let convo_mesh = meshes.add(Sphere::new(0.3));
        for (entity, pe, _) in existing.iter() {
            let pid = pe.person_id;
            if despawned.contains(&pid) {
                continue;
            }
            let is_player = Some(pid) == player.person_id;
            let person_height = if is_player { 1.0 } else { 0.8 };

            if let Some(activity) = conn.db.activity().person_id().find(&pid) {
                let indicator_color = activity_indicator_color(activity.activity_type);
                let child = commands
                    .spawn((
                        Mesh3d(indicator_mesh.clone()),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: indicator_color,
                            emissive: indicator_color.into(),
                            ..default()
                        })),
                        Transform::from_xyz(0.0, person_height + 0.8, 0.0),
                        IndicatorEntity,
                    ))
                    .id();
                if let Some(mut cmd) = commands.get_entity(entity) {
                    cmd.add_child(child);
                }
            }

            if conn.db.in_conversation().person_id().find(&pid).is_some() {
                let child = commands
                    .spawn((
                        Mesh3d(convo_mesh.clone()),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(1.0, 1.0, 0.5),
                            emissive: Color::srgb(0.5, 0.5, 0.0).into(),
                            ..default()
                        })),
                        Transform::from_xyz(0.5, person_height + 1.5, 0.0),
                        IndicatorEntity,
                    ))
                    .id();
                if let Some(mut cmd) = commands.get_entity(entity) {
                    cmd.add_child(child);
                }
            }
        }
    }

    // Every frame: lerp ONLY existing entities (already filtered to current deck)
    for (_, pe, mut transform) in existing.iter_mut() {
        if let Some(pos) = conn.db.position().person_id().find(&pe.person_id) {
            let is_player = Some(pe.person_id) == player.person_id;
            let person_height = if is_player { 1.0 } else { 0.8 };
            let target = Vec3::new(pos.x, person_height, pos.y);
            let lerp_rate = if is_player { 12.0 } else { 6.0 };
            let t = (lerp_rate * dt).min(1.0);
            transform.translation = transform.translation.lerp(target, t);
        }
    }
}

fn room_color(room_type: u8) -> Color {
    match room_type {
        // Command (dark blue / gold tones) — 0..=8
        0 => Color::srgb(0.12, 0.15, 0.45), // Bridge
        1 => Color::srgb(0.18, 0.18, 0.40), // Conference
        2 => Color::srgb(0.10, 0.12, 0.38), // CIC
        3 => Color::srgb(0.15, 0.20, 0.42), // Comms Room
        4 => Color::srgb(0.20, 0.18, 0.45), // Captain's Ready Room
        5 => Color::srgb(0.35, 0.15, 0.15), // Security Office
        6 => Color::srgb(0.40, 0.12, 0.12), // Brig
        7 => Color::srgb(0.22, 0.20, 0.38), // Admin Office
        8 => Color::srgb(0.15, 0.22, 0.50), // Observatory

        // Habitation (blue / teal tones) — 10..=18
        10 => Color::srgb(0.20, 0.30, 0.35), // Cabin Single
        11 => Color::srgb(0.22, 0.32, 0.38), // Cabin Double
        12 => Color::srgb(0.25, 0.35, 0.42), // Family Suite
        13 => Color::srgb(0.30, 0.40, 0.48), // VIP Suite
        14 => Color::srgb(0.18, 0.28, 0.32), // Crew Quarters
        15 => Color::srgb(0.22, 0.30, 0.38), // Officer Quarters
        16 => Color::srgb(0.20, 0.32, 0.36), // Passenger Quarters
        17 => Color::srgb(0.35, 0.45, 0.50), // Shared Bathroom
        18 => Color::srgb(0.32, 0.42, 0.48), // Shared Laundry

        // Food service (warm yellow / orange) — 20..=27
        20 => Color::srgb(0.50, 0.40, 0.15), // Mess Hall
        21 => Color::srgb(0.45, 0.38, 0.18), // Wardroom
        22 => Color::srgb(0.48, 0.35, 0.12), // Galley
        23 => Color::srgb(0.30, 0.28, 0.22), // Food Storage Cold
        24 => Color::srgb(0.35, 0.30, 0.18), // Food Storage Dry
        25 => Color::srgb(0.52, 0.42, 0.20), // Cafe
        26 => Color::srgb(0.55, 0.40, 0.18), // Bakery
        27 => Color::srgb(0.25, 0.35, 0.50), // Water Purification

        // Medical (white / cyan tones) — 30..=37
        30 => Color::srgb(0.60, 0.65, 0.70), // Hospital Ward
        31 => Color::srgb(0.55, 0.60, 0.68), // Surgery
        32 => Color::srgb(0.58, 0.62, 0.65), // Dental Clinic
        33 => Color::srgb(0.50, 0.55, 0.62), // Pharmacy
        34 => Color::srgb(0.52, 0.58, 0.60), // Mental Health
        35 => Color::srgb(0.55, 0.50, 0.50), // Quarantine
        36 => Color::srgb(0.35, 0.30, 0.30), // Morgue
        37 => Color::srgb(0.58, 0.63, 0.68), // Medbay

        // Recreation (green tones) — 40..=56
        40 => Color::srgb(0.25, 0.45, 0.25), // Gym
        41 => Color::srgb(0.35, 0.30, 0.40), // Theatre
        42 => Color::srgb(0.30, 0.35, 0.25), // Library
        43 => Color::srgb(0.38, 0.32, 0.42), // Chapel
        44 => Color::srgb(0.28, 0.40, 0.30), // Game Room
        45 => Color::srgb(0.40, 0.30, 0.20), // Bar
        46 => Color::srgb(0.35, 0.38, 0.30), // Art Studio
        47 => Color::srgb(0.32, 0.35, 0.38), // Music Room
        48 => Color::srgb(0.20, 0.35, 0.50), // Holodeck
        49 => Color::srgb(0.15, 0.50, 0.20), // Arboretum
        50 => Color::srgb(0.20, 0.30, 0.45), // Observation Lounge
        51 => Color::srgb(0.25, 0.45, 0.55), // Pool
        52 => Color::srgb(0.40, 0.45, 0.30), // Nursery
        53 => Color::srgb(0.35, 0.42, 0.28), // School
        54 => Color::srgb(0.30, 0.42, 0.32), // Recreation Center
        55 => Color::srgb(0.32, 0.38, 0.35), // Lounge
        56 => Color::srgb(0.42, 0.38, 0.25), // Shops

        // Engineering & Propulsion (orange / amber) — 60..=71
        60 => Color::srgb(0.50, 0.30, 0.10), // Main Engineering
        61 => Color::srgb(0.55, 0.15, 0.10), // Reactor
        62 => Color::srgb(0.50, 0.18, 0.12), // Backup Reactor
        63 => Color::srgb(0.48, 0.28, 0.12), // Engine Room
        64 => Color::srgb(0.45, 0.32, 0.15), // Power Distribution
        65 => Color::srgb(0.42, 0.30, 0.18), // Machine Shop
        66 => Color::srgb(0.38, 0.32, 0.22), // Electronics Lab
        67 => Color::srgb(0.35, 0.28, 0.18), // Parts Storage
        68 => Color::srgb(0.45, 0.20, 0.10), // Fuel Storage
        69 => Color::srgb(0.40, 0.30, 0.20), // Robotics Bay
        70 => Color::srgb(0.42, 0.28, 0.15), // Maintenance Bay
        71 => Color::srgb(0.38, 0.35, 0.25), // Cooling Plant

        // Life support (teal / cyan) — 80..=86
        80 => Color::srgb(0.15, 0.45, 0.20), // Hydroponics
        81 => Color::srgb(0.20, 0.38, 0.45), // Atmosphere Processing
        82 => Color::srgb(0.18, 0.35, 0.50), // Water Recycling
        83 => Color::srgb(0.22, 0.32, 0.35), // Waste Processing
        84 => Color::srgb(0.20, 0.40, 0.42), // Environmental Monitoring
        85 => Color::srgb(0.22, 0.42, 0.48), // Life Support Center
        86 => Color::srgb(0.25, 0.38, 0.42), // HVAC Control

        // Cargo & Storage (brown / gray) — 90..=95
        90 => Color::srgb(0.30, 0.25, 0.18), // Cargo Bay
        91 => Color::srgb(0.28, 0.25, 0.20), // Storage
        92 => Color::srgb(0.38, 0.18, 0.15), // Armory
        93 => Color::srgb(0.32, 0.30, 0.28), // Shuttle Bay
        94 => Color::srgb(0.50, 0.12, 0.12), // Airlock
        95 => Color::srgb(0.22, 0.30, 0.45), // Laboratory

        // Infrastructure (dark gray) — 100..=120
        100 => Color::srgb(0.18, 0.18, 0.22), // Corridor
        101 => Color::srgb(0.15, 0.15, 0.18), // Service Corridor
        102 => Color::srgb(0.20, 0.20, 0.24), // Cross Corridor
        110 => Color::srgb(0.20, 0.35, 0.65), // Elevator Shaft (blue)
        111 => Color::srgb(0.20, 0.55, 0.30), // Ladder Shaft (green)
        112 => Color::srgb(0.65, 0.40, 0.15), // Service Elevator (orange)
        120 => Color::srgb(0.12, 0.12, 0.15), // Service Deck

        _ => Color::srgb(0.25, 0.25, 0.25), // Unknown - neutral gray
    }
}

fn activity_indicator_color(activity_type: u8) -> Color {
    match activity_type {
        0 => Color::srgb(0.4, 0.4, 0.4),  // Idle - gray
        1 => Color::srgb(0.2, 0.5, 1.0),  // Working - blue
        2 => Color::srgb(0.9, 0.7, 0.1),  // Eating - yellow
        3 => Color::srgb(0.1, 0.1, 0.5),  // Sleeping - dark blue
        4 => Color::srgb(0.9, 0.5, 0.9),  // Socializing - pink
        5 => Color::srgb(0.3, 0.8, 0.3),  // Relaxing - green
        6 => Color::srgb(0.5, 0.8, 1.0),  // Hygiene - light blue
        7 => Color::srgb(1.0, 1.0, 1.0),  // Traveling - white
        8 => Color::srgb(0.8, 0.5, 0.1),  // Maintenance - orange
        9 => Color::srgb(0.1, 0.3, 0.8),  // On Duty - navy
        11 => Color::srgb(1.0, 0.1, 0.1), // Emergency - red
        12 => Color::srgb(0.1, 0.9, 0.3), // Exercising - bright green
        _ => Color::srgb(0.5, 0.5, 0.5),
    }
}
