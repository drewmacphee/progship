//! State management for the ProgShip client.
//!
//! Contains resource types and Bevy components used throughout the client.

use bevy::prelude::*;
use progship_client_sdk::DbConnection;

// ============================================================================
// RESOURCES
// ============================================================================

#[derive(Resource)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected(DbConnection),
    Reconnecting,
}

#[derive(Resource)]
pub struct ConnectionConfig {
    pub server_url: String,
    pub module_name: String,
    pub reconnect_delay: f32,
    pub reconnect_timer: f32,
    pub reconnect_attempts: u32,
    pub max_reconnect_delay: f32,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:3000".to_string(),
            module_name: "progship".to_string(),
            reconnect_delay: 1.0,
            reconnect_timer: 0.0,
            reconnect_attempts: 0,
            max_reconnect_delay: 30.0,
        }
    }
}

impl ConnectionConfig {
    pub fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut config = Self::default();
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--server" | "-s" if i + 1 < args.len() => {
                    config.server_url = args[i + 1].clone();
                    i += 2;
                }
                "--module" | "-m" if i + 1 < args.len() => {
                    config.module_name = args[i + 1].clone();
                    i += 2;
                }
                _ => i += 1,
            }
        }
        config
    }

    pub fn reset_backoff(&mut self) {
        self.reconnect_delay = 1.0;
        self.reconnect_attempts = 0;
    }

    pub fn advance_backoff(&mut self) {
        self.reconnect_attempts += 1;
        self.reconnect_delay = (self.reconnect_delay * 2.0).min(self.max_reconnect_delay);
        self.reconnect_timer = self.reconnect_delay;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMode {
    TopDown,
    FirstPerson,
}

#[derive(Resource)]
pub struct ViewState {
    pub current_deck: i32,
    pub prev_deck: i32,
    pub rooms_dirty: bool,
    pub minimap_dirty: bool,
    pub prev_room_count: usize,
    pub camera_height: f32,
    pub tick_timer: f32,
    pub people_sync_timer: f32,
    pub hud_timer: f32,
    pub info_timer: f32,
    pub camera_mode: CameraMode,
    pub fps_yaw: f32,
    pub fps_pitch: f32,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            current_deck: 0,
            prev_deck: -1, // Force initial rebuild
            rooms_dirty: true,
            minimap_dirty: true,
            prev_room_count: 0,
            camera_height: 150.0, // Default shows ~200m area on 400m deck
            tick_timer: 0.0,
            people_sync_timer: 0.0,
            hud_timer: 0.0,
            info_timer: 0.0,
            camera_mode: CameraMode::TopDown,
            fps_yaw: 0.0,
            fps_pitch: 0.0,
        }
    }
}

#[derive(Resource)]
pub struct PlayerState {
    pub joined: bool,
    pub person_id: Option<u64>,
    /// Accumulated movement since last server send
    pub pending_dx: f32,
    pub pending_dy: f32,
    /// Timer for throttling movement sends
    pub move_send_timer: f32,
    /// Timer for join timeout/retry
    pub join_timer: f32,
    /// Number of join attempts
    pub join_attempts: u32,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            joined: false,
            person_id: None,
            pending_dx: 0.0,
            pending_dy: 0.0,
            move_send_timer: 0.0,
            join_timer: 0.0,
            join_attempts: 0,
        }
    }
}

#[derive(Resource)]
pub struct UiState {
    pub selected_person: Option<u64>,
    pub show_ship_overview: bool,
    pub toasts: Vec<Toast>,
    pub last_event_count: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            selected_person: None,
            show_ship_overview: false,
            toasts: Vec::new(),
            last_event_count: 0,
        }
    }
}

pub struct Toast {
    pub message: String,
    pub color: Color,
    pub timer: f32,
}

// ============================================================================
// BEVY COMPONENTS
// ============================================================================

#[derive(Component)]
pub struct RoomEntity {
    pub room_id: u32,
    pub deck: i32,
}

#[derive(Component)]
pub struct RoomLabel;

#[derive(Component)]
pub struct DoorMarker;

#[derive(Component)]
pub struct PersonEntity {
    pub person_id: u64,
}

/// Marker for activity indicators and conversation bubbles (despawned separately from people)
#[derive(Component)]
pub struct IndicatorEntity;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component)]
pub struct HudText;

#[derive(Component)]
pub struct NeedsBar;

#[derive(Component)]
pub struct InfoPanel;

#[derive(Component)]
pub struct ToastContainer;
