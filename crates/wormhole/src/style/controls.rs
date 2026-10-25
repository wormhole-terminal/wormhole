//! Controls and small chrome: the inset row, tabs and chips, segmented switches, key caps
//! and color swatches.

use gpui_kit::assets::IconName;
use gpui_kit::base::{Transition, transition};
use gpui_kit::component::{Icon, Sizable, Size};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::buttons::tooltip_delay;
use super::labels::{count_badge, selected_count_badge};
use super::motion::{EASE_IN_OUT, MOTION_BASE};
use super::scale::{
    CONTROL_H, CONTROL_H_SM, Edges, KEYCAP_H, KEYCAP_MIN_W, ROW_H, SWATCH, SWATCH_SMALL, TextRole,
};
use crate::settings::UiColors;

/// A clickable list row: body text, hover fill, selected fill, at least [`ROW_H`] tall,
/// with the control corners: the row of a menu or a picker inside a card. A list that
/// runs edge to edge uses the square rows of `style::flush_row` and its kin instead.
/// Add the content with `.child(…)`; rows that want more room set their own vertical
/// padding. A selected row takes the primary's tint (`state.selected`) and its text
/// turns `selected_text`; secondary parts of it use `ui.selected_muted`, and colored
/// marks their `*_text` variants, which are judged on that tint.
#[track_caller]
pub fn row(id: impl Into<ElementId>, ui: UiColors, selected: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .min_h(px(ROW_H))
        .rounded(ui.radius)
        .text_body()
        .cursor_pointer()
        .when(selected, |d| d.bg(ui.selected).text_color(ui.selected_text))
        // The hover fill is a neutral overlay; on the selected row it would swap the
        // primary's tint for grey under text that is meant for the tint.
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
}

/// A tab or a choice chip: filled with the primary when active (`chip.selected`, text
/// `on_selected`), a hover fill otherwise. `compact`
/// is the borderless tab of a tab bar (the right panel's, the terminal's; caption
/// text, [`CONTROL_H_SM`]); without it, the outlined chip for choosing among a few
/// options in a form ([`CONTROL_H`]), which reads like the selects around it.
#[track_caller]
pub fn pill(id: impl Into<ElementId>, ui: UiColors, active: bool, compact: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .h(px(if compact { CONTROL_H_SM } else { CONTROL_H }))
        .rounded(ui.radius_chip)
        .cursor_pointer()
        .when(compact, |d| d.text_caption())
        .when(!compact, |d| d.text_body())
        .text_color(if active { ui.on_selected } else { ui.muted })
        .when(!compact, |d| d.outlined(ui, if active { ui.selected_chip } else { ui.border }))
        .when(active, |d| d.bg(ui.selected_chip))
        // Hovering the active one keeps its fill rather than trading it for the lighter hover.
        .hover(move |d| if active { d } else { d.bg(ui.hover).text_color(ui.text) })
}

/// Side of an [`icon_tab`]: a square as tall as a form control, like the icon buttons
/// beside it in the panel's tab bar.
pub const ICON_TAB: f32 = CONTROL_H;

/// A tab shown by its icon alone (the right panel's Notes / Changes / History /
/// Project): an [`ICON_TAB`] square with a `Medium` icon and the control corners, like an
/// icon button. The active one takes the selected tint (`state.selected`) with its
/// icon in `selected_text`; the others are muted, and under the pointer take `band.hover` with
/// the icon in `text.primary`, `state.active` while pressed. `count` puts a
/// [`count_badge`] beside the icon (the tab widens for it). A plain action in the
/// same bar (stash prompt) is an `icon_tab` that is never active, so every button in
/// the bar has one size. The tooltip names the tab, since nothing else does.
#[track_caller]
pub fn icon_tab(
    id: impl Into<ElementId>,
    ui: UiColors,
    icon: IconName,
    tooltip: impl Into<SharedString>,
    active: bool,
    count: Option<usize>,
) -> Stateful<Div> {
    let tooltip: SharedString = tooltip.into();
    tooltip_delay(
        div()
            .id(id)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .gap_1()
            .h(px(ICON_TAB))
            .min_w(px(ICON_TAB))
            .when(count.is_some(), |d| d.px_2())
            .rounded(ui.radius)
            .cursor_pointer()
            .text_color(if active { ui.selected_text } else { ui.muted })
            .when(active, |d| d.bg(ui.selected))
            .hover(move |d| if active { d } else { d.bg(ui.band_hover).text_color(ui.text) })
            .active(move |d| if active { d } else { d.bg(ui.active) })
            .tooltip(move |window, cx| {
                gpui_kit::component::tooltip::Tooltip::new(tooltip.clone()).build(window, cx)
            }),
    )
    // The icon takes the tab's text color.
    .child(Icon::new(icon).with_size(Size::Medium))
    .children(count.map(|n| {
        let n = super::scale::count_label(n);
        if active { selected_count_badge(ui, n) } else { count_badge(ui, n) }
    }))
}

/// The widest a [`title_tab`] grows; more tabs than fit make each of them shrink.
pub const TITLE_TAB_MAX_W: f32 = 220.0;

