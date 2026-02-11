//! ProgShip Viewer - Bevy-based visualization for the simulation

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use progship_core::components::{
    Activity, ConversationTopic, Crew, Movement, Name, Needs, Passenger, Person, Position, Room,
    RoomType, Vec3 as SimVec3,
};
use progship_core::engine::SimulationEngine;
use progship_core::generation::ShipConfig;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ProgShip - Colony Ship Simulation".to_string(),
                resolution: (1280.0, 720.0).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .insert_resource(SimWrapper(SimulationEngine::new()))
        .insert_resource(CameraState::default())
        .insert_resource(ViewerConfig::default())
        .insert_resource(CurrentDeck(0))
        .insert_resource(SelectedPerson(None))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_simulation,
                camera_controls,
                deck_switching,
                handle_click,
                render_ship_hull,
                render_rooms,
                render_people,
                render_chat_bubbles,
                render_selection,
                render_ui,
                update_text_ui,
            ),
        )
        .run();
}

#[derive(Resource)]
struct SimWrapper(SimulationEngine);

#[derive(Resource)]
struct SelectedPerson(Option<hecs::Entity>);

#[derive(Resource)]
struct CameraState {
    target: Vec2,
    zoom: f32,
    dragging: bool,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            target: Vec2::ZERO,
            zoom: 1.0, // Start more zoomed in
            dragging: false,
        }
    }
}

#[derive(Resource)]
struct CurrentDeck(i32);

#[derive(Resource)]
struct ViewerConfig {
    time_scale: f32,
}

impl Default for ViewerConfig {
    fn default() -> Self {
        Self {
            time_scale: 1.0, // Real-time for smooth visuals (use +/- to adjust)
        }
    }
}

// Marker component for text UI elements
#[derive(Component)]
struct TimeText;

#[derive(Component)]
struct DeckText;

fn setup(mut commands: Commands, mut sim: ResMut<SimWrapper>, viewer_config: Res<ViewerConfig>) {
    // Setup camera
    commands.spawn(Camera2d::default());

    // Generate ship with 5,000 people
    let config = ShipConfig {
        name: "ISV Prometheus".to_string(),
        num_decks: 10,
        rooms_per_deck: 20,
        crew_size: 1000,
        passenger_capacity: 4000,
        ship_length: 400.0,
        ship_width: 60.0,
    };
    sim.0.generate(config.clone());
    sim.0.set_time_scale(viewer_config.time_scale);

    // Spawn UI text elements
    commands.spawn((
        Text2d::new("Day 1, 00:00"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(-500.0, 320.0, 100.0),
        TimeText,
    ));

    commands.spawn((
        Text2d::new("Deck 1"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
        Transform::from_xyz(-500.0, 295.0, 100.0),
        DeckText,
    ));

    info!(
        "Generated {} with {} crew, {} passengers, {} decks",
        config.name,
        sim.0.crew_count(),
        sim.0.passenger_count(),
        config.num_decks
    );
}

fn update_simulation(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sim: ResMut<SimWrapper>,
) {
    // Time scale controls: +/= to speed up, - to slow down, 0 to pause/resume
    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        let current = sim.0.time_scale();
        sim.0.set_time_scale((current * 2.0).min(100.0));
    }
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        let current = sim.0.time_scale();
        sim.0.set_time_scale((current / 2.0).max(0.25));
    }
    if keyboard.just_pressed(KeyCode::Digit0) || keyboard.just_pressed(KeyCode::Numpad0) {
        let current = sim.0.time_scale();
        if current > 0.0 {
            sim.0.set_time_scale(0.0);
        } else {
            sim.0.set_time_scale(1.0);
        }
    }

    // Save with S key
    if keyboard.just_pressed(KeyCode::KeyS)
        && (keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight))
    {
        match std::fs::File::create("save.bin") {
            Ok(file) => match sim.0.save(std::io::BufWriter::new(file)) {
                Ok(()) => println!("Saved simulation to save.bin"),
                Err(e) => eprintln!("Failed to save: {}", e),
            },
            Err(e) => eprintln!("Failed to create save file: {}", e),
        }
    }

    // Load with L key
    if keyboard.just_pressed(KeyCode::KeyL)
        && (keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight))
    {
        match std::fs::File::open("save.bin") {
            Ok(file) => match sim.0.load(std::io::BufReader::new(file)) {
                Ok(()) => println!("Loaded simulation from save.bin"),
                Err(e) => eprintln!("Failed to load: {}", e),
            },
            Err(e) => eprintln!("Failed to open save file: {}", e),
        }
    }

    sim.0.update(time.delta_secs());
}

