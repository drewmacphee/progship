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
mod greeble;
mod input;
mod minimap;
mod networking;
mod rendering;
mod state;
mod ui;

use camera::{camera_follow_player, handle_quit, setup_camera};
use input::player_input;
use minimap::{minimap_toggle, render_minimap, MinimapState};
use networking::{auto_join_game, connect_to_server, process_messages};
use rendering::{animate_details, animate_dust_motes, sync_door_panels, sync_people, sync_rooms};
use state::{ConnectionConfig, ConnectionState, PlayerState, UiState, ViewState};
use ui::{render_hud, render_info_panel, render_toasts, setup_ui};

fn main() {
    let conn_config = ConnectionConfig::from_args();
    info!(
        "ProgShip Client — server: {} module: {}",
        conn_config.server_url, conn_config.module_name
    );

    let mut app = App::new();

    // DLSS project ID must be inserted before DefaultPlugins (which contains DlssInitPlugin)
    #[cfg(feature = "dlss")]
    app.insert_resource(bevy::anti_alias::dlss::DlssProjectId(
        bevy::asset::uuid::uuid!("a3f7e2c1-9b4d-4e8a-b6d0-1c5f3a7e9d2b"),
    ));

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "ProgShip - Colony Ship".to_string(),
            resolution: bevy::window::WindowResolution::new(1280, 720),
            present_mode: bevy::window::PresentMode::AutoVsync,
            ..default()
        }),
        ..default()
    }));

    #[cfg(feature = "solari")]
    app.add_plugins(bevy::solari::prelude::SolariPlugins);

    app.insert_resource(ConnectionState::Disconnected)
        .insert_resource(conn_config)
        .insert_resource(ViewState::default())
        .insert_resource(PlayerState::default())
        .insert_resource(UiState::default())
        .insert_resource(MinimapState::default())
        .add_systems(
            Startup,
            (setup_camera, setup_ui, greeble::init_greeble_library),
        )
        .add_systems(
            Update,
            (
                connect_to_server,
                process_messages,
                auto_join_game,
                player_input,
                minimap_toggle,
                camera_follow_player,
                handle_quit,
                sync_rooms,
                sync_people,
                sync_door_panels,
                animate_details,
                animate_dust_motes,
            ),
        )
        .add_systems(
            Update,
            (render_hud, render_info_panel, render_toasts, render_minimap),
        );

    #[cfg(feature = "solari")]
    app.add_systems(Update, rendering::attach_raytracing_meshes);

    app.run();
}
