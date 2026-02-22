//! Greeble system — procedural surface detail on walls.
//!
//! Spawns after door gaps are computed so greebles never overlap openings.
//! All linear elements (conduit trays, trim strips, pipes) are segmented —
//! they break at every door opening and corridor intersection.
//!
//! Corridor elements (what they represent):
//! - Ceiling trim: edge moulding where ceiling meets wall
//! - Conduit tray: overhead cable management channel
//! - Pipes: coolant/air/water lines running along ceiling center
//! - Vent grilles: air recirculation intake/exhaust (flush with wall)
//! - Control panels: door access panels placed beside door openings

use bevy::prelude::*;
use progship_client_sdk::Room;
use progship_logic::constants::room_types;

use crate::rendering::add_mesh_pub;
use crate::state::RoomEntity;

/// Simple deterministic RNG (xorshift32) for greeble placement.
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    #[allow(dead_code)]
    fn f32(&mut self) -> f32 {
        (self.next() & 0x00FF_FFFF) as f32 / 16_777_216.0
    }
}

/// Per-wall gap data (axis_position, gap_width). Matches RoomWalls format.
#[derive(Default, Clone)]
pub struct WallGaps {
    pub n: Vec<(f32, f32)>,
    pub s: Vec<(f32, f32)>,
    pub e: Vec<(f32, f32)>,
    pub w: Vec<(f32, f32)>,
}

impl WallGaps {
    fn get(&self, wall: u8) -> &[(f32, f32)] {
        match wall {
            0 => &self.n,
            1 => &self.s,
            2 => &self.e,
            _ => &self.w,
        }
    }
}

/// Compute solid wall segments by subtracting gap regions from a range.
/// Returns list of (segment_start, segment_end) in world coordinates.
fn compute_wall_segments(
    wall_start: f32,
    wall_end: f32,
    gaps: &[(f32, f32)],
    margin: f32,
) -> Vec<(f32, f32)> {
    let mut cuts: Vec<(f32, f32)> = gaps
        .iter()
        .map(|&(pos, w)| (pos - w / 2.0 - margin, pos + w / 2.0 + margin))
        .collect();
    cuts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let mut segments = Vec::new();
    let mut cursor = wall_start;
    for (cut_start, cut_end) in &cuts {
        if *cut_start > cursor {
            let seg_start = cursor.max(wall_start);
            let seg_end = cut_start.min(wall_end);
            if seg_end - seg_start > 0.15 {
                segments.push((seg_start, seg_end));
            }
        }
        cursor = cursor.max(*cut_end);
    }
    if cursor < wall_end && wall_end - cursor > 0.15 {
        segments.push((cursor, wall_end));
    }
    segments
}

#[derive(Resource)]
pub struct GreebleLibrary {
    pub vent_grille: Handle<Mesh>,
    pub control_panel: Handle<Mesh>,
    pub conduit_bracket: Handle<Mesh>,
    // Materials
    pub mat_dark: Handle<StandardMaterial>,
    pub mat_mid: Handle<StandardMaterial>,
    pub mat_pipe: Handle<StandardMaterial>,
    pub mat_vent: Handle<StandardMaterial>,
    pub mat_trim: Handle<StandardMaterial>,
    pub mat_panel: Handle<StandardMaterial>,
}

pub fn init_greeble_library(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mat_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.22, 0.25),
        metallic: 0.8,
        perceptual_roughness: 0.35,
        ..default()
    });
    let mat_mid = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.30, 0.33),
        metallic: 0.75,
        perceptual_roughness: 0.4,
        ..default()
    });
    let mat_pipe = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.32, 0.28),
        metallic: 0.85,
        perceptual_roughness: 0.2,
        ..default()
    });
    let mat_vent = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.18, 0.20),
        metallic: 0.6,
        perceptual_roughness: 0.5,
        ..default()
    });
    let mat_trim = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.38, 0.40),
        metallic: 0.7,
        perceptual_roughness: 0.3,
        ..default()
    });
    let mat_panel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.28, 0.32),
        metallic: 0.5,
        perceptual_roughness: 0.6,
        ..default()
    });

    // Vent grille: thin flush-mounted air vent
    let vent_grille = add_mesh_pub(&mut meshes, Cuboid::new(0.30, 0.08, 0.005));
    // Control panel: small wall-mounted door access panel
    let control_panel = add_mesh_pub(&mut meshes, Cuboid::new(0.12, 0.18, 0.02));
    // Conduit bracket: support under conduit tray
    let conduit_bracket = add_mesh_pub(&mut meshes, Cuboid::new(0.04, 0.04, 0.04));

    commands.insert_resource(GreebleLibrary {
        vent_grille,
        control_panel,
        conduit_bracket,
        mat_dark,
        mat_mid,
        mat_pipe,
        mat_vent,
        mat_trim,
        mat_panel,
    });
}

/// Spawn greebles for a room, using door gap data to avoid openings.
pub fn spawn_room_greebles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    lib: &GreebleLibrary,
    room: &Room,
    gaps: &WallGaps,
) {
    if matches!(room.room_type, 110..=120) {
        return;
    }

    if room_types::is_corridor(room.room_type) {
        spawn_corridor_greebles(commands, meshes, lib, room, gaps);
    } else {
        spawn_room_wall_greebles(commands, meshes, lib, room, gaps);
    }
}