fn camera_controls(
    mut camera_state: ResMut<CameraState>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut scroll_events: EventReader<MouseWheel>,
    mut motion_events: EventReader<MouseMotion>,
) {
    let pan_speed = 500.0 * camera_state.zoom;
    let zoom_speed = 0.1;
    let dt = 0.016;

    // Keyboard pan
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        camera_state.target.y += pan_speed * dt;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        camera_state.target.y -= pan_speed * dt;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        camera_state.target.x -= pan_speed * dt;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        camera_state.target.x += pan_speed * dt;
    }

    // Mouse drag
    camera_state.dragging =
        mouse_buttons.pressed(MouseButton::Middle) || mouse_buttons.pressed(MouseButton::Right);

    if camera_state.dragging {
        for motion in motion_events.read() {
            camera_state.target.x -= motion.delta.x * camera_state.zoom;
            camera_state.target.y += motion.delta.y * camera_state.zoom;
        }
    } else {
        motion_events.clear();
    }

    // Scroll zoom
    for scroll in scroll_events.read() {
        camera_state.zoom *= 1.0 - scroll.y * zoom_speed;
        camera_state.zoom = camera_state.zoom.clamp(0.1, 20.0); // Allow zooming in much closer
    }

    // Update camera transform
    if let Ok(mut transform) = camera_query.get_single_mut() {
        transform.translation.x = camera_state.target.x;
        transform.translation.y = camera_state.target.y;
        transform.scale = Vec3::splat(camera_state.zoom);
    }
}

fn deck_switching(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut current_deck: ResMut<CurrentDeck>,
    sim: Res<SimWrapper>,
) {
    let num_decks = sim
        .0
        .ship_layout
        .as_ref()
        .map(|l| l.decks.len())
        .unwrap_or(1) as i32;

    if keyboard.just_pressed(KeyCode::Digit1) {
        current_deck.0 = 0;
    }
    if keyboard.just_pressed(KeyCode::Digit2) && num_decks > 1 {
        current_deck.0 = 1;
    }
    if keyboard.just_pressed(KeyCode::Digit3) && num_decks > 2 {
        current_deck.0 = 2;
    }
    if keyboard.just_pressed(KeyCode::Digit4) && num_decks > 3 {
        current_deck.0 = 3;
    }
    if keyboard.just_pressed(KeyCode::Digit5) && num_decks > 4 {
        current_deck.0 = 4;
    }

    if keyboard.just_pressed(KeyCode::PageUp) && current_deck.0 < num_decks - 1 {
        current_deck.0 += 1;
    }
    if keyboard.just_pressed(KeyCode::PageDown) && current_deck.0 > 0 {
        current_deck.0 -= 1;
    }
}

fn render_ship_hull(sim: Res<SimWrapper>, mut gizmos: Gizmos) {
    let layout = match &sim.0.ship_layout {
        Some(l) => l,
        None => return,
    };

    let half_length = layout.ship_length / 2.0;
    let half_width = layout.ship_width / 2.0;

    // Draw hull outline as ellipse
    let segments = 32;
    let hull_color = Color::srgba(0.3, 0.3, 0.4, 0.5);

    for i in 0..segments {
        let t1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let t2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

        let p1 = Vec2::new(t1.cos() * half_length, t1.sin() * half_width);
        let p2 = Vec2::new(t2.cos() * half_length, t2.sin() * half_width);

        gizmos.line_2d(p1, p2, hull_color);
    }

    // Center line
    gizmos.line_2d(
        Vec2::new(-half_length, 0.0),
        Vec2::new(half_length, 0.0),
        Color::srgba(0.3, 0.3, 0.4, 0.3),
    );
}

