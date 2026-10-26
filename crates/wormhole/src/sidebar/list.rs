//! The terminal list drawn: each line of the listing set in its card and answering to what it should
//! (a row's click, drag and menu, a group head's fold and drop), with the motion the list owes it
//! (`motion`) and the rows that have just left, still drawn, among them. What the lines are made of is
//! `row` and `group_head`; the card round a line is `card`.

use std::collections::HashMap;

use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::sidebar::{Card, Entry, Listing};

use super::card::Cards;
use super::drag::DropMark;
use super::glass::{LIST_CLIP, clip_probe};
use super::row::{Frame, Place};
use super::{ItemKey, Sidebar, SidebarInput, row_times};
use crate::settings::{UiColors, store_in, ui_in};

/// What every line of one frame of the list is drawn against.
struct Lines<'a> {
    input: &'a SidebarInput,
    listing: &'a Listing,
    keys: &'a [ItemKey],
    frame: &'a Frame,
    cards: &'a Cards<'a>,
    /// Where a dragged thing would land, which a line shows.
    mark: Option<&'a DropMark>,
    /// The terminal being dragged by its row, which is not animated while it is held.
    held: Option<i64>,
    ui: UiColors,
}

impl Sidebar {
    /// The list: what lies under the sidebar's header (the window's).
    pub(super) fn render_list(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let input = self.input.clone();
        let infos = &input.rows;
        // A drag that ended outside every drop target never heard its drop.
        if !cx.has_active_drag() {
            self.over = None;
        }
        let ui = ui_in(&input.profile, cx);
        let glass = input.glass.clone();
        let listing = self.listing(cx);
        self.times_drawn = row_times(infos, &input.profile, cx);
        let keys: Vec<ItemKey> = listing
            .entries
            .iter()
            .map(|entry| match *entry {
                Entry::Band { band, .. } => ItemKey::Band(listing.bands[band].key()),
                Entry::Row { session, .. } => ItemKey::Row(infos[session].id),
            })
            .collect();
        self.plan_list_motion(&keys, &listing, cx);
        let frame = Frame::new(&input, self.width, self.hints_showing(cx), window, cx);
        let mark = self.drop_mark(&listing, infos);
        let group_wash: HashMap<u32, Hsla> = {
            let tokens = &store_in(&input.profile, cx).tokens;
            self.view
                .groups
                .iter()
                .filter_map(|group| {
                    let color = group.paint()?;
                    Some((group.id, crate::theme_bridge::hsla(color.wash(tokens))))
                })
                .collect()
        };
        let cards = Cards {
            listing: &listing,
            infos,
            ui,
            metrics: frame.metrics,
            glass: glass.as_ref(),
            wash: &group_wash,
            active: input.active,
        };
        let held = cx.has_active_drag().then(|| input.terminal_drag.map(|(id, _)| id)).flatten();
        let lines = Lines {
            input: &input,
            listing: &listing,
            keys: &keys,
            frame: &frame,
            cards: &cards,
            mark: mark.as_ref(),
            held,
            ui,
        };
        let mut items: Vec<AnyElement> =
            (0..listing.entries.len()).map(|at| self.list_line(at, &lines, window, cx)).collect();
        // The rows that have left, still drawn until their motion is over.
        for (at, line) in self.leaving_lines(&keys, &frame, &group_wash, window, cx) {
            items.insert(at, line);
        }
        // After the person's own terminals: the welcome's example group, which is none of them.
        items.extend(self.render_demo(&frame, cx));
        self.motion.remember(&input, listing);

        div()
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .children(glass.as_ref().map(|glass| clip_probe(glass, LIST_CLIP)))
            .child({
                let list = div()
                    .id("session-list")
                    .role(Role::Tree)
                    .aria_label("Terminals")
                    .size_full()
                    .flex()
                    .flex_col()
                    .pt_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .children(items);
                let list = self.with_list_drop(list.child(self.list_tail(cx)), cx);
                // It holds the keyboard for the sidebar.
                self.area_container(list, cx)
            })
            // Beside the list, not in it: a child of the scrolled list would move with its rows.
            .child(self.list_edges(ui, cx))
            .into_any_element()
    }

    /// Line `at` of the listing: a group's head or a terminal's row, in its card, with what it answers to
    /// and the motion it owes.
    fn list_line(
        &mut self,
        at: usize,
        lines: &Lines,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Lines { input, listing, keys, frame, cards, mark, held, ui } = *lines;
        let infos = &input.rows;
        let line = match listing.entries[at] {
            Entry::Band { band, position, of } => {
                self.render_group_band(listing, infos, band, (position, of), frame, window, cx)
            }
            Entry::Row { session, .. } => {
                match Place::of(listing, infos, at, input.active, frame.metrics.cards) {
                    Some(place) => self.render_row(&infos[session], &place, frame, window, cx),
                    None => div().into_any_element(),
                }
            }
        };
        // Where a dragged thing would land, as a line over this one.
        let line = match self.mark_element(mark, &keys[at], ui) {
            Some(mark) => div().relative().child(line).child(mark).into_any_element(),
            None => line,
        };
        let masked = self.motion.opens_by_layout(&keys[at]);
        let segment = cards.line(at, line, masked);
        let gap = listing.card_edges(at).1.then_some(frame.metrics.card_gap);
        let line = match listing.entries[at] {
            Entry::Row { session, .. } => {
                let id = infos[session].id;
                let group = listing.card(at).and_then(|card| match card {
                    Card::Group(band) => Some(listing.bands[band].group),
                    Card::Loose(_) | Card::Split(_) => None,
                });
                self.with_row_drop(segment.id(("row-line", id as u64)), id, group, gap, cx).into_any_element()
            }
            Entry::Band { band, .. } => {
                let group = listing.bands[band].group;
                self.with_head_drop(segment.id(("head-line", group as u64)), group, gap, cx)
                    .into_any_element()
            }
        };
        let is_held = matches!(&keys[at], ItemKey::Row(id) if Some(*id) == held);
        self.with_list_motion(keys[at].clone(), line, is_held)
    }

    /// What lies over the list's edges (`panel_list::scroll_edges`, as the panel's lists).
    fn list_edges(&self, ui: UiColors, cx: &App) -> AnyElement {
        let scrollbar = Scrollbar::vertical(&self.scroll).id("sidebar-scrollbar");
        crate::style::panel_list::scroll_edges(
            ui,
            crate::style::panel_list::is_scrolled(&self.scroll),
            scrollbar,
            cx,
        )
    }
}
