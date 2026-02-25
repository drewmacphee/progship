//! 3D rendering for the ProgShip client.
//!
//! Handles room mesh generation, people capsules, door frames, and floor colors.

use bevy::prelude::*;
use progship_client_sdk::*;
use progship_logic::constants::{room_type_icon, room_types};
use progship_logic::movement::decode_cell_rects;
use spacetimedb_sdk::Table;

use crate::state::{
    BlinkingLight, ConnectionState, DoorButton, DoorMarker, DoorPanel, DoorPlaque, DustMote,
    IndicatorEntity, PersonEntity, PlayerState, PulsingEmissive, RoomEntity, RoomLabel, UiState,
    ViewState,
};

/// Add a mesh to assets. When Solari is enabled, generates tangents for deferred GBuffer.
fn add_mesh(meshes: &mut Assets<Mesh>, mesh: impl Into<Mesh>) -> Handle<Mesh> {
    let m: Mesh = mesh.into();
    #[cfg(feature = "solari")]
    let m = {
        let backup = m.clone();
        m.with_generated_tangents().unwrap_or(backup)
    };
    meshes.add(m)
}

/// Public version of add_mesh for use by greeble module.
pub fn add_mesh_pub(meshes: &mut Assets<Mesh>, mesh: impl Into<Mesh>) -> Handle<Mesh> {
    add_mesh(meshes, mesh)
}

/// Build a flat slab mesh from cell mask rects (or fall back to bbox cuboid).
/// Each rect becomes a flat box at world coordinates. Y is thickness.
fn cell_mask_floor_mesh(room: &Room, thickness: f32) -> Vec<(Cuboid, Vec3)> {
    let rects = decode_cell_rects(&room.cells);
    if rects.is_empty() {
        // Fallback: single cuboid at room center
        return vec![(
            Cuboid::new(room.width, thickness, room.height),
            Vec3::new(room.x, 0.0, room.y),
        )];
    }

    // One cuboid per cell rect
    rects
        .iter()
        .map(|&(x0, y0, x1, y1)| {
            let w = (x1 - x0) as f32;
            let h = (y1 - y0) as f32;
            let cx = x0 as f32 + w / 2.0;
            let cy = y0 as f32 + h / 2.0;
            (Cuboid::new(w, thickness, h), Vec3::new(cx, 0.0, cy))
        })
        .collect()
}