fn render_rooms(sim: Res<SimWrapper>, current_deck: Res<CurrentDeck>, mut gizmos: Gizmos) {
    let layout = match &sim.0.ship_layout {
        Some(l) => l,
        None => return,
    };

    for &room_entity in &layout.rooms {
        let room = match sim.0.world.get::<&Room>(room_entity) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if room.deck_level != current_deck.0 {
            continue;
        }

        let (min_x, min_y, max_x, max_y) = room.world_bounds();
        let center = Vec2::new(room.world_x, room.world_y);
        let size = Vec2::new(max_x - min_x, max_y - min_y);

        let color = room_color(room.room_type);

        // Room fill
        gizmos.rect_2d(Isometry2d::from_translation(center), size, color);

        // Room border
        gizmos.rect_2d(
            Isometry2d::from_translation(center),
            size + Vec2::splat(0.5),
            Color::srgba(0.2, 0.2, 0.25, 0.8),
        );
    }
}

fn render_people(sim: Res<SimWrapper>, current_deck: Res<CurrentDeck>, mut gizmos: Gizmos) {
    for (_, (pos, _person, crew, passenger)) in sim
        .0
        .world
        .query::<(&Position, &Person, Option<&Crew>, Option<&Passenger>)>()
        .iter()
    {
        let room_entity = match &sim.0.ship_layout {
            Some(layout) if (pos.room_id as usize) < layout.rooms.len() => {
                layout.rooms[pos.room_id as usize]
            }
            _ => continue,
        };

        let room = match sim.0.world.get::<&Room>(room_entity) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if room.deck_level != current_deck.0 {
            continue;
        }

        let world_pos: SimVec3 = room.local_to_world(pos.local);

        let color = if crew.is_some() {
            Color::srgb(0.3, 0.5, 0.95)
        } else if passenger.is_some() {
            Color::srgb(0.3, 0.85, 0.4)
        } else {
            Color::srgb(0.5, 0.5, 0.5)
        };

        gizmos.circle_2d(
            Isometry2d::from_translation(Vec2::new(world_pos.x, world_pos.y)),
            0.4, // Smaller radius (0.4m = ~human shoulder width)
            color,
        );
    }
}

fn render_chat_bubbles(sim: Res<SimWrapper>, current_deck: Res<CurrentDeck>, mut gizmos: Gizmos) {
    // Pre-compute which person indices are in active conversations and their topics
    let mut active_participants: std::collections::HashMap<u32, ConversationTopic> =
        std::collections::HashMap::new();

    for (_, conv) in &sim.0.conversations.conversations {
        if conv.state == progship_core::components::ConversationState::Active {
            for &participant in &conv.participants {
                active_participants.insert(participant, conv.topic);
            }
        }
    }

    // Early exit if no conversations
    if active_participants.is_empty() {
        return;
    }

    let mut person_idx: u32 = 0;

    for (_, (pos, _person)) in sim.0.world.query::<(&Position, &Person)>().iter() {
        // Check if this person is in a conversation
        if let Some(&topic) = active_participants.get(&person_idx) {
            let room_entity = match &sim.0.ship_layout {
                Some(layout) if (pos.room_id as usize) < layout.rooms.len() => {
                    layout.rooms[pos.room_id as usize]
                }
                _ => {
                    person_idx += 1;
                    continue;
                }
            };

            let room = match sim.0.world.get::<&Room>(room_entity) {
                Ok(r) => r,
                Err(_) => {
                    person_idx += 1;
                    continue;
                }
            };

            if room.deck_level != current_deck.0 {
                person_idx += 1;
                continue;
            }

            let world_pos: SimVec3 = room.local_to_world(pos.local);

            // Draw chat bubble above person
            let bubble_pos = Vec2::new(world_pos.x, world_pos.y + 1.5);

            // Bubble color based on topic
            let bubble_color = match topic {
                ConversationTopic::Greeting | ConversationTopic::Farewell => {
                    Color::srgb(0.4, 0.8, 0.4)
                }
                ConversationTopic::Work => Color::srgb(0.4, 0.4, 0.8),
                ConversationTopic::Gossip => Color::srgb(0.8, 0.6, 0.8),
                ConversationTopic::Personal => Color::srgb(0.8, 0.8, 0.4),
                ConversationTopic::Complaint | ConversationTopic::Argument => {
                    Color::srgb(0.8, 0.4, 0.4)
                }
                ConversationTopic::Flirtation => Color::srgb(1.0, 0.5, 0.7),
                _ => Color::srgb(0.7, 0.7, 0.7),
            };

            // Draw bubble as small ellipse
            gizmos.ellipse_2d(
                Isometry2d::from_translation(bubble_pos),
                Vec2::new(1.0, 0.6),
                bubble_color,
            );

            // Draw small triangle pointer
            gizmos.line_2d(
                Vec2::new(world_pos.x - 0.3, world_pos.y + 0.9),
                Vec2::new(world_pos.x, world_pos.y + 0.6),
                bubble_color,
            );
            gizmos.line_2d(
                Vec2::new(world_pos.x + 0.3, world_pos.y + 0.9),
                Vec2::new(world_pos.x, world_pos.y + 0.6),
                bubble_color,
            );
        }

        person_idx += 1;
    }
}

