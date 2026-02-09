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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hull_width_taper_at_bow() {
        let deck_count = 20;
        let ship_beam = 150;

        // Top decks (bow) should be narrower
        assert_eq!(hull_width(0, deck_count, ship_beam), 40);
        assert_eq!(hull_width(1, deck_count, ship_beam), 40);

        // Middle decks should use full beam
        assert_eq!(hull_width(10, deck_count, ship_beam), ship_beam);
    }

    #[test]
    fn test_hull_width_taper_at_stern() {
        let deck_count = 20;
        let ship_beam = 150;

        // Bottom decks (stern) should be narrower
        assert_eq!(hull_width(18, deck_count, ship_beam), 50);
        assert_eq!(hull_width(19, deck_count, ship_beam), 50);
    }

    #[test]
    fn test_hull_width_equator() {
        let deck_count = 20;
        let ship_beam = 150;

        // Middle decks should use full beam (equator)
        for deck in 5..15 {
            assert_eq!(
                hull_width(deck, deck_count, ship_beam),
                ship_beam,
                "Deck {} should have full beam",
                deck
            );
        }
    }

    #[test]
    fn test_hull_length_taper_at_bow() {
        let deck_count = 20;
        let ship_length = 500;

        // Top decks (bow) should be shorter
        assert_eq!(hull_length(0, deck_count, ship_length), 200);
        assert_eq!(hull_length(1, deck_count, ship_length), 200);
    }

    #[test]
    fn test_hull_length_taper_at_stern() {
        let deck_count = 20;
        let ship_length = 500;

        // Bottom decks (stern) should be shorter
        assert_eq!(hull_length(18, deck_count, ship_length), 300);
        assert_eq!(hull_length(19, deck_count, ship_length), 300);
    }

    #[test]
    fn test_hull_length_equator() {
        let deck_count = 20;
        let ship_length = 500;

        // Middle decks should use full length
        for deck in 5..15 {
            assert_eq!(
                hull_length(deck, deck_count, ship_length),
                ship_length,
                "Deck {} should have full length",
                deck
            );
        }
    }

    #[test]
    fn test_hull_small_ship() {
        // Test with a smaller ship (fewer decks)
        let deck_count = 5;
        let ship_beam = 100;
        let ship_length = 400;

        // Even with few decks, taper rules should apply
        assert_eq!(hull_width(0, deck_count, ship_beam), 40);
        assert_eq!(hull_width(4, deck_count, ship_beam), 50);
        assert_eq!(hull_length(0, deck_count, ship_length), 200);
        assert_eq!(hull_length(4, deck_count, ship_length), 300);
    }
}
