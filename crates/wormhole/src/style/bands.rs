//! The band: one header for a group of rows in both panels (folder bands, section
//! bands). [`BAND_H`] tall, full width and square, filled with `band` between two
//! hairlines; the chevron in the lead column, the title at x 40 in `text_label`, and on
//! the right, ending 16 in, the count and the +.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::buttons::icon_button;
use super::controls::disclosure;
use super::labels::count_badge;
use super::rows::lead;
use super::scale::{BAND_H, CARD_ROW_RADIUS, FOLDER_NAME_MAX, SIDEBAR_INSET, TextRole};
use super::section::{ClickHandler, Section};
use super::tint::tint_layers;
use crate::settings::UiColors;

/// What a band heads, which decides what it shows on the right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandKind {
    /// A folder of terminals in the sidebar: its count only while it is folded, and the
    /// + (a new terminal there) in the count's place while the pointer is over it.
    Folder,
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
    /// After the title: a folder's `style::branch_chip` and ±N.
    pub detail: Option<AnyElement>,
    /// The count badge's text (`style::count_label`); `None` leaves it out.
    pub count: Option<SharedString>,
    /// Before the count of a folded folder: the needs-you dot (a `style::status_glyph`) while a
    /// terminal folded away in it needs input.
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
    /// "No folder" (terminals whose folder isn't known yet): its title is muted.
    pub unknown: bool,
    /// The folder has a colour (right-click the band): the band is tinted from the left, or in a
    /// card its name sits on a pill of that colour.
    pub tint: Option<BandTint>,
    /// It heads a card (`style::card_segment`): no fill or hairlines of its own (the card has
    /// them), its content 8 in from the card's padding, and a rounded hover.
    pub card: bool,
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
    let card = look.card;
    let inset = if card { CARD_INSET } else { SIDEBAR_INSET };
    let (fill, hairline) = match (drop_target, sticky) {
        (true, _) => (ui.highlight, ui.primary),
        (false, true) => (ui.band_solid, ui.border),
        (false, false) => (ui.band, ui.border),
    };
    // In a card, the colour is a pill under the name rather than a tint across the band.
    let pill = look.tint.filter(|_| card);
    let foldable = section != Section::Empty;
    // A folder shows its count only while folded, and gives its place to the + under the pointer.
    let count = count.filter(|_| kind == BandKind::Section || section.is_collapsed());
    let swap = kind == BandKind::Folder && add.is_some();
    // The count and the + trade places under the pointer by their visibility, never their display:
    // a band that moves under a resting pointer (a row above it changed height) is hovered at paint
    // but wasn't at prepaint, and GPUI won't paint a child it skipped there (an element that is
    // `display: none` during prepaint).
    let count = count.map(|count| {
        div()
            .flex_none()
            .when(dim_count, |d| d.opacity(HELD_COUNT_OPACITY))
            .when(swap, |d| d.group_hover(group.clone(), |s| s.invisible()))
            .child(count_badge(ui, count))
    });
    let add = add.map(|(tooltip, on_add)| {
        div()
            .flex_none()
            .when(swap, |d| {
                d.absolute()
                    .top_0()
                    .bottom_0()
                    .right(px(inset))
                    .flex()
                    .items_center()
                    .invisible()
                    .group_hover(group.clone(), |s| s.visible())
            })
            .child(
                icon_button((id.clone(), "add"), ui, IconName::Plus, tooltip)
                    .in_band()
                    .flush_right()
                    .on_click(move |event, window, cx| {
                        // The band underneath would otherwise fold too.
                        cx.stop_propagation();
                        on_add(event, window, cx);
                    }),
            )
    });
    // A section's band is a button that folds its rows; its title is the name until the
    // caller says it with the count in words. A folder's band is the sidebar's tree item.
    let section_band = kind == BandKind::Section;
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
                .child(
                    div()
                        .when_some(pill, |d, tint| {
                            d.flex()
                                .items_center()
                                .gap_1()
                                .px_2()
                                .py(px(PILL_PAD_Y))
                                .rounded(ui.radius_chip)
                                .bg(tint.color.opacity(PILL_FILL))
                                .child(div().flex_none().size(px(PILL_DOT)).rounded_full().bg(tint.color))
                        })
                        .flex_none()
                        .when(kind == BandKind::Folder, |d| d.max_w(relative(FOLDER_NAME_MAX)))
                        .truncate()
                        .text_label()
                        .text_color(if look.unknown { ui.muted } else { ui.text })
                        .child(title),
                )
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
        .when(section_band, |d| {
            d.role(Role::Button)
                .aria_label(spoken)
                .when(foldable, |d| d.aria_expanded(!section.is_collapsed()))
        })
        .group(group.clone())
        .relative()
        .flex()
        .flex_col()
        .justify_center()
        .w_full()
        .h(px(BAND_H))
        .flex_none()
        .when(!card, |d| {
            d.bg(fill)
                .when(!drop_target, |d| d.hover(|s| s.bg(ui.band_hover)))
                // Under the content and the hairlines, which are painted later.
                .children(look.tint.into_iter().flat_map(|tint| tint_layers(tint.color, tint.peak)))
                .when(!joined, |d| d.child(hairline_at(Edge::Top, ui, hairline)))
                .child(hairline_at(Edge::Bottom, ui, hairline))
        })
        .when(card, |d| {
            d.rounded(px(CARD_ROW_RADIUS))
                .when(sticky, |d| d.bg(ui.band_solid))
                .when(drop_target, |d| d.bg(ui.highlight))
                .when(!drop_target, |d| d.hover(|s| s.bg(ui.hover)))
        })
        .when_some(on_toggle.filter(|_| foldable), |d, on_toggle| d.cursor_pointer().on_click(on_toggle))
        .child(first)
}

/// A folder band's right side when the + takes the count's place: the + (its face less the room
/// it is pulled right by) and the padding round it.
const SWAP_SLOT: f32 = 8.0 + 20.0 + SIDEBAR_INSET;
/// A band's content in a card starts this far in from the card's padding.
pub const CARD_INSET: f32 = 8.0;
/// The colour pill under a band's name in a card: its fill's strength, its dot, its padding.
const PILL_FILL: f32 = 0.2;
const PILL_DOT: f32 = 7.0;
const PILL_PAD_Y: f32 = 1.0;

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
