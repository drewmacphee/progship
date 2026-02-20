//! Greeble system — procedural surface detail on walls.
//!
//! Spawns after door gaps are computed so greebles never overlap openings.
//! Corridors get continuous engineered runs (conduit trays, pipe bundles, trim).
//! Rooms get zone-appropriate detail at logical height bands.

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
    /// Check if a position along a wall overlaps any gap (with margin).
    fn overlaps(&self, wall: u8, pos: f32, half_extent: f32) -> bool {
        let margin = 0.15;
        let gaps = match wall {
            0 => &self.n,
            1 => &self.s,
            2 => &self.e,
            _ => &self.w,
        };
        for &(gap_pos, gap_w) in gaps {
            let gap_half = gap_w / 2.0 + margin;
            if (pos - gap_pos).abs() < gap_half + half_extent {
                return true;
            }
        }
        false
    }
}

#[derive(Resource)]
pub struct GreebleLibrary {
    pub junction_box: Handle<Mesh>,
    pub vent_grille: Handle<Mesh>,
    pub bracket: Handle<Mesh>,
    // Materials
    pub mat_dark: Handle<StandardMaterial>,
    pub mat_mid: Handle<StandardMaterial>,
    pub mat_pipe: Handle<StandardMaterial>,
    pub mat_vent: Handle<StandardMaterial>,
    pub mat_trim: Handle<StandardMaterial>,
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

    // Punctual meshes — shared across all corridors/rooms
    let junction_box = add_mesh_pub(&mut meshes, Cuboid::new(0.2, 0.15, 0.08));
    let vent_grille = add_mesh_pub(&mut meshes, Cuboid::new(0.35, 0.12, 0.02));
    let bracket = add_mesh_pub(&mut meshes, Cuboid::new(0.06, 0.06, 0.05));

