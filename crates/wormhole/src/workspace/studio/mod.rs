//! Theme Studio: its own window (⌥⌘,) with the token list on the left, a
//! gallery of every component on the live draft in the middle, and the token editor on the
//! right, so an edit shows where it lands.
//!
//! Edits go into a small theme *file* that `extends` the current theme and is
//! re-resolved on every change, so derived tokens follow (changing `primary`
//! moves focus rings and primary buttons too) and saving writes only overrides.
//! The resolved draft is previewed app-wide through `SettingsStore`, so the
//! main window follows along; closing the Studio window drops the preview.

mod completion;
mod contrast;
mod draft;
mod gallery;
mod save;
mod title;
mod token_editor;
mod token_list;

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::sync::Arc;

use gpui_kit::component::color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use wormhole_core::keyspec::KeySpec;
use wormhole_core::tokens::{self, Color, METRIC_TOKENS, ThemeFile, ThemeTokens, UI_TOKENS};

use super::aux_window;
use super::*;
use crate::terminal_view::keystroke_spec;
use crate::theme_bridge::{self, hsla, token};
use crate::vibrancy::Blur;
use draft::{ANSI_NAMES, Show, Themes, draft_for, group_of, is_metric, number, override_of, set_override};
use gallery::Area;
use save::Notice;
use title::SaveAction;
use token_editor::FieldError;

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
/// gallery included), which reaches it through `use super::*`.
fn ui(cx: &App) -> UiColors {
    crate::settings::window_ui(cx, settings(cx).studio_translucent)
}

pub(crate) struct StudioWindow {
    /// The theme being edited, as overrides on top of `draft.extends`.
    draft: ThemeFile,
    /// The draft when the window opened or was last saved: what counts as a change.
    initial: ThemeFile,
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
    /// The token list's filter text and what it shows.
    filter: Entity<InputState>,
    show: Show,
    /// The token list's bands that are open.
    open_groups: BTreeSet<&'static str>,
    /// The gallery's filter (All, Sidebar, Panel, …).
    area: Area,
    /// The Save theme menu (Save as a copy, Export, Copy JSON, Use in this project).
    save_menu: bool,
    /// Whether Use in this project can write, as of when the menu opened. (Looked up then,
    /// never while drawing: reading the main window's workspace during this window's draw
    /// would bind it to this window.)
    menu_project: bool,
    /// What the last save, export or copy did, under the title row.
    notice: Option<Notice>,
    /// The editor of the gallery's item that is being edited.
    item_editor: Entity<InputState>,
    /// The gallery's text field.
    sample_input: Entity<InputState>,
    /// The draft has edits that are not saved as a theme.
    dirty: bool,
    /// A sheet (the close question, a name clash) is up, waiting for its answer.
    asking: bool,
    /// The sheet's answer, on its way.
    question: Option<Task<()>>,
    /// What the Studio asks of the themes folder (the user's, except in tests).
    themes: Themes,
    /// The syntax sample, parsed once.
    sample: RefCell<Option<gallery::CodeSample>>,
    gallery_scroll: ScrollHandle,
    list_scroll: ScrollHandle,
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
    use wormhole_core::keymap::{ActionId, studio_action};
    match studio_action(key)? {
        ActionId::SaveTheme => Some(SaveAction::Save),
        ActionId::SaveThemeCopy => Some(SaveAction::SaveCopy),
        _ => None,
    }
}

impl Workspace {
    /// ⌥⌘, (Wormhole → Theme Studio): open the Studio window, or close it if it is open.
    #[cfg(feature = "devctl")]
    pub(crate) fn toggle_studio(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        toggle_studio(cx);
    }
}

/// Open the Studio window, or close it if it is open (asking first if it holds edits
/// that are not saved).
pub(crate) fn toggle_studio(cx: &mut App) {
    if let Some((handle, studio)) = app(cx).studio_window.clone()
        && let Ok(Ok(())) = handle
            .update(cx, |_, window, cx| studio.update(cx, |studio, cx| studio.request_close(window, cx)))
    {
        return;
    }
    open_studio(cx);
}

