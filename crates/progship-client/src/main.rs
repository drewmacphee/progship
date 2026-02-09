//! ProgShip Client - Bevy 3D game connecting to SpacetimeDB server
//!
//! Top-down 3D view of the colony ship. All simulation runs on the server.
//! The client renders the world, follows the player, and sends input.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use spacetimedb_sdk::{DbContext, Table};

use progship_client_sdk::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ProgShip - Colony Ship".to_string(),
                resolution: (1280.0, 720.0).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ConnectionState::Disconnected)
        .insert_resource(ViewState::default())
        .insert_resource(PlayerState::default())
        .insert_resource(UiState::default())
        .add_systems(Startup, setup_3d)
        .add_systems(
            Update,
            (
                connect_to_server,
                process_messages,
                auto_join_game,
                player_input,
                camera_follow_player,
                sync_rooms,
                sync_people,
                render_hud,
                render_info_panel,
                render_toasts,
            ),
        )
        .run();
}

// ============================================================================
// RESOURCES
// ============================================================================

#[derive(Resource)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected(Box<DbConnection>),
}

#[derive(Resource)]
struct ViewState {
    current_deck: i32,
    camera_height: f32,
    tick_timer: f32,
    people_sync_timer: f32,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            current_deck: 0,
            camera_height: 80.0,
            tick_timer: 0.0,
            people_sync_timer: 0.0,
        }
    }
}

#[derive(Resource)]
struct PlayerState {
    joined: bool,
    person_id: Option<u64>,
    /// Accumulated movement since last server send
    pending_dx: f32,
    pending_dy: f32,
    /// Timer for throttling movement sends
    move_send_timer: f32,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            joined: false,
            person_id: None,
            pending_dx: 0.0,
            pending_dy: 0.0,
            move_send_timer: 0.0,
        }
    }
}

#[derive(Resource, Default)]
struct UiState {
    selected_person: Option<u64>,
    show_ship_overview: bool,
    toasts: Vec<Toast>,
    last_event_count: usize,
}

struct Toast {
    message: String,
    _color: Color, // Reserved for future toast color coding
    timer: f32,
}

// ============================================================================
// BEVY COMPONENTS
// ============================================================================

#[derive(Component)]
struct RoomEntity {
    _room_id: u32, // Preserved for future room interaction
    _deck: i32,    // Preserved for future deck filtering
}

#[derive(Component)]
struct _RoomLabel; // Reserved for future 3D room labels

#[derive(Component)]
struct DoorMarker;

#[derive(Component)]
struct PersonEntity {
    person_id: u64,
}

#[derive(Component)]
struct PlayerCamera;

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct NeedsBar;

#[derive(Component)]
struct InfoPanel;

#[derive(Component)]
struct ToastContainer;

// ============================================================================
// TYPE ALIASES FOR COMPLEX BEVY QUERIES
// ============================================================================

type HudQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<HudText>,
        Without<InfoPanel>,
        Without<NeedsBar>,
        Without<ToastContainer>,
    ),
>;

type NeedsQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<NeedsBar>,
        Without<HudText>,
        Without<InfoPanel>,
        Without<ToastContainer>,
    ),
>;

type PanelQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<InfoPanel>,
        Without<HudText>,
        Without<NeedsBar>,
        Without<ToastContainer>,
    ),
>;

type ToastQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<ToastContainer>,
        Without<HudText>,
        Without<InfoPanel>,
        Without<NeedsBar>,
    ),
>;

// ============================================================================
// SETUP
// ============================================================================

fn setup_3d(mut commands: Commands) {
    // Top-down camera looking straight down
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 80.0, 0.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::NEG_Z),
        PlayerCamera,
    ));

    // Ambient light
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.9, 0.9, 1.0),
        brightness: 500.0,
    });

    // Directional light (overhead)
    commands.spawn((
        DirectionalLight {
            illuminance: 2000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 50.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // HUD - ship info (top-left)
    commands.spawn((
        Text::new("Connecting to SpacetimeDB..."),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
        HudText,
    ));

    // HUD - needs bars (bottom-left)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(10.0),
            ..default()
        },
        NeedsBar,
    ));

    // Info panel (right side — room info, selected NPC, or ship overview)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            top: Val::Px(10.0),
            max_width: Val::Px(320.0),
            ..default()
        },
        InfoPanel,
    ));

    // Toast container (top-center)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.9, 0.3)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(30.0),
            right: Val::Percent(30.0),
            top: Val::Px(50.0),
            ..default()
        },
        ToastContainer,
    ));
}

// ============================================================================
// CONNECTION
// ============================================================================