// ─── Corridor greebles ──────────────────────────────────────────────────────

const WALL_INSET: f32 = 0.15;

/// Spawn a segmented linear run: one mesh per solid wall segment between gaps.
fn spawn_segmented_run(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: &Handle<StandardMaterial>,
    re: &RoomEntity,
    is_horizontal: bool,
    wall_pos: f32,
    sign: f32,
    depth: f32,
    y: f32,
    height: f32,
    segments: &[(f32, f32)],
) {
    for &(seg_start, seg_end) in segments {
        let seg_len = seg_end - seg_start;
        let seg_center = (seg_start + seg_end) / 2.0;
        let mesh = add_mesh_pub(meshes, Cuboid::new(seg_len, height, depth));
        let (x, z, rot) = if is_horizontal {
            (seg_center, wall_pos + sign * depth / 2.0, Quat::IDENTITY)
        } else {
            (
                wall_pos + sign * depth / 2.0,
                seg_center,
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            )
        };
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(x, y, z).with_rotation(rot),
            re.clone(),
        ));
    }
}

/// Corridor greebles: segmented runs that break at every door opening.
fn spawn_corridor_greebles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    lib: &GreebleLibrary,
    room: &Room,
    gaps: &WallGaps,
) {
    let hw = room.width / 2.0;
    let hh = room.height / 2.0;
    let is_horizontal = room.width >= room.height;
    let re = RoomEntity {
        room_id: room.id,
        deck: room.deck,
    };
    let room_center = if is_horizontal { room.x } else { room.y };
    let long_half = if is_horizontal { hw } else { hh };
    let wall_start = room_center - long_half;
    let wall_end = room_center + long_half;

    // Long walls
    let (wall_a_pos, wall_b_pos, wall_a_id, wall_b_id) = if is_horizontal {
        (room.y - hh + WALL_INSET, room.y + hh - WALL_INSET, 0u8, 1u8)
    } else {
        (room.x + hw - WALL_INSET, room.x - hw + WALL_INSET, 2u8, 3u8)
    };

    for side in 0..2u8 {
        let wall_pos = if side == 0 { wall_a_pos } else { wall_b_pos };
        let sign: f32 = if side == 0 { 1.0 } else { -1.0 };
        let wall_id = if side == 0 { wall_a_id } else { wall_b_id };

        let wall_gaps = gaps.get(wall_id);
        let segments = compute_wall_segments(wall_start, wall_end, wall_gaps, 0.1);

        // Ceiling trim: thin edge strip at ceiling-wall junction (Y=2.88)
        spawn_segmented_run(
            commands,
            meshes,
            &lib.mat_trim,
            &re,
            is_horizontal,
            wall_pos,
            sign,
            0.02,
            2.88,
            0.03,
            &segments,
        );

        // Conduit tray: cable management channel (Y=2.65)
        spawn_segmented_run(
            commands,
            meshes,
            &lib.mat_dark,
            &re,
            is_horizontal,
            wall_pos,
            sign,
            0.08,
            2.65,
            0.04,
            &segments,
        );

        // Baseboard: floor-wall edge strip (Y=0.025)
        spawn_segmented_run(
            commands,
            meshes,
            &lib.mat_dark,
            &re,
            is_horizontal,
            wall_pos,
            sign,
            0.015,
            0.025,
            0.05,
            &segments,
        );

        // Conduit brackets: every 2.5m within solid segments only
        for &(seg_start, seg_end) in &segments {
            let seg_len = seg_end - seg_start;
            let bracket_count = (seg_len / 2.5).floor() as i32;
            for i in 0..bracket_count {
                let pos = seg_start + 1.25 + i as f32 * 2.5;
                if pos > seg_end - 0.3 {
                    break;
                }
                let (x, z) = if is_horizontal {
                    (pos, wall_pos + sign * 0.04)
                } else {
                    (wall_pos + sign * 0.04, pos)
                };
                commands.spawn((
                    Mesh3d(lib.conduit_bracket.clone()),
                    MeshMaterial3d(lib.mat_mid.clone()),
                    Transform::from_xyz(x, 2.58, z),
                    re.clone(),
                ));
            }
        }

        // Vent grilles: every 4m within solid segments (flush, Y=0.20)
        for &(seg_start, seg_end) in &segments {
            let seg_len = seg_end - seg_start;
            let vent_count = (seg_len / 4.0).floor() as i32;
            for i in 0..vent_count {
                let pos = seg_start + 2.0 + i as f32 * 4.0;
                if pos > seg_end - 0.5 {
                    break;
                }
                let (x, z) = if is_horizontal {
                    (pos, wall_pos + sign * 0.003)
                } else {
                    (wall_pos + sign * 0.003, pos)
                };
                commands.spawn((
                    Mesh3d(lib.vent_grille.clone()),
                    MeshMaterial3d(lib.mat_vent.clone()),
                    Transform::from_xyz(x, 0.20, z),
                    re.clone(),
                ));
            }
        }

        // Control panels: beside each door opening on this wall
        for &(gap_pos, gap_w) in wall_gaps {
            let panel_pos = gap_pos + gap_w / 2.0 + 0.25;
            if panel_pos < wall_end - 0.1 {
                let (x, z) = if is_horizontal {
                    (panel_pos, wall_pos + sign * 0.01)
                } else {
                    (wall_pos + sign * 0.01, panel_pos)
                };
                commands.spawn((
                    Mesh3d(lib.control_panel.clone()),
                    MeshMaterial3d(lib.mat_panel.clone()),
                    Transform::from_xyz(x, 1.30, z),
                    re.clone(),
                ));
            }
        }
    }

    // --- Ceiling pipes: run along corridor centerline ---
    // Pipes route across the ceiling center, above doorway height,
    // so they don't need to break at wall openings.
    let short_half = if is_horizontal { hh } else { hw };
    if short_half > 0.8 {
        let pipe_len = wall_end - wall_start;
        for pipe_offset in [-0.15f32, 0.15] {
            let pipe_mesh = add_mesh_pub(meshes, Cylinder::new(0.02, pipe_len));
            let (x, z, rot) = if is_horizontal {
                (
                    room_center,
                    room.y + pipe_offset,
                    Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                )
            } else {
                (
                    room.x + pipe_offset,
                    room_center,
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                )
            };
            commands.spawn((
                Mesh3d(pipe_mesh),
                MeshMaterial3d(lib.mat_pipe.clone()),
                Transform::from_xyz(x, 2.90, z).with_rotation(rot),
                re.clone(),
            ));
        }
    }
}