fn handle_click(
    sim: Res<SimWrapper>,
    current_deck: Res<CurrentDeck>,
    camera_state: Res<CameraState>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut selected: ResMut<SelectedPerson>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = window_query.get_single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.get_single() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Find person closest to click
    let mut closest: Option<(hecs::Entity, f32)> = None;

    for (entity, (pos, _person)) in sim.0.world.query::<(&Position, &Person)>().iter() {
        let room_entity = match &sim.0.ship_layout {
            Some(layout) if (pos.room_id as usize) < layout.rooms.len() => {
                layout.rooms[pos.room_id as usize]
            }
            _ => continue,
        };

        let room = match sim.0.world.get::<&Room>(room_entity) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if room.deck_level != current_deck.0 {
            continue;
        }

        let person_world_pos: SimVec3 = room.local_to_world(pos.local);
        let dist =
            (world_pos.x - person_world_pos.x).powi(2) + (world_pos.y - person_world_pos.y).powi(2);

        if dist < 1.0 {
            // 1m click radius (smaller for smaller dots)
            if closest.is_none() || dist < closest.unwrap().1 {
                closest = Some((entity, dist));
            }
        }
    }

    selected.0 = closest.map(|(e, _)| e);
}

fn render_selection(
    sim: Res<SimWrapper>,
    current_deck: Res<CurrentDeck>,
    camera_state: Res<CameraState>,
    selected: Res<SelectedPerson>,
    mut gizmos: Gizmos,
) {
    let Some(entity) = selected.0 else { return };

    // Highlight selected person
    let Ok(pos) = sim.0.world.get::<&Position>(entity) else {
        return;
    };

    let room_entity = match &sim.0.ship_layout {
        Some(layout) if (pos.room_id as usize) < layout.rooms.len() => {
            layout.rooms[pos.room_id as usize]
        }
        _ => return,
    };

    let Ok(room) = sim.0.world.get::<&Room>(room_entity) else {
        return;
    };

    // Draw selection ring (even if on different deck - but fainter)
    let world_pos: SimVec3 = room.local_to_world(pos.local);
    let pos_vec = Vec2::new(world_pos.x, world_pos.y);

    let alpha = if room.deck_level == current_deck.0 {
        1.0
    } else {
        0.3
    };

    gizmos.circle_2d(
        Isometry2d::from_translation(pos_vec),
        2.0,
        Color::srgba(1.0, 1.0, 0.2, alpha),
    );

    // Info panel (on current deck only)
    if room.deck_level != current_deck.0 {
        return;
    }

    let scale = camera_state.zoom;
    let panel_x = pos_vec.x + 5.0;
    let panel_y = pos_vec.y + 5.0;

    // Panel background
    let panel_size = Vec2::new(60.0 * scale, 40.0 * scale);
    gizmos.rect_2d(
        Isometry2d::from_translation(Vec2::new(
            panel_x + panel_size.x / 2.0,
            panel_y - panel_size.y / 2.0,
        )),
        panel_size,
        Color::srgba(0.1, 0.1, 0.15, 0.9),
    );

    // Get person info
    let name = sim
        .0
        .world
        .get::<&Name>(entity)
        .map(|n| format!("{} {}", n.given, n.family))
        .unwrap_or_else(|_| "Unknown".to_string());

    let role = if sim.0.world.get::<&Crew>(entity).is_ok() {
        "Crew"
    } else if sim.0.world.get::<&Passenger>(entity).is_ok() {
        "Passenger"
    } else {
        "?"
    };

    let needs = sim.0.world.get::<&Needs>(entity).ok();
    let activity = sim.0.world.get::<&Activity>(entity).ok();

    // Draw indicators (since we can't draw text, use colored bars)
    let bar_y = panel_y - 8.0 * scale;
    let bar_height = 4.0 * scale;
    let bar_width = 50.0 * scale;

    if let Some(needs) = needs {
        // Hunger bar (red)
        let hunger_w = bar_width * (1.0 - needs.hunger);
        gizmos.rect_2d(
            Isometry2d::from_translation(Vec2::new(panel_x + hunger_w / 2.0, bar_y)),
            Vec2::new(hunger_w, bar_height),
            Color::srgb(0.2, 0.8, 0.3), // Green = fed
        );

        // Fatigue bar (blue)
        let fatigue_w = bar_width * (1.0 - needs.fatigue);
        gizmos.rect_2d(
            Isometry2d::from_translation(Vec2::new(panel_x + fatigue_w / 2.0, bar_y - 6.0 * scale)),
            Vec2::new(fatigue_w, bar_height),
            Color::srgb(0.3, 0.5, 0.9), // Blue = rested
        );
    }

    // Activity indicator (white dot if active)
    if activity.is_some() {
        gizmos.circle_2d(
            Isometry2d::from_translation(Vec2::new(panel_x + panel_size.x - 5.0, panel_y - 5.0)),
            3.0 * scale,
            Color::WHITE,
        );
    }

    // Draw movement path if moving
    if let Ok(movement) = sim.0.world.get::<&Movement>(entity) {
        let layout = match &sim.0.ship_layout {
            Some(l) => l,
            None => return,
        };

        // Draw line from current position to destination
        let dest_world: SimVec3 = room.local_to_world(movement.destination);
        gizmos.line_2d(
            pos_vec,
            Vec2::new(dest_world.x, dest_world.y),
            Color::srgba(1.0, 1.0, 0.3, 0.5),
        );

        // Draw path through rooms
        if movement.path.len() > 1 {
            let mut prev_pos = pos_vec;

            for (i, &room_id) in movement.path.iter().enumerate().skip(movement.path_index) {
                if (room_id as usize) >= layout.rooms.len() {
                    continue;
                }

                if let Ok(path_room) = sim.0.world.get::<&Room>(layout.rooms[room_id as usize]) {
                    // Skip if on different deck
                    if path_room.deck_level != current_deck.0 {
                        continue;
                    }

                    let door_world: SimVec3 = path_room.door_world_position();
                    let door_pos = Vec2::new(door_world.x, door_world.y);

                    // Draw connecting line
                    if i > movement.path_index {
                        gizmos.line_2d(prev_pos, door_pos, Color::srgba(0.8, 0.8, 0.2, 0.4));
                    }

                    // Draw waypoint marker
                    gizmos.circle_2d(
                        Isometry2d::from_translation(door_pos),
                        0.5,
                        Color::srgba(1.0, 0.8, 0.2, 0.6),
                    );

                    prev_pos = door_pos;
                }
            }

            // Draw final destination marker
            if let Some(&final_room_id) = movement.path.last() {
                if (final_room_id as usize) < layout.rooms.len() {
                    if let Ok(final_room) = sim
                        .0
                        .world
                        .get::<&Room>(layout.rooms[final_room_id as usize])
                    {
                        if final_room.deck_level == current_deck.0 {
                            let final_world: SimVec3 =
                                final_room.local_to_world(movement.final_destination);
                            gizmos.circle_2d(
                                Isometry2d::from_translation(Vec2::new(
                                    final_world.x,
                                    final_world.y,
                                )),
                                1.0,
                                Color::srgba(0.2, 1.0, 0.2, 0.8),
                            );
                        }
                    }
                }
            }
        }
    }
}