pub fn sync_rooms(
    state: Res<ConnectionState>,
    mut view: ResMut<ViewState>,
    mut commands: Commands,
    existing: Query<Entity, With<RoomEntity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    greeble_lib: Option<Res<crate::greeble::GreebleLibrary>>,
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
        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.despawn();
        }
    }

    // Collect rooms for this deck
    let all_rooms: Vec<_> = conn.db.room().iter().collect();
    let deck_rooms: Vec<&Room> = all_rooms
        .iter()
        .filter(|r| r.deck == view.current_deck)
        .collect();

    let default_ceiling = 3.5_f32;

    // --- Phase 1: Spawn floors, ceilings, labels, furniture (per-room) ---
    let ceiling_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.22, 0.25),
        perceptual_roughness: 0.7,
        metallic: 0.1,
        ..default()
    });
    for room in &deck_rooms {
        let color = room_color(room.room_type);
        let wh = if room.ceiling_height > 0.0 {
            room.ceiling_height
        } else {
            default_ceiling
        };
        // Floor — use cell mask rects if available, otherwise bbox
        let floor_mat = materials.add(floor_material(color, room.room_type));
        for (cuboid, pos) in cell_mask_floor_mesh(room, 0.2) {
            commands.spawn((
                Mesh3d(add_mesh(&mut meshes, cuboid)),
                MeshMaterial3d(floor_mat.clone()),
                Transform::from_translation(pos),
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
        }
        // Ceiling
        for (cuboid, pos) in cell_mask_floor_mesh(room, 0.12) {
            commands.spawn((
                Mesh3d(add_mesh(&mut meshes, cuboid)),
                MeshMaterial3d(ceiling_mat.clone()),
                Transform::from_translation(pos + Vec3::Y * wh),
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
        }
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
            spawn_floor_markings(&mut commands, &mut meshes, &mut materials, room);
        }

        // Shaft interior geometry (ladders, elevator cars)
        if matches!(room.room_type, 110..=112) {
            spawn_shaft_interior(&mut commands, &mut meshes, &mut materials, room, wh);
        }

        // Lighting — distributed point lights for all rooms including corridors
        spawn_room_lights(&mut commands, &mut meshes, &mut materials, room);

        // Dust motes in atmospheric rooms (engineering, cargo, corridors)
        if matches!(room.room_type, 60..=71 | 80..=86 | 90..=95 | 100..=102) {
            spawn_dust_motes(&mut commands, &mut meshes, &mut materials, room, wh);
        }
    }

    // --- Phase 2: Compute perimeter wall segments from cell masks ---
    let wt: f32 = 0.15;
    let inset = wt / 2.0;

    // Lightweight per-room info for door frame positioning
    struct RoomInfo {
        n_z: f32,
        s_z: f32,
        e_x: f32,
        w_x: f32,
        ceiling_height: f32,
    }
    let mut room_info: Vec<(u32, i32, u8, RoomInfo)> = Vec::new();
    for room in &deck_rooms {
        let hw = room.width / 2.0;
        let hh = room.height / 2.0;
        room_info.push((
            room.id,
            room.deck,
            room.room_type,
            RoomInfo {
                n_z: room.y - hh + inset,
                s_z: room.y + hh - inset,
                e_x: room.x + hw - inset,
                w_x: room.x - hw + inset,
                ceiling_height: if room.ceiling_height > 0.0 {
                    room.ceiling_height
                } else {
                    default_ceiling
                },
            },
        ));
    }

    // Build room_id → deck_rooms index map
    let mut id_to_idx: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for (idx, room) in deck_rooms.iter().enumerate() {
        id_to_idx.insert(room.id, idx);
    }

    // Iterate all same-deck doors (skip cross-deck doors)
    let deck_doors: Vec<_> = conn
        .db
        .door()
        .iter()
        .filter(|d| id_to_idx.contains_key(&d.room_a) && id_to_idx.contains_key(&d.room_b))
        .collect();

    // Build per-room door edge lists for perimeter walk
    let mut room_door_edges: Vec<Vec<progship_logic::movement::DoorEdge>> =
        vec![Vec::new(); deck_rooms.len()];

    struct DoorwayCut {
        room_idx: usize,
        other_idx: usize,
        wall_side: u8,
        axis_pos: f32,
        width: f32,
        door_id: u64,
        is_open: bool,
    }
    let mut doorway_cuts: Vec<DoorwayCut> = Vec::new();
    let post_w: f32 = 0.2;

    // Per-room gap lists for greeble compatibility
    struct WallGapInfo {
        n: Vec<(f32, f32)>,
        s: Vec<(f32, f32)>,
        e: Vec<(f32, f32)>,
        w: Vec<(f32, f32)>,
    }
    let mut room_gaps: Vec<WallGapInfo> = (0..deck_rooms.len())
        .map(|_| WallGapInfo {
            n: Vec::new(),
            s: Vec::new(),
            e: Vec::new(),
            w: Vec::new(),
        })
        .collect();

    for door in &deck_doors {
        let Some(&idx_a) = id_to_idx.get(&door.room_a) else {
            continue;
        };
        let Some(&idx_b) = id_to_idx.get(&door.room_b) else {
            continue;
        };
        let ra = deck_rooms[idx_a];
        let rb = deck_rooms[idx_b];
        let both_plain = room_types::is_plain_corridor(ra.room_type)
            && room_types::is_plain_corridor(rb.room_type);

        let gap_w = if both_plain {
            door.width - 2.0 * wt
        } else {
            door.width
        };
        if gap_w < 0.1 {
            continue;
        }

        // Add door edges for perimeter walk
        room_door_edges[idx_a].push(progship_logic::movement::DoorEdge {
            door_x: door.door_x,
            door_y: door.door_y,
            width: gap_w,
            wall_side: door.wall_a,
        });
        room_door_edges[idx_b].push(progship_logic::movement::DoorEdge {
            door_x: door.door_x,
            door_y: door.door_y,
            width: gap_w,
            wall_side: door.wall_b,
        });

        // Track gaps for greeble compatibility
        match door.wall_a {
            0 => room_gaps[idx_a].n.push((door.door_x, gap_w)),
            1 => room_gaps[idx_a].s.push((door.door_x, gap_w)),
            2 => room_gaps[idx_a].e.push((door.door_y, gap_w)),
            3 => room_gaps[idx_a].w.push((door.door_y, gap_w)),
            _ => {}
        }
        match door.wall_b {
            0 => room_gaps[idx_b].n.push((door.door_x, gap_w)),
            1 => room_gaps[idx_b].s.push((door.door_x, gap_w)),
            2 => room_gaps[idx_b].e.push((door.door_y, gap_w)),
            3 => room_gaps[idx_b].w.push((door.door_y, gap_w)),
            _ => {}
        }

        if !both_plain {
            doorway_cuts.push(DoorwayCut {
                room_idx: idx_a,
                other_idx: idx_b,
                wall_side: door.wall_a,
                axis_pos: if door.wall_a < 2 {
                    door.door_x
                } else {
                    door.door_y
                },
                width: door.width,
                door_id: door.id,
                is_open: door.is_open,
            });
        }
    }

    // --- Phase 4.5: Greeble surface detail ---
    if let Some(ref lib) = greeble_lib {
        for (idx, room) in deck_rooms.iter().enumerate() {
            let g = &room_gaps[idx];
            let gaps = crate::greeble::WallGaps {
                n: g.n.clone(),
                s: g.s.clone(),
                e: g.e.clone(),
                w: g.w.clone(),
            };
            crate::greeble::spawn_room_greebles(&mut commands, &mut meshes, lib, room, &gaps);
        }
    }

    // --- Phase 5: Draw walls from cell-mask perimeter ---
    for (idx, room) in deck_rooms.iter().enumerate() {
        let wh = room_info[idx].3.ceiling_height;
        let wall_color = room_color(room.room_type).with_luminance(0.3);
        let mat = materials.add(wall_material(wall_color));

        let segments =
            progship_logic::movement::compute_room_perimeter(&room.cells, &room_door_edges[idx]);

        for seg in &segments {
            let is_horizontal = seg.direction == 0 || seg.direction == 1;
            // Inset wall slightly from the cell edge toward room interior
            let inset_offset = match seg.direction {
                0 => inset,  // NORTH wall: move +z (inward)
                1 => -inset, // SOUTH wall: move -z (inward)
                2 => -inset, // EAST wall: move -x (inward)
                3 => inset,  // WEST wall: move +x (inward)
                _ => 0.0,
            };
            let (wx, wz) = if is_horizontal {
                (seg.x, seg.z + inset_offset)
            } else {
                (seg.x + inset_offset, seg.z)
            };
            if is_horizontal {
                commands.spawn((
                    Mesh3d(add_mesh(&mut meshes, Cuboid::new(seg.length, wh, wt))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(wx, wh / 2.0, wz),
                    RoomEntity {
                        room_id: room.id,
                        deck: room.deck,
                    },
                ));
            } else {
                commands.spawn((
                    Mesh3d(add_mesh(&mut meshes, Cuboid::new(wt, wh, seg.length))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(wx, wh / 2.0, wz),
                    RoomEntity {
                        room_id: room.id,
                        deck: room.deck,
                    },
                ));
            }
        }
    }

    // --- Phase 6: Door frames ---
    let frame_color = Color::srgb(0.55, 0.55, 0.6);
    let frame_mat = materials.add(StandardMaterial {
        base_color: frame_color,
        metallic: 0.85,
        perceptual_roughness: 0.3,
        reflectance: 0.6,
        ..default()
    });
    let frame_depth = 2.0 * wt + 0.1;

    let panel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.42, 0.45),
        metallic: 0.9,
        perceptual_roughness: 0.25,
        reflectance: 0.5,
        ..default()
    });
    let panel_thickness = 0.06;

    for cut in &doorway_cuts {
        let rinfo = &room_info[cut.room_idx].3;
        let cinfo = &room_info[cut.other_idx].3;
        let room_id = room_info[cut.room_idx].0;
        let deck = room_info[cut.room_idx].1;
        let rt_a = room_info[cut.room_idx].2;
        let rt_b = room_info[cut.other_idx].2;
        let door_h = progship_logic::constants::deck_heights::door_opening_height(rt_a, rt_b);
        let wh = rinfo.ceiling_height;
        // Place frame centered between the room's wall and the corridor's wall
        let (fx, fz, horiz) = match cut.wall_side {
            0 => (cut.axis_pos, (rinfo.n_z + cinfo.s_z) / 2.0, true),
            1 => (cut.axis_pos, (rinfo.s_z + cinfo.n_z) / 2.0, true),
            2 => ((rinfo.e_x + cinfo.w_x) / 2.0, cut.axis_pos, false),
            3 => ((rinfo.w_x + cinfo.e_x) / 2.0, cut.axis_pos, false),
            _ => continue,
        };
        spawn_door_frame(
            &mut commands,
            &mut meshes,
            &frame_mat,
            fx,
            fz,
            cut.width,
            door_h,
            wh,
            frame_depth,
            post_w,
            horiz,
            room_id,
            deck,
        );

        // --- Door sliding panels ---
        let panel_hw = cut.width / 4.0; // each panel is half the door width
        let open_slide = cut.width / 4.0 + post_w * 0.4; // how far panels slide when open
        for side in [-1.0_f32, 1.0] {
            let closed_offset = side * panel_hw; // center of each half
            let current_offset = if cut.is_open {
                closed_offset + side * open_slide
            } else {
                closed_offset
            };
            let (px, pz, panel_mesh) = if horiz {
                (
                    fx + current_offset,
                    fz,
                    add_mesh(
                        &mut meshes,
                        Cuboid::new(panel_hw * 2.0, door_h - 0.04, panel_thickness),
                    ),
                )
            } else {
                (
                    fx,
                    fz + current_offset,
                    add_mesh(
                        &mut meshes,
                        Cuboid::new(panel_thickness, door_h - 0.04, panel_hw * 2.0),
                    ),
                )
            };
            commands.spawn((
                Mesh3d(panel_mesh),
                MeshMaterial3d(panel_mat.clone()),
                Transform::from_xyz(px, door_h / 2.0, pz),
                DoorPanel {
                    door_id: cut.door_id,
                    side,
                    horizontal: horiz,
                    half_width: panel_hw,
                    open_offset: open_slide,
                    frame_center: if horiz { fx } else { fz },
                },
                RoomEntity { room_id, deck },
            ));
        }

        // Door plaque: icon + room name on the corridor side of non-corridor rooms
        let room = deck_rooms[cut.room_idx];
        let other_rt = room_info[cut.other_idx].2;
        if !room_types::is_corridor(room.room_type) && room_types::is_corridor(other_rt) {
            let icon = room_type_icon(room.room_type);
            let label = if icon.is_empty() {
                room.name.clone()
            } else {
                format!("{} {}", icon, room.name)
            };

            // Offset plaque to the right of the door (looking from corridor),
            // on the corridor-facing wall surface
            let plaque_h = 1.6; // eye height
            let offset = cut.width / 2.0 + 0.6; // right of door frame
            let wall_offset = 0.02; // just off the wall surface

            let (px, pz, rot) = match cut.wall_side {
                // Room's N wall: corridor is to the north, plaque faces north (-Z in Bevy)
                0 => (
                    fx + offset,
                    rinfo.n_z - wall_offset,
                    Quat::from_rotation_y(std::f32::consts::PI),
                ),
                // Room's S wall: corridor is to the south, plaque faces south (+Z)
                1 => (fx - offset, rinfo.s_z + wall_offset, Quat::IDENTITY),
                // Room's E wall: corridor is to the east, plaque faces east (+X)
                2 => (
                    rinfo.e_x + wall_offset,
                    fz + offset,
                    Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                ),
                // Room's W wall: corridor is to the west, plaque faces west (-X)
                3 => (
                    rinfo.w_x - wall_offset,
                    fz - offset,
                    Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                ),
                _ => continue,
            };

            commands.spawn((
                Text2d::new(&label),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.9, 0.95, 1.0, 0.9)),
                Transform::from_xyz(px, plaque_h, pz).with_rotation(rot),
                DoorPlaque,
                RoomEntity { room_id, deck },
            ));
        }

        // --- Door button panel (small emissive cuboid beside door frame) ---
        let btn_h = 1.2;
        let btn_offset = cut.width / 2.0 + post_w + 0.15;
        let wall_off = 0.01;
        let btn_color = if cut.is_open {
            Color::srgb(0.1, 0.8, 0.2)
        } else {
            Color::srgb(0.8, 0.1, 0.1)
        };
        let btn_mat = materials.add(StandardMaterial {
            base_color: btn_color,
            emissive: btn_color.into(),
            ..default()
        });
        let btn_mesh = add_mesh(&mut meshes, Cuboid::new(0.12, 0.18, 0.03));
        let btn_positions: Vec<(f32, f32, Quat)> = match cut.wall_side {
            0 => vec![
                (
                    fx + btn_offset,
                    rinfo.n_z - wall_off,
                    Quat::from_rotation_y(std::f32::consts::PI),
                ),
                (fx - btn_offset, cinfo.s_z + wall_off, Quat::IDENTITY),
            ],
            1 => vec![
                (fx - btn_offset, rinfo.s_z + wall_off, Quat::IDENTITY),
                (
                    fx + btn_offset,
                    cinfo.n_z - wall_off,
                    Quat::from_rotation_y(std::f32::consts::PI),
                ),
            ],
            2 => vec![
                (
                    rinfo.e_x + wall_off,
                    fz - btn_offset,
                    Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                ),
                (
                    cinfo.w_x - wall_off,
                    fz + btn_offset,
                    Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                ),
            ],
            3 => vec![
                (
                    rinfo.w_x - wall_off,
                    fz + btn_offset,
                    Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                ),
                (
                    cinfo.e_x + wall_off,
                    fz - btn_offset,
                    Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                ),
            ],
            _ => vec![],
        };
        for (bx, bz, brot) in btn_positions {
            commands.spawn((
                Mesh3d(btn_mesh.clone()),
                MeshMaterial3d(btn_mat.clone()),
                Transform::from_xyz(bx, btn_h, bz).with_rotation(brot),
                DoorButton {
                    door_id: cut.door_id,
                },
                RoomEntity { room_id, deck },
            ));
        }
    }

    // --- Phase 7: Hull windows on ring corridors ---
    // Ring corridors' outer walls face space. Add window frames + glass panes.
    let window_frame_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.55),
        metallic: 0.85,
        perceptual_roughness: 0.25,
        reflectance: 0.6,
        ..default()
    });
    let glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.05, 0.05, 0.12, 0.2),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.05,
        metallic: 0.1,
        reflectance: 0.8,
        ..default()
    });
    let win_w = 1.4;
    let win_h = 1.0;
    let win_sill = 1.0;
    let win_frame_t = 0.06;
    let win_spacing = 5.0;

    for room in &deck_rooms {
        if room.room_type != 100 || !room.name.starts_with("Ring ") {
            continue;
        }
        let re = RoomEntity {
            room_id: room.id,
            deck: room.deck,
        };
        let outer_dir = if room.name.contains("North") {
            Some(0)
        } else if room.name.contains("South") {
            Some(1)
        } else if room.name.contains("East") {
            Some(2)
        } else if room.name.contains("West") {
            Some(3)
        } else {
            None
        };
        let Some(dir) = outer_dir else { continue };
        let hw = room.width / 2.0;
        let hh = room.height / 2.0;

        let (wall_len, wall_z, wall_x, horiz) = match dir {
            0 => (room.width, room.y - hh + wt / 2.0, room.x, true),
            1 => (room.width, room.y + hh - wt / 2.0, room.x, true),
            2 => (room.height, room.y, room.x + hw - wt / 2.0, false),
            3 => (room.height, room.y, room.x - hw + wt / 2.0, false),
            _ => continue,
        };

        let count = ((wall_len - 1.0) / win_spacing).floor().max(1.0) as i32;
        let start_offset = (wall_len - (count as f32 - 1.0) * win_spacing) / 2.0;

        for i in 0..count {
            let along = -wall_len / 2.0 + start_offset + i as f32 * win_spacing;
            let (wx, wz) = if horiz {
                (wall_x + along, wall_z)
            } else {
                (wall_x, wall_z + along)
            };
            let wy = win_sill + win_h / 2.0;

            // Glass pane
            if horiz {
                commands.spawn((
                    Mesh3d(add_mesh(&mut meshes, Cuboid::new(win_w, win_h, 0.02))),
                    MeshMaterial3d(glass_mat.clone()),
                    Transform::from_xyz(wx, wy, wz),
                    re.clone(),
                ));
            } else {
                commands.spawn((
                    Mesh3d(add_mesh(&mut meshes, Cuboid::new(0.02, win_h, win_w))),
                    MeshMaterial3d(glass_mat.clone()),
                    Transform::from_xyz(wx, wy, wz),
                    re.clone(),
                ));
            }

            // Window frame: 4 bars around the pane
            let fw = win_w + win_frame_t * 2.0;
            let fh = win_h + win_frame_t * 2.0;
            if horiz {
                for dy in [-(win_h + win_frame_t) / 2.0, (win_h + win_frame_t) / 2.0] {
                    commands.spawn((
                        Mesh3d(add_mesh(
                            &mut meshes,
                            Cuboid::new(fw, win_frame_t, win_frame_t),
                        )),
                        MeshMaterial3d(window_frame_mat.clone()),
                        Transform::from_xyz(wx, wy + dy, wz),
                        re.clone(),
                    ));
                }
                for dx in [-(win_w + win_frame_t) / 2.0, (win_w + win_frame_t) / 2.0] {
                    commands.spawn((
                        Mesh3d(add_mesh(
                            &mut meshes,
                            Cuboid::new(win_frame_t, fh, win_frame_t),
                        )),
                        MeshMaterial3d(window_frame_mat.clone()),
                        Transform::from_xyz(wx + dx, wy, wz),
                        re.clone(),
                    ));
                }
            } else {
                for dy in [-(win_h + win_frame_t) / 2.0, (win_h + win_frame_t) / 2.0] {
                    commands.spawn((
                        Mesh3d(add_mesh(
                            &mut meshes,
                            Cuboid::new(win_frame_t, win_frame_t, fw),
                        )),
                        MeshMaterial3d(window_frame_mat.clone()),
                        Transform::from_xyz(wx, wy + dy, wz),
                        re.clone(),
                    ));
                }
                for dz in [-(win_w + win_frame_t) / 2.0, (win_w + win_frame_t) / 2.0] {
                    commands.spawn((
                        Mesh3d(add_mesh(
                            &mut meshes,
                            Cuboid::new(win_frame_t, fh, win_frame_t),
                        )),
                        MeshMaterial3d(window_frame_mat.clone()),
                        Transform::from_xyz(wx, wy, wz + dz),
                        re.clone(),
                    ));
                }
            }
        }
    }
}