fn connect_to_server(mut state: ResMut<ConnectionState>) {
    if !matches!(*state, ConnectionState::Disconnected) {
        return;
    }

    info!("Connecting to SpacetimeDB...");
    *state = ConnectionState::Connecting;

    match DbConnection::builder()
        .with_uri("http://localhost:3000")
        .with_module_name("progship")
        .build()
    {
        Ok(conn) => {
            info!("Connected! Subscribing to tables...");
            conn.subscription_builder().subscribe([
                "SELECT * FROM ship_config",
                "SELECT * FROM room",
                "SELECT * FROM door",
                "SELECT * FROM corridor",
                "SELECT * FROM vertical_shaft",
                "SELECT * FROM graph_node",
                "SELECT * FROM graph_edge",
                "SELECT * FROM person",
                "SELECT * FROM position",
                "SELECT * FROM needs",
                "SELECT * FROM activity",
                "SELECT * FROM crew",
                "SELECT * FROM passenger",
                "SELECT * FROM deck_atmosphere",
                "SELECT * FROM ship_system",
                "SELECT * FROM subsystem",
                "SELECT * FROM system_component",
                "SELECT * FROM infra_edge",
                "SELECT * FROM ship_resources",
                "SELECT * FROM conversation",
                "SELECT * FROM in_conversation",
                "SELECT * FROM relationship",
                "SELECT * FROM event",
                "SELECT * FROM movement",
                "SELECT * FROM maintenance_task",
                "SELECT * FROM connected_player",
            ]);
            *state = ConnectionState::Connected(Box::new(conn));
        }
        Err(e) => {
            error!("Failed to connect: {:?}", e);
            *state = ConnectionState::Disconnected;
        }
    }
}

fn process_messages(mut state: ResMut<ConnectionState>) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };
    if let Err(e) = conn.frame_tick() {
        error!("Connection error: {:?}", e);
        *state = ConnectionState::Disconnected;
    }
}

// ============================================================================
// AUTO-JOIN
// ============================================================================

fn auto_join_game(state: Res<ConnectionState>, mut player: ResMut<PlayerState>) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };
    let Some(_config) = conn.db.ship_config().id().find(&0) else {
        return;
    };

    if !player.joined {
        info!("Subscription applied, joining game...");
        let _ = conn
            .reducers()
            .player_join("Player".into(), "One".into(), true);
        player.joined = true;
    }

    if player.person_id.is_none() {
        if let Some(my_identity) = conn.try_identity() {
            for person in conn.db.person().iter() {
                if person.owner_identity.as_ref() == Some(&my_identity) {
                    player.person_id = Some(person.id);
                    info!("Player character id: {}", person.id);
                    break;
                }
            }
        }
    }
}

// ============================================================================
// PLAYER INPUT
// ============================================================================