fn render_ui(
    sim: Res<SimWrapper>,
    camera_state: Res<CameraState>,
    current_deck: Res<CurrentDeck>,
    mut gizmos: Gizmos,
) {
    let hour = sim.0.hour_of_day();
    let scale = camera_state.zoom;

    let ui_x = camera_state.target.x - 580.0 * scale;
    let ui_y = camera_state.target.y + 320.0 * scale;

    // Clock
    let clock_center = Vec2::new(ui_x, ui_y);
    let clock_radius = 25.0 * scale;

    gizmos.circle_2d(
        Isometry2d::from_translation(clock_center),
        clock_radius,
        Color::srgba(0.2, 0.2, 0.3, 0.9),
    );

    // Hour hand
    let angle = (hour / 12.0) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    let hand_end = clock_center + Vec2::new(angle.cos(), angle.sin()) * clock_radius * 0.6;
    gizmos.line_2d(clock_center, hand_end, Color::WHITE);

    // Day/night indicator
    let is_night = hour < 6.0 || hour > 22.0;
    let indicator_color = if is_night {
        Color::srgb(0.2, 0.2, 0.5)
    } else {
        Color::srgb(1.0, 0.9, 0.3)
    };
    gizmos.circle_2d(
        Isometry2d::from_translation(clock_center + Vec2::new(0.0, -clock_radius - 12.0 * scale)),
        6.0 * scale,
        indicator_color,
    );

    // Deck indicator
    let deck_y = ui_y - 60.0 * scale;
    let num_decks = sim
        .0
        .ship_layout
        .as_ref()
        .map(|l| l.decks.len())
        .unwrap_or(1);

    for i in 0..num_decks {
        let deck_x = ui_x + (i as f32 * 15.0 - (num_decks as f32 - 1.0) * 7.5) * scale;
        let is_current = i as i32 == current_deck.0;
        let color = if is_current {
            Color::srgb(0.3, 0.7, 0.9)
        } else {
            Color::srgba(0.4, 0.4, 0.5, 0.6)
        };

        gizmos.rect_2d(
            Isometry2d::from_translation(Vec2::new(deck_x, deck_y)),
            Vec2::new(10.0, 10.0) * scale,
            color,
        );
    }

    // Population bar
    let bar_y = deck_y - 25.0 * scale;
    let crew_count = sim.0.crew_count() as f32;
    let passenger_count = sim.0.passenger_count() as f32;
    let total = (crew_count + passenger_count).max(1.0);

    let bar_width = 80.0 * scale;
    let crew_width = (crew_count / total) * bar_width;

    if crew_width > 0.0 {
        gizmos.rect_2d(
            Isometry2d::from_translation(Vec2::new(
                ui_x - bar_width / 2.0 + crew_width / 2.0,
                bar_y,
            )),
            Vec2::new(crew_width, 6.0 * scale),
            Color::srgb(0.3, 0.5, 0.9),
        );
    }

    let passenger_width = bar_width - crew_width;
    if passenger_width > 0.0 {
        gizmos.rect_2d(
            Isometry2d::from_translation(Vec2::new(
                ui_x - bar_width / 2.0 + crew_width + passenger_width / 2.0,
                bar_y,
            )),
            Vec2::new(passenger_width, 6.0 * scale),
            Color::srgb(0.3, 0.8, 0.4),
        );
    }

    // Resource bars (right side of screen)
    let res_x = camera_state.target.x + 500.0 * scale;
    let res_y = camera_state.target.y + 300.0 * scale;
    let res_bar_width = 60.0 * scale;
    let res_bar_height = 8.0 * scale;
    let res_spacing = 15.0 * scale;

    // Power (yellow)
    let power_level = sim
        .0
        .resources
        .level(progship_core::components::ResourceType::Power);
    draw_resource_bar(
        &mut gizmos,
        Vec2::new(res_x, res_y),
        res_bar_width,
        res_bar_height,
        power_level,
        Color::srgb(1.0, 0.9, 0.2),
    );

    // Oxygen (light blue)
    let oxygen_level = sim
        .0
        .resources
        .level(progship_core::components::ResourceType::Oxygen);
    draw_resource_bar(
        &mut gizmos,
        Vec2::new(res_x, res_y - res_spacing),
        res_bar_width,
        res_bar_height,
        oxygen_level,
        Color::srgb(0.5, 0.8, 1.0),
    );

    // Water (blue)
    let water_level = sim
        .0
        .resources
        .level(progship_core::components::ResourceType::Water);
    draw_resource_bar(
        &mut gizmos,
        Vec2::new(res_x, res_y - res_spacing * 2.0),
        res_bar_width,
        res_bar_height,
        water_level,
        Color::srgb(0.2, 0.4, 0.9),
    );

    // Food (green)
    let food_level = sim
        .0
        .resources
        .level(progship_core::components::ResourceType::Food);
    draw_resource_bar(
        &mut gizmos,
        Vec2::new(res_x, res_y - res_spacing * 3.0),
        res_bar_width,
        res_bar_height,
        food_level,
        Color::srgb(0.3, 0.8, 0.3),
    );

    // Fuel (orange)
    let fuel_level = sim
        .0
        .resources
        .level(progship_core::components::ResourceType::Fuel);
    draw_resource_bar(
        &mut gizmos,
        Vec2::new(res_x, res_y - res_spacing * 4.0),
        res_bar_width,
        res_bar_height,
        fuel_level,
        Color::srgb(0.9, 0.5, 0.1),
    );

    // Active conversations indicator
    let conv_count = sim.0.conversations.active_count();
    if conv_count > 0 {
        let conv_y = res_y - res_spacing * 6.0;
        // Draw small chat bubble icons for each active conversation (max 10 shown)
        for i in 0..conv_count.min(10) {
            let conv_x = res_x - 25.0 * scale + (i as f32 * 8.0 * scale);
            gizmos.ellipse_2d(
                Isometry2d::from_translation(Vec2::new(conv_x, conv_y)),
                Vec2::new(3.0, 2.0) * scale,
                Color::srgb(0.5, 0.8, 0.5),
            );
        }
    }

    // Maintenance tasks indicator
    let maint_count = sim.0.maintenance_queue.tasks.len();
    if maint_count > 0 {
        let maint_y = res_y - res_spacing * 7.0;
        // Draw wrench icons for each active maintenance task (max 10 shown)
        for i in 0..maint_count.min(10) {
            let maint_x = res_x - 25.0 * scale + (i as f32 * 8.0 * scale);
            gizmos.circle_2d(
                Isometry2d::from_translation(Vec2::new(maint_x, maint_y)),
                2.5 * scale,
                Color::srgb(0.9, 0.6, 0.2),
            );
        }
    }
}