/// Spawn composed furniture props inside rooms based on room type.
/// Uses multi-primitive compositions for visual interest.
fn spawn_furniture(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    room: &Room,
) {
    let rt = room.room_type;
    let cx = room.x;
    let cz = room.y;
    let hw = room.width / 2.0 - 0.5;
    let hh = room.height / 2.0 - 0.5;
    let re = RoomEntity {
        room_id: room.id,
        deck: room.deck,
    };

    match rt {
        // Bridge / CIC — console desks: slab top + angled screen + leg supports
        0 | 2 => {
            let desk_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.15, 0.15, 0.25),
                metallic: 0.6,
                perceptual_roughness: 0.35,
                ..default()
            });
            let screen_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.02, 0.05, 0.12),
                emissive: Color::srgb(0.05, 0.15, 0.35).into(),
                metallic: 0.2,
                perceptual_roughness: 0.1,
                ..default()
            });
            let top = add_mesh(meshes, Cuboid::new(1.5, 0.08, 0.7));
            let leg = add_mesh(meshes, Cuboid::new(0.08, 0.7, 0.08));
            let screen = add_mesh(meshes, Cuboid::new(1.2, 0.6, 0.04));
            for i in 0..3 {
                let offset = (i as f32 - 1.0) * 2.0;
                let dx = cx + offset;
                let dz = cz - hh * 0.5;
                // Desk top
                commands.spawn((
                    Mesh3d(top.clone()),
                    MeshMaterial3d(desk_mat.clone()),
                    Transform::from_xyz(dx, 0.72, dz),
                    re.clone(),
                ));
                // Two legs
                for lx in [-0.6, 0.6] {
                    commands.spawn((
                        Mesh3d(leg.clone()),
                        MeshMaterial3d(desk_mat.clone()),
                        Transform::from_xyz(dx + lx, 0.35, dz),
                        re.clone(),
                    ));
                }
                // Screen panel tilted back (pulses)
                commands.spawn((
                    Mesh3d(screen.clone()),
                    MeshMaterial3d(screen_mat.clone()),
                    Transform::from_xyz(dx, 1.05, dz - 0.3)
                        .with_rotation(Quat::from_rotation_x(-0.25)),
                    PulsingEmissive {
                        rate: 0.3 + i as f32 * 0.15,
                        phase: i as f32 * 0.33,
                        min_mul: 0.4,
                        max_mul: 1.2,
                    },
                    re.clone(),
                ));
            }
        }
        // Cabins / Quarters — bed frame + mattress + headboard
        10 | 14 | 15 | 16 => {
            let frame_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.18, 0.18, 0.22),
                metallic: 0.7,
                perceptual_roughness: 0.4,
                ..default()
            });
            let mattress_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.25, 0.30, 0.45),
                perceptual_roughness: 0.95,
                ..default()
            });
            let frame = add_mesh(meshes, Cuboid::new(1.0, 0.15, 2.0));
            let mattress = add_mesh(meshes, Cuboid::new(0.9, 0.18, 1.85));
            let headboard = add_mesh(meshes, Cuboid::new(1.0, 0.5, 0.06));
            let count = if hw > 2.0 { 2 } else { 1 };
            for i in 0..count {
                let offset = if count == 1 {
                    0.0
                } else {
                    (i as f32 - 0.5) * 2.5
                };
                let bx = cx + offset;
                let bz = cz + hh * 0.4;
                commands.spawn((
                    Mesh3d(frame.clone()),
                    MeshMaterial3d(frame_mat.clone()),
                    Transform::from_xyz(bx, 0.12, bz),
                    re.clone(),
                ));
                commands.spawn((
                    Mesh3d(mattress.clone()),
                    MeshMaterial3d(mattress_mat.clone()),
                    Transform::from_xyz(bx, 0.28, bz),
                    re.clone(),
                ));
                commands.spawn((
                    Mesh3d(headboard.clone()),
                    MeshMaterial3d(frame_mat.clone()),
                    Transform::from_xyz(bx, 0.4, bz + 0.97),
                    re.clone(),
                ));
            }
        }
        // Cabin Double / Family / VIP — larger bed with frame + mattress + headboard
        11..=13 => {
            let frame_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.20, 0.28),
                metallic: 0.6,
                perceptual_roughness: 0.4,
                ..default()
            });
            let mattress_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.30, 0.28, 0.42),
                perceptual_roughness: 0.92,
                ..default()
            });
            let bx = cx;
            let bz = cz + hh * 0.4;
            commands.spawn((
                Mesh3d(add_mesh(meshes, Cuboid::new(1.6, 0.15, 2.0))),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(bx, 0.12, bz),
                re.clone(),
            ));
            commands.spawn((
                Mesh3d(add_mesh(meshes, Cuboid::new(1.5, 0.2, 1.85))),
                MeshMaterial3d(mattress_mat),
                Transform::from_xyz(bx, 0.3, bz),
                re.clone(),
            ));
            commands.spawn((
                Mesh3d(add_mesh(meshes, Cuboid::new(1.6, 0.6, 0.06))),
                MeshMaterial3d(frame_mat),
                Transform::from_xyz(bx, 0.45, bz + 0.97),
                re.clone(),
            ));
        }
        // Mess Hall / Wardroom / Cafe — table (top + 4 legs) + benches
        20 | 21 | 25 => {
            let table_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.32, 0.22),
                perceptual_roughness: 0.75,
                reflectance: 0.3,
                ..default()
            });
            let bench_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.32, 0.28, 0.22),
                perceptual_roughness: 0.8,
                ..default()
            });
            let top = add_mesh(meshes, Cuboid::new(1.8, 0.06, 0.9));
            let t_leg = add_mesh(meshes, Cuboid::new(0.06, 0.72, 0.06));
            let bench = add_mesh(meshes, Cuboid::new(1.6, 0.06, 0.3));
            let b_leg = add_mesh(meshes, Cuboid::new(0.05, 0.42, 0.05));
            let cols = ((hw * 2.0) / 3.0).floor().max(1.0) as i32;
            let rows = ((hh * 2.0) / 3.0).floor().max(1.0) as i32;
            for r in 0..rows.min(4) {
                for c in 0..cols.min(6) {
                    let x = cx - hw + 1.5 + c as f32 * 3.0;
                    let z = cz - hh + 1.5 + r as f32 * 3.0;
                    // Table top
                    commands.spawn((
                        Mesh3d(top.clone()),
                        MeshMaterial3d(table_mat.clone()),
                        Transform::from_xyz(x, 0.75, z),
                        re.clone(),
                    ));
                    // 4 table legs
                    for (lx, lz) in [(-0.8, -0.35), (0.8, -0.35), (-0.8, 0.35), (0.8, 0.35)] {
                        commands.spawn((
                            Mesh3d(t_leg.clone()),
                            MeshMaterial3d(table_mat.clone()),
                            Transform::from_xyz(x + lx, 0.36, z + lz),
                            re.clone(),
                        ));
                    }
                    // Benches on each side
                    for bz_off in [-0.7, 0.7] {
                        commands.spawn((
                            Mesh3d(bench.clone()),
                            MeshMaterial3d(bench_mat.clone()),
                            Transform::from_xyz(x, 0.45, z + bz_off),
                            re.clone(),
                        ));
                        for lx in [-0.7, 0.7] {
                            commands.spawn((
                                Mesh3d(b_leg.clone()),
                                MeshMaterial3d(bench_mat.clone()),
                                Transform::from_xyz(x + lx, 0.21, z + bz_off),
                                re.clone(),
                            ));
                        }
                    }
                }
            }
        }
        // Hospital / Surgery / Medbay — bed frame + mattress + side rails
        30 | 31 | 37 => {
            let frame_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.70, 0.72, 0.75),
                metallic: 0.3,
                perceptual_roughness: 0.25,
                reflectance: 0.5,
                ..default()
            });
            let mattress_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.85, 0.88),
                perceptual_roughness: 0.9,
                ..default()
            });
            let base = add_mesh(meshes, Cuboid::new(0.9, 0.3, 1.8));
            let mattress = add_mesh(meshes, Cuboid::new(0.8, 0.1, 1.7));
            let rail = add_mesh(meshes, Cuboid::new(0.04, 0.3, 1.4));
            let count = ((hw * 2.0) / 2.5).floor().max(1.0) as i32;
            for i in 0..count.min(6) {
                let x = cx - hw + 1.2 + i as f32 * 2.5;
                commands.spawn((
                    Mesh3d(base.clone()),
                    MeshMaterial3d(frame_mat.clone()),
                    Transform::from_xyz(x, 0.15, cz),
                    re.clone(),
                ));
                commands.spawn((
                    Mesh3d(mattress.clone()),
                    MeshMaterial3d(mattress_mat.clone()),
                    Transform::from_xyz(x, 0.35, cz),
                    re.clone(),
                ));
                // Side rails
                for rx in [-0.47, 0.47] {
                    commands.spawn((
                        Mesh3d(rail.clone()),
                        MeshMaterial3d(frame_mat.clone()),
                        Transform::from_xyz(x + rx, 0.5, cz),
                        re.clone(),
                    ));
                }
            }
        }
        // Gym — equipment: frame + cylinder bar + weight plates
        40 => {
            let frame_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.3, 0.35),
                metallic: 0.7,
                perceptual_roughness: 0.4,
                ..default()
            });
            let weight_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.15, 0.15, 0.18),
                metallic: 0.9,
                perceptual_roughness: 0.3,
                ..default()
            });
            let post = add_mesh(meshes, Cuboid::new(0.08, 1.4, 0.08));
            let bar = add_mesh(meshes, Cylinder::new(0.03, 1.0));
            let plate = add_mesh(meshes, Cylinder::new(0.2, 0.04));
            let count = ((hw * 2.0) / 2.0).floor().max(1.0) as i32;
            for i in 0..count.min(5) {
                let x = cx - hw + 1.0 + i as f32 * 2.0;
                let z = cz + hh * 0.3;
                // Two upright posts
                for px in [-0.4, 0.4] {
                    commands.spawn((
                        Mesh3d(post.clone()),
                        MeshMaterial3d(frame_mat.clone()),
                        Transform::from_xyz(x + px, 0.7, z),
                        re.clone(),
                    ));
                }
                // Horizontal bar
                commands.spawn((
                    Mesh3d(bar.clone()),
                    MeshMaterial3d(frame_mat.clone()),
                    Transform::from_xyz(x, 1.2, z)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                    re.clone(),
                ));
                // Weight plates on each end
                for px in [-0.5, 0.5] {
                    commands.spawn((
                        Mesh3d(plate.clone()),
                        MeshMaterial3d(weight_mat.clone()),
                        Transform::from_xyz(x + px, 1.2, z)
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                        re.clone(),
                    ));
                }
            }
        }
        // Engineering / Reactor — base + body + pipe cylinders
        60..=63 => {
            let body_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.35, 0.25, 0.15),
                metallic: 0.9,
                perceptual_roughness: 0.25,
                reflectance: 0.7,
                ..default()
            });
            let pipe_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.40, 0.35),
                metallic: 0.85,
                perceptual_roughness: 0.2,
                ..default()
            });
            // Main body
            commands.spawn((
                Mesh3d(add_mesh(meshes, Cuboid::new(2.0, 1.6, 2.0))),
                MeshMaterial3d(body_mat.clone()),
                Transform::from_xyz(cx, 0.8, cz),
                re.clone(),
            ));
            // Base plate
            commands.spawn((
                Mesh3d(add_mesh(meshes, Cuboid::new(2.4, 0.15, 2.4))),
                MeshMaterial3d(body_mat),
                Transform::from_xyz(cx, 0.075, cz),
                re.clone(),
            ));
            // Vertical pipe columns
            let pipe = add_mesh(meshes, Cylinder::new(0.12, 2.0));
            for (px, pz) in [(-0.8, -0.8), (0.8, -0.8), (-0.8, 0.8), (0.8, 0.8)] {
                commands.spawn((
                    Mesh3d(pipe.clone()),
                    MeshMaterial3d(pipe_mat.clone()),
                    Transform::from_xyz(cx + px, 1.0, cz + pz),
                    re.clone(),
                ));
            }
            // Blinking status lights on machinery
            let status_light = add_mesh(meshes, Sphere::new(0.06));
            let light_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.3, 0.1),
                emissive: Color::srgb(2.0, 0.6, 0.2).into(),
                ..default()
            });
            for (i, (lx, lz)) in [(0.6, 0.0), (-0.6, 0.0)].iter().enumerate() {
                commands.spawn((
                    Mesh3d(status_light.clone()),
                    MeshMaterial3d(light_mat.clone()),
                    Transform::from_xyz(cx + lx, 1.7, cz + lz),
                    BlinkingLight {
                        rate: 0.8 + i as f32 * 0.4,
                        phase: i as f32 * 0.5,
                    },
                    re.clone(),
                ));
            }
        }
        // Hydroponics — planter troughs with soil + green tops
        80 => {
            let trough_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.3, 0.32),
                metallic: 0.4,
                perceptual_roughness: 0.6,
                ..default()
            });
            let soil_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.18, 0.12, 0.08),
                perceptual_roughness: 0.95,
                ..default()
            });
            let plant_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.15, 0.45, 0.15),
                perceptual_roughness: 0.85,
                reflectance: 0.2,
                ..default()
            });
            let trough = add_mesh(meshes, Cuboid::new(0.8, 0.5, room.height - 1.0));
            let soil = add_mesh(meshes, Cuboid::new(0.7, 0.08, room.height - 1.2));
            let plants = add_mesh(meshes, Cuboid::new(0.6, 0.35, room.height - 1.4));
            let count = ((hw * 2.0) / 1.5).floor().max(1.0) as i32;
            for i in 0..count.min(8) {
                let x = cx - hw + 0.6 + i as f32 * 1.5;
                commands.spawn((
                    Mesh3d(trough.clone()),
                    MeshMaterial3d(trough_mat.clone()),
                    Transform::from_xyz(x, 0.25, cz),
                    re.clone(),
                ));
                commands.spawn((
                    Mesh3d(soil.clone()),
                    MeshMaterial3d(soil_mat.clone()),
                    Transform::from_xyz(x, 0.52, cz),
                    re.clone(),
                ));
                commands.spawn((
                    Mesh3d(plants.clone()),
                    MeshMaterial3d(plant_mat.clone()),
                    Transform::from_xyz(x, 0.75, cz),
                    re.clone(),
                ));
            }
        }
        // Cargo Bay — stacked crates of varying sizes
        90 | 91 => {
            let crate_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.35, 0.30, 0.22),
                metallic: 0.4,
                perceptual_roughness: 0.7,
                ..default()
            });
            let crate_dark = materials.add(StandardMaterial {
                base_color: Color::srgb(0.25, 0.22, 0.18),
                metallic: 0.35,
                perceptual_roughness: 0.75,
                ..default()
            });
            let big = add_mesh(meshes, Cuboid::new(1.2, 1.0, 1.2));
            let small = add_mesh(meshes, Cuboid::new(0.8, 0.6, 0.8));
            let count = ((hw * 2.0) / 2.0).floor().max(1.0) as i32;
            for i in 0..count.min(4) {
                let x = cx - hw + 1.0 + i as f32 * 2.0;
                let z = cz - hh * 0.3;
                // Bottom crate
                commands.spawn((
                    Mesh3d(big.clone()),
                    MeshMaterial3d(crate_mat.clone()),
                    Transform::from_xyz(x, 0.5, z),
                    re.clone(),
                ));
                // Stacked smaller crate (offset for visual interest)
                commands.spawn((
                    Mesh3d(small.clone()),
                    MeshMaterial3d(crate_dark.clone()),
                    Transform::from_xyz(x + 0.15, 1.3, z - 0.1),
                    re.clone(),
                ));
            }
        }
        _ => {} // No furniture for unlisted types
    }
}

