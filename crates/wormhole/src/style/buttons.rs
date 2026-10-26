//! Icon buttons and the other small things you click: the 28 × 28 icon button, the × that
//! closes what it sits on, a hover action, and the hit area that grows a small drawing to
//! [`HIT_MIN`].

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_kit::component::{Icon, Selectable, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::rc::Rc;

use super::icon_motion::{IconMotion, MenuMark, StripFill};
use super::motion::TOOLTIP_DELAY;
use super::scale::{CONTROL_H_SM, GLYPH, HIT_MIN, ICON_SIZE, MARK_DOT, TITLE_GLYPH};
use crate::settings::UiColors;

/// The face of a band's +: smaller than a button's, so it sits inside the band's 32 pt
/// height with room above and below.
const BAND_BUTTON: f32 = 24.0;
/// The chevron of a menu beside a button.
const NARROW_GLYPH: f32 = 12.0;
/// The face of a row's hover action (send, delete): it fits a 28 pt row with 3 to spare.
pub(super) const ROW_BUTTON: f32 = 22.0;
/// How far a row's hover actions reach past its 16 pt inset, so their faces end 12 in (their
/// glyphs on x 344): the room a 28 pt hit area leaves round a [`ROW_BUTTON`] face, and 4.
pub(super) const ROW_ACTIONS_OVERHANG: f32 = (HIT_MIN - ROW_BUTTON) / 2.0 + 4.0;

/// A row's hover actions (`style::item_action`) at its right end: hidden and taking no room
/// at rest, shown under the pointer (or while `shown`, an item being edited), when the text
/// before them gives way. `group` is the row's hover group.
///
/// They collapse to no width rather than leaving the layout (`display: none`): GPUI works out a
/// group's hover anew for each phase of a frame, and a child hidden while it was prepainted but
/// shown when it is painted panics. A width is settled once, at layout, from the hover GPUI keeps
/// in the element's state; that is why it needs an `id`. Nor is it made invisible: GPUI keeps
/// that state up to date only for an element it paints, and clipping to no width hides it anyway.
#[track_caller]
pub(crate) fn row_actions(
    id: impl Into<ElementId>,
    group: impl Into<SharedString>,
    shown: bool,
    actions: Vec<AnyElement>,
) -> Stateful<Div> {
    let shown_style =
        |s: StyleRefinement| s.max_w(px(ROW_ACTIONS_MAX_W)).ml_0().mr(px(-ROW_ACTIONS_OVERHANG));
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .overflow_hidden()
        .map(|d| {
            if shown {
                d.max_w(px(ROW_ACTIONS_MAX_W)).mr(px(-ROW_ACTIONS_OVERHANG))
            } else {
                // At rest also without the gap before it, so the text runs to the inset.
                d.max_w(px(0.)).ml(px(-ROW_GAP)).group_hover(group, shown_style)
            }
        })
        .children(actions)
}

/// Room for a row's hover actions (three at most), and the gap a row puts between its parts.
const ROW_ACTIONS_MAX_W: f32 = 3.0 * HIT_MIN;
const ROW_GAP: f32 = 8.0;
/// How far an active filter's dot sits from the button's top right corner, and the ring
/// of the surface around it that keeps it apart from the glyph.
const MARK_INSET: f32 = 5.0;
const MARK_RING: f32 = 2.0;

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
/// What opens a button's menu, handed the mark that says the menu is open while it is.
type OpenMenu = Rc<dyn Fn(&MenuMark, &mut Window, &mut App)>;

/// A borderless icon-only button with a tooltip: panel toggles, a header's + and funnel,
/// refresh, the diff's controls. Build it with [`icon_button`].
///
/// It is GPUI Kit's `Button` underneath (focus, the keyboard, menus, the tooltip) with
/// the face drawn here: [`HIT_MIN`] square, the glyph muted until the pointer is over it,
/// then `text.primary` on `band.hover`; while selected (a pressed toggle, an active
/// filter, its menu open) the primary's tint with its glyph in `sidebar.row.selected.text`.
/// The Kit's own ghost hover can't be used: it comes from `accent`, which menus and lists
/// share. Its tooltip is also what VoiceOver calls it, since it has no words of its own.
///
/// Its glyph answers with its own drawing where it has a motion (`style::IconMotion`): the +, the
/// refresh and reset arrows and copy do by their icon, and the split, zoom and menu buttons say
/// which with [`Self::motion`] or [`Self::menu_chevron`].
#[derive(IntoElement)]
pub struct IconButton {
    id: ElementId,
    /// Where the button sits (margins, position); it holds the button and its focus ring, which
    /// the Kit's button would clip.
    outer: Div,
    button: Button,
    group: SharedString,
    ui: UiColors,
    icon: IconName,
    /// Drawn instead of `icon`: an app's own icon.
    drawing: Option<AnyElement>,
    glyph: f32,
    face: f32,
    /// The face's width where it isn't a square (a menu's chevron beside a button).
    face_w: f32,
    /// It takes the pointer from the title row under it (`in_title_row`, `narrow`).
    occludes: bool,
    selected: bool,
    marked: bool,
    ring: bool,
    strip: Option<StripFill>,
    motion: Option<IconMotion>,
    /// The button's own click handler, which the button runs after starting its motion.
    on_click: Option<ClickHandler>,
    menu: Option<OpenMenu>,
}

/// An [`IconButton`] with a 14 pt glyph. Add `.on_click(…)` or a menu.
#[track_caller]
pub fn icon_button(
    id: impl Into<ElementId>,
    ui: UiColors,
    icon: IconName,
    tooltip: impl Into<SharedString>,
) -> IconButton {
    let id: ElementId = id.into();
    let tooltip: SharedString = tooltip.into();
    IconButton {
        outer: div(),
        group: SharedString::from(format!("icon-button-{id}")),
        button: Button::new(id.clone()).accessibility_label(spoken_name(&tooltip)).tooltip(tooltip),
        id,
        ui,
        icon,
        drawing: None,
        glyph: GLYPH,
        face: HIT_MIN,
        face_w: HIT_MIN,
        occludes: false,
        selected: false,
        marked: false,
        ring: false,
        strip: None,
        motion: IconMotion::of(icon),
        on_click: None,
        menu: None,
    }
}

impl IconButton {
    /// The 16 pt glyph of the title rows on a 24 pt face (still clickable across [`HIT_MIN`]): the
    /// sidebar toggle, Open in…, the splits. It
    /// takes the pointer from the title row under it, so clicking it fast never reads as a
    /// double-click on the title bar (which zooms the window).
    pub fn in_title_row(mut self) -> Self {
        (self.glyph, self.face) = (TITLE_GLYPH, BAND_BUTTON);
        self.occludes = true;
        self
    }

    /// `drawing` in the glyph's place, at `glyph` points square: an app's own icon, which keeps
    /// its colors where a glyph takes the button's.
    pub fn with_drawing(mut self, drawing: impl IntoElement) -> Self {
        self.drawing = Some(drawing.into_any_element());
        self
    }

    /// What the glyph does to answer, in place of the one its icon has (or none): the splits fill
    /// their pane, the zoom's corners swap.
    pub fn motion(mut self, motion: IconMotion) -> Self {
        self.motion = Some(motion);
        self
    }

    /// A menu's chevron: a click runs `open`, which opens the menu and passes it through the
    /// mark's [`MenuMark::track`]; the chevron points up until the menu closes.
    pub fn menu_chevron(mut self, open: impl Fn(&MenuMark, &mut Window, &mut App) + 'static) -> Self {
        self.motion = Some(IconMotion::MenuChevron);
        self.menu = Some(Rc::new(open));
        self
    }

    /// A menu's chevron beside a button: a 12 pt glyph on a face `width` wide and [`HIT_MIN`] tall,
    /// taking the pointer from the title row under it like [`Self::in_title_row`].
    pub fn narrow(mut self, width: f32) -> Self {
        (self.glyph, self.face_w) = (NARROW_GLYPH, width);
        self.occludes = true;
        self
    }

    /// A band's +: a 24 pt face with a 16 pt glyph, still clickable across [`HIT_MIN`].
    pub fn in_band(mut self) -> Self {
        (self.glyph, self.face) = (TITLE_GLYPH, BAND_BUTTON);
        self
    }

    /// A row's hover action (`style::item_action`): a 22 pt face with a 14 pt glyph, still
    /// clickable across [`HIT_MIN`].
    pub fn in_row(mut self) -> Self {
        (self.glyph, self.face) = (GLYPH, ROW_BUTTON);
        self
    }

    /// What VoiceOver calls the button where its tooltip alone would make a list of identical ones
    /// (the open-in-editor icon on every file row names the file).
    pub fn spoken(mut self, label: impl Into<SharedString>) -> Self {
        self.button = self.button.accessibility_label(label);
        self
    }

    /// The small primary dot of a filter that is on (with the button selected).
    pub fn marked(mut self, marked: bool) -> Self {
        self.marked = marked;
        self
    }

    /// The glyph's column strip, filled while the column is open: the sidebar's and the right
    /// panel's toggles (`style::StripFill`).
    pub fn strip(mut self, fill: StripFill) -> Self {
        self.strip = Some(fill);
        self
    }

    /// The keyboard focus ring round its face (outside, a gap, the control's corners).
    pub fn focus_ring(mut self, shown: bool) -> Self {
        self.ring = shown;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Pulls the button out by the room around its glyph, so the glyph (not the
    /// invisible box) lines up with the text and icons of the rows above and below.
    pub fn flush_right(self) -> Self {
        let inset = self.glyph_inset();
        self.mr(px(-inset))
    }

    /// Like [`Self::flush_right`], for a button that starts a row.
    pub fn flush_left(self) -> Self {
        let inset = self.glyph_inset();
        self.ml(px(-inset))
    }

    fn glyph_inset(&self) -> f32 {
        (HIT_MIN - self.glyph) / 2.0
    }
}

impl Selectable for IconButton {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Styled for IconButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.outer.style()
    }
}

impl InteractiveElement for IconButton {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.button.interactivity()
    }
}

impl RenderOnce for IconButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let IconButton {
            id,
            outer,
            button,
            group,
            ui,
            icon,
            drawing,
            glyph,
            face,
            face_w,
            occludes,
            selected,
            marked,
            ring,
            strip,
            motion,
            on_click,
            menu,
        } = self;
        let clear = transparent_black();
        // An app's own icon, drawn in its place, has no motion of the glyph's.
        let motion = motion.filter(|_| drawing.is_none());
        let menu_mark = menu.as_ref().map(|_| MenuMark::keyed((id.clone(), "menu-open"), window, cx));
        let open = menu_mark.as_ref().is_some_and(|mark| mark.is_open(cx));
        let armed = motion.map(|motion| motion.arm(&id, icon, open, ui.icon_motion, window, cx));
        let on_click = {
            let started = armed.as_ref().and_then(|armed| armed.clicks());
            let menu = menu.zip(menu_mark);
            (started.is_some() || on_click.is_some() || menu.is_some()).then(|| {
                move |event: &ClickEvent, window: &mut Window, cx: &mut App| {
                    if let Some(started) = &started {
                        started.start(cx);
                    }
                    if let Some(run) = &on_click {
                        run(event, window, cx);
                    }
                    if let Some((open_menu, mark)) = &menu {
                        open_menu(mark, window, cx);
                    }
                }
            })
        };
        let shown = {
            let drawn = match (drawing, &armed) {
                (Some(drawing), _) => drawing,
                (None, Some(armed)) => armed.glyph(icon, glyph),
                (None, None) => Icon::new(icon).with_size(px(glyph)).into_any_element(),
            };
            match strip {
                // The fill takes the glyph's colors, which a div can't inherit as the icon does.
                Some(fill) => {
                    let (color, hover) = if selected {
                        (ui.selected_text, None)
                    } else {
                        (ui.muted, Some((group.clone(), ui.text)))
                    };
                    div()
                        .relative()
                        .size(px(glyph))
                        .child(fill.render(glyph, color, hover))
                        .child(drawn)
                        .into_any_element()
                }
                None => drawn,
            }
        };
        // The Kit draws nothing of its own; the face below does.
        let plain = ButtonCustomVariant::new(cx).color(clear).hover(clear).active(clear);
        let button = button
            .custom(plain)
            .when_some(on_click, |b, run| b.on_click(run))
            // Only a pressed one is a toggle to VoiceOver; the rest are plain buttons.
            .when(selected, |b| b.toggled(true))
            .group(group.clone())
            .w(px(face_w))
            .h(px(HIT_MIN))
            .p_0()
            .rounded(ui.radius)
            .child(
                div()
                    .relative()
                    .flex_none()
                    .w(px(face_w.min(face)))
                    .h(px(face))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(ui.radius)
                    .text_color(if selected { ui.selected_text } else { ui.muted })
                    .when(selected, |d| d.bg(ui.selected))
                    .when(!selected, |d| {
                        d.group_hover(group, move |s| s.bg(ui.band_hover).text_color(ui.text))
                    })
                    .child(shown)
                    .when(marked, |d| d.child(mark(ui))),
            );
        // The ring goes round the face, outside it, over the box the Kit's button clips to.
        let inset = px((HIT_MIN - face) / 2.0);
        let inset_x = px((face_w - face_w.min(face)) / 2.0);
        let face_box = div().absolute().top(inset).left(inset_x).right(inset_x).bottom(inset);
        outer.relative().flex_none().when(occludes, |d| d.occlude()).child(button).when(ring, |d| {
            d.child(face_box.child(super::controls::focus_ring(
                ui,
                super::controls::RingPlacement::Outside,
                ui.radius,
            )))
        })
    }
}