fn player_input(
    state: Res<ConnectionState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut view: ResMut<ViewState>,
    mut player: ResMut<PlayerState>,
    mut ui: ResMut<UiState>,
    mut scroll_events: EventReader<MouseWheel>,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };

    // WASD movement — accumulate locally, send batched
    let speed = 15.0 * time.delta_secs();
    let mut dx = 0.0f32;
    let mut dy = 0.0f32;
    if keyboard.pressed(KeyCode::KeyW) {
        dy -= speed;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        dy += speed;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        dx += speed;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        dx -= speed;
    }

    // Arrow keys for movement ONLY if not in a ladder shaft
    let in_ladder_shaft = player
        .person_id
        .and_then(|pid| {
            conn.db.position().person_id().find(&pid).and_then(|pos| {
                conn.db
                    .room()
                    .id()
                    .find(&pos.room_id)
                    .map(|r| r.room_type == 111)
            })
        })
        .unwrap_or(false);

    if !in_ladder_shaft {
        if keyboard.pressed(KeyCode::ArrowUp) {
            dy -= speed;
        }
        if keyboard.pressed(KeyCode::ArrowDown) {
            dy += speed;
        }
        if keyboard.pressed(KeyCode::ArrowLeft) {
            dx += speed;
        }
        if keyboard.pressed(KeyCode::ArrowRight) {
            dx -= speed;
        }
    }

    player.pending_dx += dx;
    player.pending_dy += dy;

    // Send movement to server at ~20Hz (every 50ms) instead of every frame
    player.move_send_timer += time.delta_secs();
    if player.move_send_timer >= 0.05 && (player.pending_dx != 0.0 || player.pending_dy != 0.0) {
        let _ = conn
            .reducers()
            .player_move(player.pending_dx, player.pending_dy);
        player.pending_dx = 0.0;
        player.pending_dy = 0.0;
        player.move_send_timer = 0.0;
    }

    // E to interact with nearest person
    if keyboard.just_pressed(KeyCode::KeyE) {
        if let Some(pid) = player.person_id {
            if let Some(my_pos) = conn.db.position().person_id().find(&pid) {
                let mut closest: Option<(u64, f32)> = None;
                for pos in conn.db.position().iter() {
                    if pos.person_id == pid {
                        continue;
                    }
                    if pos.room_id != my_pos.room_id {
                        continue;
                    }
                    let dist = ((pos.x - my_pos.x).powi(2) + (pos.y - my_pos.y).powi(2)).sqrt();
                    if dist < 15.0 && (closest.is_none() || dist < closest.unwrap().1) {
                        closest = Some((pos.person_id, dist));
                    }
                }
                if let Some((target_id, _)) = closest {
                    let _ = conn.reducers().player_interact(target_id);
                    ui.selected_person = Some(target_id);
                }
            }
        }
    }

    // F to perform context action (eat/sleep/repair/exercise/hygiene)
    if keyboard.just_pressed(KeyCode::KeyF) {
        if let Some(pid) = player.person_id {
            if let Some(pos) = conn.db.position().person_id().find(&pid) {
                if let Some(room) = conn.db.room().id().find(&pos.room_id) {
                    let action = match room.room_type {
                        9 | 10 => 2, // Mess/Galley → eat
                        5..=8 => {
                            // Quarters → sleep or hygiene
                            if let Some(needs) = conn.db.needs().person_id().find(&pid) {
                                if needs.hygiene > needs.fatigue {
                                    6
                                } else {
                                    3
                                }
                            } else {
                                3
                            }
                        }
                        2 | 3 | 4 | 21..=23 => 8, // Engineering rooms → repair
                        12 | 13 => 12,            // Recreation/Gym → exercise
                        _ => 255,                 // Invalid — server will reject
                    };
                    if action != 255 {
                        let _ = conn.reducers().player_action(action);
                        let action_name = match action {
                            2 => "Eating...",
                            3 => "Sleeping...",
                            6 => "Hygiene...",
                            8 => "Repairing...",
                            12 => "Exercising...",
                            _ => "Acting...",
                        };
                        ui.toasts.push(Toast {
                            message: action_name.to_string(),
                            _color: Color::srgb(0.5, 1.0, 0.5),
                            timer: 2.0,
                        });
                    }
                }
            }
        }
    }

    // Number keys 1-6 to use elevator/ladder (when in shaft room)
    if let Some(pid) = player.person_id {
        if let Some(pos) = conn.db.position().person_id().find(&pid) {
            if let Some(room) = conn.db.room().id().find(&pos.room_id) {
                if room.room_type == 110 {
                    // ELEVATOR_SHAFT
                    for (key, deck) in [
                        (KeyCode::Digit1, 0i32),
                        (KeyCode::Digit2, 1),
                        (KeyCode::Digit3, 2),
                        (KeyCode::Digit4, 3),
                        (KeyCode::Digit5, 4),
                        (KeyCode::Digit6, 5),
                    ] {
                        if keyboard.just_pressed(key) && deck != room.deck {
                            let _ = conn.reducers().player_use_elevator(deck);
                            ui.toasts.push(Toast {
                                message: format!("Taking elevator to Deck {}...", deck + 1),
                                _color: Color::srgb(0.5, 0.8, 1.0),
                                timer: 2.0,
                            });
                        }
                    }
                } else if room.room_type == 111 {
                    // LADDER_SHAFT
                    if keyboard.just_pressed(KeyCode::ArrowUp) {
                        let _ = conn.reducers().player_use_ladder(-1);
                        ui.toasts.push(Toast {
                            message: "Climbing up...".to_string(),
                            _color: Color::srgb(0.5, 0.8, 1.0),
                            timer: 2.0,
                        });
                    }
                    if keyboard.just_pressed(KeyCode::ArrowDown) {
                        let _ = conn.reducers().player_use_ladder(1);
                        ui.toasts.push(Toast {
                            message: "Climbing down...".to_string(),
                            _color: Color::srgb(0.5, 0.8, 1.0),
                            timer: 2.0,
                        });
                    }
                }
            }
        }
    }

    // Tab to toggle ship overview
    if keyboard.just_pressed(KeyCode::Tab) {
        ui.show_ship_overview = !ui.show_ship_overview;
        ui.selected_person = None;
    }

    // Q to select/deselect nearest NPC (without interacting)
    if keyboard.just_pressed(KeyCode::KeyQ) {
        if ui.selected_person.is_some() {
            ui.selected_person = None;
        } else if let Some(pid) = player.person_id {
            if let Some(my_pos) = conn.db.position().person_id().find(&pid) {
                let mut closest: Option<(u64, f32)> = None;
                for pos in conn.db.position().iter() {
                    if pos.person_id == pid {
                        continue;
                    }
                    if pos.room_id != my_pos.room_id {
                        continue;
                    }
                    let dist = ((pos.x - my_pos.x).powi(2) + (pos.y - my_pos.y).powi(2)).sqrt();
                    if dist < 20.0 && (closest.is_none() || dist < closest.unwrap().1) {
                        closest = Some((pos.person_id, dist));
                    }
                }
                ui.selected_person = closest.map(|(id, _)| id);
            }
        }
    }

    // Deck view follows player's current deck
    if let Some(pid) = player.person_id {
        if let Some(pos) = conn.db.position().person_id().find(&pid) {
            if let Some(room) = conn.db.room().id().find(&pos.room_id) {
                view.current_deck = room.deck;
            }
        }
    }

    // Simulation tick
    view.tick_timer += time.delta_secs();
    if view.tick_timer >= 0.1 {
        let _ = conn.reducers().tick(view.tick_timer);
        view.tick_timer = 0.0;
    }

    // Pause
    if keyboard.just_pressed(KeyCode::Space) {
        let paused = conn
            .db
            .ship_config()
            .id()
            .find(&0)
            .map(|c| c.paused)
            .unwrap_or(false);
        let _ = conn.reducers().set_paused(!paused);
    }

    // Time scale
    if keyboard.just_pressed(KeyCode::BracketRight) {
        let scale = conn
            .db
            .ship_config()
            .id()
            .find(&0)
            .map(|c| c.time_scale)
            .unwrap_or(1.0);
        let _ = conn.reducers().set_time_scale((scale * 2.0).min(100.0));
    }
    if keyboard.just_pressed(KeyCode::BracketLeft) {
        let scale = conn
            .db
            .ship_config()
            .id()
            .find(&0)
            .map(|c| c.time_scale)
            .unwrap_or(1.0);
        let _ = conn.reducers().set_time_scale((scale / 2.0).max(0.25));
    }

    // Zoom camera
    for event in scroll_events.read() {
        view.camera_height = (view.camera_height - event.y * 5.0).clamp(20.0, 200.0);
    }

    // Detect new events for toasts
    let active_events: Vec<_> = conn.db.event().iter().filter(|e| e.state != 2).collect();
    if active_events.len() > ui.last_event_count {
        for evt in active_events.iter().skip(ui.last_event_count) {
            let (msg, color) = event_toast_info(evt.event_type, evt.severity);
            if let Some(room) = conn.db.room().id().find(&evt.room_id) {
                ui.toasts.push(Toast {
                    message: format!("{} in {}", msg, room.name),
                    _color: color,
                    timer: 5.0,
                });
            }
        }
    }
    ui.last_event_count = active_events.len();

    // Tick toast timers
    let dt = time.delta_secs();
    ui.toasts.retain_mut(|t| {
        t.timer -= dt;
        t.timer > 0.0
    });
}