// ─── Room greebles ──────────────────────────────────────────────────────────

fn spawn_room_wall_greebles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    lib: &GreebleLibrary,
    room: &Room,
    gaps: &WallGaps,
) {
    let hw = room.width / 2.0;
    let hh = room.height / 2.0;
    let re = RoomEntity {
        room_id: room.id,
        deck: room.deck,
    };

    let has_conduit = matches!(room.room_type, 60..=71 | 80..=86 | 90..=95);

    for wall in 0..4u8 {
        let (wall_len, horiz) = match wall {
            0 | 1 => (room.width, true),
            _ => (room.height, false),
        };

        let wall_coord = match wall {
            0 => room.y - hh + WALL_INSET,
            1 => room.y + hh - WALL_INSET,
            2 => room.x + hw - WALL_INSET,
            3 => room.x - hw + WALL_INSET,
            _ => unreachable!(),
        };
        let sign: f32 = match wall {
            0 | 2 => 1.0,
            _ => -1.0,
        };
        let room_center = if horiz { room.x } else { room.y };
        let wall_start = room_center - wall_len / 2.0;
        let wall_end = room_center + wall_len / 2.0;
        let wall_gaps = gaps.get(wall);
        let segments = compute_wall_segments(wall_start, wall_end, wall_gaps, 0.1);

        if wall_len < 0.8 {
            continue;
        }

        // Baseboard trim: segmented at door openings
        spawn_segmented_run(
            commands,
            meshes,
            &lib.mat_dark,
            &re,
            horiz,
            wall_coord,
            sign,
            0.012,
            0.02,
            0.04,
            &segments,
        );

        // Conduit tray in industrial rooms (segmented)
        if has_conduit {
            spawn_segmented_run(
                commands,
                meshes,
                &lib.mat_dark,
                &re,
                horiz,
                wall_coord,
                sign,
                0.06,
                2.60,
                0.03,
                &segments,
            );
        }

        // Vent grilles: every 3.5m within solid segments (flush, Y=0.20)
        for &(seg_start, seg_end) in &segments {
            let seg_len = seg_end - seg_start;
            let vent_count = (seg_len / 3.5).floor() as i32;
            for i in 0..vent_count {
                let pos = seg_start + 1.75 + i as f32 * 3.5;
                if pos > seg_end - 0.5 {
                    break;
                }
                let (x, z) = if horiz {
                    (pos, wall_coord + sign * 0.003)
                } else {
                    (wall_coord + sign * 0.003, pos)
                };
                commands.spawn((
                    Mesh3d(lib.vent_grille.clone()),
                    MeshMaterial3d(lib.mat_vent.clone()),
                    Transform::from_xyz(x, 0.20, z),
                    re.clone(),
                ));
            }
        }

        // Control panels beside doors in this room
        for &(gap_pos, gap_w) in wall_gaps {
            let panel_pos = gap_pos + gap_w / 2.0 + 0.20;
            if panel_pos < wall_end - 0.1 {
                let (x, z) = if horiz {
                    (panel_pos, wall_coord + sign * 0.01)
                } else {
                    (wall_coord + sign * 0.01, panel_pos)
                };
                commands.spawn((
                    Mesh3d(lib.control_panel.clone()),
                    MeshMaterial3d(lib.mat_panel.clone()),
                    Transform::from_xyz(x, 1.30, z),
                    re.clone(),
                ));
            }
        }
    }
}
