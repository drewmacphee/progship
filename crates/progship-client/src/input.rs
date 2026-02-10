//! Player input handling for the ProgShip client.
//!
//! Handles WASD movement, elevator/ladder controls, context actions, and UI toggles.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use progship_client_sdk::*;
use spacetimedb_sdk::{DbContext, Table};

use crate::state::{ConnectionState, PlayerState, Toast, UiState, ViewState};

pub fn player_input(
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

    // WASD movement — speed scales with zoom level for comfortable panning
    let zoom_factor = (view.camera_height / 80.0).max(0.5);
    let speed = 15.0 * zoom_factor * time.delta_secs();
    let mut dx = 0.0f32;
    let mut dy = 0.0f32;
    if keyboard.pressed(KeyCode::KeyW) {
        dy += speed;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        dy -= speed;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        dx -= speed;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        dx += speed;
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
            dy += speed;
        }
        if keyboard.pressed(KeyCode::ArrowDown) {
            dy -= speed;
        }
        if keyboard.pressed(KeyCode::ArrowLeft) {
            dx -= speed;
        }
        if keyboard.pressed(KeyCode::ArrowRight) {
            dx += speed;
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
                    if dist < 15.0 {
                        if closest.is_none() || dist < closest.unwrap().1 {
                            closest = Some((pos.person_id, dist));
                        }
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
                            color: Color::srgb(0.5, 1.0, 0.5),
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
                    // ELEVATOR_SHAFT — digit keys select target deck
                    let deck_keys: &[(KeyCode, i32)] = &[
                        (KeyCode::Digit1, 0),
                        (KeyCode::Digit2, 1),
                        (KeyCode::Digit3, 2),
                        (KeyCode::Digit4, 3),
                        (KeyCode::Digit5, 4),
                        (KeyCode::Digit6, 5),
                        (KeyCode::Digit7, 6),
                        (KeyCode::Digit8, 7),
                        (KeyCode::Digit9, 8),
                        (KeyCode::Digit0, 9),
                        (KeyCode::Minus, 10),
                        (KeyCode::Equal, 11),
                    ];
                    for &(key, deck) in deck_keys {
                        if keyboard.just_pressed(key) && deck != room.deck {
                            let _ = conn.reducers().player_use_elevator(deck);
                            ui.toasts.push(Toast {
                                message: format!("Taking elevator to Deck {}...", deck + 1),
                                color: Color::srgb(0.5, 0.8, 1.0),
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
                            color: Color::srgb(0.5, 0.8, 1.0),
                            timer: 2.0,
                        });
                    }
                    if keyboard.just_pressed(KeyCode::ArrowDown) {
                        let _ = conn.reducers().player_use_ladder(1);
                        ui.toasts.push(Toast {
                            message: "Climbing down...".to_string(),
                            color: Color::srgb(0.5, 0.8, 1.0),
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

    // Zoom camera — scale step with current height for smooth feel
    for event in scroll_events.read() {
        let zoom_step = view.camera_height * 0.1; // 10% of current height per scroll
        view.camera_height = (view.camera_height - event.y * zoom_step).clamp(15.0, 500.0);
    }

    // Detect new events for toasts
    let active_events: Vec<_> = conn.db.event().iter().filter(|e| e.state != 2).collect();
    if active_events.len() > ui.last_event_count {
        for evt in active_events.iter().skip(ui.last_event_count) {
            let (msg, color) = event_toast_info(evt.event_type, evt.severity);
            if let Some(room) = conn.db.room().id().find(&evt.room_id) {
                ui.toasts.push(Toast {
                    message: format!("{} in {}", msg, room.name),
                    color,
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
