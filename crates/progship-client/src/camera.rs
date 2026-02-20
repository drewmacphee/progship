//! Camera setup and control for the ProgShip client.
//!
//! Supports top-down (default) and first-person camera modes.
//! Toggle with V key. Mouse look in first-person mode.

use bevy::prelude::MessageReader;
use bevy::prelude::*;
use progship_client_sdk::*;

use crate::state::{CameraMode, ConnectionState, PlayerCamera, PlayerState, ViewState};

pub fn setup_camera(
    mut commands: Commands,
    #[cfg(feature = "dlss")] dlss_rr_supported: Option<
        Res<bevy::anti_alias::dlss::DlssRayReconstructionSupported>,
    >,
) {
    let cam_transform =
        Transform::from_xyz(0.0, 150.0, 0.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::NEG_Z);

    // Solari: match the official bevy_solari example camera setup exactly.
    // No Bloom — Solari replaces the standard forward/deferred main pass.
    #[cfg(feature = "solari")]
    {
        info!("Spawning camera with Solari raytraced lighting.");
        let mut cam = commands.spawn((
            Camera3d::default(),
            bevy::render::camera::CameraRenderGraph::new(
                bevy::core_pipeline::core_3d::graph::Core3d,
            ),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            cam_transform,
            bevy::camera::CameraMainTextureUsages::default()
                .with(bevy::render::render_resource::TextureUsages::STORAGE_BINDING),
            Msaa::Off,
            PlayerCamera,
        ));
        cam.insert(bevy::solari::prelude::SolariLighting::default());

        // DLSS Ray Reconstruction provides denoising + upscaling for Solari
        #[cfg(feature = "dlss")]
        if dlss_rr_supported.is_some() {
            info!("DLSS Ray Reconstruction enabled.");
            cam.insert(bevy::anti_alias::dlss::Dlss::<
                bevy::anti_alias::dlss::DlssRayReconstructionFeature,
            > {
                perf_quality_mode: bevy::anti_alias::dlss::DlssPerfQualityMode::Dlaa,
                reset: Default::default(),
                _phantom_data: Default::default(),
            });
        }
    }

    // Rasterized: standard camera with bloom, SSAO, and distance fog
    #[cfg(not(feature = "solari"))]
    {
        commands.spawn((
            Camera3d::default(),
            cam_transform,
            Msaa::Off,
            bevy::post_process::bloom::Bloom {
                intensity: 0.15,
                ..default()
            },
            bevy::pbr::ScreenSpaceAmbientOcclusion {
                quality_level: bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel::High,
                constant_object_thickness: 0.25,
            },
            bevy::pbr::DistanceFog {
                color: Color::srgba(0.05, 0.05, 0.08, 1.0),
                falloff: bevy::pbr::FogFalloff::Exponential { density: 0.015 },
                ..default()
            },
            PlayerCamera,
        ));
    }

    // Minimal ambient — light comes from fixtures, not magic fill.
    // Just enough to prevent pitch-black corners.
    commands.spawn(AmbientLight {
        color: Color::srgb(0.7, 0.75, 0.85),
        brightness: 20.0,
        affects_lightmapped_meshes: true,
    });
}

pub fn camera_follow_player(
    state: Res<ConnectionState>,
    player: Res<PlayerState>,
    mut view: ResMut<ViewState>,
    mut camera_q: Query<&mut Transform, With<PlayerCamera>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
    windows: Query<&Window>,
    mut cursor_q: Query<&mut bevy::window::CursorOptions>,
) {
    // Toggle camera mode with V; Escape returns to top-down
    if keyboard.just_pressed(KeyCode::KeyV) {
        view.camera_mode = match view.camera_mode {
            CameraMode::TopDown => CameraMode::FirstPerson,
            CameraMode::FirstPerson => CameraMode::TopDown,
        };
    } else if keyboard.just_pressed(KeyCode::Escape) && view.camera_mode == CameraMode::FirstPerson
    {
        view.camera_mode = CameraMode::TopDown;
    }
    // Update cursor grab based on mode
    if let Ok(mut cursor) = cursor_q.single_mut() {
        let (grab, visible) = match view.camera_mode {
            CameraMode::FirstPerson => (bevy::window::CursorGrabMode::Locked, false),
            CameraMode::TopDown => (bevy::window::CursorGrabMode::None, true),
        };
        if cursor.grab_mode != grab {
            cursor.grab_mode = grab;
            cursor.visible = visible;
        }
    }

    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };
    let Ok(mut cam_tf) = camera_q.single_mut() else {
        return;
    };
    let Some(pid) = player.person_id else { return };
    let Some(pos) = conn.db.position().person_id().find(&pid) else {
        return;
    };

    match view.camera_mode {
        CameraMode::TopDown => {
            // Smooth camera follow — fixed top-down rotation
            let target = Vec3::new(pos.x, view.camera_height, pos.y);
            cam_tf.translation = cam_tf.translation.lerp(target, 0.08);
            cam_tf.rotation = Transform::from_xyz(0.0, 1.0, 0.0)
                .looking_at(Vec3::ZERO, Vec3::NEG_Z)
                .rotation;
        }
        CameraMode::FirstPerson => {
            // Mouse look
            let sensitivity = 0.003;
            for ev in mouse_motion.read() {
                view.fps_yaw -= ev.delta.x * sensitivity;
                view.fps_pitch = (view.fps_pitch - ev.delta.y * sensitivity).clamp(-1.4, 1.4);
            }

            // Eye height position at player location
            let eye_height = 1.6;
            let target = Vec3::new(pos.x, eye_height, pos.y);
            cam_tf.translation = cam_tf.translation.lerp(target, 0.15);

            // Apply yaw and pitch rotation
            cam_tf.rotation = Quat::from_euler(EulerRot::YXZ, view.fps_yaw, view.fps_pitch, 0.0);
        }
    }
}
