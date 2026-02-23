//! Minimap overlay for the ProgShip client.
//!
//! Renders a scaled-down deck layout in the bottom-right corner.
//! Shows room outlines colored by type and a player position marker.
//! Toggled with M key. Click to teleport camera (not player).

use bevy::prelude::*;
use progship_client_sdk::*;
use spacetimedb_sdk::Table;

use crate::state::{ConnectionState, PlayerState, ViewState};

/// Marker for the minimap root container.
#[derive(Component)]
pub struct MinimapRoot;

/// Marker for minimap room nodes.
#[derive(Component)]
pub struct MinimapRoom;

/// Marker for the player position indicator.
#[derive(Component)]
pub struct MinimapPlayer;

/// Minimap configuration and state.
#[derive(Resource)]
pub struct MinimapState {
    pub visible: bool,
    /// Size of the minimap panel in pixels.
    pub panel_size: f32,
    /// Margin from screen edge.
    pub margin: f32,
    /// Last rendered yaw (to detect rotation changes).
    prev_yaw: f32,
}

impl Default for MinimapState {
    fn default() -> Self {
        Self {
            visible: true,
            panel_size: 350.0,
            margin: 10.0,
            prev_yaw: f32::MAX,
        }
    }
}

/// Toggle minimap visibility with M key.
pub fn minimap_toggle(keyboard: Res<ButtonInput<KeyCode>>, mut minimap: ResMut<MinimapState>) {
    if keyboard.just_pressed(KeyCode::KeyM) {
        minimap.visible = !minimap.visible;
    }
}

