//! SpacetimeDB networking for the ProgShip client.
//!
//! Handles connection, subscription, message processing, and auto-join.

use bevy::prelude::*;
use progship_client_sdk::*;
use spacetimedb_sdk::{DbContext, Table};

use crate::state::{ConnectionState, PlayerState};

pub fn connect_to_server(mut state: ResMut<ConnectionState>) {
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
            *state = ConnectionState::Connected(conn);
        }
        Err(e) => {
            error!("Failed to connect: {:?}", e);
            *state = ConnectionState::Disconnected;
        }
    }
}

pub fn process_messages(mut state: ResMut<ConnectionState>) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };
    if let Err(e) = conn.frame_tick() {
        error!("Connection error: {:?}", e);
        *state = ConnectionState::Disconnected;
    }
}

pub fn auto_join_game(state: Res<ConnectionState>, mut player: ResMut<PlayerState>) {
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
