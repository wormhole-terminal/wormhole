//! A setting row's config keys: shown under the pointer, copied by a click on them, and offered by the
//! row's menu (Copy Key, Copy as JSON, Reset to Default).
//!
//! Hover shows a key and a pointer can click it, but neither reaches a keyboard or VoiceOver, so the
//! menu is also opened by ⌃↩ on any control of the row and by VoiceOver's "Show menu" action
//! (`Action::ShowContextMenu`), and the key is a labelled button of its own.
//!
//! The row hands over data, not behaviour: its keys, a function that reads their current values as
//! JSON (the app's own settings, which this layer does not know) and, while changed, a way to put them
//! back. That is all, so a row of another source (a Probe's, later) is drawn the same. What a row
//! costs a frame is three listeners on the row and one on the key (`frame_tests` says how much); the
//! menu and the JSON exist only once the menu is open.
//!
//! Two small facts live in one global, read once per row: whether ⌥ is held (every key of the page is
//! shown) and which row has just copied (it says Copied for a moment).

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::accesskit::Action;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_text::{keys, text};

use crate::platform::{self, Item, Menu};
use crate::ui::Handler;

/// What a row's reset does, from a click on the arrow or the menu.
pub type Reset = Rc<dyn Fn(&mut Window, &mut App)>;

/// How long a copied key says Copied.
const COPIED_FOR: Duration = Duration::from_millis(1200);

/// The config keys a row writes, and how to read them as JSON. `Copy`, and built from statics, so a
/// row makes none of it afresh each frame.
#[derive(Clone, Copy)]
pub struct RowKeys {
    pub keys: &'static [&'static str],
    /// The current values of the given keys as one line of JSON (`{"key": value}`).
    pub json: fn(&App, &[&str]) -> String,
}

impl RowKeys {
    /// A row with no config key (a summary, a button): no key, no menu.
    pub const NONE: Self = Self { keys: &[], json: |_, _| String::new() };

    /// What a click on the key and Copy Key put on the clipboard: the keys, a line each.
    fn text(&self) -> String {
        self.keys.join("\n")
    }
}

/// ⌥ and the last copy, for every row of the window. One small value in one global.
#[derive(Default)]
struct Shown {
    option: bool,
    /// The window and row that copied last, and which copy it was, so an older copy's timer leaves a
    /// newer one's Copied alone. The window is named because the timer cannot clear this once its
    /// window is gone; the entry then matches no window that is open and is never drawn again.
    copied: Option<(WindowId, ElementId, u64)>,
    copies: u64,
}

impl Global for Shown {}

/// Tell the rows whether ⌥ is held. The window says so from its own frame, from the modifiers it holds,
/// so a key let go while the window was in the background cannot leave the keys shown.
pub(crate) fn sync_option(held: bool, cx: &mut App) {
    if cx.try_global::<Shown>().is_none_or(|shown| shown.option != held) {
        cx.default_global::<Shown>().option = held;
    }
}

/// Whether the rows show their keys because ⌥ is held.
pub(crate) fn option_held(cx: &App) -> bool {
    cx.try_global::<Shown>().is_some_and(|shown| shown.option)
}

/// What every row of a frame needs of [`Shown`], read once.
#[derive(Clone, Copy)]
pub(super) struct KeyView {
    option: bool,
    copied: Option<u64>,
}

impl KeyView {
    pub(super) fn read(row: &ElementId, cx: &App) -> Self {
        let shown = cx.try_global::<Shown>();
        Self {
            option: shown.is_some_and(|shown| shown.option),
            copied: shown
                .and_then(|shown| shown.copied.as_ref())
                .filter(|(window, copied, _)| {
                    copied == row && cx.windows().iter().any(|w| w.window_id() == *window)
                })
                .map(|(_, _, copy)| *copy),
        }
    }
}