/// Spawn a door frame (two posts + lintel + transom) at the given position.
/// `door_height`: height of the door opening. `wall_height`: full ceiling height.
/// `horizontal`: true if the wall runs along X (N/S walls), false for Z (E/W walls).
#[allow(clippy::too_many_arguments)]
fn spawn_door_frame(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    frame_mat: &Handle<StandardMaterial>,
    x: f32,
    z: f32,
    door_width: f32,
    door_height: f32,
    wall_height: f32,
    frame_depth: f32,
    post_w: f32,
    horizontal: bool,
    room_id: u32,
    deck: i32,
) {
    let re = RoomEntity { room_id, deck };
    let lintel_h: f32 = 0.12;
    let transom_h = wall_height - door_height - lintel_h;
    if horizontal {
        // Wall along X: posts offset in X, frame depth in Z
        let post_mesh = add_mesh(meshes, Cuboid::new(post_w, door_height, frame_depth));
        for sign in [-1.0_f32, 1.0] {
            commands.spawn((
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(x + sign * door_width / 2.0, door_height / 2.0, z),
                DoorMarker,
                re.clone(),
            ));
        }
        let lintel = add_mesh(
            meshes,
            Cuboid::new(door_width + post_w * 2.0, lintel_h, frame_depth),
        );
        commands.spawn((
            Mesh3d(lintel),
            MeshMaterial3d(frame_mat.clone()),
            Transform::from_xyz(x, door_height + lintel_h / 2.0, z),
            DoorMarker,
            re.clone(),
        ));
        // Transom wall above door opening
        if transom_h > 0.05 {
            let transom = add_mesh(
                meshes,
                Cuboid::new(door_width + post_w * 2.0, transom_h, frame_depth),
            );
            commands.spawn((
                Mesh3d(transom),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(x, door_height + lintel_h + transom_h / 2.0, z),
                DoorMarker,
                re,
            ));
        }
    } else {
        // Wall along Z: posts offset in Z, frame depth in X
        let post_mesh = add_mesh(meshes, Cuboid::new(frame_depth, door_height, post_w));
        for sign in [-1.0_f32, 1.0] {
            commands.spawn((
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(x, door_height / 2.0, z + sign * door_width / 2.0),
                DoorMarker,
                re.clone(),
            ));
        }
        let lintel = add_mesh(
            meshes,
            Cuboid::new(frame_depth, lintel_h, door_width + post_w * 2.0),
        );
        commands.spawn((
            Mesh3d(lintel),
            MeshMaterial3d(frame_mat.clone()),
            Transform::from_xyz(x, door_height + lintel_h / 2.0, z),
            DoorMarker,
            re.clone(),
        ));
        if transom_h > 0.05 {
            let transom = add_mesh(
                meshes,
                Cuboid::new(frame_depth, transom_h, door_width + post_w * 2.0),
            );
            commands.spawn((
                Mesh3d(transom),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(x, door_height + lintel_h + transom_h / 2.0, z),
                DoorMarker,
                re,
            ));
        }
    }
}