/// A terminal's tab in its title row: [`CONTROL_H`] tall with the control
/// corners, a 16 pt status slot and the label in 13 pt. The active tab is quiet: a raised
/// neutral fill (`tab.active.background`, `surface.elevated` by default), its label in `text.primary` 500, since the primary's
/// tint already marks the selected terminal in the sidebar. The others are muted with a hover
/// fill; one that needs input wears the attention tint and ring until it is answered. Add the
/// status slot, the label and the × with `.child(…)`.
#[track_caller]
pub fn title_tab(id: impl Into<ElementId>, ui: UiColors, active: bool, needs_you: bool) -> Stateful<Div> {
    let quiet = !active && !needs_you;
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .h(px(CONTROL_H))
        .max_w(px(TITLE_TAB_MAX_W))
        .px_2()
        .rounded(ui.radius)
        .text_label()
        .font_weight(if active { FontWeight::MEDIUM } else { FontWeight::NORMAL })
        .text_color(if active { ui.text } else { ui.muted })
        .cursor_pointer()
        .when(active, |d| d.bg(ui.tab_active))
        .when(!active && needs_you, |d| d.bg(ui.attention_tint).outlined(ui, ui.attention_ring))
        .hover(move |d| if quiet { d.bg(ui.hover).text_color(ui.text) } else { d })
}

/// A bar of controls along the top of a panel or page section.
#[track_caller]
pub fn toolbar() -> Div {
    div().flex().items_center().gap_2().px_2().min_h(px(CONTROL_H))
}

/// The chevron that shows whether a section is open (12 pt, muted): it turns a quarter, in
/// [`MOTION_BASE`] and in-out, as the section opens or folds, and swaps at once under
/// Reduce Motion. Build it with [`disclosure`].
#[derive(IntoElement)]
pub struct Disclosure {
    ui: UiColors,
    collapsed: bool,
}

/// See [`Disclosure`].
pub fn disclosure(ui: UiColors, collapsed: bool) -> Disclosure {
    Disclosure { ui, collapsed }
}

impl RenderOnce for Disclosure {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = if self.collapsed { 0.0 } else { QUARTER_TURN };
        // Keyed by where it is drawn; the first value is taken as it is, so a chevron only turns
        // on a change, never as it first appears.
        let policy = Transition::new(MOTION_BASE).ease(EASE_IN_OUT.easing());
        let angle = transition("disclosure", open, policy, window, cx);
        Icon::new(IconName::ChevronRight).xsmall().text_color(self.ui.muted).rotate(radians(angle))
    }
}

/// A chevron pointing right turned to point down.
const QUARTER_TURN: f32 = std::f32::consts::FRAC_PI_2;

/// A shortcut drawn as a key cap: one for every hint (the sidebar's ⌘1–9, the palette,
/// the shortcut sheet, settings). `surface.elevated` with a `border.strong`
/// outline and the label in `text.primary`, [`KEYCAP_H`] tall and at least
/// [`KEYCAP_MIN_W`] wide, so ⌘1 and esc line up.
#[track_caller]
pub fn keycap(ui: UiColors, text: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .h(px(KEYCAP_H))
        .min_w(px(KEYCAP_MIN_W))
        .px_1()
        .rounded(ui.radius_chip)
        .outlined(ui, ui.border_strong)
        .bg(ui.elevated)
        .text_caption()
        .font_weight(FontWeight::MEDIUM)
        .tabular_nums()
        .text_color(ui.text)
        .whitespace_nowrap()
        .child(text.into())
}

/// How far a segmented control's track reaches around its segments.
const SEGMENT_INSET: f32 = 2.0;
/// The gap between a [`form_segment`]'s icon and its label.
const SEGMENT_ICON_GAP: f32 = 6.0;

/// A switch between two or three views of the same thing in a panel ("Split |
/// Unified"): the [`segment`]s sit in one rounded track, so they read as one control, and
/// the active one is filled like a selected chip. Add the segments with `.child(…)`.
/// In a form, where it stands beside selects and buttons, it is a [`segmented_form`].
#[track_caller]
pub fn segmented(ui: UiColors) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .p(px(SEGMENT_INSET))
        // Concentric with the segments inside it.
        .rounded(ui.radius_chip + px(SEGMENT_INSET))
        .bg(ui.hover)
}

/// One option of a [`segmented`] control; add `.on_click(…)`.
#[track_caller]
pub fn segment(
    id: impl Into<ElementId>,
    ui: UiColors,
    label: impl Into<SharedString>,
    active: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .px_2()
        .h(px(CONTROL_H_SM - 2.0 * SEGMENT_INSET))
        .rounded(ui.radius_chip)
        .text_caption()
        .cursor_pointer()
        .text_color(if active { ui.on_selected } else { ui.muted })
        .when(active, |d| d.bg(ui.selected_chip))
        .hover(move |d| if active { d } else { d.text_color(ui.text) })
        .child(label.into())
}

