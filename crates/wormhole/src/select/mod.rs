//! The app's select look: a bordered trigger with an up/down chevron that
//! shows a focus ring while its menu is open, and a menu with a search box and
//! roomy rows. The menu floats in GPUI Kit's `Popover` on a lifted surface, so
//! opening one never pushes the page around and it reads clearly above it.

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::{ActiveTheme as _, Icon, Selectable, Sizable};
use gpui_kit::prelude::*;
// Named: under gpui-fast the prelude and the root each bring a derive called IntoElement.
use gpui_kit::{IntoElement, *};

use crate::settings::UiColors;
use crate::style::{CONTROL_H, Edges, ROW_H, TextRole};

/// The panel picker's trigger: 24 high, between the 22 of a chip and a control's 28.
const PICKER_H: f32 = 24.0;
/// The widest a panel picker grows before its branch name truncates.
const PICKER_MAX_W: f32 = 160.0;

/// The closed select: current value on the left, chevrons on the right.
/// Pass it to `Popover::trigger`, which marks it open while the menu shows.
#[derive(IntoElement)]
pub struct SelectTrigger {
    id: ElementId,
    ui: UiColors,
    label: AnyElement,
    open: bool,
    compact: bool,
    width: Option<Pixels>,
}

impl SelectTrigger {
    pub fn new(id: impl Into<ElementId>, ui: UiColors, label: impl IntoElement) -> Self {
        Self { id: id.into(), ui, label: label.into_any_element(), open: false, compact: false, width: None }
    }

    /// The panel headers' variant (the branch picker): 24 high, caption text in 500, a
    /// `border.strong` outline that turns primary while the menu is open.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }
}

impl Selectable for SelectTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl RenderOnce for SelectTrigger {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let ui = self.ui;
        div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .min_w_0()
            .when(self.compact, |d| {
                d.h(px(PICKER_H))
                    .pl_2()
                    .pr_1()
                    .gap_1()
                    .text_caption()
                    .font_weight(FontWeight::MEDIUM)
                    .max_w(px(PICKER_MAX_W))
                    .flex_none()
                    .outlined(ui, if self.open { ui.primary } else { ui.border_strong })
            })
            .when(!self.compact, |d| {
                d.h(px(CONTROL_H))
                    .px_2()
                    .text_body()
                    .outlined(ui, if self.open { ui.primary } else { ui.border })
            })
            .when_some(self.width, |d, width| d.w(width))
            .rounded(ui.radius)
            .cursor_pointer()
            .hover(|d| d.bg(ui.hover))
            // A soft ring around the open select, like a focused input (the panel's picker
            // marks itself with the outline alone).
            .when(self.open && !self.compact, |d| {
                d.shadow(vec![BoxShadow {
                    color: Hsla { a: 0.3, ..ui.primary },
                    offset: point(px(0.0), px(0.0)),
                    blur_radius: px(0.0),
                    spread_radius: px(2.0),
                    inset: false,
                }])
            })
            .child(div().min_w_0().truncate().child(self.label))
            .child(chevrons(ui))
    }
}

/// The up/down chevrons that mark a select.
pub fn chevrons(ui: UiColors) -> Icon {
    Icon::new(IconName::ChevronsUpDown).xsmall().text_color(ui.muted)
}

/// One option in a select menu: a check column (filled for the current value),
/// then whatever is added with `.child(…)`. The current value is marked by the check
/// alone; a filled row on top of it is too heavy in a dark menu. `compact` is the
/// panel's menu: 28 rows of 13 pt text, the check in a 16 pt column, `band.hover`
/// under the pointer.
#[track_caller]
pub fn option(id: impl Into<ElementId>, ui: UiColors, selected: bool, compact: bool) -> Stateful<Div> {
    div()
        .id(id)
        // Never squeezed to its minimum when the list is taller than its scroll box: a row with
        // a second line kept its text while the row shrank under it.
        .flex_none()
        .flex()
        .items_center()
        .gap_2()
        .min_h(px(if compact { ROW_H } else { ROW_H + 4.0 }))
        .px_2()
        .rounded(ui.radius)
        .when(compact, |d| d.text_label().font_weight(FontWeight::NORMAL).hover(|d| d.bg(ui.band_hover)))
        .when(!compact, |d| d.text_body().hover(|d| d.bg(ui.hover)))
        .cursor_pointer()
        .child(
            div()
                .w(px(if compact { 16.0 } else { 14.0 }))
                .flex_none()
                .flex()
                .justify_center()
                .text_color(ui.primary)
                .when(selected, |d| d.child(Icon::new(IconName::Check).xsmall())),
        )
}

/// A light theme has darker text than background.
fn is_light(ui: UiColors) -> bool {
    ui.text.l < ui.background.l
}

