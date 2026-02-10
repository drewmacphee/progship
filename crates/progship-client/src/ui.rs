//! UI rendering for the ProgShip client.
//!
//! Handles HUD overlay, status panel, room info, conversation bubbles, and toast notifications.

use bevy::prelude::*;
use progship_client_sdk::*;
use spacetimedb_sdk::Table;

use crate::state::{
    ConnectionConfig, ConnectionState, HudText, InfoPanel, NeedsBar, PlayerState, ToastContainer,
    UiState, ViewState,
};

pub fn setup_ui(mut commands: Commands) {
    // HUD - ship info (top-left)
    commands.spawn((
        Text::new("Connecting to SpacetimeDB..."),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
        HudText,
    ));

    // HUD - needs bars (bottom-left)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(10.0),
            ..default()
        },
        NeedsBar,
    ));

    // Info panel (right side — room info, selected NPC, or ship overview)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            top: Val::Px(10.0),
            max_width: Val::Px(320.0),
            ..default()
        },
        InfoPanel,
    ));

    // Toast container (top-center)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.9, 0.3)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(30.0),
            right: Val::Percent(30.0),
            top: Val::Px(50.0),
            ..default()
        },
        ToastContainer,
    ));
}

pub fn render_hud(
    state: Res<ConnectionState>,
    config: Res<ConnectionConfig>,
    view: Res<ViewState>,
    player: Res<PlayerState>,
    mut hud_q: Query<
        &mut Text,
        (
            With<HudText>,
            Without<NeedsBar>,
            Without<InfoPanel>,
            Without<ToastContainer>,
        ),
    >,
    mut needs_q: Query<
        &mut Text,
        (
            With<NeedsBar>,
            Without<HudText>,
            Without<InfoPanel>,
            Without<ToastContainer>,
        ),
    >,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        ConnectionState::Reconnecting => {
            if let Ok(mut text) = hud_q.get_single_mut() {
                **text = format!(
                    "Reconnecting to {}... (attempt {}, {:.0}s)",
                    config.server_url,
                    config.reconnect_attempts,
                    config.reconnect_timer.max(0.0)
                );
            }
            return;
        }
        _ => {
            if let Ok(mut text) = hud_q.get_single_mut() {
                **text = format!("Connecting to {}...", config.server_url);
            }
            return;
        }
    };

    // Top-left: ship info
    if let Ok(mut text) = hud_q.get_single_mut() {
        let config = conn.db.ship_config().id().find(&0);
        let person_count = conn.db.person().count();
        let active_events: Vec<_> = conn.db.event().iter().filter(|e| e.state != 2).collect();

        let (ship_name, sim_time, time_scale, paused) = match config {
            Some(c) => (c.name.clone(), c.sim_time, c.time_scale, c.paused),
            None => ("No Ship".into(), 0.0, 1.0, false),
        };

        let hours = sim_time % 24.0;
        let day = (sim_time / 24.0) as u32 + 1;
        let h = hours as u32;
        let m = ((hours - h as f64) * 60.0) as u32;

        let pause_str = if paused { " [PAUSED]" } else { "" };
        let event_str = if !active_events.is_empty() {
            format!(" | {} EVENTS", active_events.len())
        } else {
            String::new()
        };

        // Get player's current room and context action
        let (room_name, context_hint) = player
            .person_id
            .and_then(|pid| conn.db.position().person_id().find(&pid))
            .and_then(|pos| conn.db.room().id().find(&pos.room_id))
            .map(|r| (r.name.clone(), context_action_hint(r.room_type)))
            .unwrap_or_default();

        // Get player activity
        let activity_str = player
            .person_id
            .and_then(|pid| conn.db.activity().person_id().find(&pid))
            .map(|a| format!(" ({})", activity_name(a.activity_type)))
            .unwrap_or_default();

        // Atmosphere info for current deck
        let atmo_str = conn
            .db
            .deck_atmosphere()
            .deck()
            .find(&view.current_deck)
            .map(|a| {
                let o2_pct = a.oxygen * 100.0;
                let temp = a.temperature;
                let warn = if o2_pct < 19.0 {
                    " LOW O2!"
                } else if temp > 30.0 {
                    " HOT!"
                } else if temp < 15.0 {
                    " COLD!"
                } else {
                    ""
                };
                format!("O2:{:.0}% {:.0}C{}", o2_pct, temp, warn)
            })
            .unwrap_or_default();

        **text = format!(
            "{} | Day {} {:02}:{:02}{} | {}x{}\n\
             Deck {} | {} | {} aboard | {}\n\
             {}{}\n\
             [WASD] Move [E] Talk [F]{} [Q] Inspect [Tab] Overview [Space] Pause",
            ship_name,
            day,
            h,
            m,
            pause_str,
            time_scale,
            event_str,
            view.current_deck + 1,
            room_name,
            person_count,
            atmo_str,
            activity_str,
            if activity_str.is_empty() { "" } else { "" },
            context_hint,
        );
    }

    // Bottom-left: player needs bars
    if let Ok(mut text) = needs_q.get_single_mut() {
        if let Some(pid) = player.person_id {
            if let Some(needs) = conn.db.needs().person_id().find(&pid) {
                let bar = |val: f32, label: &str, invert: bool| -> String {
                    let display = if invert { 1.0 - val } else { val };
                    let filled = (display * 10.0) as usize;
                    let empty = 10 - filled.min(10);
                    let status = if invert {
                        if val > 0.7 {
                            "!!"
                        } else if val > 0.4 {
                            "!"
                        } else {
                            ""
                        }
                    } else {
                        if val < 0.3 {
                            "!!"
                        } else if val < 0.6 {
                            "!"
                        } else {
                            ""
                        }
                    };
                    format!(
                        "{}: [{}{}] {:.0}%{}",
                        label,
                        "#".repeat(filled.min(10)),
                        "-".repeat(empty),
                        display * 100.0,
                        status
                    )
                };

                **text = format!(
                    "{}\n{}\n{}\n{}\n{}\n{}\nMorale: {:.0}%",
                    bar(needs.health, "HP     ", false),
                    bar(needs.hunger, "Hunger ", true),
                    bar(needs.fatigue, "Energy ", true),
                    bar(needs.social, "Social ", true),
                    bar(needs.comfort, "Comfort", true),
                    bar(needs.hygiene, "Hygiene", true),
                    needs.morale * 100.0,
                );
            }
        } else {
            let join_msg = if player.join_attempts >= 3 {
                "Failed to join — is the ship initialized?\nRun: spacetime call progship init_ship -- '\"Ship Name\"' 12 200 800 -s <server>"
            } else if player.joined {
                &format!("Joining game... ({:.0}s)", player.join_timer)
            } else {
                "Waiting for server..."
            };
            **text = join_msg.to_string();
        }
    }
}

