//! Virtualized rows for the right panel's lists (Notes, Changes, History), each row a stretch
//! of a card.
//!
//! A plain `overflow_y_scroll` div is not an option for those: scrolling
//! notifies the whole workspace view, so every wheel tick would rebuild every
//! row. GPUI's `list` asks for the rows in view only, so a tick costs the same
//! for 20 rows as for 20 000.

use std::ops::Range;
use std::rc::Rc;

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style;

/// Laid out above and below the viewport so fast scrolling doesn't pop in.
const OVERDRAW: f32 = 300.0;

/// Where a row of a carded list stands (`Rows::element_carded`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardPart {
    /// The first line of a card: its head.
    Head,
    /// A line under a head, in the same card.
    Body,
    /// Not in a card.
    Outside,
}

/// What draws a list's rows as cards: the surface's colours and where each row stands.
type Carded<T> = (UiColors, fn(&T) -> CardPart, Option<style::CardBacking>);

/// How a row sits in its card, given the part of the row after it: `None` for a row outside any
/// card, else whether it is the card's top (a head) and whether it is also its bottom (nothing
/// of the card follows).
fn card_edges(part: CardPart, next: Option<CardPart>) -> Option<(bool, bool)> {
    match part {
        CardPart::Outside => None,
        CardPart::Head | CardPart::Body => {
            Some((part == CardPart::Head, next.is_none_or(|next| next != CardPart::Body)))
        }
    }
}

/// The rows of one list plus the state that scrolls them.
///
/// Rows are a cheap description (indices, labels), not elements: the render
/// callback turns the visible ones into elements, looking their data up in
/// whatever the caller captured.
pub struct Rows<T> {
    rows: Rc<Vec<T>>,
    state: ListState,
}

impl<T: PartialEq> Rows<T> {
    pub fn new() -> Self {
        Self { rows: Rc::default(), state: ListState::new(0, ListAlignment::Top, px(OVERDRAW)) }
    }

    #[cfg(feature = "devctl")]
    pub fn state(&self) -> ListState {
        self.state.clone()
    }

    /// The rows as they are now (what the keyboard walks).
    pub fn items(&self) -> &[T] {
        &self.rows
    }

    /// Scroll just enough to show row `ix` (the one the keys moved to).
    pub fn reveal(&self, ix: usize) {
        self.state.scroll_to_reveal_item(ix);
    }