// What the last frame drew of the keys, for the tests: the keys shown without the pointer.
#[cfg(test)]
thread_local! {
    pub(crate) static SHOWN_KEYS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// What a row's key shows, for the row to draw: the key is faint and shown under the pointer, while ⌥ is
/// held and while it says Copied; a click copies the keys.
pub(super) struct KeyFace {
    /// The key, or Copied for a moment after a copy.
    pub shown: SharedString,
    /// The key itself, which VoiceOver reads as the button's description.
    pub key: &'static str,
    pub copied: bool,
    /// Hidden until the pointer is over the row.
    pub dormant: bool,
    pub copy: Handler,
}

/// The key of a row, `None` for a row without one.
pub(super) fn key_face(row: &ElementId, view: KeyView, keys: RowKeys) -> Option<KeyFace> {
    let key = *keys.keys.first()?;
    let always = view.option || view.copied.is_some();
    #[cfg(test)]
    if always {
        SHOWN_KEYS.with(|shown| shown.borrow_mut().push(key.to_string()));
    }
    let copied = view.copied.is_some();
    let on_copy = row.clone();
    Some(KeyFace {
        shown: SharedString::from(if copied { text(keys::SETTINGS_ROW_COPIED) } else { key }),
        key,
        copied,
        dormant: !always,
        copy: Rc::new(move |_, window, cx| copy(&on_copy, keys.text(), window, cx)),
    })
}

/// Put `words` on the clipboard and have `row` say so: for a moment in its key's place, and to VoiceOver.
fn copy(row: &ElementId, words: String, window: &mut Window, cx: &mut App) {
    platform::clipboard::write(words, cx);
    let said = text(keys::SETTINGS_ROW_COPIED);
    platform::a11y::announce(said, false);
    let shown = cx.default_global::<Shown>();
    shown.copies += 1;
    let copy = shown.copies;
    shown.copied = Some((window.window_handle().window_id(), row.clone(), copy));
    window.refresh();
    window
        .spawn(cx, async move |cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            cx.update(|window, cx| {
                let shown = cx.default_global::<Shown>();
                if shown.copied.as_ref().is_some_and(|(_, _, latest)| *latest == copy) {
                    shown.copied = None;
                    window.refresh();
                }
            })
            .ok();
        })
        .detach();
}

/// What the row's menu is made from. Cloned into each listener: cheaper than a shared pointer made
/// per row, as a clone copies a few words.
#[derive(Clone)]
struct RowMenu {
    row: ElementId,
    keys: RowKeys,
    /// Present while the value is changed.
    reset: Option<Reset>,
}

