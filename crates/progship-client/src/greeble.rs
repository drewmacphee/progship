//! Greeble scattering system — procedural surface detail on walls and ceilings.
//!
//! Uses deterministic RNG seeded from room ID to place small detail meshes
//! (panels, pipes, vents, conduit boxes) on room surfaces. Density varies
//! by room zone type. Bevy auto-batches entities sharing the same mesh+material.

use bevy::prelude::*;
use progship_client_sdk::Room;

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
    /// Returns float in [0, 1)
    fn f32(&mut self) -> f32 {
        (self.next() & 0x00FF_FFFF) as f32 / 16_777_216.0
    }
}

/// Shared greeble mesh and material handles, created once at startup.
#[derive(Resource)]
pub struct GreebleLibrary {
    /// (mesh, material, half_width, half_height, orientation)
    /// orientation: 0=wall, 1=ceiling
    pub entries: Vec<GreebleEntry>,
}

pub struct GreebleEntry {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub half_w: f32,
    pub half_h: f32,
}

/// Initialize the greeble mesh library at startup.
pub fn init_greeble_library(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let metal_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.22, 0.25),
        metallic: 0.8,
        perceptual_roughness: 0.35,
        ..default()
    });
    let metal_mid = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.30, 0.33),
        metallic: 0.75,
        perceptual_roughness: 0.4,
        ..default()
    });
    let pipe_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.32, 0.28),
        metallic: 0.85,
        perceptual_roughness: 0.2,
        ..default()
    });
    let vent_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.18, 0.20),
        metallic: 0.6,
        perceptual_roughness: 0.5,
        ..default()
    });

    let entries = vec![
        // 0: Small flat panel (wall detail)
        GreebleEntry {
            mesh: add_mesh_pub(&mut meshes, Cuboid::new(0.4, 0.3, 0.03)),
            material: metal_dark.clone(),
            half_w: 0.2,
            half_h: 0.15,
        },
        // 1: Conduit box
        GreebleEntry {
            mesh: add_mesh_pub(&mut meshes, Cuboid::new(0.25, 0.2, 0.12)),
            material: metal_mid.clone(),
            half_w: 0.125,
            half_h: 0.1,
        },
        // 2: Horizontal pipe segment
        GreebleEntry {
            mesh: add_mesh_pub(&mut meshes, Cylinder::new(0.04, 0.6)),
            material: pipe_mat.clone(),
            half_w: 0.3,
            half_h: 0.04,
        },
        // 3: Vent grille
        GreebleEntry {
            mesh: add_mesh_pub(&mut meshes, Cuboid::new(0.35, 0.15, 0.02)),
            material: vent_mat.clone(),
            half_w: 0.175,
            half_h: 0.075,
        },
        // 4: Support bracket (L-shape approximated as small cuboid)
        GreebleEntry {
            mesh: add_mesh_pub(&mut meshes, Cuboid::new(0.12, 0.12, 0.08)),
            material: metal_dark.clone(),
            half_w: 0.06,
            half_h: 0.06,
        },
        // 5: Wide panel
        GreebleEntry {
            mesh: add_mesh_pub(&mut meshes, Cuboid::new(0.6, 0.2, 0.02)),
            material: metal_mid,
            half_w: 0.3,
            half_h: 0.1,
        },
    ];

    commands.insert_resource(GreebleLibrary { entries });
}

/// Greeble density by room zone type. Returns minimum spacing between greebles.
fn greeble_spacing(room_type: u8) -> f32 {
    match room_type {
        60..=71 => 0.6,   // Engineering: dense
        90..=95 => 0.8,   // Cargo: moderate-dense
        80..=86 => 0.9,   // Life support: moderate
        100..=102 => 1.0, // Corridors: moderate
        0..=8 => 1.5,     // Command: sparse (clean)
        30..=37 => 1.8,   // Medical: sparse (clinical)
        10..=18 => 2.5,   // Habitation: very sparse
        _ => 1.2,         // Default: moderate
    }
}

/// Spawn greebles on the walls of a room using deterministic placement.
pub fn spawn_room_greebles(
    commands: &mut Commands,
    library: &GreebleLibrary,
    room: &Room,
    wall_height: f32,
) {
    if library.entries.is_empty() {
        return;
    }
    // Skip shafts and service decks
    if matches!(room.room_type, 110..=120) {
        return;
    }

    let spacing = greeble_spacing(room.room_type);
    let mut rng = Rng::new(room.id.wrapping_mul(2654435761)); // Knuth multiplicative hash

    let hw = room.width / 2.0;
    let hh = room.height / 2.0;
    let margin = 0.3; // edge avoidance
    let door_margin = 0.8; // extra clearance near room center (where doors often are)
    let re = RoomEntity {
        room_id: room.id,
        deck: room.deck,
    };

    // Place greebles on each wall (north=0, south=1, east=2, west=3)
    for wall in 0..4 {
        let (wall_len, wall_x, wall_z, horiz) = match wall {
            0 => (room.width, room.x, room.y - hh + 0.08, true),
            1 => (room.width, room.x, room.y + hh - 0.08, true),
            2 => (room.height, room.x + hw - 0.08, room.y, false),
            3 => (room.height, room.x - hw + 0.08, room.y, false),
            _ => unreachable!(),
        };

        let usable = wall_len - 2.0 * margin;
        if usable < spacing {
            continue;
        }

        // Simple grid-based placement with jitter (approximates Poisson disk)
        let count = (usable / spacing).floor() as i32;
        for i in 0..count {
            // Skip some positions randomly for organic feel
            if rng.f32() > 0.6 {
                continue;
            }

            let along = -usable / 2.0
                + spacing * 0.5
                + i as f32 * spacing
                + (rng.f32() - 0.5) * spacing * 0.4;
            let y = 1.5 + (rng.f32() - 0.5) * 1.5; // height: 0.75 to 2.25m

            // Skip near room center (door zones)
            if along.abs() < door_margin {
                continue;
            }

            let entry_idx = (rng.next() as usize) % library.entries.len();
            let entry = &library.entries[entry_idx];

            let (gx, gz, rot) = if horiz {
                (wall_x + along, wall_z, Quat::IDENTITY)
            } else {
                (
                    wall_x,
                    wall_z + along,
                    Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                )
            };

            commands.spawn((
                Mesh3d(entry.mesh.clone()),
                MeshMaterial3d(entry.material.clone()),
                Transform::from_xyz(gx, y, gz).with_rotation(rot),
                re.clone(),
            ));
        }
    }
}
