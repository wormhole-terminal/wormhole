//! The band: one header for a group of rows in both panels (folder bands, section
//! bands). [`BAND_H`] tall, full width and square, filled with `band` between two
//! hairlines; the chevron in the lead column, the title at x 40 in `text_label`, and on
//! the right, ending 16 in, the count and the +. A sidebar folder band may have a second line
//! under its title ([`FolderMeta`]: where the folder is, what git says of it), which makes it
//! [`FOLDER_BAND_H`] tall.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::buttons::icon_button;
use super::controls::disclosure;
use super::labels::count_badge;
use super::rows::lead;
use super::scale::{
    BAND_H, FOLDER_BAND_H, FOLDER_NAME_MAX, ITEM_LEAD_W, META_LINE, NAME_LINE, SIDEBAR_INSET, TextRole,
};
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
    band_with(id.into(), ui, parts, FolderLook::default(), None)
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
    /// The folder has a colour (right-click the band): the band is tinted from the left.
    pub tint: Option<BandTint>,
}

/// The second line of a sidebar folder band, under its name and at its x 40: the folder's path,
/// a gap, the "±N" of its uncommitted changes, a gap, its branch (a [`branch_tag`]). It is a
/// caption in `text.muted`. The line gives way from the left: the path shortens first, with an
/// ellipsis, then the branch tag down to its least; the "±N" never does (core's
/// `band_line_widths` works out how much, the caller says it here).
///
/// [`branch_tag`]: super::labels::branch_tag
pub struct FolderMeta {
    /// `~/Projects/app`: the folder, spelt as `display_path` does.
    pub path: SharedString,
    /// The width the path is cut to when the line is too narrow for all of it; `Some(0.0)` leaves
    /// it out. `None` draws it whole.
    pub path_w: Option<f32>,
    /// The uncommitted changes, when there are any to show.
    pub changes: Option<usize>,
    /// The branch: a `style::branch_tag`, with its tooltip and, on a narrow line, the `max_w` it
    /// is cut to.
    pub branch: Option<AnyElement>,
    /// After the branch: its pull request, a `style::pull_request_mark`.
    pub pull_request: Option<AnyElement>,
}

/// A sidebar folder band ([`BandKind::Folder`]) with its [`FolderLook`]. With a [`FolderMeta`] it
/// has a second line and is [`FOLDER_BAND_H`] tall, without one (the "No folder" band, the compact
/// density) it is one line, [`BAND_H`].
#[track_caller]
pub fn folder_band(
    id: impl Into<ElementId>,
    ui: UiColors,
    parts: BandParts,
    look: FolderLook,
    meta: Option<FolderMeta>,
) -> Stateful<Div> {
    band_with(id.into(), ui, BandParts { kind: BandKind::Folder, ..parts }, look, meta)
}

/// How strong a folder's count stays while ⌘ is held.
const HELD_COUNT_OPACITY: f32 = 0.4;

#[track_caller]
fn band_with(
    id: ElementId,
    ui: UiColors,
    parts: BandParts,
    look: FolderLook,
    meta: Option<FolderMeta>,
) -> Stateful<Div> {
    let two_lines = meta.is_some();
    let dim_count = look.cmd_held;
    let BandParts { kind, section, title, detail, count, add, on_toggle, drop_target, sticky, joined } =
        parts;
    let group = SharedString::from(format!("band-{id}"));
    let (fill, hairline) = match (drop_target, sticky) {
        (true, _) => (ui.highlight, ui.primary),
        (false, true) => (ui.band_solid, ui.border),
        (false, false) => (ui.band, ui.border),
    };
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
                    .right(px(SIDEBAR_INSET))
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
        .map(|d| if two_lines { d.h(rems(NAME_LINE)) } else { d.h(px(BAND_H)) })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .pl_4()
                .when(foldable, |d| d.child(lead(disclosure(ui, section.is_collapsed()))))
                .child(
                    div()
                        // With a second line the first has only marks after the name, so the name
                        // takes what they leave; without one the branch chip needs its share.
                        .when(!two_lines, |d| {
                            d.flex_none()
                                .when(kind == BandKind::Folder, |d| d.max_w(relative(FOLDER_NAME_MAX)))
                        })
                        .when(two_lines, |d| d.min_w_0())
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
                .pr_4()
                // Room for the + where there is no count to stand in for.
                .when(swap, |d| d.relative().min_w(px(SWAP_SLOT)))
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
        .h(px(if two_lines { FOLDER_BAND_H } else { BAND_H }))
        .when(two_lines, |d| d.py_1())
        .flex_none()
        .bg(fill)
        .when(!drop_target, |d| d.hover(|s| s.bg(ui.band_hover)))
        // Under the content and the hairlines, which are painted later.
        .children(look.tint.into_iter().flat_map(|tint| tint_layers(tint.color, tint.peak)))
        .when(!joined, |d| d.child(hairline_at(Edge::Top, ui, hairline)))
        .child(hairline_at(Edge::Bottom, ui, hairline))
        .when_some(on_toggle.filter(|_| foldable), |d, on_toggle| d.cursor_pointer().on_click(on_toggle))
        .child(first)
        .children(meta.map(|meta| meta_line(ui, meta)))
}

/// A folder band's second line: its path, "±N", branch and pull request, from the title's x.
#[track_caller]
fn meta_line(ui: UiColors, meta: FolderMeta) -> Div {
    let FolderMeta { path, path_w, changes, branch, pull_request } = meta;
    div()
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .h(rems(META_LINE))
        .pl(px(SIDEBAR_INSET + ITEM_LEAD_W + 8.0))
        .pr_4()
        .min_w_0()
        .text_caption()
        .text_color(ui.muted)
        .when(path_w != Some(0.0), |d| {
            d.child(div().flex_none().when_some(path_w, |d, w| d.w(px(w))).truncate().child(path))
        })
        .children(
            changes.map(|n| div().flex_none().tabular_nums().whitespace_nowrap().child(format!("±{n}"))),
        )
        .children(branch)
        .children(pull_request)
}

/// A folder band's right side when the + takes the count's place: the + (its face less the room
/// it is pulled right by) and the padding round it.
const SWAP_SLOT: f32 = 8.0 + 20.0 + SIDEBAR_INSET;

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