fn draw_resource_bar(
    gizmos: &mut Gizmos,
    pos: Vec2,
    width: f32,
    height: f32,
    level: f32,
    color: Color,
) {
    // Background
    gizmos.rect_2d(
        Isometry2d::from_translation(pos),
        Vec2::new(width, height),
        Color::srgba(0.2, 0.2, 0.25, 0.8),
    );

    // Fill based on level
    let fill_width = width * level.clamp(0.0, 1.0);
    if fill_width > 0.1 {
        gizmos.rect_2d(
            Isometry2d::from_translation(pos - Vec2::new((width - fill_width) / 2.0, 0.0)),
            Vec2::new(fill_width, height - 1.0),
            color,
        );
    }
}

fn room_color(room_type: RoomType) -> Color {
    match room_type {
        RoomType::Bridge => Color::srgba(0.8, 0.2, 0.2, 0.7),
        RoomType::ConferenceRoom => Color::srgba(0.7, 0.3, 0.3, 0.7),
        RoomType::Engineering => Color::srgba(0.8, 0.5, 0.1, 0.7),
        RoomType::ReactorRoom => Color::srgba(0.9, 0.3, 0.1, 0.7),
        RoomType::MaintenanceBay => Color::srgba(0.7, 0.5, 0.2, 0.7),
        RoomType::LifeSupport => Color::srgba(0.3, 0.7, 0.9, 0.7),
        RoomType::Hydroponics => Color::srgba(0.2, 0.7, 0.3, 0.7),
        RoomType::WaterRecycling => Color::srgba(0.3, 0.5, 0.8, 0.7),
        RoomType::Medical => Color::srgba(0.95, 0.95, 0.95, 0.7),
        RoomType::Cargo => Color::srgba(0.5, 0.4, 0.3, 0.7),
        RoomType::Quarters => Color::srgba(0.35, 0.45, 0.6, 0.7),
        RoomType::QuartersCrew => Color::srgba(0.3, 0.4, 0.6, 0.7),
        RoomType::QuartersOfficer => Color::srgba(0.4, 0.5, 0.7, 0.7),
        RoomType::QuartersPassenger => Color::srgba(0.3, 0.6, 0.4, 0.7),
        RoomType::Mess => Color::srgba(0.6, 0.5, 0.3, 0.7),
        RoomType::Galley => Color::srgba(0.5, 0.4, 0.2, 0.7),
        RoomType::Recreation => Color::srgba(0.6, 0.7, 0.3, 0.7),
        RoomType::Gym => Color::srgba(0.5, 0.6, 0.2, 0.7),
        RoomType::Observatory => Color::srgba(0.2, 0.3, 0.7, 0.7),
        RoomType::Corridor => Color::srgba(0.45, 0.45, 0.5, 0.6),
        RoomType::Elevator => Color::srgba(0.5, 0.5, 0.55, 0.7),
        RoomType::Airlock => Color::srgba(0.3, 0.3, 0.3, 0.7),
        RoomType::Storage => Color::srgba(0.45, 0.4, 0.35, 0.7),
        RoomType::Laboratory => Color::srgba(0.5, 0.6, 0.7, 0.7),
    }
}