/// Animate door panels: smoothly slide open/closed based on server door state.
pub fn sync_door_panels(
    state: Res<ConnectionState>,
    mut panels: Query<(&DoorPanel, &mut Transform)>,
    time: Res<Time>,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };
    let dt = time.delta_secs();
    let speed = 2.5;

    for (panel, mut tf) in panels.iter_mut() {
        let is_open = conn
            .db
            .door()
            .iter()
            .find(|d| d.id == panel.door_id)
            .is_none_or(|d| d.is_open);

        let closed_pos = panel.frame_center + panel.side * panel.half_width;
        let open_pos = closed_pos + panel.side * panel.open_offset;
        let target = if is_open { open_pos } else { closed_pos };

        if panel.horizontal {
            let diff = target - tf.translation.x;
            if diff.abs() > 0.001 {
                let step = diff.signum() * speed * dt;
                if step.abs() >= diff.abs() {
                    tf.translation.x = target;
                } else {
                    tf.translation.x += step;
                }
            }
        } else {
            let diff = target - tf.translation.z;
            if diff.abs() > 0.001 {
                let step = diff.signum() * speed * dt;
                if step.abs() >= diff.abs() {
                    tf.translation.z = target;
                } else {
                    tf.translation.z += step;
                }
            }
        }
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
                if let Ok(mut cmd) = commands.get_entity(entity) {
                    cmd.despawn(); // recursive: also despawns indicator children
                }
            }
        }

        // Despawn indicators on surviving entities (will be recreated below)
        for entity in indicators.iter() {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
        }

        // Spawn only NEW people (not already in scene)
        let capsule_mesh = add_mesh(&mut meshes, Capsule3d::new(0.4, 1.2));

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
        let indicator_mesh = add_mesh(&mut meshes, Sphere::new(0.2));
        let convo_mesh = add_mesh(&mut meshes, Sphere::new(0.3));
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
                if let Ok(mut cmd) = commands.get_entity(entity) {
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
                if let Ok(mut cmd) = commands.get_entity(entity) {
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

/// PBR floor material tuned by room zone.
fn floor_material(color: Color, room_type: u8) -> StandardMaterial {
    let (roughness, metallic) = match room_type {
        // Medical: smooth clinical tile
        30..=37 => (0.3, 0.0),
        // Engineering/propulsion: industrial grating
        60..=71 => (0.65, 0.5),
        // Hydroponics: slightly damp concrete
        80..=86 => (0.8, 0.0),
        // Cargo: rough industrial
        90..=95 => (0.85, 0.15),
        // Corridors/infrastructure: worn non-slip
        100..=120 => (0.75, 0.1),
        // Habitation/recreation: carpet/composite
        _ => (0.9, 0.0),
    };
    StandardMaterial {
        base_color: color,
        perceptual_roughness: roughness,
        metallic,
        reflectance: 0.3,
        ..default()
    }
}

/// PBR wall material — painted steel panels with slight sheen.
fn wall_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.65,
        metallic: 0.15,
        reflectance: 0.4,
        ..default()
    }
}

