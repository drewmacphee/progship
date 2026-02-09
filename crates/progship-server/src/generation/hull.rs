//! Hull dimension calculations for ship tapering.
//!
//! Provides functions to compute hull width and length per deck, implementing
//! bow and stern tapering for the colony ship's aerodynamic profile.

/// Hull width for a given deck, applying taper at bow (top decks) and stern (bottom decks).
pub(super) fn hull_width(deck: u32, deck_count: u32, ship_beam: usize) -> usize {
    match deck {
        0..=1 => 40,
        d if d >= deck_count.saturating_sub(2) => 50,
        _ => ship_beam,
    }
}

/// Hull length for a given deck, applying taper at bow (top decks) and stern (bottom decks).
pub(super) fn hull_length(deck: u32, deck_count: u32, ship_length: usize) -> usize {
    match deck {
        0..=1 => 200,
        d if d >= deck_count.saturating_sub(2) => 300,
        _ => ship_length,
    }
}