/// Open the Studio window, or bring it to the front.
pub(crate) fn open_studio(cx: &mut App) {
    if let Some((handle, _)) = app(cx).studio_window.clone()
        && handle.update(cx, |_, window, cx| crate::background::activate_window(window, cx)).is_ok()
    {
        return;
    }
    let options = aux_window::window_options(
        "Theme Studio",
        size(px(WINDOW_SIZE.0), px(WINDOW_SIZE.1)),
        size(px(WINDOW_MIN.0), px(WINDOW_MIN.1)),
        cx,
    );
    // Deferred: the new window renders right away, and this may run inside
    // another window's update.
    cx.defer(move |cx| {
        let opened = crate::background::open_window(options, cx, |window, cx| {
            cx.new(|cx| StudioWindow::new(window, cx))
        });
        match opened {
            Ok((handle, studio)) => app_mut(cx).studio_window = Some((handle, studio.downgrade())),
            Err(err) => tracing::error!("opening the Theme Studio window failed: {err:#}"),
        }
    });
}

#[cfg(feature = "devctl")]
impl Workspace {
    /// Run `f` on the open Studio (devctl drives it from a window's tick).
    pub(crate) fn with_studio(
        &self,
        cx: &mut App,
        f: impl FnOnce(&mut StudioWindow, &mut Window, &mut Context<StudioWindow>),
    ) -> anyhow::Result<()> {
        with_studio(cx, f)
    }

    /// devctl: render the Studio window to a PNG.
    pub(crate) fn devctl_studio_shot(&self, path: &std::path::Path, cx: &mut App) -> anyhow::Result<()> {
        devctl_studio_shot(path, cx)
    }

    /// devctl `state`: the Studio's line while it is open: whether it has edits, and whether a
    /// sheet (which no shot shows) is waiting for an answer.
    pub(crate) fn devctl_studio_state(&self, cx: &App) -> Option<String> {
        let (_, studio) = app(cx).studio_window.clone()?;
        let studio = studio.upgrade()?;
        let studio = studio.read(cx);
        Some(format!("studio dirty={} asking={}", studio.dirty, studio.asking))
    }
}

#[cfg(feature = "devctl")]
fn with_studio(
    cx: &mut App,
    f: impl FnOnce(&mut StudioWindow, &mut Window, &mut Context<StudioWindow>),
) -> anyhow::Result<()> {
    let (handle, studio) =
        app(cx).studio_window.clone().ok_or_else(|| anyhow::anyhow!("Theme Studio not open"))?;
    let studio = studio.upgrade().ok_or_else(|| anyhow::anyhow!("Theme Studio closed"))?;
    handle.update(cx, |_, window, cx| studio.update(cx, |s, cx| f(s, window, cx)))
}

#[cfg(feature = "devctl")]
fn devctl_studio_shot(path: &std::path::Path, cx: &mut App) -> anyhow::Result<()> {
    let (handle, _) =
        app(cx).studio_window.clone().ok_or_else(|| anyhow::anyhow!("Theme Studio not open"))?;
    // Draw first: a window behind others may not have redrawn since its last change.
    let image = handle.update(cx, |_, window, cx| {
        window.draw(cx).clear(cx);
        window.render_to_image()
    })??;
    image.save(path)?;
    Ok(())
}