/// Spawn floor border strips and hazard markings for rooms.
/// Non-corridor rooms get a thin colored perimeter strip; engineering/cargo get hazard striping.
fn spawn_floor_markings(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    room: &Room,
) {
    let re = RoomEntity {
        room_id: room.id,
        deck: room.deck,
    };
    let y = 0.11; // just above floor
    let stripe_w = 0.08;
    let inset = 0.2; // stripe inset from wall edge

    // Zone-colored perimeter border strips from cell mask
    let zone_color = zone_stripe_color(room.room_type);
    let stripe_mat = materials.add(StandardMaterial {
        base_color: zone_color,
        emissive: zone_color.into(),
        perceptual_roughness: 0.5,
        ..default()
    });

    let segments = progship_logic::movement::compute_room_perimeter(&room.cells, &[]);
    for seg in &segments {
        if seg.length < 0.5 {
            continue;
        }
        let stripe_len = seg.length - 2.0 * inset;
        if stripe_len < 0.2 {
            continue;
        }
        let is_horizontal = seg.direction == 0 || seg.direction == 1;
        let inset_offset = match seg.direction {
            0 => inset,
            1 => -inset,
            2 => -inset,
            3 => inset,
            _ => 0.0,
        };
        if is_horizontal {
            let mesh = add_mesh(meshes, Cuboid::new(stripe_len, 0.01, stripe_w));
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(stripe_mat.clone()),
                Transform::from_xyz(seg.x, y, seg.z + inset_offset),
                re.clone(),
            ));
        } else {
            let mesh = add_mesh(meshes, Cuboid::new(stripe_w, 0.01, stripe_len));
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(stripe_mat.clone()),
                Transform::from_xyz(seg.x + inset_offset, y, seg.z),
                re.clone(),
            ));
        }
    }

    // Hazard striping for dangerous rooms (engineering, cargo, airlock)
    let is_hazard = matches!(room.room_type, 60..=71 | 90..=94);
    if is_hazard {
        let hw = room.width / 2.0;
        let hh = room.height / 2.0;
        let yellow = materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.75, 0.0),
            emissive: Color::srgb(0.15, 0.12, 0.0).into(),
            perceptual_roughness: 0.6,
            ..default()
        });
        let dash = add_mesh(meshes, Cuboid::new(0.5, 0.01, stripe_w * 2.0));
        let n_z = room.y - hh + 0.5;
        let count = (room.width / 1.2).floor() as i32;
        for i in 0..count.min(20) {
            let x = room.x - hw + 0.8 + i as f32 * 1.2;
            commands.spawn((
                Mesh3d(dash.clone()),
                MeshMaterial3d(yellow.clone()),
                Transform::from_xyz(x, y + 0.005, n_z),
                re.clone(),
            ));
        }
    }
}