    /// The list element. `render` turns one visible row into an element; it is
    /// called for the rows in view only. Fills the space its parent column
    /// gives it (the parent needs a bounded height). `cursor` is the row the keys are on:
    /// VoiceOver follows it as the list's active descendant, and the ring is drawn inside it in
    /// these colors while it shows.
    pub fn element(
        &self,
        cursor: Option<(usize, Option<UiColors>)>,
        render: impl Fn(&T, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> List
    where
        T: 'static,
    {
        self.build(cursor, None, render)
    }

    /// Like [`Self::element`] for the right panel: the rows are the lines of cards. A row that
    /// is `part` [`CardPart::Head`] starts a card and the [`CardPart::Body`] rows after it
    /// continue it; the card ends at the last of them, and a row outside any card (the spacer
    /// at the foot) stands alone. Each row is drawn as its stretch of the card
    /// (`style::panel_segment`), so the list stays virtual.
    pub fn element_carded(
        &self,
        cursor: Option<(usize, Option<UiColors>)>,
        ui: UiColors,
        backing: Option<style::CardBacking>,
        part: fn(&T) -> CardPart,
        render: impl Fn(&T, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> List
    where
        T: 'static,
    {
        self.build(cursor, Some((ui, part, backing)), render)
    }

    fn build(
        &self,
        cursor: Option<(usize, Option<UiColors>)>,
        carded: Option<Carded<T>>,
        render: impl Fn(&T, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> List
    where
        T: 'static,
    {
        let rows = self.rows.clone();
        list(self.state.clone(), move |ix, window, cx| match rows.get(ix) {
            // Items are laid out at their content width unless told otherwise.
            Some(row) => {
                let keyed = cursor.filter(|(at, _)| *at == ix);
                let content = render(row, window, cx);
                let content = match &carded {
                    Some((ui, part, backing)) => match card_edges(part(row), rows.get(ix + 1).map(part)) {
                        None => content,
                        Some((top, bottom)) => {
                            let segment = style::panel_segment(*ui, top, bottom, true, false, content);
                            match backing {
                                // The glass is the card's, named by its head's line.
                                Some(back) => {
                                    let head = (0..=ix).rev().find(|&k| part(&rows[k]) != CardPart::Body);
                                    back(segment, head.unwrap_or(ix) as u64, bottom)
                                }
                                None => segment,
                            }
                            .into_any_element()
                        }
                    },
                    None => content,
                };
                div()
                    .id(("list-row", ix))
                    .w_full()
                    .relative()
                    .when(keyed.is_some(), |d| d.role(Role::ListItem).aria_active_descendant())
                    .child(content)
                    .children(
                        keyed
                            .and_then(|(_, ring)| ring)
                            .map(|ui| style::focus_ring(ui, style::RingPlacement::Inside, px(0.))),
                    )
                    .into_any_element()
            }
            None => div().into_any_element(),
        })
        .flex_1()
        .min_h_0()
    }

    /// Swap in new rows. Only what differs is invalidated, so the scroll
    /// position and the measured heights of the other rows survive a refresh.
    pub fn set(&mut self, rows: Vec<T>) {
        self.set_with(rows, |_| None);
    }

    /// Like [`Self::set`], for rows that hold keyboard focus (a text field).
    /// `focus` gives a row's handle; the list keeps rendering a focused row
    /// that has scrolled out of view, so typing into it still works.
    pub fn set_with(&mut self, rows: Vec<T>, focus: impl Fn(&T) -> Option<FocusHandle>) {
        if *self.rows == rows {
            return;
        }
        for (old, new) in splices(&self.rows, &rows) {
            self.state.splice_focusable(old, rows[new].iter().map(&focus));
        }
        self.rows = Rc::new(rows);
    }
}

impl<T: PartialEq> Default for Rows<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// The splices, as `(old range, new range)`, that turn `old` into `new`.
///
/// Same length: one splice per run of differing rows, so nothing shifts. A row
/// that gains or loses a line of text changes height, not position.
/// Otherwise: one splice over whatever lies between the common head and tail.
fn splices<T: PartialEq>(old: &[T], new: &[T]) -> Vec<(Range<usize>, Range<usize>)> {
    if old.len() != new.len() {
        let head = old.iter().zip(new).take_while(|(a, b)| a == b).count();
        let tail = old[head..].iter().rev().zip(new[head..].iter().rev()).take_while(|(a, b)| a == b).count();
        return vec![(head..old.len() - tail, head..new.len() - tail)];
    }
    let mut runs: Vec<Range<usize>> = Vec::new();
    for (ix, (a, b)) in old.iter().zip(new).enumerate() {
        if a == b {
            continue;
        }
        match runs.last_mut() {
            Some(run) if run.end == ix => run.end += 1,
            _ => runs.push(ix..ix + 1),
        }
    }
    runs.into_iter().map(|run| (run.clone(), run)).collect()
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would pull in GPUI's `test` macro and shadow `#[test]`.
    use super::{CardPart, card_edges, splices};

    #[test]
    fn a_card_is_a_head_and_the_body_rows_after_it() {
        use CardPart::{Body, Head, Outside};
        // A head alone is both the top and the bottom of its card.
        assert_eq!(card_edges(Head, Some(Head)), Some((true, true)));
        assert_eq!(card_edges(Head, Some(Body)), Some((true, false)));
        assert_eq!(card_edges(Body, Some(Body)), Some((false, false)));
        assert_eq!(card_edges(Body, Some(Head)), Some((false, true)));
        assert_eq!(card_edges(Body, Some(Outside)), Some((false, true)));
        assert_eq!(card_edges(Body, None), Some((false, true)));
        assert_eq!(card_edges(Outside, Some(Head)), None);
    }

    #[test]
    fn identical_rows_need_no_splice() {
        assert!(splices(&[1, 2, 3], &[1, 2, 3]).is_empty());
    }

    #[test]
    fn same_length_splices_only_the_differing_runs() {
        assert_eq!(splices(&[1, 2, 3, 4, 5, 6], &[1, 9, 9, 4, 5, 7]), vec![(1..3, 1..3), (5..6, 5..6)]);
    }

    #[test]
    fn insert_in_the_middle_keeps_head_and_tail() {
        assert_eq!(splices(&[1, 2, 3, 4], &[1, 2, 8, 9, 3, 4]), vec![(2..2, 2..4)]);
    }

    #[test]
    fn remove_from_the_middle_keeps_head_and_tail() {
        assert_eq!(splices(&[1, 2, 3, 4, 5], &[1, 5]), vec![(1..4, 1..1)]);
    }

    #[test]
    fn prepend_and_append() {
        assert_eq!(splices(&[1, 2], &[0, 1, 2]), vec![(0..0, 0..1)]);
        assert_eq!(splices(&[1, 2], &[1, 2, 3]), vec![(2..2, 2..3)]);
    }

    #[test]
    fn repeated_rows_never_overlap_head_and_tail() {
        // Head and tail both match the same rows here; the ranges must stay valid.
        assert_eq!(splices(&[7, 7, 7], &[7, 7, 7, 7]), vec![(3..3, 3..4)]);
        assert_eq!(splices(&[7, 7, 7, 7], &[7, 7]), vec![(2..4, 2..2)]);
    }

    #[test]
    fn from_and_to_empty() {
        assert_eq!(splices::<u8>(&[], &[1, 2]), vec![(0..0, 0..2)]);
        assert_eq!(splices(&[1, 2], &[]), vec![(0..2, 0..0)]);
    }
}