    commands.insert_resource(GreebleLibrary {
        junction_box,
        vent_grille,
        bracket,
        mat_dark,
        mat_mid,
        mat_pipe,
        mat_vent,
        mat_trim,
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

/// Corridor greebles: continuous engineered runs along the long axis.
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
    let long_len = room.width.max(room.height);
    let re = RoomEntity {
        room_id: room.id,
        deck: room.deck,
    };

    // Long walls: for horizontal corridors = N(0) and S(1); vertical = E(2) and W(3)
    let (wall_a_pos, wall_b_pos, wall_a_id, wall_b_id) = if is_horizontal {
        (room.y - hh + WALL_INSET, room.y + hh - WALL_INSET, 0u8, 1u8)
    } else {
        (room.x + hw - WALL_INSET, room.x - hw + WALL_INSET, 2u8, 3u8)
    };

    let protrude = 0.04;
    let room_center = if is_horizontal { room.x } else { room.y };

    // --- Ceiling trim strip (continuous, full length, both sides) ---
    let trim_mesh = add_mesh_pub(meshes, Cuboid::new(long_len, 0.03, 0.02));
    for side in 0..2 {
        let wall_pos = if side == 0 { wall_a_pos } else { wall_b_pos };
        let sign: f32 = if side == 0 { 1.0 } else { -1.0 };
        let (x, z, rot) = if is_horizontal {
            (room.x, wall_pos + sign * 0.01, Quat::IDENTITY)
        } else {
            (
                wall_pos + sign * 0.01,
                room.y,
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            )
        };
        commands.spawn((
            Mesh3d(trim_mesh.clone()),
            MeshMaterial3d(lib.mat_trim.clone()),
            Transform::from_xyz(x, 2.88, z).with_rotation(rot),
            re.clone(),
        ));
    }

    // --- Floor baseboard strip (continuous, both sides) ---
    let baseboard_mesh = add_mesh_pub(meshes, Cuboid::new(long_len, 0.05, 0.015));
    for side in 0..2 {
        let wall_pos = if side == 0 { wall_a_pos } else { wall_b_pos };
        let sign: f32 = if side == 0 { 1.0 } else { -1.0 };
        let (x, z, rot) = if is_horizontal {
            (room.x, wall_pos + sign * 0.008, Quat::IDENTITY)
        } else {
            (
                wall_pos + sign * 0.008,
                room.y,
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            )
        };
        commands.spawn((
            Mesh3d(baseboard_mesh.clone()),
            MeshMaterial3d(lib.mat_dark.clone()),
            Transform::from_xyz(x, 0.025, z).with_rotation(rot),
            re.clone(),
        ));
    }

    // --- Overhead conduit tray (full length, both sides, Y=2.65) ---
    let conduit_mesh = add_mesh_pub(meshes, Cuboid::new(long_len, 0.04, 0.10));
    for side in 0..2 {
        let wall_pos = if side == 0 { wall_a_pos } else { wall_b_pos };
        let sign: f32 = if side == 0 { 1.0 } else { -1.0 };
        let (x, z, rot) = if is_horizontal {
            (room.x, wall_pos + sign * (protrude + 0.05), Quat::IDENTITY)
        } else {
            (
                wall_pos + sign * (protrude + 0.05),
                room.y,
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            )
        };
        commands.spawn((
            Mesh3d(conduit_mesh.clone()),
            MeshMaterial3d(lib.mat_dark.clone()),
            Transform::from_xyz(x, 2.65, z).with_rotation(rot),
            re.clone(),
        ));
    }

    // --- Pipe bundle (2 parallel pipes, full length, Y=2.45) ---
    for pipe_offset in [0.0f32, 0.05] {
        let pipe_mesh = add_mesh_pub(meshes, Cylinder::new(0.02, long_len));
        for side in 0..2 {
            let wall_pos = if side == 0 { wall_a_pos } else { wall_b_pos };
            let sign: f32 = if side == 0 { 1.0 } else { -1.0 };
            let depth = protrude + 0.03 + pipe_offset;
            let (x, z, rot) = if is_horizontal {
                (
                    room.x,
                    wall_pos + sign * depth,
                    Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                )
            } else {
                (
                    wall_pos + sign * depth,
                    room.y,
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                )
            };
            commands.spawn((
                Mesh3d(pipe_mesh.clone()),
                MeshMaterial3d(lib.mat_pipe.clone()),
                Transform::from_xyz(x, 2.45, z).with_rotation(rot),
                re.clone(),
            ));
        }
    }

    // --- Support brackets under conduit (every 2m, gap-aware) ---
    let bracket_spacing = 2.0;
    let bracket_count = (long_len / bracket_spacing).floor() as i32;
    for side in 0..2u8 {
        let wall_pos = if side == 0 { wall_a_pos } else { wall_b_pos };
        let sign: f32 = if side == 0 { 1.0 } else { -1.0 };
        let wall_id = if side == 0 { wall_a_id } else { wall_b_id };

        for i in 0..bracket_count {
            let along_offset =
                -(long_len / 2.0) + bracket_spacing / 2.0 + i as f32 * bracket_spacing;
            let along_world = room_center + along_offset;

            if gaps.overlaps(wall_id, along_world, 0.03) {
                continue;
            }

            let (x, z) = if is_horizontal {
                (along_world, wall_pos + sign * protrude)
            } else {
                (wall_pos + sign * protrude, along_world)
            };
            commands.spawn((
                Mesh3d(lib.bracket.clone()),
                MeshMaterial3d(lib.mat_mid.clone()),
                Transform::from_xyz(x, 2.58, z),
                re.clone(),
            ));
        }
    }

    // --- Vent grilles (every 4m, floor level, gap-aware) ---
    let vent_spacing = 4.0;
    let vent_count = (long_len / vent_spacing).floor() as i32;
    for side in 0..2u8 {
        let wall_pos = if side == 0 { wall_a_pos } else { wall_b_pos };
        let sign: f32 = if side == 0 { 1.0 } else { -1.0 };
        let wall_id = if side == 0 { wall_a_id } else { wall_b_id };

        for i in 0..vent_count {
            let along_offset = -(long_len / 2.0) + vent_spacing / 2.0 + i as f32 * vent_spacing;
            let along_world = room_center + along_offset;

            if gaps.overlaps(wall_id, along_world, 0.175) {
                continue;
            }

            let (x, z) = if is_horizontal {
                (along_world, wall_pos + sign * protrude)
            } else {
                (wall_pos + sign * protrude, along_world)
            };
            commands.spawn((
                Mesh3d(lib.vent_grille.clone()),
                MeshMaterial3d(lib.mat_vent.clone()),
                Transform::from_xyz(x, 0.25, z),
                re.clone(),
            ));
        }
    }

    // --- Junction boxes (only near corridor ends, within 0.6m of short walls) ---
    let mut rng = Rng::new(room.id.wrapping_mul(2654435761));
    let end_zone = 0.6;
    for side in 0..2u8 {
        let wall_pos = if side == 0 { wall_a_pos } else { wall_b_pos };
        let sign: f32 = if side == 0 { 1.0 } else { -1.0 };
        let wall_id = if side == 0 { wall_a_id } else { wall_b_id };

        for end in 0..2 {
            let along_world = if end == 0 {
                room_center - long_len / 2.0 + end_zone * rng.f32().max(0.3)
            } else {
                room_center + long_len / 2.0 - end_zone * rng.f32().max(0.3)
            };
            if gaps.overlaps(wall_id, along_world, 0.1) {
                continue;
            }
            let jy = 1.4 + (rng.f32() - 0.5) * 0.3;
            let (x, z) = if is_horizontal {
                (along_world, wall_pos + sign * protrude)
            } else {
                (wall_pos + sign * protrude, along_world)
            };
            commands.spawn((
                Mesh3d(lib.junction_box.clone()),
                MeshMaterial3d(lib.mat_mid.clone()),
                Transform::from_xyz(x, jy, z),
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
    let mut rng = Rng::new(room.id.wrapping_mul(2654435761));
    let hw = room.width / 2.0;
    let hh = room.height / 2.0;
    let protrude = 0.03;
    let re = RoomEntity {
        room_id: room.id,
        deck: room.deck,
    };

    let density = match room.room_type {
        60..=71 => 0.6,  // Engineering: dense
        90..=95 => 0.4,  // Cargo: moderate
        80..=86 => 0.4,  // Life support: moderate
        0..=8 => 0.15,   // Command: sparse
        30..=37 => 0.1,  // Medical: very sparse
        10..=18 => 0.05, // Habitation: minimal
        _ => 0.25,
    };

    let has_upper = matches!(room.room_type, 60..=71 | 80..=86 | 90..=95);

    // Walls: N(0), S(1), E(2), W(3)
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

        let corner_margin = 0.3;
        let usable = wall_len - 2.0 * corner_margin;
        if usable < 0.5 {
            continue;
        }

        // --- Baseboard trim (continuous, full wall length) ---
        let baseboard_mesh = add_mesh_pub(meshes, Cuboid::new(wall_len, 0.04, 0.012));
        let (bx, bz, brot) = if horiz {
            (room_center, wall_coord + sign * 0.006, Quat::IDENTITY)
        } else {
            (
                wall_coord + sign * 0.006,
                room_center,
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            )
        };
        commands.spawn((
            Mesh3d(baseboard_mesh),
            MeshMaterial3d(lib.mat_dark.clone()),
            Transform::from_xyz(bx, 0.02, bz).with_rotation(brot),
            re.clone(),
        ));

        // --- Upper band (conduit bracket, 2.4-2.6m) — industrial rooms only ---
        if has_upper {
            let spacing = 2.5;
            let count = (usable / spacing).floor() as i32;
            for i in 0..count {
                let along_offset = -usable / 2.0 + spacing / 2.0 + i as f32 * spacing;
                let along_world = room_center + along_offset;
                if gaps.overlaps(wall, along_world, 0.03) {
                    continue;
                }
                let (x, z) = if horiz {
                    (along_world, wall_coord + sign * protrude)
                } else {
                    (wall_coord + sign * protrude, along_world)
                };
                commands.spawn((
                    Mesh3d(lib.bracket.clone()),
                    MeshMaterial3d(lib.mat_mid.clone()),
                    Transform::from_xyz(x, 2.5, z),
                    re.clone(),
                ));
            }
        }

        // --- Mid band (junction boxes, 1.0-1.8m) — density-based ---
        let mid_count = (usable * density).round() as i32;
        for _ in 0..mid_count {
            let along_offset = -usable / 2.0 + rng.f32() * usable;
            let along_world = room_center + along_offset;
            if gaps.overlaps(wall, along_world, 0.1) {
                continue;
            }
            let jy = 1.2 + rng.f32() * 0.6;
            let (x, z) = if horiz {
                (along_world, wall_coord + sign * protrude)
            } else {
                (wall_coord + sign * protrude, along_world)
            };
            commands.spawn((
                Mesh3d(lib.junction_box.clone()),
                MeshMaterial3d(if rng.f32() > 0.5 {
                    lib.mat_dark.clone()
                } else {
                    lib.mat_mid.clone()
                }),
                Transform::from_xyz(x, jy, z),
                re.clone(),
            ));
        }

        // --- Lower band (vents, 0.2-0.4m) — regular spacing ---
        let vent_spacing = 3.5;
        let vent_count = (usable / vent_spacing).floor() as i32;
        for i in 0..vent_count {
            let along_offset = -usable / 2.0 + vent_spacing / 2.0 + i as f32 * vent_spacing;
            let along_world = room_center + along_offset;
            if gaps.overlaps(wall, along_world, 0.175) {
                continue;
            }
            let (x, z) = if horiz {
                (along_world, wall_coord + sign * protrude)
            } else {
                (wall_coord + sign * protrude, along_world)
            };
            commands.spawn((
                Mesh3d(lib.vent_grille.clone()),
                MeshMaterial3d(lib.mat_vent.clone()),
                Transform::from_xyz(x, 0.3, z),
                re.clone(),
            ));
        }
    }
}