/// `row` with its context menu: a right click, ⌃↩ on a control inside it, VoiceOver's "Show menu".
/// Nothing is built until it opens.
#[track_caller]
pub(super) fn with_menu(
    row: Stateful<Div>,
    id: ElementId,
    keys: RowKeys,
    reset: Option<Reset>,
) -> Stateful<Div> {
    if keys.keys.is_empty() {
        return row;
    }
    let menu = RowMenu { row: id, keys, reset };
    let (on_click, on_key, on_a11y) = (menu.clone(), menu.clone(), menu);
    // Where the row is, noted as it is laid out: a key or VoiceOver opens the menu at the row, not at a
    // pointer that may be anywhere.
    let place = Rc::new(Cell::new(None));
    let (for_key, for_a11y, noted) = (place.clone(), place.clone(), place.clone());
    row.on_mouse_down(MouseButton::Right, move |event, window, cx| {
        // Not the page's, nor a control's own menu.
        cx.stop_propagation();
        open(&on_click, event.position, window, cx);
    })
    .on_key_down(move |event, window, cx| {
        let keystroke = &event.keystroke;
        let held = keystroke.modifiers;
        if keystroke.key == "enter" && held.control && !held.alt && !held.platform && !held.shift {
            cx.stop_propagation();
            open(&on_key, menu_at(&for_key, window), window, cx);
        }
    })
    .on_a11y_action(Action::ShowContextMenu, move |_, window, cx| {
        open(&on_a11y, menu_at(&for_a11y, window), window, cx)
    })
    .child(
        canvas(move |bounds, _, _| noted.set(Some(bounds)), |_, (), _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
    )
}

/// Content that stands on its own (the theme cards) with a row's menu: right-click, ⌃↩ and VoiceOver's
/// "Show menu" give Copy Key, Copy as JSON and Reset to Default for `keys`, as on a row. It has no label to
/// show a key beside, so the menu is the way to them.
#[track_caller]
pub fn keyed_block(
    id: impl Into<ElementId>,
    keys: RowKeys,
    reset: Option<Reset>,
    content: AnyElement,
) -> Stateful<Div> {
    let id = id.into();
    with_menu(div().id(id.clone()).w_full().child(content), id, keys, reset)
}

/// Where a menu from the keyboard or VoiceOver opens: under the row's text, at its bottom edge. The
/// pointer's place only when the row has not been laid out yet.
fn menu_at(place: &Cell<Option<Bounds<Pixels>>>, window: &Window) -> Point<Pixels> {
    match place.get() {
        Some(row) => point(row.left() + px(MENU_INSET), row.bottom()),
        None => window.mouse_position(),
    }
}

/// How far in from the row's left edge a menu from the keyboard opens.
const MENU_INSET: f32 = 16.0;

/// A control with a context menu of its own (a text field's cut, copy and paste): a right press on it
/// goes no further up, so the row's menu does not open beside it, and the row's menu stays everywhere
/// else on the row. `Input` handles that press on its way up and leaves it to go on, and shows its menu
/// on the release, which this does not stop. Wrap every text field that sits in a row with keys.
#[track_caller]
pub fn own_menu<E: InteractiveElement>(control: E) -> E {
    control.on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
}

fn open(menu: &RowMenu, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
    let (for_key, for_json, for_reset) = (menu.clone(), menu.clone(), menu.reset.clone());
    let reset = text(keys::SETTINGS_ROW_RESET_MENU);
    let menu = Menu::new()
        .item(Item::new(text(keys::SETTINGS_ROW_COPY_KEY)).on_select(move |window, cx| {
            copy(&for_key.row, for_key.keys.text(), window, cx);
        }))
        .item(Item::new(text(keys::SETTINGS_ROW_COPY_JSON)).on_select(move |window, cx| {
            let json = (for_json.keys.json)(cx, for_json.keys.keys);
            copy(&for_json.row, json, window, cx);
        }))
        .separator()
        .item(Item::new(reset).enabled(for_reset.is_some()).on_select(move |window, cx| {
            if let Some(reset) = &for_reset {
                reset(window, cx);
            }
        }));
    platform::show_menu(menu, at, window, cx);
}

#[cfg(all(test, feature = "devctl"))]
mod window_tests {
    use gpui_kit::component::input::{Input, InputState};
    use gpui_kit::prelude::*;
    use gpui_kit::{
        Context, Entity, IntoElement, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Pixels, Point,
        Render, TestAppContext, VisualTestContext, Window, WindowOptions, div, point, px,
    };

    use super::{RowKeys, own_menu, with_menu};
    use crate::settings::SettingsStore;

    const ROW_W: f32 = 400.0;
    const FIELD_X: f32 = 200.0;

    /// A row with a key and a text field at its right, the field wrapped in [`own_menu`] or not.
    struct Row {
        input: Entity<InputState>,
        own: bool,
    }

    impl Render for Row {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let field = div().w(px(ROW_W - FIELD_X)).child(Input::new(&self.input));
            let field = if self.own { own_menu(field).into_any_element() } else { field.into_any_element() };
            let row =
                div().id("row").flex().w(px(ROW_W)).h(px(40.0)).child(div().w(px(FIELD_X))).child(field);
            with_menu(row, "row".into(), RowKeys { keys: &["a.b"], json: |_, _| String::new() }, None)
        }
    }

    fn right_click(cx: &mut VisualTestContext, at: Point<Pixels>) {
        let (button, modifiers) = (MouseButton::Right, Modifiers::none());
        cx.simulate_event(MouseDownEvent {
            button,
            position: at,
            modifiers,
            click_count: 1,
            first_mouse: false,
        });
        cx.simulate_event(MouseUpEvent { button, position: at, modifiers, click_count: 1 });
    }

    /// Whether the row's menu opened for a right click at `x` on the row, with the field wrapped or not.
    fn row_menu_opens(cx: &mut TestAppContext, own: bool, x: f32) -> bool {
        let window = cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_global(SettingsStore::for_test());
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                let input = cx.new(|cx| InputState::new(window, cx));
                cx.new(|_| Row { input, own })
            })
            .expect("opens the window")
        });
        let mut cx = VisualTestContext::from_window(window.0, cx);
        cx.run_until_parked();
        crate::platform::record::close();
        right_click(&mut cx, point(px(x), px(20.0)));
        let opened = crate::platform::record::open().is_some();
        crate::platform::record::close();
        opened
    }

    #[gpui_kit::test]
    fn the_rows_menu_opens_on_the_row_beside_a_text_field(cx: &mut TestAppContext) {
        assert!(row_menu_opens(cx, true, 20.0));
    }

    #[gpui_kit::test]
    fn a_text_field_keeps_its_own_menu_and_the_row_opens_none_beside_it(cx: &mut TestAppContext) {
        assert!(!row_menu_opens(cx, true, FIELD_X + 40.0), "the field's own menu wins inside the field");
    }

    #[gpui_kit::test]
    fn without_own_menu_a_text_field_would_open_both(cx: &mut TestAppContext) {
        // Why `own_menu` exists: `Input` lets the press go on up to the row.
        assert!(row_menu_opens(cx, false, FIELD_X + 40.0));
    }
}
