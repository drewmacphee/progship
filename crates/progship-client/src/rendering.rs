//! 3D rendering for the ProgShip client.
//!
//! Handles room mesh generation, people capsules, door frames, and floor colors.

use bevy::prelude::*;
use progship_client_sdk::*;
use spacetimedb_sdk::Table;

use crate::state::{
    ConnectionState, DoorMarker, PersonEntity, PlayerState, RoomEntity, UiState, ViewState,
};

pub fn sync_rooms(
    state: Res<ConnectionState>,
    view: Res<ViewState>,
    mut commands: Commands,
    existing: Query<Entity, With<RoomEntity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };

    for entity in existing.iter() {
        if let Some(cmd) = commands.get_entity(entity) {
            cmd.despawn_recursive();
        }
    }

    // Collect doors for this deck
    let doors: Vec<_> = conn.db.door().iter().collect();

    // Collect all rooms for cross-deck door filtering
    let all_rooms: Vec<_> = conn.db.room().iter().collect();

    for room in conn.db.room().iter() {
        if room.deck != view.current_deck {
            continue;
        }

        let color = room_color(room.room_type);
        let w = room.width;
        let h = room.height;
        let wall_height = 3.0;
        let wall_thickness = 0.3;

        // Floor
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(w, 0.2, h))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                ..default()
            })),
            Transform::from_xyz(room.x, 0.0, -room.y),
            RoomEntity {
                room_id: room.id,
                deck: room.deck,
            },
        ));

        let wall_color = color.with_luminance(0.3);

        // Collect door world positions per wall from the Door table.
        // door_x/door_y store the absolute world position of the door center.
        // wall: NORTH=0, SOUTH=1, EAST=2, WEST=3
        let mut north_doors: Vec<(f32, f32)> = Vec::new(); // (world_x, door_width)
        let mut south_doors: Vec<(f32, f32)> = Vec::new();
        let mut east_doors: Vec<(f32, f32)> = Vec::new(); // (world_y, door_width)
        let mut west_doors: Vec<(f32, f32)> = Vec::new();

        for door in &doors {
            // Skip doors not connected to this room
            let is_a = door.room_a == room.id;
            let is_b = door.room_b == room.id;
            if !is_a && !is_b {
                continue;
            }

            // Skip cross-deck doors (elevator/ladder connections)
            let other_id = if is_a { door.room_b } else { door.room_a };
            if let Some(other_room) = all_rooms.iter().find(|r| r.id == other_id) {
                if other_room.deck != room.deck {
                    continue;
                }
            }

            // Which wall of THIS room is the door on?
            let wall = if is_a { door.wall_a } else { door.wall_b };

            // Use absolute door position directly
            let door_world_x = door.door_x;
            let door_world_y = door.door_y;

            // Place the gap on THIS room's wall at the door's world position
            match wall {
                0 | 1 => {
                    // NORTH or SOUTH: door position is along X axis
                    let list = if wall == 0 {
                        &mut north_doors
                    } else {
                        &mut south_doors
                    };
                    list.push((door_world_x, door.width));
                }
                2 | 3 => {
                    // EAST or WEST: door position is along Y axis
                    let list = if wall == 2 {
                        &mut east_doors
                    } else {
                        &mut west_doors
                    };
                    list.push((door_world_y, door.width));
                }
                _ => {}
            }
        }

        // North wall (NORTH = low Y = fore; in 3D: z = -(room.y - h/2) = less negative z)
        let north_pos: Vec<f32> = north_doors.iter().map(|d| d.0).collect();
        let north_widths: Vec<f32> = north_doors.iter().map(|d| d.1).collect();
        spawn_wall_with_gaps(
            &mut commands,
            &mut meshes,
            &mut materials,
            wall_color,
            room.x,
            -(room.y - h / 2.0),
            w,
            wall_height,
            wall_thickness,
            true,
            &north_pos,
            room.x,
            &north_widths,
            room.id,
            room.deck,
        );
        // South wall (SOUTH = high Y = aft; in 3D: z = -(room.y + h/2) = more negative z)
        let south_pos: Vec<f32> = south_doors.iter().map(|d| d.0).collect();
        let south_widths: Vec<f32> = south_doors.iter().map(|d| d.1).collect();
        spawn_wall_with_gaps(
            &mut commands,
            &mut meshes,
            &mut materials,
            wall_color,
            room.x,
            -(room.y + h / 2.0),
            w,
            wall_height,
            wall_thickness,
            true,
            &south_pos,
            room.x,
            &south_widths,
            room.id,
            room.deck,
        );
        // East wall (vertical, at x = room.x + w/2)
        let east_pos: Vec<f32> = east_doors.iter().map(|d| d.0).collect();
        let east_widths: Vec<f32> = east_doors.iter().map(|d| d.1).collect();
        spawn_wall_with_gaps(
            &mut commands,
            &mut meshes,
            &mut materials,
            wall_color,
            room.x + w / 2.0,
            -room.y,
            h,
            wall_height,
            wall_thickness,
            false,
            &east_pos,
            room.y,
            &east_widths,
            room.id,
            room.deck,
        );
        // West wall (vertical, at x = room.x - w/2)
        let west_pos: Vec<f32> = west_doors.iter().map(|d| d.0).collect();
        let west_widths: Vec<f32> = west_doors.iter().map(|d| d.1).collect();
        spawn_wall_with_gaps(
            &mut commands,
            &mut meshes,
            &mut materials,
            wall_color,
            room.x - w / 2.0,
            -room.y,
            h,
            wall_height,
            wall_thickness,
            false,
            &west_pos,
            room.y,
            &west_widths,
            room.id,
            room.deck,
        );

        // Door frame markers (gold pillars at each side of door gaps)
        let door_color = Color::srgb(0.8, 0.7, 0.2);
        let door_mat = materials.add(StandardMaterial {
            base_color: door_color,
            ..default()
        });
        let pillar_mesh = meshes.add(Cuboid::new(0.2, wall_height + 0.5, 0.2));

        // East/West doors: pillars along Z axis
        for &(dy, dw) in east_doors.iter() {
            let door_world_x = room.x + w / 2.0;
            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(door_world_x, wall_height / 2.0 + 0.25, -(dy - dw / 2.0)),
                DoorMarker,
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(door_world_x, wall_height / 2.0 + 0.25, -(dy + dw / 2.0)),
                DoorMarker,
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
        }
        for &(dy, dw) in west_doors.iter() {
            let door_world_x = room.x - w / 2.0;
            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(door_world_x, wall_height / 2.0 + 0.25, -(dy - dw / 2.0)),
                DoorMarker,
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(door_world_x, wall_height / 2.0 + 0.25, -(dy + dw / 2.0)),
                DoorMarker,
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
        }
        // North/South doors: pillars along X axis
        for &(dx, dw) in north_doors.iter() {
            let door_world_z = -(room.y - h / 2.0); // NORTH = low Y
            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(dx - dw / 2.0, wall_height / 2.0 + 0.25, door_world_z),
                DoorMarker,
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(dx + dw / 2.0, wall_height / 2.0 + 0.25, door_world_z),
                DoorMarker,
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
        }
        for &(dx, dw) in south_doors.iter() {
            let door_world_z = -(room.y + h / 2.0); // SOUTH = high Y
            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(dx - dw / 2.0, wall_height / 2.0 + 0.25, door_world_z),
                DoorMarker,
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(dx + dw / 2.0, wall_height / 2.0 + 0.25, door_world_z),
                DoorMarker,
                RoomEntity {
                    room_id: room.id,
                    deck: room.deck,
                },
            ));
        }
    }

    // Corridor floors already rendered by their Room entries (type 17/24)
    // The Corridor table is for data only (carries flags, connectivity), not rendering.

    // Render vertical shafts (elevators/ladders)
    for shaft in conn.db.vertical_shaft().iter() {
        let color = if shaft.shaft_type == 0 {
            Color::srgb(0.35, 0.35, 0.4) // Elevator
        } else {
            Color::srgb(0.3, 0.3, 0.35) // Ladder
        };
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(shaft.width, 0.25, shaft.height))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                ..default()
            })),
            Transform::from_xyz(shaft.x, 0.0, -shaft.y),
            RoomEntity {
                room_id: u32::MAX,
                deck: view.current_deck,
            },
        ));
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
        // No doors — solid wall
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
    // Convert door positions to offsets along the wall
    let mut gaps: Vec<(f32, f32)> = door_positions
        .iter()
        .zip(door_widths.iter())
        .map(|(&dp, &dw)| {
            let offset = if horizontal {
                dp - room_center
            } else {
                -(dp - room_center)
            };
            (offset - dw / 2.0, offset + dw / 2.0)
        })
        .collect();
    gaps.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let half_len = wall_length / 2.0;
    let mut cursor = -half_len;

    for (gap_start, gap_end) in &gaps {
        let seg_len = gap_start - cursor;
        if seg_len > 0.1 {
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
    let seg_len = half_len - cursor;
    if seg_len > 0.1 {
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

pub fn sync_people(
    state: Res<ConnectionState>,
    mut view: ResMut<ViewState>,
    player: Res<PlayerState>,
    ui: Res<UiState>,
    mut commands: Commands,
    mut existing: Query<(Entity, &PersonEntity, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };

    // Every frame: smoothly update ALL existing person transforms toward server positions
    for (_, pe, mut transform) in existing.iter_mut() {
        if let Some(pos) = conn.db.position().person_id().find(&pe.person_id) {
            if let Some(room) = conn.db.room().id().find(&pos.room_id) {
                if room.deck == view.current_deck {
                    let is_player = Some(pe.person_id) == player.person_id;
                    let target = Vec3::new(pos.x, transform.translation.y, -pos.y);
                    // Player lerps faster for responsiveness
                    let lerp_speed = if is_player { 0.5 } else { 0.2 };
                    transform.translation = transform.translation.lerp(target, lerp_speed);
                } else {
                    transform.translation.y = -100.0;
                }
            }
        }
    }

    // Full rebuild at 5Hz — despawn/respawn NPCs (not the player)
    view.people_sync_timer += time.delta_secs();
    if view.people_sync_timer < 0.2 {
        return;
    }
    view.people_sync_timer = 0.0;

    // Collect existing person IDs and despawn non-player entities
    let mut existing_player_entity: Option<Entity> = None;
    let mut entities_to_despawn = Vec::new();
    for (entity, pe, _) in existing.iter() {
        if Some(pe.person_id) == player.person_id {
            existing_player_entity = Some(entity);
        } else {
            entities_to_despawn.push(entity);
        }
    }
    for entity in entities_to_despawn {
        if let Some(cmd) = commands.get_entity(entity) {
            cmd.despawn_recursive();
        }
    }

    let capsule_mesh = meshes.add(Capsule3d::new(0.4, 1.2));
    let indicator_mesh = meshes.add(Sphere::new(0.2));

    for pos in conn.db.position().iter() {
        let Some(room) = conn.db.room().id().find(&pos.room_id) else {
            continue;
        };
        if room.deck != view.current_deck {
            continue;
        }

        let is_player = Some(pos.person_id) == player.person_id;

        // Skip spawning player if entity already exists (it persists across rebuilds)
        if is_player && existing_player_entity.is_some() {
            continue;
        }

        let person = conn.db.person().id().find(&pos.person_id);
        let is_crew = person.as_ref().map(|p| p.is_crew).unwrap_or(false);
        let is_selected = ui.selected_person == Some(pos.person_id);

        // Color: bright green for player, blue for crew, yellow for passengers
        let base_color = if is_player {
            Color::srgb(0.0, 1.0, 0.2)
        } else if is_crew {
            Color::srgb(0.3, 0.5, 1.0)
        } else {
            Color::srgb(0.9, 0.8, 0.3)
        };

        // Health-based tinting
        let needs = conn.db.needs().person_id().find(&pos.person_id);
        let health = needs.as_ref().map(|n| n.health).unwrap_or(1.0);
        let final_color = if health < 0.5 {
            Color::srgb(1.0, 0.2, 0.2)
        } else if is_selected {
            Color::srgb(1.0, 1.0, 1.0) // Highlight selected
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
            Transform::from_xyz(pos.x, person_height, -pos.y).with_scale(Vec3::new(
                1.0,
                if is_player { 1.2 } else { 1.0 },
                1.0,
            )),
            PersonEntity {
                person_id: pos.person_id,
            },
        ));

        // Activity indicator (small sphere above head)
        if let Some(activity) = conn.db.activity().person_id().find(&pos.person_id) {
            let indicator_color = activity_indicator_color(activity.activity_type);
            commands.spawn((
                Mesh3d(indicator_mesh.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: indicator_color,
                    emissive: indicator_color.into(),
                    ..default()
                })),
                Transform::from_xyz(pos.x, person_height * 2.0 + 0.8, -pos.y),
                PersonEntity {
                    person_id: pos.person_id,
                },
            ));
        }

        // Conversation bubble (flat disc above the activity indicator)
        if conn
            .db
            .in_conversation()
            .person_id()
            .find(&pos.person_id)
            .is_some()
        {
            commands.spawn((
                Mesh3d(meshes.add(Sphere::new(0.3))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 1.0, 0.5),
                    emissive: Color::srgb(0.5, 0.5, 0.0).into(),
                    ..default()
                })),
                Transform::from_xyz(pos.x + 0.5, person_height * 2.0 + 1.5, -pos.y),
                PersonEntity {
                    person_id: pos.person_id,
                },
            ));
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
        110 => Color::srgb(0.30, 0.30, 0.35), // Elevator Shaft
        111 => Color::srgb(0.28, 0.28, 0.32), // Ladder Shaft
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
