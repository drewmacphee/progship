//! ProgShip Client - Bevy 3D game connecting to SpacetimeDB server
//!
//! Top-down 3D view of the colony ship. All simulation runs on the server.
//! The client renders the world, follows the player, and sends input.

// TODO: Fix these clippy lints incrementally
#![allow(clippy::needless_range_loop)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::useless_format)]
#![allow(dead_code)]
#![allow(unused_variables)]

use bevy::prelude::*;

mod camera;
mod input;
mod networking;
mod rendering;
mod state;
mod ui;

use camera::{camera_follow_player, setup_camera};
use input::player_input;
use networking::{auto_join_game, connect_to_server, process_messages};
use rendering::{sync_people, sync_rooms};
use state::{ConnectionState, PlayerState, UiState, ViewState};
use ui::{render_hud, render_info_panel, render_toasts, setup_ui};

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
        .add_systems(Startup, (setup_camera, setup_ui))
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