/// Spawn/update the minimap overlay when dirty.
pub fn render_minimap(
    state: Res<ConnectionState>,
    mut view: ResMut<ViewState>,
    player: Res<PlayerState>,
    mut minimap: ResMut<MinimapState>,
    mut commands: Commands,
    existing_roots: Query<Entity, With<MinimapRoot>>,
) {
    // Rebuild when dirty, visibility toggled, player moved, or camera rotated significantly
    let yaw_changed = (view.fps_yaw - minimap.prev_yaw).abs() > 0.05;
    let needs_rebuild =
        view.minimap_dirty || minimap.is_changed() || player.is_changed() || yaw_changed;
    if !needs_rebuild {
        return;
    }
    view.minimap_dirty = false;
    minimap.prev_yaw = view.fps_yaw;

    // Clean up old minimap (root despawn_recursive handles all children)
    for entity in existing_roots.iter() {
        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.despawn();
        }
    }

    if !minimap.visible {
        return;
    }

    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };

    // Collect rooms on current deck
    let rooms: Vec<_> = conn
        .db
        .room()
        .iter()
        .filter(|r| r.deck == view.current_deck)
        .collect();

    if rooms.is_empty() {
        return;
    }

    // Find deck bounds (room.x/y are centers, not bottom-left)
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for room in &rooms {
        let hw = room.width / 2.0;
        let hh = room.height / 2.0;
        min_x = min_x.min(room.x - hw);
        min_y = min_y.min(room.y - hh);
        max_x = max_x.max(room.x + hw);
        max_y = max_y.max(room.y + hh);
    }

    let deck_w = (max_x - min_x).max(1.0);
    let deck_h = (max_y - min_y).max(1.0);
    let aspect = deck_w / deck_h;

    // Scale minimap to fit panel, preserving aspect ratio
    let (panel_w, panel_h) = if aspect > 1.0 {
        (minimap.panel_size, minimap.panel_size / aspect)
    } else {
        (minimap.panel_size * aspect, minimap.panel_size)
    };

    let scale_x = panel_w / deck_w;
    let scale_y = panel_h / deck_h;

    // Spawn minimap container (bottom-right corner with dark background)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(minimap.margin),
                bottom: Val::Px(minimap.margin),
                width: Val::Px(panel_w + 4.0),
                height: Val::Px(panel_h + 24.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.8)),
            ZIndex(10),
            MinimapRoot,
        ))
        .with_children(|parent| {
            // Deck label
            parent.spawn((
                Text::new(format!("DECK {}", view.current_deck + 1)),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.8, 0.9)),
                Node {
                    margin: UiRect::new(Val::Px(4.0), Val::Px(0.0), Val::Px(2.0), Val::Px(2.0)),
                    ..default()
                },
            ));

            // Map area container
            parent
                .spawn(Node {
                    width: Val::Px(panel_w + 4.0),
                    height: Val::Px(panel_h),
                    position_type: PositionType::Relative,
                    ..default()
                })
                .with_children(|map| {
                    // Render each room as a small colored rectangle
                    for room in &rooms {
                        let hw = room.width / 2.0;
                        let hh = room.height / 2.0;
                        let rx = (room.x - hw - min_x) * scale_x + 2.0;
                        let ry = (room.y - hh - min_y) * scale_y;
                        let rw = (room.width * scale_x).max(1.0);
                        let rh = (room.height * scale_y).max(1.0);

                        let color = minimap_room_color(room.room_type);

                        map.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(rx),
                                top: Val::Px(ry),
                                width: Val::Px(rw),
                                height: Val::Px(rh),
                                border: UiRect::all(Val::Px(0.5)),
                                ..default()
                            },
                            BackgroundColor(color),
                            BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.4)),
                            MinimapRoom,
                        ));
                    }

                    // Player position marker
                    if let Some(pid) = player.person_id {
                        if let Some(pos) = conn.db.position().person_id().find(&pid) {
                            if conn
                                .db
                                .room()
                                .id()
                                .find(&pos.room_id)
                                .map(|r| r.deck == view.current_deck)
                                .unwrap_or(false)
                            {
                                let px = (pos.x - min_x) * scale_x + 2.0;
                                let py = (pos.y - min_y) * scale_y;

                                // Outer glow ring
                                map.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(px - 6.0),
                                        top: Val::Px(py - 6.0),
                                        width: Val::Px(12.0),
                                        height: Val::Px(12.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(1.0, 0.8, 0.0, 0.3)),
                                    MinimapPlayer,
                                ));

                                // Inner dot
                                map.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(px - 4.0),
                                        top: Val::Px(py - 4.0),
                                        width: Val::Px(8.0),
                                        height: Val::Px(8.0),
                                        border: UiRect::all(Val::Px(1.5)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(1.0, 1.0, 0.0)),
                                    BorderColor::all(Color::srgb(1.0, 0.4, 0.0)),
                                    MinimapPlayer,
                                ));

                                // Direction indicator — line of dots in look direction
                                // fps_yaw: 0 = +Z in Bevy, but minimap Y-axis = world Z
                                // sin(yaw) points left/right (X on minimap), cos(yaw) points forward (Y on minimap)
                                let yaw = view.fps_yaw;
                                let dx = -yaw.sin(); // minimap X direction
                                let dy = yaw.cos(); // minimap Y direction (positive = down on screen)
                                let dot_len = 10.0;
                                for i in 1..=3 {
                                    let t = i as f32 * (dot_len / 3.0);
                                    let dot_size = 4.0 - i as f32 * 0.5; // taper: 3.5, 3.0, 2.5
                                    let dot_x = px + dx * t - dot_size / 2.0;
                                    let dot_y = py + dy * t - dot_size / 2.0;
                                    map.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: Val::Px(dot_x),
                                            top: Val::Px(dot_y),
                                            width: Val::Px(dot_size),
                                            height: Val::Px(dot_size),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(1.0, 0.6, 0.0)),
                                        MinimapPlayer,
                                    ));
                                }
                            }
                        }
                    }
                });
        });
}

/// Simplified room colors for minimap (fewer distinct shades, bolder colors).
fn minimap_room_color(room_type: u8) -> Color {
    match room_type {
        0..=8 => Color::srgb(0.15, 0.15, 0.55),     // Command — blue
        10..=18 => Color::srgb(0.25, 0.35, 0.40),   // Habitation — teal
        20..=27 => Color::srgb(0.55, 0.45, 0.15),   // Food — yellow
        30..=37 => Color::srgb(0.65, 0.70, 0.75),   // Medical — white
        40..=56 => Color::srgb(0.20, 0.50, 0.25),   // Recreation — green
        60..=71 => Color::srgb(0.55, 0.30, 0.10),   // Engineering — orange
        80..=86 => Color::srgb(0.20, 0.45, 0.50),   // Life Support — cyan
        90..=95 => Color::srgb(0.35, 0.28, 0.18),   // Cargo — brown
        100..=102 => Color::srgb(0.15, 0.15, 0.18), // Corridors — dark gray
        110..=111 => Color::srgb(0.30, 0.30, 0.38), // Shafts — lighter gray
        120 => Color::srgb(0.10, 0.10, 0.12),       // Service deck
        _ => Color::srgb(0.25, 0.25, 0.25),         // Unknown
    }
}
