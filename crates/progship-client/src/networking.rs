//! SpacetimeDB networking for the ProgShip client.
//!
//! Handles connection, subscription, message processing, auto-join,
//! and automatic reconnection with exponential backoff.

use bevy::prelude::*;
use progship_client_sdk::*;
use spacetimedb_sdk::{DbContext, Table};

use crate::state::{ConnectionConfig, ConnectionState, PlayerState, Toast, UiState};

const JOIN_TIMEOUT_SECS: f32 = 30.0;
const MAX_JOIN_ATTEMPTS: u32 = 3;

pub fn connect_to_server(
    mut state: ResMut<ConnectionState>,
    mut config: ResMut<ConnectionConfig>,
    time: Res<Time>,
    mut ui: ResMut<UiState>,
) {
    match &*state {
        ConnectionState::Connected(_) | ConnectionState::Connecting => return,
        ConnectionState::Reconnecting => {
            config.reconnect_timer -= time.delta_secs();
            if config.reconnect_timer > 0.0 {
                return;
            }
        }
        ConnectionState::Disconnected => {}
    }

    let attempt_msg = if config.reconnect_attempts > 0 {
        format!(
            " (attempt {}, next retry in {:.0}s)",
            config.reconnect_attempts + 1,
            config.reconnect_delay
        )
    } else {
        String::new()
    };
    info!("Connecting to {}{}...", config.server_url, attempt_msg);
    *state = ConnectionState::Connecting;

    match DbConnection::builder()
        .with_uri(&config.server_url)
        .with_module_name(&config.module_name)
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
            config.reset_backoff();
            if config.reconnect_attempts > 0 {
                ui.toasts.push(Toast {
                    message: "Reconnected to server".into(),
                    color: bevy::color::Color::srgb(0.3, 1.0, 0.3),
                    timer: 3.0,
                });
            }
            *state = ConnectionState::Connected(conn);
        }
        Err(e) => {
            error!("Failed to connect: {:?}", e);
            config.advance_backoff();
            ui.toasts.push(Toast {
                message: format!(
                    "Connection failed — retrying in {:.0}s",
                    config.reconnect_delay
                ),
                color: bevy::color::Color::srgb(1.0, 0.3, 0.3),
                timer: 5.0,
            });
            *state = ConnectionState::Reconnecting;
        }
    }
}

pub fn process_messages(
    mut state: ResMut<ConnectionState>,
    mut config: ResMut<ConnectionConfig>,
    mut player: ResMut<PlayerState>,
    mut ui: ResMut<UiState>,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };
    if let Err(e) = conn.frame_tick() {
        error!("Connection error: {:?}", e);
        ui.toasts.push(Toast {
            message: "Disconnected from server — reconnecting...".into(),
            color: bevy::color::Color::srgb(1.0, 0.5, 0.2),
            timer: 5.0,
        });
        config.advance_backoff();
        // Reset player state so we re-join on reconnect
        player.joined = false;
        player.person_id = None;
        player.join_timer = 0.0;
        player.join_attempts = 0;
        *state = ConnectionState::Reconnecting;
    }
}

pub fn auto_join_game(
    state: Res<ConnectionState>,
    mut player: ResMut<PlayerState>,
    time: Res<Time>,
    mut ui: ResMut<UiState>,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };

    // Check if ship exists
    let Some(_config) = conn.db.ship_config().id().find(&0) else {
        return;
    };

    if !player.joined {
        info!("Subscription applied, joining game...");
        match conn
            .reducers()
            .player_join("Player".into(), "One".into(), true)
        {
            Ok(_) => {
                player.joined = true;
                player.join_timer = 0.0;
            }
            Err(e) => {
                error!("Failed to call player_join: {:?}", e);
                ui.toasts.push(Toast {
                    message: "Failed to join game — retrying...".into(),
                    color: bevy::color::Color::srgb(1.0, 0.5, 0.2),
                    timer: 3.0,
                });
            }
        }
    }

    if player.person_id.is_none() && player.joined {
        // Track join timeout
        player.join_timer += time.delta_secs();

        if let Some(my_identity) = conn.try_identity() {
            for person in conn.db.person().iter() {
                if person.owner_identity.as_ref() == Some(&my_identity) {
                    player.person_id = Some(person.id);
                    info!("Player character id: {}", person.id);
                    ui.toasts.push(Toast {
                        message: "Joined the ship!".into(),
                        color: bevy::color::Color::srgb(0.3, 1.0, 0.3),
                        timer: 3.0,
                    });
                    return;
                }
            }
        }

        // Timeout — retry join
        if player.join_timer > JOIN_TIMEOUT_SECS {
            player.join_attempts += 1;
            if player.join_attempts < MAX_JOIN_ATTEMPTS {
                warn!(
                    "Join timed out after {:.0}s, retrying (attempt {})",
                    JOIN_TIMEOUT_SECS,
                    player.join_attempts + 1
                );
                player.joined = false;
                player.join_timer = 0.0;
                ui.toasts.push(Toast {
                    message: format!(
                        "Join timed out — retrying ({}/{})",
                        player.join_attempts + 1,
                        MAX_JOIN_ATTEMPTS
                    ),
                    color: bevy::color::Color::srgb(1.0, 0.8, 0.2),
                    timer: 5.0,
                });
            } else {
                ui.toasts.push(Toast {
                    message: "Failed to join after multiple attempts. Is the ship initialized?"
                        .into(),
                    color: bevy::color::Color::srgb(1.0, 0.3, 0.3),
                    timer: 10.0,
                });
            }
        }
    }
}