/// What VoiceOver calls a button whose tooltip ends in its key ("Settings (⌘,)"): the words
/// alone, since key glyphs are never read as text.
fn spoken_name(tooltip: &str) -> String {
    let keys =
        |inside: &str| inside.chars().any(|c| matches!(c, '⌘' | '⌥' | '⇧' | '⌃')) || inside.starts_with('F');
    match tooltip.strip_suffix(')').and_then(|rest| rest.rsplit_once(" (")) {
        Some((words, inside)) if keys(inside) => words.to_string(),
        _ => tooltip.to_string(),
    }
}

/// An active filter's dot on its [`IconButton`], ringed by the surface it sits on.
#[track_caller]
fn mark(ui: UiColors) -> Div {
    let mut dot = div()
        .absolute()
        .top(px(MARK_INSET - MARK_RING))
        .right(px(MARK_INSET - MARK_RING))
        .size(px(MARK_DOT + 2.0 * MARK_RING))
        .rounded_full()
        .bg(ui.primary)
        .border_color(ui.panel);
    let ring = Some(px(MARK_RING).into());
    let b = &mut dot.style().border_widths;
    (b.top, b.right, b.bottom, b.left) = (ring, ring, ring, ring);
    dot
}

/// What a [`close_x`] sits on, which decides its colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedFill {
    /// No selection: a plain row or an inactive tab.
    Plain,
    /// A selected tint (the current terminal's row, the "Filtered" chip): the tint's text color,
    /// and the inset fill under the pointer.
    Tint,
    /// The solid primary (the active tab): `text.on_selected` and a hover overlay.
    Solid,
}

