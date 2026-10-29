//! Theme Studio: its own window (⌥⌘,) with the token list on the left, a
//! gallery of every component on the live draft in the middle, and the token editor on the
//! right, so an edit shows where it lands.
//!
//! Edits go into a small theme *file* that `extends` the current theme and is
//! re-resolved on every change, so derived tokens follow (changing `primary`
//! moves focus rings and primary buttons too) and saving writes only overrides.
//! The resolved draft is previewed app-wide through `SettingsStore`, so the
//! main window follows along; closing the Studio window drops the preview.

mod commit;
mod completion;
mod contrast;
mod convert;
#[cfg(feature = "devctl")]
pub(crate) mod devctl;
mod document;
mod draft;
mod gallery;
mod modal;
mod names;
mod opening;
mod origin_tag;
mod save;
mod shell;
mod switch;
mod theme_list;
mod theme_menu;
mod themes;
mod title;
mod title_chips;
mod token_editor;
mod token_field;
mod token_list;
mod token_rows;
mod undo;
mod update_from;
mod use_theme;
mod versions;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::color_picker::{ColorPickerEvent, ColorPickerState};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::slider::{SliderEvent, SliderState};
use gpui_kit::*;
use wormhole_core::keys::keyspec::KeySpec;
use wormhole_core::theme::tokens::{Color, ThemeTokens};

use crate::app_state::{AppState, app, app_mut};
use crate::aux_window;
use crate::platform::vibrancy::Blur;
use crate::settings::{SettingsStore, UiColors, settings};
use crate::style::{self, Edges};
use crate::terminal_view::keystroke_spec;
use crate::theme_bridge::{self, hsla, token};
use document::history::{StepLabel, UndoStack};
use document::{Document, is_metric, number, places};
use draft::group_of;
use gallery::{Area, GalleryView};
use save::Notice;
use themes::Themes;
use title::SaveAction;
use token_editor::TokenEditorView;
use token_field::FieldError;
use token_list::TokenListView;

/// The window's size, and the smallest it may get: the token list
/// (300) and the editor (320) at their widths with the gallery still two columns (680, so the
/// sidebar and panel specimens stand side by side), and enough height for the editor's name,
/// value, rule and uses without scrolling.
const WINDOW_SIZE: (f32, f32) = (1344.0, 900.0);
const WINDOW_MIN: (f32, f32) = (1300.0, 640.0);
/// The token the Studio opens on.
const FIRST_TOKEN: &str = "primary";

/// The Studio's colours: an opaque window keeps the theme's own text colours, a translucent one
/// the main window's. It shadows `settings::ui` for everything drawn in this window (the
/// gallery included), whose files import it.
fn ui(cx: &App) -> UiColors {
    crate::settings::window_ui(cx, settings(cx).studio_translucent)
}

pub(crate) struct StudioWindow {
    /// The theme being edited, as overrides on top of its `extends`, and the file as last saved.
    doc: Document,
    selected: Option<String>,
    picker: Entity<ColorPickerState>,
    expression: Entity<InputState>,
    /// What is wrong with the value in the field, checked on every keystroke. While there is
    /// something, the draft keeps the last good value.
    field_error: Option<FieldError>,
    /// The names the field offers for what is being typed, while there are any.
    completion: Option<completion::Open>,
    /// A metric's value on a slider beside its stepper.
    slider: Entity<SliderState>,
    /// The draft's name, edited in place in the title row.
    name: Entity<InputState>,
    renaming: bool,
    /// Where the name in the title row was last laid out: the theme menu hangs from it.
    name_bounds: Rc<Cell<Bounds<Pixels>>>,
    /// The name being asked for before a copy with no file yet is saved: whether the window closes
    /// once it is. Return gives the name and saves; Esc leaves the copy unsaved.
    naming: Option<bool>,
    /// The gallery's filter (All, Sidebar, Panel, …), which narrows the token list too.
    area: Area,
    /// The three columns, each a view of its own so that scrolling or animating one redraws
    /// that one alone. What they show is the window's; they observe it.
    list: Entity<TokenListView>,
    gallery: Entity<GalleryView>,
    editor: Entity<TokenEditorView>,
    /// Every change to the file, to undo and redo; kept across a save.
    history: UndoStack,
    /// The text fields that hold typing which is not a step yet.
    typing: undo::Typing,
    /// Ends a drag of a colour after a pause in it.
    gesture_timer: Option<Task<()>>,
    /// This is the window in front, so the Edit menu speaks of its history.
    active: bool,
    /// The sheet or panel over the window, if one is open.
    modal: Option<modal::Modal>,
    /// What the base was last compared with its source as.
    update_check: update_from::Check,
    /// Whether the pick the app shows names the file being edited, and which pick that was. Worked
    /// out when the document or the pick changes, never in a draw (it reads the themes folder).
    own_in_use: bool,
    shown_pick: String,
    /// How many times an edit changed the draft or its starting point, for what is made from
    /// them (the token list's rows) to know when to be made again.
    revision: u64,
    /// The Save theme menu (Save as a copy, Export, Copy JSON, Use in this project).
    save_menu: bool,
    /// Whether Use in this project can write, as of when the menu opened. (Looked up then,
    /// never while drawing: reading the main window's workspace during this window's draw
    /// would bind it to this window.)
    menu_project: bool,
    /// What the last save, export or copy did, under the title row.
    notice: Option<Notice>,
    /// A sheet (the close question, a name clash) is up, waiting for its answer.
    asking: bool,
    /// The sheet's answer, on its way.
    question: Option<Task<()>>,
    /// What the Studio asks of the themes folder (the user's, except in tests).
    themes: Themes,
    /// An Export… waiting for its file dialog.
    export: Option<Task<()>>,
    blur: Blur,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

/// Where a token's new value came from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Edit {
    /// Dragging the color picker: it already shows the value.
    Picker,
    /// The expression field, a reset, a fix, or devctl: the picker has to follow.
    Field,
}