// ============================================================================
// CAMERA
// ============================================================================

fn camera_follow_player(
    state: Res<ConnectionState>,
    player: Res<PlayerState>,
    view: Res<ViewState>,
    mut camera_q: Query<&mut Transform, With<PlayerCamera>>,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };
    let Ok(mut cam_tf) = camera_q.get_single_mut() else {
        return;
    };
    let Some(pid) = player.person_id else { return };
    let Some(pos) = conn.db.position().person_id().find(&pid) else {
        return;
    };

    // Smooth camera follow — only move position, keep fixed top-down rotation
    let target = Vec3::new(pos.x, view.camera_height, -pos.y);
    cam_tf.translation = cam_tf.translation.lerp(target, 0.08);
}

// ============================================================================
// SYNC: SpacetimeDB → Bevy 3D entities
// ============================================================================

fn sync_rooms(
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
        commands.entity(entity).despawn_recursive();
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
                _room_id: room.id,
                _deck: room.deck,
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
                    _room_id: room.id,
                    _deck: room.deck,
                },
            ));
            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(door_world_x, wall_height / 2.0 + 0.25, -(dy + dw / 2.0)),
                DoorMarker,
                RoomEntity {
                    _room_id: room.id,
                    _deck: room.deck,
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
                    _room_id: room.id,
                    _deck: room.deck,
                },
            ));
            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(door_world_x, wall_height / 2.0 + 0.25, -(dy + dw / 2.0)),
                DoorMarker,
                RoomEntity {
                    _room_id: room.id,
                    _deck: room.deck,
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
                    _room_id: room.id,
                    _deck: room.deck,
                },
            ));
            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(dx + dw / 2.0, wall_height / 2.0 + 0.25, door_world_z),
                DoorMarker,
                RoomEntity {
                    _room_id: room.id,
                    _deck: room.deck,
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
                    _room_id: room.id,
                    _deck: room.deck,
                },
            ));
            commands.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(dx + dw / 2.0, wall_height / 2.0 + 0.25, door_world_z),
                DoorMarker,
                RoomEntity {
                    _room_id: room.id,
                    _deck: room.deck,
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
                _room_id: u32::MAX,
                _deck: view.current_deck,
            },
        ));
    }
}

/// Parameters for wall spawning
struct WallParams<'a> {
    color: Color,
    wall_x: f32,
    wall_z: f32,
    wall_length: f32,
    wall_height: f32,
    wall_thickness: f32,
    horizontal: bool,
    door_positions: &'a [f32],
    room_center: f32,
    door_widths: &'a [f32],
    room_id: u32,
    deck: i32,
}

/// Spawn a wall with gaps cut out for doors
fn spawn_wall_with_gaps(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    params: WallParams,
) {
    let (_room_id, _deck) = (params.room_id, params.deck); // Prepare for component construction
    let mat = materials.add(StandardMaterial {
        base_color: params.color,
        ..default()
    });

    if params.door_positions.is_empty() {
        // No doors — solid wall
        if params.horizontal {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(params.wall_length, params.wall_height, params.wall_thickness))),
                MeshMaterial3d(mat),
                Transform::from_xyz(params.wall_x, params.wall_height / 2.0, params.wall_z),
                RoomEntity { _room_id, _deck },
            ));
        } else {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(params.wall_thickness, params.wall_height, params.wall_length))),
                MeshMaterial3d(mat),
                Transform::from_xyz(params.wall_x, params.wall_height / 2.0, params.wall_z),
                RoomEntity { _room_id, _deck },
            ));
        }
        return;
    }

    // Build wall segments around door gaps
    // Convert door positions to offsets along the wall
    let mut gaps: Vec<(f32, f32)> = params.door_positions
        .iter()
        .zip(params.door_widths.iter())
        .map(|(&dp, &dw)| {
            let offset = if params.horizontal {
                dp - params.room_center
            } else {
                -(dp - params.room_center)
            };
            (offset - dw / 2.0, offset + dw / 2.0)
        })
        .collect();
    gaps.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let half_len = params.wall_length / 2.0;
    let mut cursor = -half_len;

    for (gap_start, gap_end) in &gaps {
        let seg_len = gap_start - cursor;
        if seg_len > 0.1 {
            let seg_center = cursor + seg_len / 2.0;
            if params.horizontal {
                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(seg_len, params.wall_height, params.wall_thickness))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(params.wall_x + seg_center, params.wall_height / 2.0, params.wall_z),
                    RoomEntity { _room_id, _deck },
                ));
            } else {
                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(params.wall_thickness, params.wall_height, seg_len))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(params.wall_x, params.wall_height / 2.0, params.wall_z + seg_center),
                    RoomEntity { _room_id, _deck },
                ));
            }
        }
        cursor = *gap_end;
    }

    // Final segment after last gap
    let seg_len = half_len - cursor;
    if seg_len > 0.1 {
        let seg_center = cursor + seg_len / 2.0;
        if params.horizontal {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(seg_len, params.wall_height, params.wall_thickness))),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(params.wall_x + seg_center, params.wall_height / 2.0, params.wall_z),
                RoomEntity { _room_id, _deck },
            ));
        } else {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(params.wall_thickness, params.wall_height, seg_len))),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(params.wall_x, params.wall_height / 2.0, params.wall_z + seg_center),
                RoomEntity { _room_id, _deck },
            ));
        }
    }
}