/// Spawn interior geometry for shaft rooms: ladders, elevator cars, call panels.
fn spawn_shaft_interior(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    room: &Room,
    wall_height: f32,
) {
    let cx = room.x;
    let cz = room.y;
    let re = RoomEntity {
        room_id: room.id,
        deck: room.deck,
    };

    match room.room_type {
        // Ladder shaft (111) — vertical rails + rungs
        111 => {
            let rail_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.25, 0.50, 0.30),
                metallic: 0.8,
                perceptual_roughness: 0.3,
                ..default()
            });
            let rung_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.35, 0.35, 0.38),
                metallic: 0.85,
                perceptual_roughness: 0.25,
                ..default()
            });
            // Two vertical rails on opposite sides of the shaft
            let rail = add_mesh(meshes, Cuboid::new(0.06, wall_height, 0.06));
            let rail_offset = room.width.min(room.height) / 2.0 - 0.4;
            for dx in [-0.25, 0.25] {
                commands.spawn((
                    Mesh3d(rail.clone()),
                    MeshMaterial3d(rail_mat.clone()),
                    Transform::from_xyz(cx + dx, wall_height / 2.0, cz - rail_offset),
                    re.clone(),
                ));
            }
            // Rungs every 0.3m
            let rung = add_mesh(meshes, Cylinder::new(0.025, 0.5));
            let rung_count = (wall_height / 0.3).floor() as i32;
            for i in 1..rung_count {
                let y = i as f32 * 0.3;
                commands.spawn((
                    Mesh3d(rung.clone()),
                    MeshMaterial3d(rung_mat.clone()),
                    Transform::from_xyz(cx, y, cz - rail_offset)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                    re.clone(),
                ));
            }
        }
        // Elevator shaft (110) / Service elevator (112) — car platform + guide rails
        110 | 112 => {
            let car_color = if room.room_type == 110 {
                Color::srgb(0.20, 0.30, 0.55)
            } else {
                Color::srgb(0.55, 0.35, 0.15)
            };
            let car_mat = materials.add(StandardMaterial {
                base_color: car_color,
                metallic: 0.7,
                perceptual_roughness: 0.35,
                ..default()
            });
            let rail_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.3, 0.35),
                metallic: 0.85,
                perceptual_roughness: 0.2,
                ..default()
            });
            let panel_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.08, 0.08, 0.12),
                emissive: Color::srgb(0.02, 0.06, 0.15).into(),
                metallic: 0.3,
                perceptual_roughness: 0.15,
                ..default()
            });

            // Elevator car platform (sitting at floor level)
            let car_w = room.width.min(room.height) - 0.8;
            commands.spawn((
                Mesh3d(add_mesh(meshes, Cuboid::new(car_w, 0.1, car_w))),
                MeshMaterial3d(car_mat.clone()),
                Transform::from_xyz(cx, 0.15, cz),
                re.clone(),
            ));
            // Low walls on car (waist-height railing)
            let rail_h = 1.0;
            let rail_panel = add_mesh(meshes, Cuboid::new(car_w, rail_h, 0.04));
            let side_panel = add_mesh(meshes, Cuboid::new(0.04, rail_h, car_w));
            // Back wall of car
            commands.spawn((
                Mesh3d(rail_panel.clone()),
                MeshMaterial3d(car_mat.clone()),
                Transform::from_xyz(cx, 0.2 + rail_h / 2.0, cz + car_w / 2.0 - 0.02),
                re.clone(),
            ));
            // Side panels
            for dx in [-1.0, 1.0] {
                commands.spawn((
                    Mesh3d(side_panel.clone()),
                    MeshMaterial3d(car_mat.clone()),
                    Transform::from_xyz(cx + dx * (car_w / 2.0 - 0.02), 0.2 + rail_h / 2.0, cz),
                    re.clone(),
                ));
            }
            // Guide rails in corners (floor to ceiling)
            let guide = add_mesh(meshes, Cuboid::new(0.08, wall_height, 0.08));
            let corner = room.width.min(room.height) / 2.0 - 0.15;
            for (gx, gz) in [
                (-corner, -corner),
                (corner, -corner),
                (-corner, corner),
                (corner, corner),
            ] {
                commands.spawn((
                    Mesh3d(guide.clone()),
                    MeshMaterial3d(rail_mat.clone()),
                    Transform::from_xyz(cx + gx, wall_height / 2.0, cz + gz),
                    re.clone(),
                ));
            }
            // Control panel on back wall
            commands.spawn((
                Mesh3d(add_mesh(meshes, Cuboid::new(0.4, 0.6, 0.04))),
                MeshMaterial3d(panel_mat),
                Transform::from_xyz(cx, 1.3, cz + car_w / 2.0 - 0.06),
                PulsingEmissive {
                    rate: 0.2,
                    phase: 0.0,
                    min_mul: 0.5,
                    max_mul: 1.5,
                },
                re.clone(),
            ));
        }
        _ => {}
    }
}

/// Zone stripe color — bright tinted guide strips per zone type.
fn zone_stripe_color(room_type: u8) -> Color {
    match room_type {
        0..=8 => Color::srgb(0.3, 0.3, 0.8),    // Command: blue
        10..=18 => Color::srgb(0.2, 0.5, 0.6),  // Habitation: teal
        20..=27 => Color::srgb(0.7, 0.55, 0.1), // Food: warm yellow
        30..=37 => Color::srgb(0.5, 0.8, 0.9),  // Medical: cyan
        40..=56 => Color::srgb(0.2, 0.6, 0.3),  // Recreation: green
        60..=71 => Color::srgb(0.8, 0.4, 0.1),  // Engineering: orange
        80..=86 => Color::srgb(0.1, 0.6, 0.4),  // Life support: teal-green
        90..=95 => Color::srgb(0.5, 0.4, 0.2),  // Cargo: brown
        _ => Color::srgb(0.3, 0.3, 0.3),        // Infrastructure: gray
    }
}