pub fn render_info_panel(
    state: Res<ConnectionState>,
    view: Res<ViewState>,
    player: Res<PlayerState>,
    ui: Res<UiState>,
    mut panel_q: Query<
        &mut Text,
        (
            With<InfoPanel>,
            Without<HudText>,
            Without<NeedsBar>,
            Without<ToastContainer>,
        ),
    >,
) {
    let conn = match &*state {
        ConnectionState::Connected(c) => c,
        _ => return,
    };
    let Ok(mut text) = panel_q.get_single_mut() else {
        return;
    };

    if ui.show_ship_overview {
        // Ship overview (Tab)
        let config = conn.db.ship_config().id().find(&0);
        let crew_count = conn.db.crew().count();
        let passenger_count = conn.db.passenger().count();
        let active_events: Vec<_> = conn.db.event().iter().filter(|e| e.state != 2).collect();

        let mut overview = format!("=== SHIP OVERVIEW ===\n");
        if let Some(c) = &config {
            overview += &format!("{}\n\n", c.name);
        }
        overview += &format!("Crew: {}  Passengers: {}\n\n", crew_count, passenger_count);

        // Resources
        if let Some(res) = conn.db.ship_resources().id().find(&0) {
            overview += &format!("--- Resources ---\n");
            overview += &format!("Power: {:.0}/{:.0}\n", res.power, res.power_cap);
            overview += &format!("Food:  {:.0}/{:.0}\n", res.food, res.food_cap);
            overview += &format!("Water: {:.0}/{:.0}\n", res.water, res.water_cap);
            overview += &format!("O2:    {:.0}/{:.0}\n", res.oxygen, res.oxygen_cap);
            overview += &format!("Fuel:  {:.0}/{:.0}\n", res.fuel, res.fuel_cap);
            overview += &format!(
                "Parts: {:.0}/{:.0}\n\n",
                res.spare_parts, res.spare_parts_cap
            );
        }

        // Systems
        let systems: Vec<_> = conn.db.ship_system().iter().collect();
        let degraded: Vec<_> = systems.iter().filter(|s| s.overall_health < 0.9).collect();
        if !degraded.is_empty() {
            overview += &format!(
                "--- Systems ({} issue{}) ---\n",
                degraded.len(),
                if degraded.len() > 1 { "s" } else { "" }
            );
            for sys in degraded.iter().take(5) {
                let status = system_status_str(sys.overall_status);
                overview += &format!(
                    "{}: {:.0}% [{}]\n",
                    sys.name,
                    sys.overall_health * 100.0,
                    status
                );
            }
            overview += "\n";
        }

        // Active events
        if !active_events.is_empty() {
            overview += &format!("--- Events ({}) ---\n", active_events.len());
            for evt in active_events.iter().take(5) {
                let etype = event_type_name(evt.event_type);
                let room_name = conn
                    .db
                    .room()
                    .id()
                    .find(&evt.room_id)
                    .map(|r| r.name.clone())
                    .unwrap_or("?".into());
                overview += &format!(
                    "{} in {} [{:.0}%]\n",
                    etype,
                    room_name,
                    evt.severity * 100.0
                );
            }
            overview += "\n";
        }

        // Deck atmospheres
        overview += "--- Atmosphere ---\n";
        for deck_idx in 0..6 {
            if let Some(atmo) = conn.db.deck_atmosphere().deck().find(&deck_idx) {
                let warn = if atmo.oxygen < 0.19 { " !" } else { "" };
                overview += &format!(
                    "Dk{}: O2:{:.0}% {:.0}C {:.0}kPa{}\n",
                    deck_idx + 1,
                    atmo.oxygen * 100.0,
                    atmo.temperature,
                    atmo.pressure,
                    warn
                );
            }
        }

        **text = overview;
        return;
    }

    if let Some(selected_id) = ui.selected_person {
        // NPC info panel
        let Some(person) = conn.db.person().id().find(&selected_id) else {
            **text = "".into();
            return;
        };

        let mut info = format!("=== {} {} ===\n", person.given_name, person.family_name);
        info += if person.is_crew { "Crew" } else { "Passenger" };

        if let Some(crew) = conn.db.crew().person_id().find(&selected_id) {
            info += &format!(
                "\n{} - {}\nShift: {}\n",
                department_name(crew.department),
                rank_name(crew.rank),
                shift_name(crew.shift)
            );
        }
        if let Some(passenger) = conn.db.passenger().person_id().find(&selected_id) {
            info += &format!(
                "\n{}\nDest: {}\n",
                passenger.profession, passenger.destination
            );
        }

        if let Some(needs) = conn.db.needs().person_id().find(&selected_id) {
            info += &format!("\n--- Needs ---\n");
            info += &format!(
                "HP: {:.0}%  Morale: {:.0}%\n",
                needs.health * 100.0,
                needs.morale * 100.0
            );
            info += &format!(
                "Hunger: {:.0}%  Fatigue: {:.0}%\n",
                needs.hunger * 100.0,
                needs.fatigue * 100.0
            );
            info += &format!(
                "Social: {:.0}%  Hygiene: {:.0}%\n",
                needs.social * 100.0,
                needs.hygiene * 100.0
            );
        }

        if let Some(activity) = conn.db.activity().person_id().find(&selected_id) {
            info += &format!("\nActivity: {}\n", activity_name(activity.activity_type));
        }

        if let Some(pos) = conn.db.position().person_id().find(&selected_id) {
            if let Some(room) = conn.db.room().id().find(&pos.room_id) {
                info += &format!("Location: {}\n", room.name);
            }
        }

        // Conversation
        if let Some(in_conv) = conn.db.in_conversation().person_id().find(&selected_id) {
            if let Some(conv) = conn.db.conversation().id().find(&in_conv.conversation_id) {
                let other_id = if conv.participant_a == selected_id {
                    conv.participant_b
                } else {
                    conv.participant_a
                };
                let other_name = conn
                    .db
                    .person()
                    .id()
                    .find(&other_id)
                    .map(|p| format!("{} {}", p.given_name, p.family_name))
                    .unwrap_or("?".into());
                info += &format!(
                    "\nTalking to: {}\nTopic: {}\n",
                    other_name,
                    topic_name(conv.topic)
                );
            }
        }

        **text = info;
        return;
    }

    // Default: show current room info
    let Some(pid) = player.person_id else {
        **text = "".into();
        return;
    };
    let Some(pos) = conn.db.position().person_id().find(&pid) else {
        **text = "".into();
        return;
    };
    let Some(room) = conn.db.room().id().find(&pos.room_id) else {
        **text = "".into();
        return;
    };

    let mut info = format!(
        "=== {} ===\n{}\n\n",
        room.name,
        room_type_name(room.room_type)
    );

    // People in room
    let people_here: Vec<_> = conn
        .db
        .position()
        .iter()
        .filter(|p| p.room_id == room.id)
        .collect();
    info += &format!("Occupants: {}/{}\n", people_here.len(), room.capacity);
    for p in people_here.iter().take(8) {
        if let Some(person) = conn.db.person().id().find(&p.person_id) {
            let role = if Some(p.person_id) == player.person_id {
                " (You)"
            } else if person.is_crew {
                " [C]"
            } else {
                " [P]"
            };
            let activity_str = conn
                .db
                .activity()
                .person_id()
                .find(&p.person_id)
                .map(|a| format!(" - {}", activity_name(a.activity_type)))
                .unwrap_or_default();
            info += &format!(
                "  {} {}{}{}\n",
                person.given_name, person.family_name, role, activity_str
            );
        }
    }
    if people_here.len() > 8 {
        info += &format!("  ...and {} more\n", people_here.len() - 8);
    }

    // Subsystems in room
    let subsystems_here: Vec<_> = conn
        .db
        .subsystem()
        .iter()
        .filter(|s| s.node_id == room.node_id)
        .collect();
    if !subsystems_here.is_empty() {
        info += "\n--- Subsystems ---\n";
        for sub in &subsystems_here {
            let status = system_status_str(sub.status);
            info += &format!("{}: {:.0}% [{}]\n", sub.name, sub.health * 100.0, status);
        }
    }

    // Active events in room
    let events_here: Vec<_> = conn
        .db
        .event()
        .iter()
        .filter(|e| e.room_id == room.id && e.state != 2)
        .collect();
    if !events_here.is_empty() {
        info += "\n--- EVENTS ---\n";
        for evt in &events_here {
            info += &format!(
                "!! {} [{:.0}% severity]\n",
                event_type_name(evt.event_type),
                evt.severity * 100.0
            );
        }
    }

    **text = info;
}

