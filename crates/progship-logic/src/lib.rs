//! Pure simulation logic for ProgShip.
//!
//! This crate contains all game logic that is independent of any database,
//! engine, or runtime. Functions take plain data and return results, making
//! them unit-testable and portable across SpacetimeDB (WASM), native CLI
//! tools, and any future engine.
//!
//! # Module Overview
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`actions`] | Room-type–validated player actions and needs effects |
//! | [`archetypes`] | Personality-derived behavioral archetypes (7 types) |
//! | [`atmosphere`] | Per-room O2/CO2/temperature/pressure simulation |
//! | [`config`] | System selection algorithm (weighted scoring) |
//! | [`constants`] | Room types, activity types, groups, shifts (u8 IDs) |
//! | [`conversation`] | Conversation memory, topic avoidance, gossip propagation |
//! | [`cylinder`] | O'Neill cylinder ship geometry, sectors, ring corridors |
//! | [`duty`] | Shift scheduling, duty fitness, sleep windows |
//! | [`economy`] | Resource scarcity, rationing, production rates |
//! | [`geometry`] | Ship layout validation (room bounds, doors, connectivity) |
//! | [`health`] | Injury severity, medical recovery, death determination |
//! | [`lod`] | Level-of-detail tiers for 5,000+ agent simulation scale-up |
//! | [`manifest`] | Dynamic facility manifest from systems + population |
//! | [`mission`] | Mission config, destinations, propulsion, voyage profile |
//! | [`movement`] | Room-bounded movement, door traversal, wall-sliding |
//! | [`pathfinding`] | BFS pathfinding over door connectivity graph |
//! | [`population`] | Crew sizing, department allocation, genetic diversity |
//! | [`security`] | Access control, lockdown, patrol routing |
//! | [`ship_config`] | Player-facing ship configuration builder and validation |
//! | [`skills`] | Skill checks, experience gain, training, and decay |
//! | [`supplies`] | Voyage supply manifest and mass budget validation |
//! | [`systems`] | System variant definitions (power, life support, etc.) |
//! | [`utility`] | Personality-driven utility AI for activity selection |

pub mod actions;
pub mod archetypes;
pub mod atmosphere;
pub mod config;
pub mod constants;
pub mod conversation;
pub mod cylinder;
pub mod duty;
pub mod economy;
pub mod geometry;
pub mod health;
pub mod lod;
pub mod manifest;
pub mod mission;
pub mod movement;
pub mod pathfinding;
pub mod population;
pub mod security;
pub mod service_decks;
pub mod ship_config;
pub mod skills;
pub mod supplies;
pub mod systems;
pub mod utility;