fn sync_people(
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
        commands.entity(entity).despawn_recursive();
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
        0 => Color::srgb(0.15, 0.2, 0.5),         // Bridge - dark blue
        1 => Color::srgb(0.25, 0.25, 0.4),        // Conference
        2 => Color::srgb(0.45, 0.25, 0.15),       // Engineering - brown
        3 => Color::srgb(0.5, 0.15, 0.1),         // Reactor - dark red
        4 => Color::srgb(0.35, 0.3, 0.2),         // Maintenance - tan
        5..=8 => Color::srgb(0.25, 0.35, 0.25),   // Quarters - green
        9 => Color::srgb(0.5, 0.4, 0.15),         // Mess - warm yellow
        10 => Color::srgb(0.4, 0.35, 0.15),       // Galley
        11 => Color::srgb(0.6, 0.6, 0.7),         // Medical - light blue
        12 => Color::srgb(0.25, 0.4, 0.35),       // Recreation - teal
        13 => Color::srgb(0.35, 0.4, 0.25),       // Gym
        14..=15 => Color::srgb(0.25, 0.25, 0.25), // Cargo/Storage - gray
        16 => Color::srgb(0.5, 0.1, 0.1),         // Airlock - red
        17 => Color::srgb(0.2, 0.2, 0.25),        // Corridor - dark gray
        18 => Color::srgb(0.35, 0.35, 0.4),       // Elevator
        19 => Color::srgb(0.25, 0.35, 0.5),       // Laboratory - blue
        20 => Color::srgb(0.15, 0.25, 0.4),       // Observatory
        21 => Color::srgb(0.25, 0.4, 0.5),        // Life Support - cyan
        22 => Color::srgb(0.15, 0.5, 0.15),       // Hydroponics - green
        23 => Color::srgb(0.15, 0.3, 0.5),        // Water Recycling - blue
        _ => Color::srgb(0.25, 0.25, 0.25),
    }
}

// ============================================================================
// HUD
// ============================================================================