pub fn render_toasts(
    ui: Res<UiState>,
    mut toast_q: Query<
        &mut Text,
        (
            With<ToastContainer>,
            Without<HudText>,
            Without<NeedsBar>,
            Without<InfoPanel>,
        ),
    >,
) {
    let Ok(mut text) = toast_q.get_single_mut() else {
        return;
    };
    if ui.toasts.is_empty() {
        **text = "".into();
    } else {
        let toast_text: Vec<String> = ui.toasts.iter().map(|t| t.message.clone()).collect();
        **text = toast_text.join("\n");
    }
}

// Helper functions
fn activity_name(activity_type: u8) -> &'static str {
    match activity_type {
        0 => "Idle",
        1 => "Working",
        2 => "Eating",
        3 => "Sleeping",
        4 => "Socializing",
        5 => "Relaxing",
        6 => "Hygiene",
        7 => "Traveling",
        8 => "Maintenance",
        9 => "On Duty",
        10 => "Off Duty",
        11 => "Emergency",
        12 => "Exercising",
        _ => "Unknown",
    }
}

fn room_type_name(room_type: u8) -> &'static str {
    match room_type {
        0 => "Bridge",
        1 => "Conference Room",
        2 => "Engineering",
        3 => "Reactor",
        4 => "Maintenance Bay",
        5 => "Quarters",
        6 => "Crew Quarters",
        7 => "Officer Quarters",
        8 => "Passenger Quarters",
        9 => "Mess Hall",
        10 => "Galley",
        11 => "Medical Bay",
        12 => "Recreation",
        13 => "Gym",
        14 => "Cargo Bay",
        15 => "Storage",
        16 => "Airlock",
        17 => "Corridor",
        18 => "Elevator",
        19 => "Laboratory",
        20 => "Observatory",
        21 => "Life Support",
        22 => "Hydroponics",
        23 => "Water Recycling",
        24 => "Service Corridor",
        25 => "Elevator Shaft",
        26 => "Ladder Shaft",
        27 => "Surgery",
        28 => "Pharmacy",
        29 => "Recovery Ward",
        30 => "Chapel",
        31 => "Laundry",
        32 => "Shops",
        33 => "Lounge",
        34 => "CIC",
        35 => "Cooling Plant",
        36 => "Power Distribution",
        37 => "HVAC Control",
        38 => "Parts Storage",
        39 => "Waste Processing",
        40 => "Comms Room",
        _ => "Unknown",
    }
}

