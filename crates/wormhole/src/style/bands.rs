//! The band: one header for a section of rows in the right panel. [`BAND_H`] tall, full width and
//! square, filled with `band` between two hairlines; the chevron in the lead column, the title at
//! x 40 in `text_label`, and on the right, ending 16 in, the count and the +.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::buttons::icon_button;
use super::controls::disclosure;
use super::labels::count_badge;
use super::rows::lead;
use super::scale::{BAND_H, SIDEBAR_INSET, TextRole};
use super::section::{ClickHandler, Section};
use crate::settings::UiColors;

/// What a band heads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandKind {
    /// A section of the right panel (Next steps, Notes, Changes …): the count whenever
    /// there is one, and the + always.
    Section,
}

/// The parts of a [`band`].
pub struct BandParts {
    pub kind: BandKind,
    /// Open, folded, or empty (a section with nothing in it has no chevron, and its title
    /// starts at x 16).
    pub section: Section,
    pub title: SharedString,
    /// After the title: a mark such as the pin.
    pub detail: Option<AnyElement>,
    /// The count badge's text (`style::count_label`); `None` leaves it out.
    pub count: Option<SharedString>,
    /// Before the count: a dot (a `style::status_glyph`) while something folded away needs you.
    pub waiting: Option<AnyElement>,
    /// The + at the right edge: its tooltip and what it does. Its click leaves the band's
    /// own state alone.
    pub add: Option<(&'static str, ClickHandler)>,
    /// Folds and unfolds the band; a band with nothing to fold ignores it.
    pub on_toggle: Option<ClickHandler>,
    /// A terminal is being dragged over it: `state.highlight` between primary hairlines.
    pub drop_target: bool,
    /// Stuck to the top of a scrolling list: the opaque `band.solid`, so rows scrolling
    /// under it don't show through.
    pub sticky: bool,
    /// The band right above already draws the hairline between them, so two folded bands
    /// in a row share one.
    pub joined: bool,
}

/// See [`BandParts`]. `id` names the band and its + (a folder's path, a section's name).
#[track_caller]
pub fn band(id: impl Into<ElementId>, ui: UiColors, parts: BandParts) -> Stateful<Div> {
    band_with(id.into(), ui, parts, FolderLook::default(), Vec::new())
}

/// The head of a card in the right panel: a section's [`band`] drawn as the card's first line
/// ([`BAND_H`] tall, the chevron and title 8 in, rounded under the pointer). `trailing` sits
/// before the count and the + (the Changes card's compare picker and refresh button).
#[track_caller]
pub fn section_card(
    id: impl Into<ElementId>,
    ui: UiColors,
    parts: BandParts,
    trailing: Vec<AnyElement>,
) -> Stateful<Div> {
    let look = FolderLook { card: true, ..FolderLook::default() };
    band_with(id.into(), ui, BandParts { kind: BandKind::Section, ..parts }, look, trailing)
}

/// The tint of a coloured folder's band: the folder's colour, and how strong it is at the band's
/// left edge. It fades out the way the title row's project tint does ([`super::tint_layers`]),
/// so the two read as one mark; `peak` is what keeps the band's second line readable on it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandTint {
    pub color: Hsla,
    pub peak: f32,
}

/// What sets a sidebar folder band apart from a plain [`band`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FolderLook {
    /// ⌘ is held: the count drops to 40 %, so the terminals' ⌘N keycaps read as one column
    /// at x 334.
    pub cmd_held: bool,
    /// The folder has a colour (right-click the band): the band is tinted from the left, or in a
    /// card its name sits on a pill of that colour.
    pub tint: Option<BandTint>,
    /// It heads a card (`style::card_segment`): no fill or hairlines of its own (the card has
    /// them), its content 8 in from the card's padding, and a rounded hover.
    pub card: bool,
}