/// Returns (color, intensity_multiplier) for per-room point lights.
/// Distributed ceiling lights for a room. Places multiple point lights based on
/// room size: small rooms get 1, medium get 2, large get 3-4. Corridors get
/// strip lighting every 3-4m along their length.
///
/// In Solari mode, spawns emissive mesh panels instead of PointLights since
/// Solari only supports DirectionalLight + emissive meshes for lighting.
fn spawn_room_lights(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    room: &Room,
) {
    let (color, intensity) = room_light(room.room_type);
    let is_corridor = room_types::is_corridor(room.room_type);
    let y = if room.ceiling_height > 0.0 {
        room.ceiling_height - 0.2
    } else {
        3.3
    };

    // Collect light positions and per-light intensities
    let mut positions: Vec<(f32, f32, f32)> = Vec::new();
    let mut light_intensities: Vec<f32> = Vec::new();

    if is_corridor {
        let corridor_intensity = intensity * 600.0;
        let spacing = 3.5;
        let long = room.width.max(room.height);
        let is_horizontal = room.width >= room.height;
        let count = ((long / spacing).ceil() as i32).max(1);
        let start_offset = -(long / 2.0) + spacing / 2.0;

        for i in 0..count {
            let offset = start_offset + i as f32 * spacing;
            let (lx, lz) = if is_horizontal {
                (room.x + offset, room.y)
            } else {
                (room.x, room.y + offset)
            };
            positions.push((lx, y, lz));
            light_intensities.push(corridor_intensity);
        }
    } else {
        let room_intensity = intensity * 1000.0;
        let w = room.width;
        let h = room.height;
        let long = w.max(h);

        if long < 5.0 {
            positions.push((room.x, y, room.y));
            light_intensities.push(room_intensity);
        } else if long < 10.0 {
            let per_light = room_intensity * 0.6;
            let is_wide = w >= h;
            for frac in [0.33, 0.67] {
                let offset = long * (frac - 0.5);
                let (lx, lz) = if is_wide {
                    (room.x + offset, room.y)
                } else {
                    (room.x, room.y + offset)
                };
                positions.push((lx, y, lz));
                light_intensities.push(per_light);
            }
        } else {
            let per_light = room_intensity * 0.4;
            let offsets_x = [w * -0.25, w * 0.25];
            let offsets_z = [h * -0.25, h * 0.25];
            for &ox in &offsets_x {
                for &oz in &offsets_z {
                    positions.push((room.x + ox, y, room.y + oz));
                    light_intensities.push(per_light);
                }
            }
        }
    }

    let re = RoomEntity {
        room_id: room.id,
        deck: room.deck,
    };

    // Solari: emissive ceiling panels (Solari ignores PointLight)
    #[cfg(feature = "solari")]
    {
        let color_lin = color.to_linear();
        let panel_mesh = add_mesh(meshes, Cuboid::new(0.4, 0.04, 0.4));
        for (i, &(lx, ly, lz)) in positions.iter().enumerate() {
            // Scale emissive power to match visual brightness expectations
            let emissive_power = light_intensities[i] * 40.0;
            let emissive = LinearRgba::new(
                color_lin.red * emissive_power,
                color_lin.green * emissive_power,
                color_lin.blue * emissive_power,
                1.0,
            );
            let mat = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive,
                unlit: false,
                ..default()
            });
            commands.spawn((
                Mesh3d(panel_mesh.clone()),
                MeshMaterial3d(mat),
                Transform::from_xyz(lx, ly, lz),
                re.clone(),
            ));
        }
    }

    // Rasterized: traditional point lights
    #[cfg(not(feature = "solari"))]
    {
        let range_base = if is_corridor {
            3.5 * 1.2
        } else {
            room.width.max(room.height).min(8.0)
        };
        for (i, &(lx, ly, lz)) in positions.iter().enumerate() {
            commands.spawn((
                PointLight {
                    color,
                    intensity: light_intensities[i],
                    range: range_base,
                    shadows_enabled: !is_corridor && i == 0,
                    ..default()
                },
                Transform::from_xyz(lx, ly, lz),
                re.clone(),
            ));
        }
    }
}

fn room_light(room_type: u8) -> (Color, f32) {
    match room_type {
        // Command — cool white, bright
        0..=8 => (Color::srgb(0.85, 0.88, 1.0), 3.0),
        // Habitation — warm white, moderate
        10..=16 => (Color::srgb(1.0, 0.92, 0.80), 1.5),
        // Bathrooms/laundry — neutral, bright
        17..=18 => (Color::srgb(0.95, 0.95, 1.0), 2.0),
        // Food service — warm amber
        20..=27 => (Color::srgb(1.0, 0.88, 0.65), 2.5),
        // Medical — clinical white, very bright
        30..=37 => (Color::srgb(0.95, 0.97, 1.0), 4.0),
        // Recreation — warm daylight
        40..=56 => (Color::srgb(0.95, 0.92, 0.85), 2.0),
        // Engineering — deep amber/industrial, darker pools
        60..=71 => (Color::srgb(1.0, 0.65, 0.25), 1.5),
        // Life support — cyan tint
        80..=86 => (Color::srgb(0.80, 0.95, 1.0), 2.0),
        // Cargo — dim utility, widely spaced
        90..=95 => (Color::srgb(0.90, 0.85, 0.75), 0.8),
        // Corridors — neutral cool white, dim
        100..=115 => (Color::srgb(0.85, 0.88, 0.95), 1.0),
        // Fallback
        _ => (Color::srgb(0.90, 0.90, 0.90), 1.5),
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

/// When Solari is enabled, attach `RaytracingMesh3d` to all mesh entities so they
/// participate in hardware raytracing (shadows, GI, reflections).
#[cfg(feature = "solari")]
pub fn attach_raytracing_meshes(
    query: Query<(Entity, &Mesh3d), Without<bevy::solari::prelude::RaytracingMesh3d>>,
    mut commands: Commands,
) {
    for (entity, mesh3d) in &query {
        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.insert(bevy::solari::prelude::RaytracingMesh3d(mesh3d.0.clone()));
        }
    }
}

/// Animate blinking lights and pulsing emissive elements.
pub fn animate_details(
    time: Res<Time>,
    blink_query: Query<(&BlinkingLight, &MeshMaterial3d<StandardMaterial>)>,
    pulse_query: Query<(&PulsingEmissive, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let t = time.elapsed_secs();

    for (blink, mat_handle) in &blink_query {
        if let Some(mat) = materials.get_mut(mat_handle) {
            let phase = t * blink.rate + blink.phase * std::f32::consts::TAU;
            let on = phase.sin() > 0.0;
            let mul = if on { 1.0 } else { 0.05 };
            mat.emissive = LinearRgba::new(2.0 * mul, 0.6 * mul, 0.2 * mul, 1.0);
        }
    }

    for (pulse, mat_handle) in &pulse_query {
        if let Some(mat) = materials.get_mut(mat_handle) {
            let phase =
                t * pulse.rate * std::f32::consts::TAU + pulse.phase * std::f32::consts::TAU;
            let factor =
                pulse.min_mul + (pulse.max_mul - pulse.min_mul) * (0.5 + 0.5 * phase.sin());
            mat.emissive = LinearRgba::new(0.05 * factor, 0.15 * factor, 0.35 * factor, 1.0);
        }
    }
}

/// Spawn a handful of floating dust motes in atmospheric rooms.
fn spawn_dust_motes(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    room: &Room,
    wall_height: f32,
) {
    let dust_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.7, 0.6, 0.4),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let mote = add_mesh(meshes, Sphere::new(0.02));
    let re = RoomEntity {
        room_id: room.id,
        deck: room.deck,
    };

    let area = room.width * room.height;
    let count = ((area / 8.0).floor() as i32).clamp(2, 12);
    let mut seed = room.id.wrapping_mul(48271).max(1);
    let hw = room.width / 2.0 - 0.3;
    let hh = room.height / 2.0 - 0.3;

    for _ in 0..count {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let fx = (seed & 0xFFFF) as f32 / 65536.0;
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let fz = (seed & 0xFFFF) as f32 / 65536.0;
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let fy = (seed & 0xFFFF) as f32 / 65536.0;

        let x = room.x + (fx - 0.5) * 2.0 * hw;
        let z = room.y + (fz - 0.5) * 2.0 * hh;
        let y = 0.5 + fy * (wall_height - 1.0);

        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let drift_x = ((seed & 0xFF) as f32 / 255.0 - 0.5) * 0.02;
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let drift_z = ((seed & 0xFF) as f32 / 255.0 - 0.5) * 0.02;

        commands.spawn((
            Mesh3d(mote.clone()),
            MeshMaterial3d(dust_mat.clone()),
            Transform::from_xyz(x, y, z),
            DustMote {
                drift: Vec3::new(drift_x, 0.015, drift_z),
                lifetime: 8.0 + fy * 6.0,
                age: fy * 8.0,
            },
            re.clone(),
        ));
    }
}

/// Animate dust motes: drift upward and wrap when lifetime expires.
pub fn animate_dust_motes(time: Res<Time>, mut query: Query<(&mut DustMote, &mut Transform)>) {
    let dt = time.delta_secs();
    for (mut mote, mut tf) in &mut query {
        mote.age += dt;
        if mote.age >= mote.lifetime {
            mote.age -= mote.lifetime;
            tf.translation.y -= mote.drift.y * mote.lifetime;
        }
        tf.translation += mote.drift * dt;
    }
}
