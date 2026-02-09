//! Camera setup and control for the ProgShip client.
//!
//! Handles top-down camera setup and smooth following of the player.

use bevy::prelude::*;
use progship_client_sdk::*;

use crate::state::{ConnectionState, PlayerCamera, PlayerState, ViewState};

pub fn setup_camera(mut commands: Commands) {
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
}

pub fn camera_follow_player(
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