fn update_text_ui(
    sim: Res<SimWrapper>,
    current_deck: Res<CurrentDeck>,
    camera_state: Res<CameraState>,
    mut time_query: Query<(&mut Text2d, &mut Transform), (With<TimeText>, Without<DeckText>)>,
    mut deck_query: Query<(&mut Text2d, &mut Transform), (With<DeckText>, Without<TimeText>)>,
) {
    // Update time text
    let day = (sim.0.sim_time / 24.0).floor() as i32 + 1;
    let hour = sim.0.hour_of_day();
    let minutes = ((hour % 1.0) * 60.0) as i32;
    let hour_int = hour as i32;

    for (mut text, mut transform) in &mut time_query {
        **text = format!("Day {}, {:02}:{:02}", day, hour_int, minutes);
        // Keep text at fixed screen position relative to camera
        transform.translation.x = camera_state.target.x - 520.0 * camera_state.zoom;
        transform.translation.y = camera_state.target.y + 320.0 * camera_state.zoom;
        transform.scale = Vec3::splat(camera_state.zoom);
    }

    // Update deck text
    for (mut text, mut transform) in &mut deck_query {
        **text = format!("Deck {}", current_deck.0 + 1);
        transform.translation.x = camera_state.target.x - 520.0 * camera_state.zoom;
        transform.translation.y = camera_state.target.y + 295.0 * camera_state.zoom;
        transform.scale = Vec3::splat(camera_state.zoom);
    }
}
