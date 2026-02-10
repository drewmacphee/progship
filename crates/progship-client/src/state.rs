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
}

#[derive(Resource)]
pub struct ViewState {
    pub current_deck: i32,
    pub camera_height: f32,
    pub tick_timer: f32,
    pub people_sync_timer: f32,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            current_deck: 0,
            camera_height: 150.0, // Default shows ~200m area on 400m deck
            tick_timer: 0.0,
            people_sync_timer: 0.0,
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
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            joined: false,
            person_id: None,
            pending_dx: 0.0,
            pending_dy: 0.0,
            move_send_timer: 0.0,
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