/// The segmented control of a settings row: as tall as the selects and buttons
/// around it ([`CONTROL_H`]), the track in `state.hover` with the control corners, the
/// chosen [`form_segment`] the solid primary. Add the segments with `.child(…)`.
#[track_caller]
pub fn segmented_form(ui: UiColors) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .h(px(CONTROL_H))
        .p(px(SEGMENT_INSET))
        .rounded(ui.radius)
        .bg(ui.hover)
}

/// One option of a [`segmented_form`], with an optional icon before its label ("Dark",
/// "Light", "Match system"); add `.on_click(…)`.
#[track_caller]
pub fn form_segment(
    id: impl Into<ElementId>,
    ui: UiColors,
    icon: Option<IconName>,
    label: impl Into<SharedString>,
    active: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(SEGMENT_ICON_GAP))
        .px_2()
        .h(px(CONTROL_H - 2.0 * SEGMENT_INSET))
        .rounded(ui.radius_chip)
        .text_label()
        .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
        .cursor_pointer()
        .text_color(if active { ui.on_selected } else { ui.muted })
        .when(active, |d| d.bg(ui.selected_chip))
        .hover(move |d| if active { d } else { d.text_color(ui.text) })
        .children(icon.map(|icon| Icon::new(icon).with_size(px(super::scale::GLYPH))))
        .child(label.into())
}

/// A color to pick (the primary in Settings → Appearance): a [`SWATCH`] square in the
/// small corners, and when it is the chosen one a `text.primary` ring with a gap, so the
/// ring reads on any color. Add `.on_click(…)`.
#[track_caller]
pub fn swatch(id: impl Into<ElementId>, ui: UiColors, color: Hsla, chosen: bool) -> Stateful<Div> {
    sized_swatch(id, ui, color, chosen, SWATCH)
}

/// A [`swatch`] at [`SWATCH_SMALL`], for a strip as low as a theme card's name: a theme
/// family's versions there. Add `.on_click(…)`.
#[track_caller]
pub fn swatch_small(id: impl Into<ElementId>, ui: UiColors, color: Hsla, chosen: bool) -> Stateful<Div> {
    sized_swatch(id, ui, color, chosen, SWATCH_SMALL)
}

#[track_caller]
fn sized_swatch(
    id: impl Into<ElementId>,
    ui: UiColors,
    color: Hsla,
    chosen: bool,
    size: f32,
) -> Stateful<Div> {
    let ring = UiColors { border_width: px(SWATCH_RING), ..ui };
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(size + 2.0 * (SWATCH_RING + SWATCH_GAP)))
        .rounded(ui.radius_sm + px(SWATCH_RING + SWATCH_GAP))
        .cursor_pointer()
        .when(chosen, |d| d.outlined(ring, ui.text))
        .child(div().size(px(size)).rounded(ui.radius_sm).bg(color).outlined(ui, ui.border))
}

/// The ring round a chosen [`swatch`] and the gap inside it.
const SWATCH_RING: f32 = 2.0;
const SWATCH_GAP: f32 = 2.0;

/// Where a [`focus_ring`] sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RingPlacement {
    /// Round a control, `FOCUS_RING_OFFSET` outside it with matching corners.
    Outside,
    /// Inside a row's edges, so rows packed edge to edge don't cover each other's ring.
    Inside,
}

/// The keyboard focus ring: `metrics.focus_ring_width` in `border.focus`, laid over
/// a `.relative()` element whose corners are `radius`. Show it only for keyboard focus,
/// like `:focus-visible`: `.when(focus.is_focused(window) && window.last_input_was_keyboard(),
/// |d| d.child(style::focus_ring(…)))`.
#[track_caller]
pub fn focus_ring(ui: UiColors, placement: RingPlacement, radius: Pixels) -> Div {
    let out = match placement {
        RingPlacement::Outside => px(super::scale::FOCUS_RING_OFFSET) + ui.focus_ring_width,
        RingPlacement::Inside => px(0.),
    };
    let ring = UiColors { border_width: ui.focus_ring_width, ..ui };
    div()
        .absolute()
        .top(-out)
        .left(-out)
        .right(-out)
        .bottom(-out)
        .rounded(radius + out)
        .outlined(ring, ui.focus)
}

/// How far a [`uses_outline`] stands off what it marks, and its line: GPUI draws a dash twice
/// the line's width, so a hairline would read as dots.
const USES_OUTLINE_OFFSET: f32 = 2.0;
const USES_OUTLINE_W: f32 = 1.5;

/// The Theme Studio's mark on what the selected token is drawn on: a dashed line in the
/// primary, laid over a `.relative()` element a little outside it, so an edit shows where it
/// lands. Dashed, so it never reads as the solid focus ring.
#[track_caller]
pub fn uses_outline(ui: UiColors, radius: Pixels) -> Div {
    let out = px(USES_OUTLINE_OFFSET);
    let line = UiColors { border_width: px(USES_OUTLINE_W), ..ui };
    div()
        .absolute()
        .top(-out)
        .left(-out)
        .right(-out)
        .bottom(-out)
        .rounded(radius + out)
        .outlined(line, ui.primary)
        .border_dashed()
}