fn department_name(dept: u8) -> &'static str {
    match dept {
        0 => "Command",
        1 => "Engineering",
        2 => "Medical",
        3 => "Science",
        4 => "Security",
        5 => "Operations",
        6 => "Civilian",
        _ => "Unknown",
    }
}

fn rank_name(rank: u8) -> &'static str {
    match rank {
        0 => "Crewman",
        1 => "Specialist",
        2 => "Petty Officer",
        3 => "Chief",
        4 => "Ensign",
        5 => "Lieutenant",
        6 => "Commander",
        7 => "Captain",
        _ => "Unknown",
    }
}

fn shift_name(shift: u8) -> &'static str {
    match shift {
        0 => "Alpha (06:00-14:00)",
        1 => "Beta (14:00-22:00)",
        2 => "Gamma (22:00-06:00)",
        _ => "Unknown",
    }
}

fn topic_name(topic: u8) -> &'static str {
    match topic {
        0 => "Greeting",
        1 => "Work",
        2 => "Gossip",
        3 => "Personal",
        4 => "Complaint",
        5 => "Request",
        6 => "Flirtation",
        7 => "Argument",
        8 => "Farewell",
        _ => "Unknown",
    }
}

fn system_status_str(status: u8) -> &'static str {
    match status {
        0 => "OK",
        1 => "DEGRADED",
        2 => "CRITICAL",
        3 => "OFFLINE",
        4 => "DESTROYED",
        _ => "?",
    }
}

pub fn event_type_name(event_type: u8) -> &'static str {
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

pub fn context_action_hint(room_type: u8) -> &'static str {
    match room_type {
        9 | 10 => " Eat",
        5..=8 => " Sleep/Wash",
        2 | 3 | 4 | 21..=23 => " Repair",
        12 | 13 => " Exercise",
        25 => " Elevator [1-6]",
        26 => " Ladder [Up/Down]",
        _ => "",
    }
}