fn render_hud(
    state: Res<ConnectionState>,
    view: Res<ViewState>,
    player: Res<PlayerState>,
    mut hud_q: HudQuery,
    mut needs_q: NeedsQuery,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => {
            if let Ok(mut text) = hud_q.get_single_mut() {
                **text = "Connecting...".into();
            }
            return;
        }
    };

    // Top-left: ship info
    if let Ok(mut text) = hud_q.get_single_mut() {
        let config = conn.db.ship_config().id().find(&0);
        let person_count = conn.db.person().count();
        let active_events: Vec<_> = conn.db.event().iter().filter(|e| e.state != 2).collect();

        let (ship_name, sim_time, time_scale, paused) = match config {
            Some(c) => (c.name.clone(), c.sim_time, c.time_scale, c.paused),
            None => ("No Ship".into(), 0.0, 1.0, false),
        };

        let hours = sim_time % 24.0;
        let day = (sim_time / 24.0) as u32 + 1;
        let h = hours as u32;
        let m = ((hours - h as f64) * 60.0) as u32;

        let pause_str = if paused { " [PAUSED]" } else { "" };
        let event_str = if !active_events.is_empty() {
            format!(" | {} EVENTS", active_events.len())
        } else {
            String::new()
        };

        // Get player's current room and context action
        let (room_name, context_hint) = player
            .person_id
            .and_then(|pid| conn.db.position().person_id().find(&pid))
            .and_then(|pos| conn.db.room().id().find(&pos.room_id))
            .map(|r| (r.name.clone(), context_action_hint(r.room_type)))
            .unwrap_or_default();

        // Get player activity
        let activity_str = player
            .person_id
            .and_then(|pid| conn.db.activity().person_id().find(&pid))
            .map(|a| format!(" ({})", activity_name(a.activity_type)))
            .unwrap_or_default();

        // Atmosphere info for current deck
        let atmo_str = conn
            .db
            .deck_atmosphere()
            .deck()
            .find(&view.current_deck)
            .map(|a| {
                let o2_pct = a.oxygen * 100.0;
                let temp = a.temperature;
                let warn = if o2_pct < 19.0 {
                    " LOW O2!"
                } else if temp > 30.0 {
                    " HOT!"
                } else if temp < 15.0 {
                    " COLD!"
                } else {
                    ""
                };
                format!("O2:{:.0}% {:.0}C{}", o2_pct, temp, warn)
            })
            .unwrap_or_default();

        **text = format!(
            "{} | Day {} {:02}:{:02}{} | {}x{}\n\
             Deck {} | {} | {} aboard | {}\n\
             {}{}\n\
             [WASD] Move [E] Talk [F]{} [Q] Inspect [Tab] Overview [Space] Pause",
            ship_name,
            day,
            h,
            m,
            pause_str,
            time_scale,
            event_str,
            view.current_deck + 1,
            room_name,
            person_count,
            atmo_str,
            activity_str,
            "",  // Placeholder for future activity details
            context_hint,
        );
    }

    // Bottom-left: player needs bars
    if let Ok(mut text) = needs_q.get_single_mut() {
        if let Some(pid) = player.person_id {
            if let Some(needs) = conn.db.needs().person_id().find(&pid) {
                let bar = |val: f32, label: &str, invert: bool| -> String {
                    let display = if invert { 1.0 - val } else { val };
                    let filled = (display * 10.0) as usize;
                    let empty = 10 - filled.min(10);
                    let status = if invert {
                        if val > 0.7 {
                            "!!"
                        } else if val > 0.4 {
                            "!"
                        } else {
                            ""
                        }
                    } else if val < 0.3 {
                        "!!"
                    } else if val < 0.6 {
                        "!"
                    } else {
                        ""
                    };
                    format!(
                        "{}: [{}{}] {:.0}%{}",
                        label,
                        "#".repeat(filled.min(10)),
                        "-".repeat(empty),
                        display * 100.0,
                        status
                    )
                };

                **text = format!(
                    "{}\n{}\n{}\n{}\n{}\n{}\nMorale: {:.0}%",
                    bar(needs.health, "HP     ", false),
                    bar(needs.hunger, "Hunger ", true),
                    bar(needs.fatigue, "Energy ", true),
                    bar(needs.social, "Social ", true),
                    bar(needs.comfort, "Comfort", true),
                    bar(needs.hygiene, "Hygiene", true),
                    needs.morale * 100.0,
                );
            }
        } else {
            **text = "Joining game...".into();
        }
    }
}

// ============================================================================
// INFO PANEL (right side)
// ============================================================================