impl StudioWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::with_themes(Themes::USER, window, cx)
    }

    fn with_themes(themes: Themes, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let base = cx.global::<SettingsStore>().base_tokens().clone();
        let draft = draft_for(&base, themes.resolve);

        let picker = cx.new(|cx| ColorPickerState::new(window, cx));
        let expression =
            cx.new(|cx| InputState::new(window, cx).placeholder("#hex or expression, e.g. primary / 15%"));
        let name =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.name.clone().unwrap_or_default()));
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter tokens"));
        let slider = cx.new(|_| SliderState::new().min(0.0).max(token_editor::METRIC_MAX).step(1.0));
        let item_editor = cx.new(|cx| InputState::new(window, cx).default_value("Rename this task"));
        let sample_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search themes"));
        let subscriptions = vec![
            cx.subscribe_in(&picker, window, |this, _, event: &ColorPickerEvent, window, cx| {
                let ColorPickerEvent::Change(Some(color)) = event else { return };
                this.set_selected_value(Some(theme_bridge::color(*color).to_hex()), Edit::Picker, window, cx);
            }),
            cx.subscribe_in(&expression, window, |this, input, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    let value = input.read(cx).value().to_string();
                    this.check_field(&value, cx);
                    this.refresh_completions(cx);
                }
                // Return takes the highlighted name while the list is up; otherwise it applies.
                InputEvent::PressEnter { .. } if this.completion.is_some() => {
                    this.take_completion(None, window, cx);
                }
                InputEvent::PressEnter { .. } => {
                    let value = input.read(cx).value().trim().to_string();
                    this.apply_field(value, window, cx);
                }
                InputEvent::Blur => this.close_completions(cx),
                InputEvent::Focus => {}
            }),
            cx.subscribe_in(&slider, window, |this, _, event: &SliderEvent, window, cx| {
                if let SliderEvent::Change(value) = event {
                    this.set_selected_value(Some(number(value.start().round())), Edit::Picker, window, cx);
                }
            }),
            cx.subscribe_in(&name, window, |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } | InputEvent::Blur => this.finish_rename(window, cx),
                _ => {}
            }),
            cx.observe(&filter, |_, _, cx| cx.notify()),
            // However the window goes away (⌘W, Esc, the close button), unsaved
            // edits must stop showing everywhere else.
            cx.on_release(|_, cx| {
                SettingsStore::preview_tokens(cx, None);
                cx.refresh_windows();
            }),
        ];
        let translucent = settings(cx).studio_translucent;
        let blur = Blur::install(window, aux_window::wants_blur(settings(cx), translucent));
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        // The window's own close button asks too. (`remove_window`, which the keys and
        // the Close button use, skips this, so they go through `request_close`.)
        let this = cx.weak_entity();
        window.on_window_should_close(cx, move |window, cx| {
            this.update(cx, |studio, cx| studio.should_close(window, cx)).unwrap_or(true)
        });
        let mut studio = Self {
            initial: draft.clone(),
            draft,
            selected: None,
            picker,
            expression,
            field_error: None,
            completion: None,
            slider,
            name,
            renaming: false,
            filter,
            show: Show::All,
            open_groups: BTreeSet::new(),
            area: Area::All,
            save_menu: false,
            menu_project: false,
            notice: None,
            item_editor,
            sample_input,
            dirty: false,
            asking: false,
            question: None,
            themes,
            sample: RefCell::new(None),
            gallery_scroll: ScrollHandle::new(),
            list_scroll: ScrollHandle::new(),
            export: None,
            blur,
            focus_handle,
            _subscriptions: subscriptions,
        };
        // The Studio opens on the main accent color, so its editor is never empty and the
        // gallery already outlines something.
        studio.select_token(FIRST_TOKEN, window, cx);
        studio
    }

    /// Select a token: the editor shows it, the list opens its band, the gallery outlines its
    /// uses.
    pub(super) fn select_token(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.selected = Some(name.to_string());
        self.field_error = None;
        if let Some(group) = group_of(name) {
            self.open_groups.insert(group);
        }
        self.show_token(name, true, window, cx);
    }

    /// Fill the editor with `name`'s value. `picker`: point the color picker at it too.
    /// Not while the change came from the picker itself: what comes back is rounded to
    /// 8-bit hex, which would pull the sliders (and the hue of a grey) out from under the
    /// pointer mid-drag.
    fn show_token(&mut self, name: &str, picker: bool, window: &mut Window, cx: &mut Context<Self>) {
        let tokens = cx.global::<SettingsStore>().tokens.clone();
        let current = override_of(&self.draft, name).unwrap_or_else(|| {
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
        let mut draft = self.draft.clone();
        if let Err(err) = set_override(&mut draft, name, value) {
            self.field_error = Some(FieldError::plain(err));
            cx.notify();
            return;
        }
        match (self.themes.resolve)(&draft) {
            Ok(resolved) => {
                self.draft = draft;
                self.dirty = true;
                self.field_error = None;
                self.notice = None;
                SettingsStore::preview_tokens(cx, Some(resolved));
                cx.refresh_windows();
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
        match closing(self.dirty, self.asking, quitting) {
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

    /// Discard: back to the theme the draft started from.
    fn discard_changes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let base = cx.global::<SettingsStore>().base_tokens().clone();
        self.draft = draft_for(&base, self.themes.resolve);
        self.initial = self.draft.clone();
        self.dirty = false;
        self.field_error = None;
        self.notice = None;
        let name = self.draft.name.clone().unwrap_or_default();
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

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_select(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.select_token(name, window, cx);
    }

    /// `studio-save <name>`: save under that name, replacing a theme of that name without a
    /// question (a script that saves twice under one name means to). Without a name, Save
    /// theme as the person's ⌘S does it, so a name clash shows its notice.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_save(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if name.is_empty() {
            return self.request_save(window, cx);
        }
        let name = name.to_string();
        self.name.update(cx, |i, cx| i.set_value(name.clone(), window, cx));
        self.write_theme(name, cx);
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_scroll(&mut self, y: f32, cx: &mut Context<Self>) {
        self.gallery_scroll.set_offset(point(px(0.0), px(-y)));
        cx.notify();
    }

    /// Type `value` into the field and press Return, as the person would: an expression
    /// that doesn't work stays in the field with its error.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_set(&mut self, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.expression.update(cx, |e, cx| e.set_value(value.to_string(), window, cx));
        self.check_field(value, cx);
        self.apply_field(value.trim().to_string(), window, cx);
    }

    /// `studio-type <text>`: type `text` into the field without pressing Return, so what the
    /// field offers for it shows.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_type(&mut self, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.expression.update(cx, |e, cx| e.set_value(value.to_string(), window, cx));
        self.check_field(value, cx);
        self.refresh_completions(cx);
    }

    /// `studio-filter <all|changed|low> [text]` and `studio-area <area>`.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_filter(&mut self, arg: &str, window: &mut Window, cx: &mut Context<Self>) {
        let (show, text) = arg.split_once(' ').unwrap_or((arg, ""));
        self.show = match show {
            "changed" => Show::Changed,
            "low" => Show::LowContrast,
            _ => Show::All,
        };
        self.filter.update(cx, |f, cx| f.set_value(text.to_string(), window, cx));
        cx.notify();
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_area(&mut self, arg: &str, cx: &mut Context<Self>) {
        self.area =
            Area::ALL.iter().copied().find(|a| a.label().eq_ignore_ascii_case(arg)).unwrap_or_default();
        cx.notify();
    }

    /// `studio-resize <w> <h>`: the window's size, as far as its minimum lets it (AppKit holds
    /// the minimum against the person's hand on the edge, not against a resize from code).
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_resize(&mut self, arg: &str, window: &mut Window) {
        let mut dims = arg.split_whitespace().filter_map(|n| n.parse::<f32>().ok());
        if let (Some(w), Some(h)) = (dims.next(), dims.next()) {
            window.resize(size(px(w.max(WINDOW_MIN.0)), px(h.max(WINDOW_MIN.1))));
        }
    }

    /// `studio-menu [save|copy|switch|json|project]`: open the Save theme menu, or pick
    /// one of its entries (Export's file dialog is the person's to answer, so it isn't one).
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_menu(&mut self, arg: &str, cx: &mut Context<Self>) {
        let action = match arg {
            "save" => Some(SaveAction::Save),
            "copy" => Some(SaveAction::SaveCopy),
            "switch" => Some(SaveAction::SaveCopyAndSwitch),
            "json" => Some(SaveAction::CopyJson),
            "project" => Some(SaveAction::UseInProject),
            _ => None,
        };
        // devctl runs inside the main window's update, where its workspace (which Use in this
        // project reads) can't be read: go on once that is over.
        cx.defer(move |cx| {
            let _ = with_studio(cx, |studio, window, cx| match action {
                Some(action) => studio.run_save_action(action, window, cx),
                None => studio.open_save_menu(cx),
            });
        });
    }
}

impl Render for StudioWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let translucent = settings(cx).studio_translucent;
        self.blur.set_enabled(aux_window::wants_blur(settings(cx), translucent));
        let surfaces = aux_window::surfaces(cx, translucent);
        div()
            .id("studio-window")
            .track_focus(&self.focus_handle)
            .key_context("StudioWindow")
            .on_key_down(cx.listener(Self::on_key))
            .size_full()
            .flex()
            .flex_col()
            .text_color(ui.text)
            .child(self.render_title_bar(surfaces.panel, chrome::lights_end(window), cx))
            .children(self.render_notice(surfaces.background, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    // Each region paints its own surface (no stacked translucent layers).
                    .child(self.render_token_list(surfaces.panel, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .flex()
                            .bg(surfaces.background)
                            .child(self.render_gallery(window, cx)),
                    )
                    .child(self.render_token_editor(surfaces.panel_right, cx)),
            )
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
    if name.ends_with("border_width") {
        chip.rounded(ui.radius_sm).outlined(line, ui.muted)
    } else if name.ends_with("focus_ring_width") {
        chip.rounded(ui.radius_sm).outlined(line, ui.focus)
    } else {
        chip.rounded(px(value)).outlined(ui, ui.muted)
    }
}

pub(super) fn tokens_of(cx: &App) -> Arc<ThemeTokens> {
    cx.global::<SettingsStore>().tokens.clone()
}

pub(super) fn t(tokens: &ThemeTokens, name: &str) -> Hsla {
    token(tokens, name)
}

#[cfg(test)]
mod tests;
#[cfg(all(test, feature = "devctl"))]
mod window_tests;