/// What closing the window does right now.
#[derive(Debug, PartialEq, Eq)]
enum Closing {
    Now,
    /// Put the question up.
    Ask,
    /// It is up already.
    AskedAlready,
}

fn closing(dirty: bool, asking: bool, quitting: bool) -> Closing {
    match (dirty && !quitting, asking) {
        (false, _) => Closing::Now,
        (true, false) => Closing::Ask,
        (true, true) => Closing::AskedAlready,
    }
}

/// The save `key` runs, read from the key table's Theme Studio rows (⌘S Save theme, ⇧⌘S Save
/// Copy), which the shortcut sheet lists from the same place.
fn save_key(key: &KeySpec) -> Option<SaveAction> {
    use wormhole_core::keys::keymap::{ActionId, studio_action};
    match studio_action(key)? {
        ActionId::SaveTheme => Some(SaveAction::Save),
        ActionId::DuplicateTheme => Some(SaveAction::Duplicate),
        _ => None,
    }
}

/// Open the Studio window, or close it if it is open (asking first if it holds edits
/// that are not saved).
pub(crate) fn toggle_studio(cx: &mut App) {
    if let Some((handle, studio)) = current_window(cx)
        && handle
            .update(cx, |_, window, cx| studio.update(cx, |studio, cx| studio.request_close(window, cx)))
            .is_ok()
    {
        return;
    }
    open_studio(cx);
}

/// The Studio window while it is open, and its view. The shared state keeps the view untyped
/// (`crate::app_state::Slot`, as it does About's), so it doesn't name the Studio; `None` once the view is
/// gone, as a weak handle that no longer upgrades.
fn current_window(cx: &App) -> Option<(AnyWindowHandle, Entity<StudioWindow>)> {
    let (handle, view) = app(cx).studio_window.clone()?;
    let studio = view.upgrade()?.downcast::<StudioWindow>().ok()?;
    Some((handle, studio))
}

/// Open the Studio window, or bring it to the front.
pub(crate) fn open_studio(cx: &mut App) {
    if let Some((handle, _)) = app(cx).studio_window.clone()
        && handle.update(cx, |_, window, cx| crate::background::activate_window(window, cx)).is_ok()
    {
        return;
    }
    let options = aux_window::window_options(
        wormhole_text::text(wormhole_text::keys::STUDIO_TITLE),
        size(px(WINDOW_SIZE.0), px(WINDOW_SIZE.1)),
        size(px(WINDOW_MIN.0), px(WINDOW_MIN.1)),
        cx,
    );
    // Deferred: the new window renders right away, and this may run inside
    // another window's update.
    cx.defer(move |cx| {
        // Read before the window exists: the front window's project is looked at here, never while
        // this window draws.
        let doc = opening::open_document(&opening_inputs(cx), &Themes::USER);
        let opened = crate::window_open::open(options, cx, |window, cx| {
            cx.new(|cx| StudioWindow::with_themes(Themes::USER, doc, window, cx))
        });
        match opened {
            Ok((handle, studio)) => app_mut(cx).studio_window = Some((handle, studio.downgrade().into())),
            Err(err) => tracing::error!("opening the Theme Studio window failed: {err:#}"),
        }
    });
}