fn render_info_panel(
    state: Res<ConnectionState>,
    _view: Res<ViewState>,
    player: Res<PlayerState>,
    ui: Res<UiState>,
    mut panel_q: PanelQuery,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };
    let Ok(mut text) = panel_q.get_single_mut() else {
        return;
    };

    if ui.show_ship_overview {
        // Ship overview (Tab)
        let config = conn.db.ship_config().id().find(&0);
        let crew_count = conn.db.crew().count();
        let passenger_count = conn.db.passenger().count();
        let active_events: Vec<_> = conn.db.event().iter().filter(|e| e.state != 2).collect();

        let mut overview = "=== SHIP OVERVIEW ===\n".to_string();
        if let Some(c) = &config {
            overview += &format!("{}\n\n", c.name);
        }
        overview += &format!("Crew: {}  Passengers: {}\n\n", crew_count, passenger_count);

        // Resources
        if let Some(res) = conn.db.ship_resources().id().find(&0) {
            overview += "--- Resources ---\n";
            overview += &format!("Power: {:.0}/{:.0}\n", res.power, res.power_cap);
            overview += &format!("Food:  {:.0}/{:.0}\n", res.food, res.food_cap);
            overview += &format!("Water: {:.0}/{:.0}\n", res.water, res.water_cap);
            overview += &format!("O2:    {:.0}/{:.0}\n", res.oxygen, res.oxygen_cap);
            overview += &format!("Fuel:  {:.0}/{:.0}\n", res.fuel, res.fuel_cap);
            overview += &format!(
                "Parts: {:.0}/{:.0}\n\n",
                res.spare_parts, res.spare_parts_cap
            );
        }

        // Systems
        let systems: Vec<_> = conn.db.ship_system().iter().collect();
        let degraded: Vec<_> = systems.iter().filter(|s| s.overall_health < 0.9).collect();
        if !degraded.is_empty() {
            overview += &format!(
                "--- Systems ({} issue{}) ---\n",
                degraded.len(),
                if degraded.len() > 1 { "s" } else { "" }
            );
            for sys in degraded.iter().take(5) {
                let status = system_status_str(sys.overall_status);
                overview += &format!(
                    "{}: {:.0}% [{}]\n",
                    sys.name,
                    sys.overall_health * 100.0,
                    status
                );
            }
            overview += "\n";
        }

        // Active events
        if !active_events.is_empty() {
            overview += &format!("--- Events ({}) ---\n", active_events.len());
            for evt in active_events.iter().take(5) {
                let etype = event_type_name(evt.event_type);
                let room_name = conn
                    .db
                    .room()
                    .id()
                    .find(&evt.room_id)
                    .map(|r| r.name.clone())
                    .unwrap_or("?".into());
                overview += &format!(
                    "{} in {} [{:.0}%]\n",
                    etype,
                    room_name,
                    evt.severity * 100.0
                );
            }
            overview += "\n";
        }

        // Deck atmospheres
        overview += "--- Atmosphere ---\n";
        for deck_idx in 0..6 {
            if let Some(atmo) = conn.db.deck_atmosphere().deck().find(&deck_idx) {
                let warn = if atmo.oxygen < 0.19 { " !" } else { "" };
                overview += &format!(
                    "Dk{}: O2:{:.0}% {:.0}C {:.0}kPa{}\n",
                    deck_idx + 1,
                    atmo.oxygen * 100.0,
                    atmo.temperature,
                    atmo.pressure,
                    warn
                );
            }
        }

        **text = overview;
        return;
    }

    if let Some(selected_id) = ui.selected_person {
        // NPC info panel
        let Some(person) = conn.db.person().id().find(&selected_id) else {
            **text = "".into();
            return;
        };

        let mut info = format!("=== {} {} ===\n", person.given_name, person.family_name);
        info += if person.is_crew { "Crew" } else { "Passenger" };

        if let Some(crew) = conn.db.crew().person_id().find(&selected_id) {
            info += &format!(
                "\n{} - {}\nShift: {}\n",
                department_name(crew.department),
                rank_name(crew.rank),
                shift_name(crew.shift)
            );
        }
        if let Some(passenger) = conn.db.passenger().person_id().find(&selected_id) {
            info += &format!(
                "\n{}\nDest: {}\n",
                passenger.profession, passenger.destination
            );
        }

        if let Some(needs) = conn.db.needs().person_id().find(&selected_id) {
            info += "\n--- Needs ---\n";
            info += &format!(
                "HP: {:.0}%  Morale: {:.0}%\n",
                needs.health * 100.0,
                needs.morale * 100.0
            );
            info += &format!(
                "Hunger: {:.0}%  Fatigue: {:.0}%\n",
                needs.hunger * 100.0,
                needs.fatigue * 100.0
            );
            info += &format!(
                "Social: {:.0}%  Hygiene: {:.0}%\n",
                needs.social * 100.0,
                needs.hygiene * 100.0
            );
        }

        if let Some(activity) = conn.db.activity().person_id().find(&selected_id) {
            info += &format!("\nActivity: {}\n", activity_name(activity.activity_type));
        }

        if let Some(pos) = conn.db.position().person_id().find(&selected_id) {
            if let Some(room) = conn.db.room().id().find(&pos.room_id) {
                info += &format!("Location: {}\n", room.name);
            }
        }

        // Conversation
        if let Some(in_conv) = conn.db.in_conversation().person_id().find(&selected_id) {
            if let Some(conv) = conn.db.conversation().id().find(&in_conv.conversation_id) {
                let other_id = if conv.participant_a == selected_id {
                    conv.participant_b
                } else {
                    conv.participant_a
                };
                let other_name = conn
                    .db
                    .person()
                    .id()
                    .find(&other_id)
                    .map(|p| format!("{} {}", p.given_name, p.family_name))
                    .unwrap_or("?".into());
                info += &format!(
                    "\nTalking to: {}\nTopic: {}\n",
                    other_name,
                    topic_name(conv.topic)
                );
            }
        }

        **text = info;
        return;
    }

    // Default: show current room info
    let Some(pid) = player.person_id else {
        **text = "".into();
        return;
    };
    let Some(pos) = conn.db.position().person_id().find(&pid) else {
        **text = "".into();
        return;
    };
    let Some(room) = conn.db.room().id().find(&pos.room_id) else {
        **text = "".into();
        return;
    };

    let mut info = format!(
        "=== {} ===\n{}\n\n",
        room.name,
        room_type_name(room.room_type)
    );

    // People in room
    let people_here: Vec<_> = conn
        .db
        .position()
        .iter()
        .filter(|p| p.room_id == room.id)
        .collect();
    info += &format!("Occupants: {}/{}\n", people_here.len(), room.capacity);
    for p in people_here.iter().take(8) {
        if let Some(person) = conn.db.person().id().find(&p.person_id) {
            let role = if Some(p.person_id) == player.person_id {
                " (You)"
            } else if person.is_crew {
                " [C]"
            } else {
                " [P]"
            };
            let activity_str = conn
                .db
                .activity()
                .person_id()
                .find(&p.person_id)
                .map(|a| format!(" - {}", activity_name(a.activity_type)))
                .unwrap_or_default();
            info += &format!(
                "  {} {}{}{}\n",
                person.given_name, person.family_name, role, activity_str
            );
        }
    }
    if people_here.len() > 8 {
        info += &format!("  ...and {} more\n", people_here.len() - 8);
    }

    // Subsystems in room
    let subsystems_here: Vec<_> = conn
        .db
        .subsystem()
        .iter()
        .filter(|s| s.node_id == room.node_id)
        .collect();
    if !subsystems_here.is_empty() {
        info += "\n--- Subsystems ---\n";
        for sub in &subsystems_here {
            let status = system_status_str(sub.status);
            info += &format!("{}: {:.0}% [{}]\n", sub.name, sub.health * 100.0, status);
        }
    }

    // Active events in room
    let events_here: Vec<_> = conn
        .db
        .event()
        .iter()
        .filter(|e| e.room_id == room.id && e.state != 2)
        .collect();
    if !events_here.is_empty() {
        info += "\n--- EVENTS ---\n";
        for evt in &events_here {
            info += &format!(
                "!! {} [{:.0}% severity]\n",
                event_type_name(evt.event_type),
                evt.severity * 100.0
            );
        }
    }

    **text = info;
}

