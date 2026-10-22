//! What the sidebar's rows, cards and header measure at each density, with the cards on or off
//! (RF-10, RF-40): one place for the numbers, so a row, a group's head and the settings preview
//! draw the same.

use wormhole_core::sidebar::Density;

use super::scale::{TILE, TILE_DENSE_FOLD, TILE_MINI, TILE_SMALL};

/// The measurements of the terminal list at a density.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub density: Density,
    /// Groups are cards. Off, they run the sidebar's width.
    pub cards: bool,
    /// A group's colour fades out across its card (Settings → Fade group colours); off, it fills
    /// the card evenly.
    pub fade_wash: bool,
    /// A terminal's row: its height, its status tile and the gap after it.
    pub row_h: f32,
    pub tile: f32,
    pub tile_gap: f32,
    /// The row has a second line under its name.
    pub two_lines: bool,
    /// The row's padding at its sides, and the corner of its selection and hover.
    pub row_px: f32,
    pub row_radius: f32,
    /// The card: how far it is in from the sidebar's sides (the same off, where the rows stand in by
    /// it too), the padding between its edge and its rows, its corner, and the gap under it.
    pub card_margin: f32,
    pub card_pad: f32,
    pub card_radius: f32,
    pub card_gap: f32,
    /// A group's head: its height, and the tiles it keeps while folded (and their gap).
    pub head_h: f32,
    pub folded_tile: f32,
    pub folded_gap: f32,
    /// A pinned group with nothing in it, one line.
    pub empty_h: f32,
    /// The "Terminals" header: its height, how far in it stands, the + and whether the title is the
    /// smaller one.
    pub header_h: f32,
    pub header_in: f32,
    pub add: f32,
    pub small_title: bool,
}

impl Metrics {
    pub fn of(density: Density, cards: bool) -> Self {
        let dense = density == Density::Dense;
        let (row_h, tile, tile_gap) = match density {
            Density::Regular => (48.0, TILE, 10.0),
            Density::Compact => (32.0, TILE_MINI, 8.0),
            Density::Dense => (26.0, TILE_SMALL, 8.0),
        };
        // Without cards the rows stand in from the sidebar's sides by what a card's margin was, less.
        let (card_margin, card_pad) = match (dense, cards) {
            (false, true) => (10.0, 4.0),
            (true, true) => (4.0, 2.0),
            (false, false) => (6.0, 0.0),
            (true, false) => (4.0, 0.0),
        };
        Self {
            density,
            cards,
            fade_wash: false,
            row_h,
            tile,
            tile_gap,
            two_lines: density.two_lines(),
            row_px: if dense { 4.0 } else { 8.0 },
            row_radius: if dense { 6.0 } else { 8.0 },
            card_margin,
            card_pad,
            card_radius: if dense { 8.0 } else { 12.0 },
            card_gap: if dense { 4.0 } else { 8.0 },
            head_h: if dense { 26.0 } else { 36.0 },
            folded_tile: if dense { TILE_DENSE_FOLD } else { TILE_MINI },
            folded_gap: 4.0,
            empty_h: if dense { 24.0 } else { 32.0 },
            header_h: if dense { 22.0 } else { 28.0 },
            header_in: if dense { 8.0 } else { 12.0 },
            add: if dense { 20.0 } else { 24.0 },
            small_title: dense,
        }
    }

    /// The same measurements, with a group's colour fading across its card or not.
    pub fn fading(self, fade_wash: bool) -> Self {
        Self { fade_wash, ..self }
    }
}

impl Default for Metrics {
    /// Regular, in cards: what the shared parts of the list (the studio's gallery) draw by default.
    fn default() -> Self {
        Self::of(Density::Regular, true)
    }
}

#[cfg(test)]
mod tests {
    use super::Metrics;
    use wormhole_core::sidebar::Density;

    #[test]
    fn rows_and_cards_are_what_the_board_measures() {
        let regular = Metrics::of(Density::Regular, true);
        assert_eq!(
            (regular.row_h, regular.tile, regular.card_margin, regular.card_pad),
            (48.0, 32.0, 10.0, 4.0)
        );
        assert_eq!((regular.card_radius, regular.card_gap, regular.head_h), (12.0, 8.0, 36.0));
        let compact = Metrics::of(Density::Compact, true);
        assert_eq!((compact.row_h, compact.tile, compact.two_lines), (32.0, 20.0, false));
        assert_eq!((compact.card_margin, compact.card_gap), (10.0, 8.0), "cards as Regular");
        let dense = Metrics::of(Density::Dense, true);
        assert_eq!((dense.row_h, dense.tile, dense.card_margin, dense.card_pad), (26.0, 16.0, 4.0, 2.0));
        assert_eq!(
            (dense.card_radius, dense.card_gap, dense.head_h, dense.row_radius),
            (8.0, 4.0, 26.0, 6.0)
        );
        assert_eq!((dense.folded_tile, dense.empty_h, dense.header_h), (14.0, 24.0, 22.0));
    }

    #[test]
    fn a_groups_colour_fills_the_card_evenly_unless_fading_is_asked_for() {
        assert!(!Metrics::of(Density::Regular, true).fade_wash);
        assert!(!Metrics::default().fade_wash);
        assert!(Metrics::of(Density::Regular, true).fading(true).fade_wash);
    }

    #[test]
    fn without_cards_the_rows_stand_in_by_six_or_four() {
        assert_eq!(Metrics::of(Density::Regular, false).card_margin, 6.0);
        assert_eq!(Metrics::of(Density::Dense, false).card_margin, 4.0);
        assert_eq!(Metrics::of(Density::Compact, false).card_pad, 0.0);
    }
}
