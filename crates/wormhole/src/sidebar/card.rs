//! A line of the list set in its card: a group's head and rows share one, a terminal in no group is a
//! card of its own, and each card is a glass of its own. The list draws each of its lines this way, and
//! so does a row that is leaving it (`motion::leaving`), which is drawn from the listing it was in.

use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::view::sidebar::{Card, Entry, Listing};

use super::facts::RowInfo;
use super::glass::{GlassKind, Insets, LIST_CLIP, glass_key, probe};
use crate::platform::vibrancy::Glass;
use crate::settings::UiColors;
use crate::style::{self, Metrics};

/// What the lines of a listing are set against.
pub(super) struct Cards<'a> {
    pub(super) listing: &'a Listing,
    pub(super) infos: &'a [RowInfo],
    pub(super) ui: UiColors,
    pub(super) metrics: Metrics,
    pub(super) glass: Option<&'a Rc<Glass>>,
    /// A group's colour, which washes its card from the left.
    pub(super) wash: &'a HashMap<u32, Hsla>,
    /// The terminal on screen: a card of one row takes the selection's colour.
    pub(super) active: usize,
}

impl Cards<'_> {
    /// Line `at` of the listing (`line`, a group's head or a terminal's row) as a stretch of its card.
    /// `masked`: the line is cut by a box that opens (a row coming in by its height), so its glass is too.
    pub(super) fn line(&self, at: usize, line: AnyElement, masked: bool) -> Div {
        let Self { listing, infos, ui, metrics, glass, wash, active } = *self;
        // Each line is a stretch of its card: a group's head and rows, or a terminal's own.
        let (top, bottom) = listing.card_edges(at);
        let wash = match listing.card(at) {
            Some(Card::Group(band)) => wash.get(&listing.bands[band].group).copied(),
            _ => None,
        };
        let segment = match (listing.card(at), &listing.entries[at]) {
            (Some(Card::Loose(_)), Entry::Row { session, .. }) => {
                style::card_segment_solo(ui, metrics, *session == active, line)
            }
            _ => style::card_segment(ui, metrics, top, bottom, wash, line),
        };
        // Each card is a glass of its own.
        let Some(glass) = glass.filter(|_| metrics.cards) else { return segment };
        let (key, inset) = match listing.card(at) {
            Some(Card::Group(band)) => (
                glass_key(GlassKind::Card, 1 + band as u64),
                Insets(
                    metrics.card_margin,
                    0.0,
                    metrics.card_margin,
                    if bottom { metrics.card_gap } else { 0.0 },
                ),
            ),
            Some(Card::Split(first)) => (
                glass_key(GlassKind::Loose, infos[first].id as u64),
                Insets(
                    metrics.card_margin,
                    0.0,
                    metrics.card_margin,
                    if bottom { metrics.card_gap } else { 0.0 },
                ),
            ),
            Some(Card::Loose(session)) => (
                glass_key(GlassKind::Loose, infos[session].id as u64),
                Insets(metrics.card_margin, 0.0, metrics.card_margin, metrics.card_gap),
            ),
            None => (0, Insets(0.0, 0.0, 0.0, 0.0)),
        };
        segment.child(probe(glass, key, inset, metrics.card_radius, Some(LIST_CLIP), masked))
    }
}