/// What the theme in use says to open the Studio on.
fn opening_inputs(cx: &App) -> opening::Inputs {
    let store = cx.global::<SettingsStore>();
    let shown = settings(cx).shown_theme(store.system().appearance).to_string();
    // A project's theme is edited only while it is in effect and the project may be written to.
    let project = store.has_project_theme().then(|| project_theme(cx)).flatten();
    opening::Inputs { shown, project, in_use: store.base_tokens().clone() }
}

/// The theme file of the project the front terminal is in, when the project is trusted, has its
/// theme switched on and has one.
fn project_theme(cx: &App) -> Option<opening::ProjectTheme> {
    let folder = save::project_for_theme(cx).ok()?;
    let text = folder.theme_json().ok().flatten()?.to_string();
    Some(opening::ProjectTheme { root: folder.root, text })
}

impl StudioWindow {
    /// The file has edits that are not saved.
    fn dirty(&self) -> bool {
        self.doc.dirty()
    }

    /// A window on `doc`, the theme that was in use when it was asked for.
    fn with_themes(themes: Themes, doc: Document, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let picker = cx.new(|cx| ColorPickerState::new(window, cx));
        let expression = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(wormhole_text::text(wormhole_text::keys::STUDIO_EXPRESSION_PLACEHOLDER))
        });
        let name = cx.new(|cx| InputState::new(window, cx).default_value(doc.name().to_string()));
        let slider = cx.new(|_| SliderState::new().min(0.0).max(token_editor::METRIC_MAX).step(1.0));
        let this = cx.entity();
        let list = cx.new(|cx| TokenListView::new(&this, window, cx));
        let gallery = cx.new(|cx| GalleryView::new(&this, window, cx));
        let editor = cx.new(|cx| TokenEditorView::new(&this, cx));
        let filter = list.read(cx).filter.clone();
        let subscriptions = vec![
            cx.subscribe_in(&picker, window, |this, _, event: &ColorPickerEvent, window, cx| {
                let ColorPickerEvent::Change(Some(color)) = event else { return };
                this.set_selected_value(Some(theme_bridge::color(*color).to_hex()), Edit::Picker, window, cx);
            }),
            cx.subscribe_in(&expression, window, |this, input, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    this.typing.expression = true;
                    let value = input.read(cx).value().to_string();
                    this.check_field(&value, cx);
                    this.refresh_completions(cx);
                }
                // Return takes the highlighted name while the list is up; otherwise it applies.
                InputEvent::PressEnter { .. } if this.completion.is_some() => {
                    this.typing.expression = false;
                    this.take_completion(None, window, cx);
                }
                InputEvent::PressEnter { .. } => {
                    // Return makes the typing a step of the file: ⌘Z undoes that, not the typing.
                    this.typing.expression = false;
                    let value = input.read(cx).value().trim().to_string();
                    this.apply_field(value, window, cx);
                }
                InputEvent::Blur => {
                    this.typing.expression = false;
                    this.close_completions(cx);
                }
                InputEvent::Focus => this.typing.expression = false,
            }),
            cx.subscribe_in(&slider, window, |this, _, event: &SliderEvent, window, cx| {
                if let SliderEvent::Change(value) = event {
                    this.set_selected_value(Some(number(value.start().round())), Edit::Picker, window, cx);
                }
            }),
            cx.subscribe_in(&name, window, |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => {
                    this.typing.name = false;
                    this.commit_name(window, cx);
                }
                InputEvent::Blur => {
                    this.typing.name = false;
                    this.naming = None;
                    this.finish_rename(window, cx);
                }
                InputEvent::Change => this.typing.name = true,
                InputEvent::Focus => this.typing.name = false,
            }),
            cx.subscribe(&filter, |this, _, event: &InputEvent, _| match event {
                InputEvent::Change => this.typing.filter = true,
                _ => this.typing.filter = false,
            }),
            cx.observe_global::<SettingsStore>(|this, cx| this.settings_changed(cx)),
            cx.observe_window_activation(window, |this, window, cx| {
                this.active = window.is_window_active();
                this.publish_edit_titles(cx);
            }),
            // However the window goes away (⌘W, Esc, the close button), unsaved
            // edits must stop showing everywhere else.
            cx.on_release(|_, cx| {
                crate::window::menu_state::set_edit_titles(Default::default(), cx);
                SettingsStore::preview_tokens(cx, None);
                cx.refresh_windows();
            }),
        ];
        let translucent = settings(cx).studio_translucent;
        let blur = Blur::install(window, style::wants_blur(settings(cx), translucent));
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        // The window's own close button asks too. (`remove_window`, which the keys and
        // the Close button use, skips this, so they go through `request_close`.)
        let this = cx.weak_entity();
        window.on_window_should_close(cx, move |window, cx| {
            this.update(cx, |studio, cx| studio.should_close(window, cx)).unwrap_or(true)
        });
        let mut studio = Self {
            doc,
            selected: None,
            picker,
            expression,
            field_error: None,
            completion: None,
            slider,
            name,
            renaming: false,
            name_bounds: Rc::default(),
            naming: None,
            area: Area::All,
            list,
            gallery,
            editor,
            history: UndoStack::default(),
            typing: undo::Typing::default(),
            gesture_timer: None,
            active: window.is_window_active(),
            modal: None,
            update_check: update_from::Check::default(),
            own_in_use: false,
            shown_pick: String::new(),
            revision: 0,
            save_menu: false,
            menu_project: false,
            notice: None,
            asking: false,
            question: None,
            themes,
            export: None,
            blur,
            focus_handle,
            _subscriptions: subscriptions,
        };
        // The Studio opens on the main accent color, so its editor is never empty and the
        // gallery already outlines something.
        studio.refresh_update_check();
        studio.refresh_own_in_use(cx);
        studio.select_token(FIRST_TOKEN, window, cx);
        studio
    }

    /// Select a token: the editor shows it, the list opens its band, the gallery outlines its
    /// uses.
    fn select_token(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.selected = Some(name.to_string());
        self.field_error = None;
        self.history.end_gesture();
        self.gesture_timer = None;
        if let Some(group) = group_of(name) {
            self.list.update(cx, |list, cx| list.open_group(group, cx));
        }
        self.show_token(name, true, window, cx);
    }

    /// Fill the editor with `name`'s value. `picker`: point the color picker at it too.
    /// Not while the change came from the picker itself: what comes back is rounded to
    /// 8-bit hex, which would pull the sliders (and the hue of a grey) out from under the
    /// pointer mid-drag.
    fn show_token(&mut self, name: &str, picker: bool, window: &mut Window, cx: &mut Context<Self>) {
        let tokens = cx.global::<SettingsStore>().tokens.clone();
        let current = self.doc.own(name).unwrap_or_else(|| {
            tokens
                .get(name)
                .map(|c| c.to_hex())
                .or_else(|| tokens.metrics.get(name).map(number))
                .unwrap_or_default()
        });
        if picker && let Some(color) = tokens.get(name) {
            self.picker.update(cx, |p, cx| p.set_value(hsla(color), window, cx));
        }
        if picker && let Some(value) = tokens.metrics.get(name).filter(|_| is_metric(name)) {
            self.slider.update(cx, |s, cx| s.set_value(value.min(token_editor::METRIC_MAX), window, cx));
        }
        // The field is rewritten, which it doesn't report as a change: no list for the old text.
        self.completion = None;
        self.expression.update(cx, |e, cx| e.set_value(current, window, cx));
        cx.notify();
    }

    /// Set (or with `None`, reset to derived) the selected token's value, then
    /// re-resolve the draft and preview it everywhere. A value the draft can't take
    /// leaves the last good one in place and says why in the field.
    fn set_selected_value(
        &mut self,
        value: Option<String>,
        from: Edit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(name) = self.selected.clone() else { return };
        self.set_value(&name, value, from, window, cx);
    }

    /// [`Self::set_selected_value`] for any token (a contrast fix changes another one).
    fn set_value(
        &mut self,
        name: &str,
        value: Option<String>,
        from: Edit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut draft = self.doc.file.clone();
        let reset = value.is_none();
        if let Err(err) = places::set_own(&mut draft, self.doc.shown.as_deref(), name, value) {
            self.field_error = Some(FieldError::plain(err));
            cx.notify();
            return;
        }
        match (self.themes.resolve)(&places::folded(&draft, self.doc.shown.as_deref())) {
            Ok(resolved) => {
                let label = if reset {
                    StepLabel::Reset(name.to_string())
                } else {
                    StepLabel::Token(name.to_string())
                };
                self.record_step(label, from == Edit::Picker, cx);
                self.doc.file = draft;
                self.touched(cx);
                self.field_error = None;
                self.notice = None;
                SettingsStore::preview_tokens(cx, Some(resolved));
                cx.refresh_windows();
                self.publish_edit_titles(cx);
                if self.selected.as_deref() == Some(name) {
                    self.show_token(name, from == Edit::Field, window, cx);
                }
            }
            Err(err) => {
                self.field_error = Some(FieldError::from_resolve(&err));
                cx.notify();
            }
        }
    }

    /// Whether the window may close right now. With edits that are not saved as a theme
    /// the answer is no, and a question goes up instead (which closes the window itself
    /// if it is answered).
    fn should_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let quitting = cx.try_global::<AppState>().is_some_and(|app| app.quitting);
        match closing(self.dirty(), self.asking, quitting) {
            Closing::Now => true,
            Closing::Ask => {
                self.ask_before_closing(window, cx);
                false
            }
            Closing::AskedAlready => false,
        }
    }

    /// Esc, ⌘W, ⌥⌘, and the window's own button. (`remove_window` does not ask the close
    /// guard, so they come through here.)
    fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save_menu = false;
        if self.should_close(window, cx) {
            window.remove_window();
        }
    }

    /// Revert to Saved: back to the file as it was last read or saved, as one step to undo.
    fn discard_changes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.record_step(StepLabel::Revert, false, cx);
        self.doc.revert();
        self.doc.settle_shown();
        self.touched(cx);
        self.field_error = None;
        self.notice = None;
        self.publish_edit_titles(cx);
        let name = self.doc.name().to_string();
        self.name.update(cx, |input, cx| input.set_value(name, window, cx));
        SettingsStore::preview_tokens(cx, None);
        cx.refresh_windows();
        if let Some(name) = self.selected.clone() {
            self.select_token(&name, window, cx);
        }
    }

    /// The window's own keys: Save theme and Save Copy as the key table has them (⌘S, ⇧⌘S);
    /// Esc and ⌘W close (Esc first leaves the name being edited or the open menu).
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keys = &event.keystroke;
        let m = &keys.modifiers;
        // A sheet or panel is open: it takes the arrows, Return and Esc.
        if !m.modified() && self.modal_key(keys.key.as_str(), window, cx) {
            cx.stop_propagation();
            return;
        }
        if let Some(action) = save_key(&keystroke_spec(keys)) {
            self.run_save_action(action, window, cx);
            cx.stop_propagation();
            return;
        }
        match keys.key.as_str() {
            "escape" if !m.modified() && self.save_menu => {
                self.save_menu = false;
                cx.notify();
            }
            "escape" if !m.modified() && self.renaming => self.cancel_rename(window, cx),
            "escape" if !m.modified() => self.request_close(window, cx),
            // ⌥⌘, closes it too: the Theme Studio action toggles it.
            "w" if m.platform => self.request_close(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

fn color_chip(ui: UiColors, color: Color) -> Div {
    div().size(px(14.0)).flex_none().rounded(ui.radius_sm).outlined(ui, ui.border).bg(hsla(color))
}

/// A metric's swatch: a box with that corner radius, or that outline (a focus ring in
/// the focus color).
fn metric_chip(ui: UiColors, name: &str, value: f32) -> Div {
    let chip = div().size(px(14.0)).flex_none().bg(ui.hover);
    let line = UiColors { border_width: px(value), ..ui };
    if name.ends_with("border_width") || name.ends_with("divider_width") {
        chip.rounded(ui.radius_sm).outlined(line, ui.muted)
    } else if name.ends_with("focus_ring_width") {
        chip.rounded(ui.radius_sm).outlined(line, ui.focus)
    } else {
        chip.rounded(px(value)).outlined(ui, ui.muted)
    }
}

fn tokens_of(cx: &App) -> Arc<ThemeTokens> {
    cx.global::<SettingsStore>().tokens.clone()
}

fn t(tokens: &ThemeTokens, name: &str) -> Hsla {
    token(tokens, name)
}

#[cfg(all(test, feature = "devctl"))]
mod draw_tests;
#[cfg(all(test, feature = "devctl"))]
mod guard_tests;
#[cfg(all(test, feature = "devctl"))]
mod proof_tests;
#[cfg(test)]
mod tests;
#[cfg(all(test, feature = "devctl"))]
mod window_tests;

/// Canonical text of every colour of a theme, for tests that compare two resolutions.
#[cfg(all(test, feature = "devctl"))]
fn tests_support_dump(t: &ThemeTokens) -> String {
    let mut out = format!("{:?}|{:?}|{:?}|{:?}|", t.appearance, t.terminal, t.syntax, t.metrics);
    for (k, v) in t.ui_tokens() {
        out.push_str(&format!("{k}={};", v.to_hex()));
    }
    out
}
#[cfg(all(test, feature = "devctl"))]
mod review_tests;
#[cfg(all(test, feature = "devctl"))]
mod v1_tests;