// ============================================================================
// TOAST NOTIFICATIONS (top-center)
// ============================================================================

fn render_toasts(
    ui: Res<UiState>,
    mut toast_q: ToastQuery,
) {
    let Ok(mut text) = toast_q.get_single_mut() else {
        return;
    };
    if ui.toasts.is_empty() {
        **text = "".into();
    } else {
        let toast_text: Vec<String> = ui.toasts.iter().map(|t| t.message.clone()).collect();
        **text = toast_text.join("\n");
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn activity_name(activity_type: u8) -> &'static str {
    match activity_type {
        0 => "Idle",
        1 => "Working",
        2 => "Eating",
        3 => "Sleeping",
        4 => "Socializing",
        5 => "Relaxing",
        6 => "Hygiene",
        7 => "Traveling",
        8 => "Maintenance",
        9 => "On Duty",
        10 => "Off Duty",
        11 => "Emergency",
        12 => "Exercising",
        _ => "Unknown",
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

fn room_type_name(room_type: u8) -> &'static str {
    match room_type {
        0 => "Bridge",
        1 => "Conference Room",
        2 => "Engineering",
        3 => "Reactor",
        4 => "Maintenance Bay",
        5 => "Quarters",
        6 => "Crew Quarters",
        7 => "Officer Quarters",
        8 => "Passenger Quarters",
        9 => "Mess Hall",
        10 => "Galley",
        11 => "Medical Bay",
        12 => "Recreation",
        13 => "Gym",
        14 => "Cargo Bay",
        15 => "Storage",
        16 => "Airlock",
        17 => "Corridor",
        18 => "Elevator",
        19 => "Laboratory",
        20 => "Observatory",
        21 => "Life Support",
        22 => "Hydroponics",
        23 => "Water Recycling",
        24 => "Service Corridor",
        25 => "Elevator Shaft",
        26 => "Ladder Shaft",
        27 => "Surgery",
        28 => "Pharmacy",
        29 => "Recovery Ward",
        30 => "Chapel",
        31 => "Laundry",
        32 => "Shops",
        33 => "Lounge",
        34 => "CIC",
        35 => "Cooling Plant",
        36 => "Power Distribution",
        37 => "HVAC Control",
        38 => "Parts Storage",
        39 => "Waste Processing",
        40 => "Comms Room",
        _ => "Unknown",
    }
}

fn department_name(dept: u8) -> &'static str {
    match dept {
        0 => "Command",
        1 => "Engineering",
        2 => "Medical",
        3 => "Science",
        4 => "Security",
        5 => "Operations",
        6 => "Civilian",
        _ => "Unknown",
    }
}

fn rank_name(rank: u8) -> &'static str {
    match rank {
        0 => "Crewman",
        1 => "Specialist",
        2 => "Petty Officer",
        3 => "Chief",
        4 => "Ensign",
        5 => "Lieutenant",
        6 => "Commander",
        7 => "Captain",
        _ => "Unknown",
    }
}

fn shift_name(shift: u8) -> &'static str {
    match shift {
        0 => "Alpha (06:00-14:00)",
        1 => "Beta (14:00-22:00)",
        2 => "Gamma (22:00-06:00)",
        _ => "Unknown",
    }
}

fn topic_name(topic: u8) -> &'static str {
    match topic {
        0 => "Greeting",
        1 => "Work",
        2 => "Gossip",
        3 => "Personal",
        4 => "Complaint",
        5 => "Request",
        6 => "Flirtation",
        7 => "Argument",
        8 => "Farewell",
        _ => "Unknown",
    }
}

fn system_status_str(status: u8) -> &'static str {
    match status {
        0 => "OK",
        1 => "DEGRADED",
        2 => "CRITICAL",
        3 => "OFFLINE",
        4 => "DESTROYED",
        _ => "?",
    }
}

fn event_type_name(event_type: u8) -> &'static str {
    match event_type {
        0 => "System Failure",
        1 => "Medical Emergency",
        2 => "Fire",
        3 => "Hull Breach",
        4 => "Discovery",
        5 => "Celebration",
        6 => "Altercation",
        7 => "Resource Shortage",
        _ => "Unknown Event",
    }
}

fn event_toast_info(event_type: u8, severity: f32) -> (String, Color) {
    let name = event_type_name(event_type);
    let color = if severity > 0.7 {
        Color::srgb(1.0, 0.2, 0.2) // Red - critical
    } else if severity > 0.4 {
        Color::srgb(1.0, 0.7, 0.1) // Orange - warning
    } else if event_type == 4 || event_type == 5 {
        Color::srgb(0.3, 1.0, 0.3) // Green - positive
    } else {
        Color::srgb(1.0, 0.9, 0.3) // Yellow - info
    };
    (format!("!! {}", name), color)
}

fn context_action_hint(room_type: u8) -> &'static str {
    match room_type {
        9 | 10 => " Eat",
        5..=8 => " Sleep/Wash",
        2 | 3 | 4 | 21..=23 => " Repair",
        12 | 13 => " Exercise",
        25 => " Elevator [1-6]",
        26 => " Ladder [Up/Down]",
        _ => "",
    }
}