/// A sidebar group's head ([`BandKind::Folder`]) with its [`FolderLook`]: one line, [`BAND_H`].
#[track_caller]
pub fn folder_band(
    id: impl Into<ElementId>,
    ui: UiColors,
    parts: BandParts,
    look: FolderLook,
) -> Stateful<Div> {
    band_with(id.into(), ui, BandParts { kind: BandKind::Folder, ..parts }, look, Vec::new())
}

/// How strong a folder's count stays while ⌘ is held.
const HELD_COUNT_OPACITY: f32 = 0.4;

#[track_caller]
fn band_with(
    id: ElementId,
    ui: UiColors,
    parts: BandParts,
    look: FolderLook,
    trailing: Vec<AnyElement>,
) -> Stateful<Div> {
    let dim_count = look.cmd_held;
    let BandParts {
        kind,
        section,
        title,
        detail,
        count,
        waiting,
        add,
        on_toggle,
        drop_target,
        sticky,
        joined,
    } = parts;
    let group = SharedString::from(format!("band-{id}"));
    let inset = SIDEBAR_INSET;
    let (fill, hairline) = match (drop_target, sticky) {
        (true, _) => (ui.highlight, ui.primary),
        (false, true) => (ui.band_solid, ui.border),
        (false, false) => (ui.band, ui.border),
    };
    let foldable = section != Section::Empty;
    let count = count.map(|count| div().flex_none().child(count_badge(ui, count)));
    let add = add.map(|(tooltip, on_add)| {
        div().flex_none().child(
            icon_button((id.clone(), "add"), ui, IconName::Plus, tooltip).in_band().flush_right().on_click(
                move |event, window, cx| {
                    // The band underneath would otherwise fold too.
                    cx.stop_propagation();
                    on_add(event, window, cx);
                },
            ),
        )
    });
    // A section's band is a button that folds its rows; its title is the name until the
    // caller says it with the count in words.
    let BandKind::Section = kind;
    let spoken = title.clone();
    // The first line: the chevron, the title with what follows it, and the count or the +.
    let first = div()
        .flex()
        .items_center()
        .w_full()
        .h(px(BAND_H))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .pl(px(inset))
                .when(foldable, |d| d.child(lead(disclosure(ui, section.is_collapsed()))))
                .child(div().min_w_0().truncate().text_label().text_color(ui.text).child(title))
                .children(detail),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .justify_end()
                .gap_1()
                .pl_2()
                .pr(px(inset))
                // Room for the + where there is no count to stand in for.
                .when(swap, |d| d.relative().min_w(px(SWAP_SLOT - SIDEBAR_INSET + inset)))
                .children(trailing)
                .children(waiting.filter(|_| section.is_collapsed()))
                .children(count)
                .children(add),
        );
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(spoken)
        .when(foldable, |d| d.aria_expanded(!section.is_collapsed()))
        .group(group.clone())
        .relative()
        .flex()
        .flex_col()
        .justify_center()
        .w_full()
        .h(px(BAND_H))
        .flex_none()
        .bg(fill)
        .when(!drop_target, |d| d.hover(|s| s.bg(ui.band_hover)))
        .when(!joined, |d| d.child(hairline_at(Edge::Top, ui, hairline)))
        .child(hairline_at(Edge::Bottom, ui, hairline))
        .when_some(on_toggle.filter(|_| foldable), |d, on_toggle| d.cursor_pointer().on_click(on_toggle))
        .child(first)
}

#[derive(Clone, Copy)]
enum Edge {
    Top,
    Bottom,
}

/// A band's hairline (the theme's border width), laid over its edge so it takes no height:
/// rows and bands stack at exactly 32 / 44 / 28.
#[track_caller]
fn hairline_at(edge: Edge, ui: UiColors, color: Hsla) -> Div {
    div()
        .absolute()
        .left_0()
        .right_0()
        .map(|d| match edge {
            Edge::Top => d.top_0(),
            Edge::Bottom => d.bottom_0(),
        })
        .h(ui.border_width)
        .bg(color)
}
