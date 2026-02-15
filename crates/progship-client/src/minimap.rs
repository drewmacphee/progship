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
}

impl Default for MinimapState {
    fn default() -> Self {
        Self {
            visible: true,
            panel_size: 200.0,
            margin: 10.0,
        }
    }
}

/// Toggle minimap visibility with M key.
pub fn minimap_toggle(keyboard: Res<ButtonInput<KeyCode>>, mut minimap: ResMut<MinimapState>) {
    if keyboard.just_pressed(KeyCode::KeyM) {
        minimap.visible = !minimap.visible;
    }
}

/// Spawn/update the minimap overlay each frame.
pub fn render_minimap(
    state: Res<ConnectionState>,
    view: Res<ViewState>,
    player: Res<PlayerState>,
    minimap: Res<MinimapState>,
    mut commands: Commands,
    existing_roots: Query<Entity, With<MinimapRoot>>,
) {
    // Clean up old minimap (root despawn_recursive handles all children)
    for entity in existing_roots.iter() {
        if let Some(cmd) = commands.get_entity(entity) {
            cmd.despawn_recursive();
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

    // Find deck bounds
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for room in &rooms {
        min_x = min_x.min(room.x);
        min_y = min_y.min(room.y);
        max_x = max_x.max(room.x + room.width);
        max_y = max_y.max(room.y + room.height);
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
                height: Val::Px(panel_h + 4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.8)),
            ZIndex(10),
            MinimapRoot,
        ))
        .with_children(|parent| {
            // Render each room as a small colored rectangle
            for room in &rooms {
                let rx = (room.x - min_x) * scale_x + 2.0;
                let ry = (max_y - (room.y + room.height)) * scale_y + 2.0;
                let rw = (room.width * scale_x).max(1.0);
                let rh = (room.height * scale_y).max(1.0);

                let color = minimap_room_color(room.room_type);

                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(rx),
                        top: Val::Px(ry),
                        width: Val::Px(rw),
                        height: Val::Px(rh),
                        ..default()
                    },
                    BackgroundColor(color),
                    MinimapRoom,
                ));
            }

            // Player position marker
            if let Some(pid) = player.person_id {
                if let Some(pos) = conn.db.position().person_id().find(&pid) {
                    // Check player is on this deck
                    if conn
                        .db
                        .room()
                        .id()
                        .find(&pos.room_id)
                        .map(|r| r.deck == view.current_deck)
                        .unwrap_or(false)
                    {
                        let px = (pos.x - min_x) * scale_x + 2.0;
                        let py = (max_y - pos.y) * scale_y + 2.0;

                        parent.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(px - 3.0),
                                top: Val::Px(py - 3.0),
                                width: Val::Px(6.0),
                                height: Val::Px(6.0),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(1.0, 1.0, 0.0)),
                            BorderColor(Color::srgb(1.0, 0.5, 0.0)),
                            MinimapPlayer,
                        ));
                    }
                }
            }
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