/// The surface of every select menu: the theme's popover color, nudged lighter in a
/// dark theme so the menu sits above the panel behind it instead of blending into
/// it, and always opaque.
pub fn menu_bg(ui: UiColors, cx: &App) -> Hsla {
    let popover: Hsla = cx.theme().popover;
    let l = if is_light(ui) { popover.l } else { (popover.l + 0.035).min(0.95) };
    Hsla { l, a: 1.0, ..popover }
}

/// The shadow shared by every select menu: a faint hairline edge and one soft shadow,
/// centered on the menu (a hair lower, as light falls) so it lifts the menu evenly on
/// every side. Layers that are offset far downward, or shrunk with a negative spread,
/// leave the shadow pooled under the menu and off the shape of its corners. The blur
/// is small because GPUI takes it as the gaussian's deviation, not CSS's doubled
/// figure, and the ink is light: a heavy dark halo just muddies a dark panel.
pub fn menu_shadow(ui: UiColors) -> Vec<BoxShadow> {
    let ink = hsla(0.0, 0.0, 0.0, if is_light(ui) { 0.10 } else { 0.28 });
    let shadow = |color: Hsla, y: f32, blur: f32, spread: f32| BoxShadow {
        color,
        offset: point(px(0.0), px(y)),
        blur_radius: px(blur),
        spread_radius: px(spread),
        inset: false,
    };
    vec![shadow(Hsla { a: 0.12, ..ui.text }, 0.0, 0.0, 1.0), shadow(ink, 1.0, 5.0, 0.0)]
}

/// Give a popover the shared menu surface: `width` wide, `bg` behind it, rounded,
/// lifted by [`menu_shadow`], and unpadded so the menu lays out its own search row
/// and list.
pub fn menu_surface(popover: Popover, ui: UiColors, width: Pixels, bg: Hsla) -> Popover {
    popover.w(width).p_0().bg(bg).rounded(ui.radius_lg).shadow(menu_shadow(ui))
}

/// The search row at the top of a menu, a hairline under it. The panel's menu (`compact`)
/// has a search field instead: a 28 pt well in `input.background` with the search glyph,
/// outlined in the primary while it has the keyboard (it does while the menu is open).
#[track_caller]
pub fn search_row(ui: UiColors, compact: bool, content: impl IntoElement) -> Div {
    if compact {
        return div().p_1().child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .h(px(ROW_H))
                .px_2()
                .rounded(ui.radius)
                .bg(ui.input)
                .outlined(ui, ui.primary)
                .child(Icon::new(IconName::Search).xsmall().text_color(ui.muted))
                .child(div().flex_1().min_w_0().child(content)),
        );
    }
    div().px_2().py_2().rule_bottom(ui).child(content)
}

/// One option of a [`SearchSelect`]: what it stores, its label, and an optional
/// second line (a path, say). Search matches both lines.
#[derive(Clone)]
pub struct Choice<V> {
    pub value: V,
    label: SharedString,
    detail: Option<SharedString>,
    /// A glyph before the label in its own color: the branch mark in the branch picker.
    mark: Option<(IconName, Hsla)>,
}

impl<V> Choice<V> {
    pub fn new(value: V, label: impl Into<SharedString>) -> Self {
        Self { value, label: label.into(), detail: None, mark: None }
    }

    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn mark(mut self, icon: IconName, color: Hsla) -> Self {
        self.mark = Some((icon, color));
        self
    }

    /// Whether the label or the second line contains the search (case ignored).
    fn matches(&self, needle: &Needle) -> bool {
        needle.is_empty()
            || needle.found_in(&self.label)
            || self.detail.as_ref().is_some_and(|d| needle.found_in(d))
    }
}

/// A search text prepared once per filtering: trimmed and lowercased. Matching a thousand
/// options then costs no allocation each (lowercasing the query and both lines of every
/// option for every keystroke did): an ASCII search is compared byte by byte ignoring case,
/// anything else lowercases the option a character at a time as it goes.
pub struct Needle {
    chars: Vec<char>,
    /// The search as lowercase bytes, when it is ASCII (the usual case).
    ascii: Option<Vec<u8>>,
}

