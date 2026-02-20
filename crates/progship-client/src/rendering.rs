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

    let wall_height = 3.0;

    // --- Phase 1: Spawn floors, labels, furniture (per-room) ---
    for room in &deck_rooms {
        let color = room_color(room.room_type);
        commands.spawn((
            Mesh3d(add_mesh(
                &mut meshes,
                Cuboid::new(room.width, 0.2, room.height),
            )),
            MeshMaterial3d(materials.add(floor_material(color, room.room_type))),
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

        // Lighting — distributed point lights for all rooms including corridors
        spawn_room_lights(&mut commands, room);
    }

    // --- Phase 2: Per-room inset walls ---
    // Every room gets 4 walls, each 0.15m thick, inset 0.15m from the room edge.
    // Walls run the FULL length of each side (corners overlap at 90 deg, no gaps).
    // Two adjacent rooms = two back-to-back 0.15m walls = 0.3m total visual thickness.
    let wt: f32 = 0.15;
    let inset = wt / 2.0;

    struct RoomWalls {
        n_z: f32,
        s_z: f32,
        e_x: f32,
        w_x: f32,
        h_len: f32,
        v_len: f32,
        cx: f32,
        cz: f32,
        n_gaps: Vec<(f32, f32)>,
        s_gaps: Vec<(f32, f32)>,
        e_gaps: Vec<(f32, f32)>,
        w_gaps: Vec<(f32, f32)>,
    }

    let mut room_walls: Vec<(u32, i32, u8, RoomWalls)> = Vec::new();
    for room in &deck_rooms {
        let cx = room.x;
        let cz = room.y;
        let hw = room.width / 2.0;
        let hh = room.height / 2.0;
        room_walls.push((
            room.id,
            room.deck,
            room.room_type,
            RoomWalls {
                n_z: cz - hh + inset,
                s_z: cz + hh - inset,
                e_x: cx + hw - inset,
                w_x: cx - hw + inset,
                h_len: room.width,
                v_len: room.height,
                cx,
                cz,
                n_gaps: Vec::new(),
                s_gaps: Vec::new(),
                e_gaps: Vec::new(),
                w_gaps: Vec::new(),
            },
        ));
    }

    // --- Phase 3+4: Door-table-driven wall cuts ---
    // Read the Door table to determine where to cut gaps in walls.
    // This guarantees visual openings match server-side movement exactly.
    let post_w: f32 = 0.2;

    // Build room_id → deck_rooms index map
    let mut id_to_idx: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for (idx, room) in deck_rooms.iter().enumerate() {
        id_to_idx.insert(room.id, idx);
    }

    struct DoorwayCut {
        room_idx: usize,
        other_idx: usize,
        wall_side: u8,
        axis_pos: f32,
        width: f32,
    }
    let mut doorway_cuts: Vec<DoorwayCut> = Vec::new();

    // Iterate all same-deck doors (skip cross-deck doors)
    let deck_doors: Vec<_> = conn
        .db
        .door()
        .iter()
        .filter(|d| id_to_idx.contains_key(&d.room_a) && id_to_idx.contains_key(&d.room_b))
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

        // Determine gap width: corridors open fully (minus wall insets),
        // rooms/shafts use the Door table width directly.
        let gap_w = if both_plain {
            door.width - 2.0 * wt
        } else {
            door.width
        };
        if gap_w < 0.1 {
            continue;
        }

        // Determine which wall the door is on using wall_a/wall_b
        // wall_a is the wall side of room_a, wall_b is the wall side of room_b
        // NORTH=0 (low Y), SOUTH=1 (high Y), EAST=2 (high X), WEST=3 (low X)
        match door.wall_a {
            0 => {
                // room_a NORTH wall -> gap at door_x along x-axis
                room_walls[idx_a].3.n_gaps.push((door.door_x, gap_w));
            }
            1 => {
                room_walls[idx_a].3.s_gaps.push((door.door_x, gap_w));
            }
            2 => {
                room_walls[idx_a].3.e_gaps.push((door.door_y, gap_w));
            }
            3 => {
                room_walls[idx_a].3.w_gaps.push((door.door_y, gap_w));
            }
            _ => {}
        }
        match door.wall_b {
            0 => {
                room_walls[idx_b].3.n_gaps.push((door.door_x, gap_w));
            }
            1 => {
                room_walls[idx_b].3.s_gaps.push((door.door_x, gap_w));
            }
            2 => {
                room_walls[idx_b].3.e_gaps.push((door.door_y, gap_w));
            }
            3 => {
                room_walls[idx_b].3.w_gaps.push((door.door_y, gap_w));
            }
            _ => {}
        }

        // Only add door frame cuts for non-corridor-corridor pairs
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
            });
        }
    }

    // --- Phase 5: Draw walls ---
    for (room_id, deck, room_type, walls) in &room_walls {
        let wall_color = room_color(*room_type).with_luminance(0.3);
        // N wall (horizontal)
        let np: Vec<f32> = walls.n_gaps.iter().map(|g| g.0).collect();
        let nw: Vec<f32> = walls.n_gaps.iter().map(|g| g.1).collect();
        spawn_wall_with_gaps(
            &mut commands,
            &mut meshes,
            &mut materials,
            wall_color,
            walls.cx,
            walls.n_z,
            walls.h_len,
            wall_height,
            wt,
            true,
            &np,
            walls.cx,
            &nw,
            *room_id,
            *deck,
        );
        // S wall
        let sp: Vec<f32> = walls.s_gaps.iter().map(|g| g.0).collect();
        let sw_: Vec<f32> = walls.s_gaps.iter().map(|g| g.1).collect();
        spawn_wall_with_gaps(
            &mut commands,
            &mut meshes,
            &mut materials,
            wall_color,
            walls.cx,
            walls.s_z,
            walls.h_len,
            wall_height,
            wt,
            true,
            &sp,
            walls.cx,
            &sw_,
            *room_id,
            *deck,
        );
        // E wall (vertical)
        let ep: Vec<f32> = walls.e_gaps.iter().map(|g| g.0).collect();
        let ew_: Vec<f32> = walls.e_gaps.iter().map(|g| g.1).collect();
        spawn_wall_with_gaps(
            &mut commands,
            &mut meshes,
            &mut materials,
            wall_color,
            walls.e_x,
            walls.cz,
            walls.v_len,
            wall_height,
            wt,
            false,
            &ep,
            walls.cz,
            &ew_,
            *room_id,
            *deck,
        );
        // W wall
        let wp: Vec<f32> = walls.w_gaps.iter().map(|g| g.0).collect();
        let ww: Vec<f32> = walls.w_gaps.iter().map(|g| g.1).collect();
        spawn_wall_with_gaps(
            &mut commands,
            &mut meshes,
            &mut materials,
            wall_color,
            walls.w_x,
            walls.cz,
            walls.v_len,
            wall_height,
            wt,
            false,
            &wp,
            walls.cz,
            &ww,
            *room_id,
            *deck,
        );
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
    let lintel_height: f32 = 0.3;

    for cut in &doorway_cuts {
        let rwalls = &room_walls[cut.room_idx].3;
        let cwalls = &room_walls[cut.other_idx].3;
        let room_id = room_walls[cut.room_idx].0;
        let deck = room_walls[cut.room_idx].1;
        // Place frame centered between the room's wall and the corridor's wall
        let (fx, fz, horiz) = match cut.wall_side {
            0 => (cut.axis_pos, (rwalls.n_z + cwalls.s_z) / 2.0, true),
            1 => (cut.axis_pos, (rwalls.s_z + cwalls.n_z) / 2.0, true),
            2 => ((rwalls.e_x + cwalls.w_x) / 2.0, cut.axis_pos, false),
            3 => ((rwalls.w_x + cwalls.e_x) / 2.0, cut.axis_pos, false),
            _ => continue,
        };
        spawn_door_frame(
            &mut commands,
            &mut meshes,
            &frame_mat,
            fx,
            fz,
            cut.width,
            wall_height,
            frame_depth,
            post_w,
            lintel_height,
            horiz,
            room_id,
            deck,
        );
    }
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
                metallic: 0.6,
                perceptual_roughness: 0.35,
                ..default()
            });
            let desk = add_mesh(meshes, Cuboid::new(1.5, 0.8, 0.6));
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
                perceptual_roughness: 0.95,
                ..default()
            });
            let bed = add_mesh(meshes, Cuboid::new(1.0, 0.4, 2.0));
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
                perceptual_roughness: 0.92,
                ..default()
            });
            let bed = add_mesh(meshes, Cuboid::new(1.6, 0.4, 2.0));
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
                perceptual_roughness: 0.75,
                reflectance: 0.3,
                ..default()
            });
            let table = add_mesh(meshes, Cuboid::new(1.8, 0.75, 0.9));
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
                metallic: 0.3,
                perceptual_roughness: 0.25,
                reflectance: 0.5,
                ..default()
            });
            let bed = add_mesh(meshes, Cuboid::new(0.9, 0.5, 1.8));
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
                metallic: 0.7,
                perceptual_roughness: 0.4,
                ..default()
            });
            let equip = add_mesh(meshes, Cuboid::new(1.0, 1.2, 0.8));
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
                metallic: 0.9,
                perceptual_roughness: 0.25,
                reflectance: 0.7,
                ..default()
            });
            let machine = add_mesh(meshes, Cuboid::new(2.0, 2.0, 2.0));
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
                perceptual_roughness: 0.85,
                reflectance: 0.2,
                ..default()
            });
            let planter = add_mesh(meshes, Cuboid::new(0.8, 0.6, room.height - 1.0));
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
                metallic: 0.4,
                perceptual_roughness: 0.7,
                ..default()
            });
            let crate_mesh = add_mesh(meshes, Cuboid::new(1.2, 1.2, 1.2));
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
    let mat = materials.add(wall_material(color));

    if door_positions.is_empty() {
        // No doors — solid wall (corner posts handle corner fill)
        if horizontal {
            commands.spawn((
                Mesh3d(add_mesh(
                    meshes,
                    Cuboid::new(wall_length, wall_height, wall_thickness),
                )),
                MeshMaterial3d(mat),
                Transform::from_xyz(wall_x, wall_height / 2.0, wall_z),
                RoomEntity { room_id, deck },
            ));
        } else {
            commands.spawn((
                Mesh3d(add_mesh(
                    meshes,
                    Cuboid::new(wall_thickness, wall_height, wall_length),
                )),
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
                    Mesh3d(add_mesh(
                        meshes,
                        Cuboid::new(seg_len, wall_height, wall_thickness),
                    )),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(wall_x + seg_center, wall_height / 2.0, wall_z),
                    RoomEntity { room_id, deck },
                ));
            } else {
                commands.spawn((
                    Mesh3d(add_mesh(
                        meshes,
                        Cuboid::new(wall_thickness, wall_height, seg_len),
                    )),
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
                Mesh3d(add_mesh(
                    meshes,
                    Cuboid::new(seg_len, wall_height, wall_thickness),
                )),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(wall_x + seg_center, wall_height / 2.0, wall_z),
                RoomEntity { room_id, deck },
            ));
        } else {
            commands.spawn((
                Mesh3d(add_mesh(
                    meshes,
                    Cuboid::new(wall_thickness, wall_height, seg_len),
                )),
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
        let post_mesh = add_mesh(meshes, Cuboid::new(post_w, wall_height, frame_depth));
        for sign in [-1.0_f32, 1.0] {
            commands.spawn((
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(x + sign * door_width / 2.0, wall_height / 2.0, z),
                DoorMarker,
                re.clone(),
            ));
        }
        let lintel = add_mesh(
            meshes,
            Cuboid::new(door_width + post_w * 2.0, lintel_height, frame_depth),
        );
        commands.spawn((
            Mesh3d(lintel),
            MeshMaterial3d(frame_mat.clone()),
            Transform::from_xyz(x, wall_height - lintel_height / 2.0, z),
            DoorMarker,
            re,
        ));
    } else {
        // Wall along Z: posts offset in Z, frame depth in X
        let post_mesh = add_mesh(meshes, Cuboid::new(frame_depth, wall_height, post_w));
        for sign in [-1.0_f32, 1.0] {
            commands.spawn((
                Mesh3d(post_mesh.clone()),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(x, wall_height / 2.0, z + sign * door_width / 2.0),
                DoorMarker,
                re.clone(),
            ));
        }
        let lintel = add_mesh(
            meshes,
            Cuboid::new(frame_depth, lintel_height, door_width + post_w * 2.0),
        );
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

/// Returns (color, intensity_multiplier) for per-room point lights.
/// Distributed ceiling lights for a room. Places multiple point lights based on
/// room size: small rooms get 1, medium get 2, large get 3-4. Corridors get
/// strip lighting every 3-4m along their length.
fn spawn_room_lights(commands: &mut Commands, room: &Room) {
    let (color, intensity) = room_light(room.room_type);
    let is_corridor = room_types::is_corridor(room.room_type);
    let y = 2.8;

    if is_corridor {
        // Corridor strip lighting — dim, evenly spaced along the long axis
        let corridor_intensity = intensity * 600.0;
        let spacing = 3.5;
        let long = room.width.max(room.height);
        let is_horizontal = room.width >= room.height;
        let count = ((long / spacing).ceil() as i32).max(1);
        let start_offset = -(long / 2.0) + spacing / 2.0;
        let range = spacing * 1.2;

        for i in 0..count {
            let offset = start_offset + i as f32 * spacing;
            let (lx, lz) = if is_horizontal {
                (room.x + offset, room.y)
            } else {
                (room.x, room.y + offset)
            };
            commands.spawn((
                PointLight {
                    color,
                    intensity: corridor_intensity,
                    range,
                    shadows_enabled: false,
                    ..default()
                },
                Transform::from_xyz(lx, y, lz),
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
        }
    } else {
        // Room lighting — distribute based on size
        let room_intensity = intensity * 1000.0;
        let w = room.width;
        let h = room.height;
        let long = w.max(h);
        let range = long.min(8.0);

        // Enable shadows on one key light per room (the first one)
        let shadow_first = !cfg!(feature = "solari");

        if long < 5.0 {
            // Small room: single centered light
            commands.spawn((
                PointLight {
                    color,
                    intensity: room_intensity,
                    range,
                    shadows_enabled: shadow_first,
                    ..default()
                },
                Transform::from_xyz(room.x, y, room.y),
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
        } else if long < 10.0 {
            // Medium room: 2 lights at 1/3 and 2/3
            let per_light = room_intensity * 0.6;
            let is_wide = w >= h;
            for (i, frac) in [0.33, 0.67].iter().enumerate() {
                let offset = long * (frac - 0.5);
                let (lx, lz) = if is_wide {
                    (room.x + offset, room.y)
                } else {
                    (room.x, room.y + offset)
                };
                commands.spawn((
                    PointLight {
                        color,
                        intensity: per_light,
                        range,
                        shadows_enabled: shadow_first && i == 0,
                        ..default()
                    },
                    Transform::from_xyz(lx, y, lz),
                    RoomEntity {
                        room_id: room.id,
                        deck: room.deck,
                    },
                ));
            }
        } else {
            // Large room: 4 lights in a grid
            let per_light = room_intensity * 0.4;
            let offsets_x = [w * -0.25, w * 0.25];
            let offsets_z = [h * -0.25, h * 0.25];
            for (i, &ox) in offsets_x.iter().enumerate() {
                for &oz in &offsets_z {
                    commands.spawn((
                        PointLight {
                            color,
                            intensity: per_light,
                            range,
                            shadows_enabled: shadow_first && i == 0 && oz == offsets_z[0],
                            ..default()
                        },
                        Transform::from_xyz(room.x + ox, y, room.y + oz),
                        RoomEntity {
                            room_id: room.id,
                            deck: room.deck,
                        },
                    ));
                }
            }
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