/// A small × that removes or closes what it sits on. Hide it until the row is
/// hovered with `.invisible().group_hover(…, |d| d.visible())`. `on` is the fill it
/// sits on (the current session's tint, the active tab's solid primary), so it takes
/// that fill's colors. Its drawing is [`CONTROL_H_SM`]; wrap it in [`hit_area`] where
/// there is room for the bigger target.
#[track_caller]
pub fn close_x(id: impl Into<ElementId>, ui: UiColors, on: SelectedFill) -> Stateful<Div> {
    let (rest, hover, hover_bg) = match on {
        SelectedFill::Plain => (ui.muted, ui.text, ui.hover),
        SelectedFill::Tint => (ui.selected_text, ui.selected_text, ui.selected_inset),
        SelectedFill::Solid => (ui.on_selected_muted, ui.on_selected, ui.hover),
    };
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(CONTROL_H_SM))
        .rounded(ui.radius_sm)
        .text_color(rest)
        .cursor_pointer()
        .hover(move |d| d.text_color(hover).bg(hover_bg))
        .child(icon(IconName::X))
}

/// Grows the target of a small drawing (`drawn` points square: a 22 pt ×) to [`HIT_MIN`]
/// without moving or resizing what it draws: `target` gets the square round it, which takes the
/// clicks (add the role, tooltip and click handler there) and hangs out over the neighbors by the
/// difference. The row lays out only a slot the drawing's size: Taffy loses track of a flex row's
/// width when a child has negative side margins (the tab strip's names were squeezed away).
#[track_caller]
pub fn hit_area(
    id: impl Into<ElementId>,
    drawn: f32,
    child: impl IntoElement,
    target: impl FnOnce(Stateful<Div>) -> Stateful<Div>,
) -> Div {
    let overhang = ((HIT_MIN - drawn) / 2.0).max(0.0);
    let square = div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(drawn.max(HIT_MIN)))
        .m(px(-overhang))
        .cursor_pointer()
        .child(child);
    div().flex_none().size(px(drawn)).child(target(square))
}

/// A chrome icon on its own: a folder in the sidebar, a glyph beside a label.
pub fn icon(name: IconName) -> Icon {
    Icon::new(name).with_size(ICON_SIZE)
}

/// The tooltip delay for things that build their own tooltip (GPUI's own default, kept
/// in step with [`TOOLTIP_DELAY`]).
pub(super) fn tooltip_delay<E: StatefulInteractiveElement>(element: E) -> E {
    element.tooltip_show_delay(TOOLTIP_DELAY)
}

#[cfg(test)]
mod tests {
    use super::spoken_name;

    #[test]
    fn a_buttons_spoken_name_leaves_its_key_out() {
        assert_eq!(spoken_name("Settings (⌘,)"), "Settings");
        assert_eq!(spoken_name("Hide sidebar (⌃⌘S)"), "Hide sidebar");
        assert_eq!(
            spoken_name("Show right panel (⌥⌘I) · hides the sidebar to make room"),
            "Show right panel (⌥⌘I) · hides the sidebar to make room"
        );
        assert_eq!(spoken_name("Refresh"), "Refresh");
        assert_eq!(spoken_name("Compare (main)"), "Compare (main)");
    }
}