impl Needle {
    pub fn new(query: &str) -> Self {
        let chars: Vec<char> = query.trim().chars().flat_map(char::to_lowercase).collect();
        let ascii = chars.iter().all(char::is_ascii).then(|| chars.iter().map(|c| *c as u8).collect());
        Self { chars, ascii }
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Whether `text` contains the search, ignoring case. (Lowercased per character, so a
    /// Greek final sigma matches either form.)
    pub fn found_in(&self, text: &str) -> bool {
        if self.chars.is_empty() {
            return true;
        }
        match &self.ascii {
            Some(needle) if text.is_ascii() => ascii_contains(text.as_bytes(), needle),
            _ => text.char_indices().any(|(start, _)| {
                let mut lowered = text[start..].chars().flat_map(char::to_lowercase);
                self.chars.iter().all(|wanted| lowered.next() == Some(*wanted))
            }),
        }
    }
}

/// Whether `haystack` contains `needle` (lowercase, not empty) ignoring ASCII case.
fn ascii_contains(haystack: &[u8], needle: &[u8]) -> bool {
    let (first, rest) = (needle[0], &needle[1..]);
    let Some(last_start) = haystack.len().checked_sub(needle.len()) else { return false };
    (0..=last_start).any(|at| {
        haystack[at].to_ascii_lowercase() == first
            && haystack[at + 1..at + needle.len()].eq_ignore_ascii_case(rest)
    })
}

/// The options that match `query`, in their original order.
pub fn filter<V: Clone>(options: &[Choice<V>], query: &str) -> Vec<Choice<V>> {
    let needle = Needle::new(query);
    options.iter().filter(|c| c.matches(&needle)).cloned().collect()
}

/// Where a [`SearchSelect`] gets its options.
enum Options<V> {
    /// Built by the caller already.
    Eager(Rc<[Choice<V>]>),
    /// Built when the menu is drawn open.
    Lazy(Rc<dyn Fn() -> Rc<[Choice<V>]>>),
}

type OpenChange = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type Pick<V> = Rc<dyn Fn(&V, &mut Window, &mut App)>;

/// A select over a short list: the trigger, and a floating menu with a search box
/// that filters the options as you type. The owner keeps the state (whether it is
/// open, the search input) and re-renders when they change: it passes `open`, learns
/// of user toggles through [`Self::on_open_change`], and of a choice through
/// [`Self::on_pick`].
pub struct SearchSelect<V> {
    id: SharedString,
    ui: UiColors,
    trigger: SelectTrigger,
    search: Entity<InputState>,
    query: String,
    focus: FocusHandle,
    options: Options<V>,
    current: Option<V>,
    open: bool,
    hint: Option<SharedString>,
    anchor: Anchor,
    menu_width: Pixels,
    bg: Hsla,
    compact: bool,
    /// Whether the menu has its search box (a list of a few options doesn't need one).
    searchable: bool,
    on_open_change: Option<OpenChange>,
    on_pick: Option<Pick<V>>,
}

impl<V: Clone + PartialEq + 'static> SearchSelect<V> {
    pub fn new(
        id: impl Into<SharedString>,
        ui: UiColors,
        trigger: SelectTrigger,
        search: &Entity<InputState>,
        cx: &App,
    ) -> Self {
        Self {
            id: id.into(),
            ui,
            trigger,
            query: search.read(cx).value().to_string(),
            focus: search.read(cx).focus_handle(cx),
            search: search.clone(),
            options: Options::Eager(Rc::from(Vec::new())),
            current: None,
            open: false,
            hint: None,
            anchor: Anchor::TopLeft,
            menu_width: px(280.0),
            bg: menu_bg(ui, cx),
            compact: false,
            searchable: true,
            on_open_change: None,
            on_pick: None,
        }
    }

    /// The menu without its search box: a handful of options (scrollback sizes, icon themes).
    pub fn without_search(mut self) -> Self {
        self.searchable = false;
        self
    }

    pub fn options(mut self, options: Vec<Choice<V>>) -> Self {
        self.options = Options::Eager(options.into());
        self
    }

    /// The options, built only while the menu is open. A select that is built on every
    /// frame of a busy view (the branch picker in a panel header) should take this: the
    /// closed select then costs nothing for however many options it has.
    pub fn options_with(mut self, build: impl Fn() -> Rc<[Choice<V>]> + 'static) -> Self {
        self.options = Options::Lazy(Rc::new(build));
        self
    }

    /// The value shown as chosen (with a ✓).
    pub fn current(mut self, current: V) -> Self {
        self.current = Some(current);
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// A line of explanation above the options.
    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Which corner of the trigger the menu lines up with.
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn menu_width(mut self, width: Pixels) -> Self {
        self.menu_width = width;
        self
    }

    /// The tighter menu (smaller text, shorter rows) for the side panel.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    pub fn on_open_change(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }

    pub fn on_pick(mut self, f: impl Fn(&V, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(f));
        self
    }

    pub fn into_popover(self) -> Popover {
        let Self {
            id,
            ui,
            trigger,
            search,
            query,
            focus,
            options,
            current,
            open,
            hint,
            anchor,
            menu_width,
            bg,
            compact,
            searchable,
            on_open_change,
            on_pick,
        } = self;
        let popover = Popover::new(SharedString::from(format!("{id}-menu")))
            .anchor(anchor)
            .offset(px(6.0))
            .open(open)
            // Typing goes straight into the search box when the menu opens.
            .track_focus(&focus)
            .on_open_change(move |open, window, cx| {
                if let Some(f) = &on_open_change {
                    f(*open, window, cx);
                }
            })
            .trigger(trigger);
        menu_surface(popover, ui, menu_width, bg).content(move |_, _, _| {
            let all: Rc<[Choice<V>]> = match &options {
                Options::Eager(options) => options.clone(),
                Options::Lazy(build) => build(),
            };
            let needle = Needle::new(&query);
            let matches: Vec<usize> = (0..all.len()).filter(|&ix| all[ix].matches(&needle)).collect();
            let row = {
                let (all, current, on_pick) = (all.clone(), current.clone(), on_pick.clone());
                move |position: usize, ix: usize| {
                    option_row(position, &all[ix], ui, current.as_ref(), compact, on_pick.clone())
                }
            };
            // The explanation only helps before you start narrowing the list. The panel's menu
            // says it under the options, the others above them.
            let hint = hint.clone().filter(|_| query.trim().is_empty());
            let (hint, footer) = match compact {
                true => (None, hint.map(|h| menu_footer(ui, h))),
                false => {
                    (hint.map(|h| div().px_2().py_1().text_caption().text_color(ui.muted).child(h)), None)
                }
            };
            // Past a hundred rows (one line each: a list has to be uniform to be drawn from
            // the visible part only) the rows in view are all that is built, not every match.
            let list = if virtualized(matches.len(), all.iter().any(|c| c.detail.is_some())) {
                let (count, matches) = (matches.len(), Rc::new(matches));
                div()
                    .flex()
                    .flex_col()
                    .p_1()
                    .children(hint)
                    .child(
                        uniform_list("search-select-list", count, move |range, _, _| {
                            range.map(|position| row(position, matches[position])).collect()
                        })
                        .h(px(VIRTUAL_LIST_HEIGHT)),
                    )
                    .into_any_element()
            } else {
                div()
                    .id("search-select-list")
                    .flex()
                    .flex_col()
                    .p_1()
                    .max_h(px(280.0))
                    .overflow_y_scroll()
                    .children(hint)
                    .children(matches.iter().enumerate().map(|(position, ix)| row(position, *ix)))
                    .when(matches.is_empty(), |d| {
                        d.child(div().px_2().py_2().text_body().text_color(ui.muted).child("No matches"))
                    })
                    .into_any_element()
            };
            div()
                .flex()
                .flex_col()
                .when(searchable, |d| {
                    d.child(search_row(ui, compact, {
                        let input = Input::new(&search).appearance(false);
                        if compact { input.xsmall() } else { input.small() }
                    }))
                })
                .child(list)
                .children(footer)
        })
    }
}

/// A line of explanation under a panel menu's options, past a hairline.
#[track_caller]
fn menu_footer(ui: UiColors, text: SharedString) -> Div {
    div().mx_1().mb_1().px_2().pt_2().rule_top(ui).text_caption().text_color(ui.muted).child(text)
}

/// From how many rows on a menu draws only the rows in view.
const VIRTUALIZE_AT: usize = 100;
/// The height of a virtualized list: what a scrolling one is capped at (280), less its
/// padding (`p_1`, 4 a side).
const VIRTUAL_LIST_HEIGHT: f32 = 280.0 - 2.0 * 4.0;

/// Whether a list of `rows` is drawn from its visible part only: it is long, and no row
/// has a second line (so every row is the same height).
fn virtualized(rows: usize, any_detail: bool) -> bool {
    rows >= VIRTUALIZE_AT && !any_detail
}

/// One row of the menu: a check for the current value, the label, and the second line.
#[track_caller]
fn option_row<V: Clone + PartialEq + 'static>(
    position: usize,
    choice: &Choice<V>,
    ui: UiColors,
    current: Option<&V>,
    compact: bool,
    on_pick: Option<Pick<V>>,
) -> Stateful<Div> {
    let value = choice.value.clone();
    option(("search-option", position), ui, current == Some(&choice.value), compact)
        .children(choice.mark.map(|(icon, color)| Icon::new(icon).xsmall().text_color(color).flex_none()))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().truncate().child(choice.label.clone()))
                .children(
                    choice
                        .detail
                        .clone()
                        .map(|d| div().truncate().text_caption().text_color(ui.muted).child(d)),
                ),
        )
        .on_click(move |_, window, cx| {
            if let Some(pick) = &on_pick {
                pick(&value, window, cx);
            }
        })
}

#[cfg(test)]
mod tests;
#[cfg(all(test, feature = "devctl"))]
mod window_tests;
