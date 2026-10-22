//! The settings window: the nav on the left (title, search, the sections in four
//! groups), a section's page on the right. Opens with ⌘,. Typing in the search shows the
//! settings it finds as live rows; Esc clears the search, then closes the window.
//!
//! Above the window and the shared state: it may use them; the app root, the palette and the status
//! item use it (and, until phase 5, the welcome's agents step). The shared state keeps its page untyped.

pub(crate) mod devctl;
mod font_picker;
mod nav;
mod preview_column;
mod recorder;
mod registry;
mod sections;

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::color_picker::{ColorPickerEvent, ColorPickerState};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::slider::{SliderEvent, SliderState, SliderValue};
use gpui_kit::*;
use wormhole_core::config::settings_index::{self, Query};
use wormhole_core::theme::fonts::FontSlot;
use wormhole_core::view::sidebar::Density;

use crate::platform::vibrancy::Blur;
use crate::settings::{SettingsStore, update_settings};
use crate::settings::{UiColors, settings};
use crate::style::{self, TextRole};
use crate::theme_bridge;
use crate::workspace::app::{AppState, app, app_mut};
use crate::workspace::aux_window;
use crate::workspace::chrome::TitleDrag as _;
use font_picker::FontPickerState;
use preview_column::{COLUMN_SWITCH_W, PREVIEW_W};

pub(crate) use sections::open_new_profile_sheet;
pub(crate) use wormhole_core::config::settings_index::SectionId as Section;

/// Where a page's title row starts: under the traffic lights, which the nav runs beneath like
/// System Settings.
const PAGE_TOP: f32 = style::TITLE_ROW_H;
/// The content column: the widest a page grows.
const COLUMN_W: f32 = 760.0;
/// A page's intro line wraps at a readable length, not across the whole column.
const INTRO_W: f32 = 560.0;
/// The card that says nothing matched.
const NO_RESULTS_W: f32 = 420.0;
/// The window: the size of a page (the nav and the content column) and the least the window
/// shrinks to. It opens wider than the page by the preview column (`preview_column`), to the
/// right, so the page keeps its width.
const WINDOW_SIZE: (f32, f32) = (920.0, 720.0);
const WINDOW_MIN: (f32, f32) = (640.0, 420.0);

/// What the Terminals & sidebar preview shows beside the current look while the pointer is
/// over another option.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SidebarLook {
    Density(Density),
}

/// The settings window's colours: an opaque window keeps the theme's own text colours, a
/// translucent one the main window's. It shadows `settings::ui` for everything drawn in this
/// window, whose files import it.
fn ui(cx: &App) -> UiColors {
    crate::settings::window_ui(cx, settings(cx).settings_translucent)
}

/// What the pages show of the window in front: its terminal's project (the "set by this project"
/// marks). Read when Settings opens and once a
/// second after, never in a frame: GPUI makes an entity read while a window draws that window's,
/// so the workspace would run its tick in Settings, and lose it when Settings closed.
#[derive(Clone, Debug, Default, PartialEq)]
struct FrontWindow {
    project: Option<PathBuf>,
}

impl FrontWindow {
    fn read(cx: &App) -> Self {
        AppState::front_window(cx)
            .and_then(|window| {
                window.workspace.read_with(cx, |ws, _| Self { project: ws.active_project_root() }).ok()
            })
            .unwrap_or_default()
    }
}

pub(crate) struct SettingsWindow {
    section: Section,
    /// The window in front, as the last refresh read it.
    front: FrontWindow,
    search: Entity<InputState>,
    /// The search shows its results page; picking a section from it shows that section with
    /// the matches marked, until the search changes again.
    show_results: bool,
    font_picker: FontPickerState,
    /// A shortcut field waiting for keys, or showing keys it refused.
    recording: Option<recorder::Recording>,
    /// Appearance → the "+" after the primary swatches: any color.
    primary_picker: Entity<ColorPickerState>,
    /// The select whose menu is open (by its row id), and the search box the select menus share.
    open_select: Option<&'static str>,
    menu_search: Entity<InputState>,
    opacity_slider: Entity<SliderState>,
    line_height_slider: Entity<SliderState>,
    /// Fonts → which font the specimen shows.
    specimen: FontSlot,
    /// Appearance → every theme, not just the first rows.
    themes_all: bool,
    /// Terminals & sidebar → the option under the pointer, previewed beside the current look.
    sidebar_preview: Option<SidebarLook>,
    /// The window is wide enough for the preview column (read at each draw): the previews are in
    /// it, and their pages leave them out.
    preview_column: bool,
    /// Advanced → "Reset to defaults…" was pressed: the row asks before it resets.
    confirm_reset: bool,
    /// Coding agents → the agents that aren't on PATH are listed.
    others_open: bool,
    /// Profile → the fields of the profile Wormhole is in (made again when it changes).
    profile_form: Option<sections::ProfileForm>,
    /// Profile → "Delete profile…" was pressed: the row asks before it deletes.
    confirm_delete_profile: bool,
    /// The "New profile" sheet over the window.
    new_profile: Option<sections::NewProfileSheet>,
    /// Something that went wrong on a page (an import, writing an agent's hooks), shown at the
    /// top of that page until the page changes.
    notice: Option<(Section, SharedString)>,
    /// The page's scroll position: a page opens at its top.
    content_scroll: ScrollHandle,
    /// The row the palette asked for (`reveal_row`): marked, and its anchor in the page, until
    /// another section or a search takes over.
    revealed: Option<(&'static str, ScrollAnchor)>,
    /// The revealed row still has to be scrolled to (once the page is laid out).
    reveal_pending: bool,
    projects: sections::ProjectPicker,
    theme_picker: sections::ThemePicker,
    /// What the Agents page shows; read in the background (see `AgentsSnapshot`).
    agents: Option<sections::AgentsSnapshot>,
    agents_task: Option<Task<()>>,
    /// Coding agents → Command line tool: where it stands.
    cli_tool: sections::CliToolPanel,
    /// The window's rem size as of the last draw, for boxes that are sized in rems and drawn
    /// from a cache (the theme cards).
    rem: Pixels,
    blur: Blur,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
    /// Keeps the project picker's list current.
    _refresh: Task<()>,
}

/// The settings window while it is open, and its page. The shared state keeps the page untyped, so it
/// doesn't name this window, which sits above it.
fn current_window(cx: &App) -> Option<(AnyWindowHandle, Entity<SettingsWindow>)> {
    let (handle, page) = app(cx).settings_window.clone()?;
    Some((handle, page.downcast::<SettingsWindow>().ok()?))
}

/// Bring up the settings window (showing `section`), or focus it if it is open.
pub(crate) fn open_settings(section: Option<Section>, cx: &mut App) {
    if let Some((handle, page)) = current_window(cx) {
        let shown = handle.update(cx, |_, window, cx| {
            page.update(cx, |page, cx| {
                if let Some(section) = section {
                    page.show_section(section, cx);
                }
                cx.notify();
            });
            crate::background::activate_window(window, cx);
        });
        if shown.is_ok() {
            return;
        }
    }
    let mut options = aux_window::window_options(
        "Settings",
        size(px(WINDOW_SIZE.0), px(WINDOW_SIZE.1)),
        size(px(WINDOW_MIN.0), px(WINDOW_MIN.1)),
        cx,
    );
    options.window_bounds = Some(WindowBounds::Windowed(opening_bounds(cx)));
    // Deferred: the new window renders right away, and this may run inside
    // another window's update.
    cx.defer(move |cx| {
        let opened = crate::background::open_window(options, cx, |window, cx| {
            cx.new(|cx| SettingsWindow::new(section, window, cx))
        });
        match opened {
            Ok((handle, page)) => app_mut(cx).settings_window = Some((handle, page.into())),
            Err(err) => tracing::error!("opening the settings window failed: {err:#}"),
        }
    });
}

/// Where a new window opens: centred as a window of [`WINDOW_SIZE`] would be, then grown to the
/// right by the preview column, so the page stays where it was. A screen too narrow for that
/// (the Dock, a small display) moves it left and then narrows it, to stay inside the usable area;
/// below [`COLUMN_SWITCH_W`] the previews are inline in their pages.
fn opening_bounds(cx: &App) -> Bounds<Pixels> {
    let page = Bounds::centered(None, size(px(WINDOW_SIZE.0), px(WINDOW_SIZE.1)), cx);
    match cx.primary_display() {
        Some(display) => grown_right(page, PREVIEW_W, display.visible_bounds()),
        None => Bounds { size: size(page.size.width + px(PREVIEW_W), page.size.height), ..page },
    }
}

/// `page` with `extra` more width on its right: its left edge stays, unless the screen's usable
/// area `screen` is too narrow for that, which moves the window left, and then narrows it.
fn grown_right(page: Bounds<Pixels>, extra: f32, screen: Bounds<Pixels>) -> Bounds<Pixels> {
    let width = (page.size.width + px(extra)).min(screen.size.width);
    let overflow = (page.origin.x + width - (screen.origin.x + screen.size.width)).max(px(0.0));
    let x = (page.origin.x - overflow).max(screen.origin.x);
    Bounds { origin: point(x, page.origin.y), size: size(width, page.size.height) }
}

/// Bring up the settings window at the row `id` itself (the command palette): its
/// section, scrolled so the row sits a title row below the top, with that one row marked (a
/// search would mark every row with the same words). Until another section is picked or a search
/// starts.
pub(crate) fn reveal_row(id: &str, cx: &mut App) {
    let Some((section, _, entry)) = settings_index::entry(id) else { return };
    // Never open a section this build hides (a Preview section whose flag is off): a stale recent
    // id could still name one, and the page would only bounce back to Appearance.
    if !section.is_visible(settings(cx)) {
        return;
    }
    open_settings(Some(section), cx);
    // After `open_settings`'s own deferred opening, so the window exists by then.
    cx.defer(move |cx| {
        let Some((handle, page)) = current_window(cx) else { return };
        let shown = handle.update(cx, |_, window, cx| {
            page.update(cx, |page, cx| {
                page.clear_search(window, cx);
                page.show_section(section, cx);
                let anchor = ScrollAnchor::for_handle(page.content_scroll.clone());
                page.revealed = Some((entry.id, anchor));
                page.reveal_pending = true;
                cx.notify();
            });
            crate::background::activate_window(window, cx);
        });
        if let Err(err) = shown {
            tracing::warn!("revealing setting {}: {err:#}", entry.id);
        }
    });
}

/// How far below the page's top a revealed row lands: a title row, so it reads as the first
/// thing on the page rather than as cut off.
const REVEAL_INSET: f32 = style::TITLE_ROW_H;

/// The icon the settings nav gives a section (the palette's settings show it too).
pub(crate) fn section_icon(section: Section) -> IconName {
    nav::section_icon(section)
}

/// A slider over `range` in `step`s, starting at `value`.
fn slider(min: f32, max: f32, step: f32, value: f32, cx: &mut App) -> Entity<SliderState> {
    cx.new(|_| SliderState::new().min(min).max(max).step(step).default_value(value))
}

fn slider_value(value: &SliderValue) -> f32 {
    match value {
        SliderValue::Single(value) => *value,
        SliderValue::Range(_, end) => *end,
    }
}

impl SettingsWindow {
    fn new(section: Option<Section>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let current = settings(cx).clone();
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search settings"));
        let menu_search = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));
        let font_picker = FontPickerState::new(window, cx);
        let primary_picker = cx.new(|cx| ColorPickerState::new(window, cx));
        let asked_opacity = cx.global::<SettingsStore>().asked_opacity();
        let opacity_slider = slider(30.0, 100.0, 1.0, (asked_opacity * 100.0).round(), cx);
        let line_height_slider = slider(1.0, 2.0, 0.05, current.line_height, cx);
        let (projects, project_subscriptions) = sections::ProjectPicker::new(window, cx);
        let mut subscriptions = vec![
            cx.subscribe_in(&primary_picker, window, |_, _, event: &ColorPickerEvent, _, cx| {
                let ColorPickerEvent::Change(Some(color)) = event else { return };
                let hex = theme_bridge::color(*color).with_alpha(1.0).to_hex();
                // Dragging a slider fires for shades that round to the same hex.
                if settings(cx).primary_color != hex {
                    update_settings(cx, |s| s.choose_primary(hex));
                }
            }),
            cx.subscribe_in(&search, window, |this, input, event: &InputEvent, _, cx| {
                if let InputEvent::Change = event {
                    this.show_results = !Query::new(&input.read(cx).value()).is_empty();
                    // The search's marks take over from a revealed row's.
                    if this.show_results {
                        this.revealed = None;
                    }
                    cx.notify();
                }
            }),
            cx.subscribe_in(&menu_search, window, |_, _, event: &InputEvent, _, cx| {
                if let InputEvent::Change = event {
                    cx.notify();
                }
            }),
            cx.subscribe_in(&font_picker.search, window, |_, _, event: &InputEvent, _, cx| {
                if let InputEvent::Change = event {
                    cx.notify();
                }
            }),
            cx.subscribe(&opacity_slider, |_, _, event: &SliderEvent, cx| {
                let SliderEvent::Change(value) = event else { return };
                let opacity = (slider_value(value) / 100.0).clamp(0.3, 1.0);
                // Against the opacity asked for: Reduce Transparency's 100 % is not a setting.
                if (cx.global::<SettingsStore>().asked_opacity() - opacity).abs() > f32::EPSILON {
                    update_settings(cx, |s| s.opacity = opacity);
                }
            }),
            cx.subscribe(&line_height_slider, |_, _, event: &SliderEvent, cx| {
                let SliderEvent::Change(value) = event else { return };
                // Steps of 0.05, without the float noise the slider adds.
                let line_height = (slider_value(value) * 20.0).round() / 20.0;
                if (settings(cx).line_height - line_height).abs() > f32::EPSILON {
                    update_settings(cx, |s| s.line_height = line_height);
                }
            }),
        ];
        subscriptions.extend(project_subscriptions);
        // Closed while its field records: the hotkey comes back with the window gone.
        subscriptions.push(cx.on_release(|_, cx| {
            if cx.has_global::<AppState>() {
                app_mut(cx).menu.pause_hotkey(false);
            }
        }));
        let theme_picker = sections::ThemePicker::new(window, cx);
        let translucent = settings(cx).settings_translucent;
        let blur = Blur::install(window, aux_window::wants_blur(settings(cx), translucent));
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        // Sessions move between folders and windows come to the front; keep the project list
        // and what the pages show of the front window current while the window is open.
        let refresh = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let alive = this.update_in(cx, |this, window, cx| {
                    this.read_front(cx);
                    this.refresh_known_projects(window, cx);
                    this.theme_picker.sync_placeholder(window, cx);
                });
                if alive.is_err() {
                    break;
                }
            }
        });
        let mut this = Self {
            section: section.unwrap_or(Section::Appearance),
            front: FrontWindow::read(cx),
            search,
            show_results: false,
            font_picker,
            recording: None,
            primary_picker,
            open_select: None,
            menu_search,
            opacity_slider,
            line_height_slider,
            specimen: FontSlot::Terminal,
            themes_all: false,
            sidebar_preview: None,
            preview_column: false,
            confirm_reset: false,
            others_open: false,
            profile_form: None,
            confirm_delete_profile: false,
            new_profile: None,
            notice: None,
            content_scroll: ScrollHandle::new(),
            revealed: None,
            reveal_pending: false,
            projects,
            theme_picker,
            agents: None,
            agents_task: None,
            cli_tool: sections::CliToolPanel::read(),
            rem: window.rem_size(),
            blur,
            focus_handle,
            _subscriptions: subscriptions,
            _refresh: refresh,
        };
        // The nav marks an agent whose hooks need repair, so the agents are read up front.
        this.load_agents(cx);
        this.load_cli_tool(cx);
        this
    }

    /// See [`FrontWindow`]: from the refresh, never from `render`.
    fn read_front(&mut self, cx: &mut Context<Self>) {
        let front = FrontWindow::read(cx);
        if front != self.front {
            self.front = front;
            cx.notify();
        }
    }

    fn show_section(&mut self, section: Section, cx: &mut Context<Self>) {
        if self.section != section || self.show_results {
            self.notice = None;
            self.content_scroll.set_offset(point(px(0.0), px(0.0)));
        }
        self.section = section;
        self.show_results = false;
        self.open_select = None;
        self.confirm_reset = false;
        self.confirm_delete_profile = false;
        self.revealed = None;
        cx.notify();
    }

    /// The row `reveal_row` asked for, if it is on this page: `render_page` marks it and
    /// anchors it, so the scroll can find it once the page is laid out.
    fn revealed_row(&self) -> Option<(&'static str, ScrollAnchor)> {
        self.revealed.clone().filter(|_| !self.show_results)
    }

    /// Scroll to the revealed row, registered from the draw that laid it out: the anchor knows
    /// where the row is only after that draw, and the scroll takes effect a frame later.
    fn scroll_to_revealed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.reveal_pending {
            return;
        }
        let Some((_, anchor)) = self.revealed_row() else { return };
        self.reveal_pending = false;
        cx.on_next_frame(window, move |_, window, cx| {
            anchor.scroll_to(window, cx);
            // Right after the anchor's own next-frame scroll: the inset, and a redraw, which
            // `set_offset` doesn't ask for.
            cx.on_next_frame(window, |this, _, cx| {
                let offset = this.content_scroll.offset();
                this.content_scroll.set_offset(point(offset.x, offset.y + px(REVEAL_INSET)));
                cx.notify();
            });
        });
    }

    fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.update(cx, |search, cx| search.set_value("", window, cx));
        self.show_results = false;
        cx.notify();
    }

    /// Esc: whatever is open first (a menu, the reset question), then the search, then the
    /// window.
    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.new_profile.take().is_some()
            || self.font_picker.open.take().is_some()
            || self.open_select.take().is_some()
            || self.end_recording(cx)
            || std::mem::take(&mut self.confirm_reset)
            || std::mem::take(&mut self.confirm_delete_profile)
        {
            cx.notify();
        } else if !self.search.read(cx).value().is_empty() {
            self.clear_search(window, cx);
        } else {
            window.remove_window();
        }
    }

    /// Keep the sliders where the settings are (a change in config.json, a reset arrow),
    /// unless one is being dragged to that value right now.
    fn sync_sliders(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let store = cx.global::<SettingsStore>();
        let (opacity, line_height) = ((store.asked_opacity() * 100.0).round(), store.settings.line_height);
        for (slider, value) in [(&self.opacity_slider, opacity), (&self.line_height_slider, line_height)] {
            if (slider_value(&slider.read(cx).value()) - value).abs() > 0.001 {
                slider.update(cx, |slider, cx| slider.set_value(value, window, cx));
            }
        }
    }

    /// Point the primary picker at the primary in effect (the theme's own while
    /// none is chosen), unless it already shows it. Compared as hex so a slider
    /// drag in progress, whose color rounds to the setting just saved, is not
    /// rewritten under the user's hand.
    fn sync_primary_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(primary) = cx.global::<SettingsStore>().tokens.get("primary") else { return };
        let shown = self.primary_picker.read(cx).value().map(|c| theme_bridge::color(c).to_hex());
        if shown.as_deref() != Some(primary.to_hex().as_str()) {
            self.primary_picker
                .update(cx, |picker, cx| picker.set_value(theme_bridge::hsla(primary), window, cx));
        }
    }

    /// The page's title row and the line under it; in a search, the results' count instead.
    fn render_header(&self, query: &Query, hits: usize, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let (title, intro): (SharedString, SharedString) = if self.show_results {
            let sections = settings_index::search_visible(query, settings(cx))
                .iter()
                .map(|hit| hit.section)
                .collect::<std::collections::HashSet<_>>()
                .len();
            let intro = match hits {
                0 => "No setting has these words in its name, description or config key.".to_string(),
                _ => format!(
                    "{hits} {} in {sections} {} match “{}”. Pick a section to see it in context.",
                    if hits == 1 { "setting" } else { "settings" },
                    if sections == 1 { "section" } else { "sections" },
                    query.text()
                ),
            };
            ("Results".into(), intro.into())
        } else {
            (self.section.title().into(), self.section.info().intro.into())
        };
        let action = if self.show_results {
            Some(style::key_hint(ui, "esc", "clears the search, then closes").into_any_element())
        } else {
            self.header_action(cx)
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .h(px(style::TITLE_ROW_H))
                    .child(div().text_title().font_weight(FontWeight::BOLD).child(title))
                    .children(action),
            )
            .child(div().max_w(px(INTRO_W)).text_body().text_color(ui.muted).child(intro))
            .into_any_element()
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_primary_picker(window, cx);
        self.sync_sliders(window, cx);
        self.sync_profile_form(window, cx);
        self.rem = window.rem_size();
        let ui = ui(cx);
        // A preview section can disappear under the person (its flag was switched off in another
        // window): fall back to the first always-on section rather than draw an empty page.
        if !self.section.is_visible(settings(cx)) {
            self.section = Section::Appearance;
        }
        let translucent = settings(cx).settings_translucent;
        self.blur.set_enabled(aux_window::wants_blur(settings(cx), translucent));
        let surfaces = aux_window::surfaces(cx, translucent);
        let query = Query::new(&self.search.read(cx).value());
        let hits = settings_index::search_visible(&query, settings(cx)).len();

        // The column comes with width: the page is drawn knowing whether it has the previews.
        self.preview_column = window.viewport_size().width >= px(COLUMN_SWITCH_W);
        let body = if self.show_results && hits == 0 {
            self.render_no_results(&query, ui, cx)
        } else if self.show_results {
            self.render_results(&query, cx)
        } else {
            self.render_section(self.section, cx)
        };
        self.scroll_to_revealed(window, cx);
        let content = div()
            .id("settings-content")
            // Each region paints its own surface: stacked translucent layers
            // would make the nav less see-through than the main window's.
            .bg(surfaces.background)
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .track_scroll(&self.content_scroll)
            .px_8()
            .pb_8()
            .pt(px(PAGE_TOP))
            .child(
                div()
                    .max_w(px(COLUMN_W))
                    .flex()
                    .flex_col()
                    .gap_6()
                    .child(self.render_header(&query, hits, ui, cx))
                    .children(self.render_problems(ui, cx))
                    .child(body),
            );

        div()
            .id("settings-window")
            .track_focus(&self.focus_handle)
            .key_context("SettingsWindow")
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let keys = &event.keystroke;
                if this.record_key(keys, cx) {
                    cx.stop_propagation();
                } else if keys.modifiers.platform && keys.key == "w" {
                    window.remove_window();
                } else if keys.key == "escape" {
                    this.escape(window, cx);
                }
            }))
            .on_modifiers_changed(cx.listener(|this, event: &ModifiersChangedEvent, _, cx| {
                this.record_modifiers(&event.modifiers, cx)
            }))
            .size_full()
            .relative()
            .flex()
            .text_color(ui.text)
            // Drag the window by its top strip (the title bar is transparent); first, so it is
            // behind what is drawn over it (`chrome::TitleDrag`).
            .child(div().absolute().top_0().left_0().right_0().h(px(style::TITLE_ROW_H)).title_drag())
            .child(self.render_nav(&query, surfaces.panel, cx))
            .child(content)
            .children(self.preview_column.then(|| self.render_preview_column(surfaces.panel_right, cx)))
            .children(self.render_new_profile(cx))
    }
}

impl SettingsWindow {
    /// Problems, at the top of the page they concern: what is wrong in config.json on
    /// Advanced (elsewhere only that changes aren't saved meanwhile), and the page's own.
    fn render_problems(&self, ui: UiColors, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let store = cx.global::<SettingsStore>();
        let mut problems = Vec::new();
        if self.show_results {
            return problems;
        }
        // A preview section tied to a flag says so at the top, with a way to the flags. Only a dev
        // build ever shows such a section, so this never reaches a release.
        if let Some(flag) = self.section.feature_flag() {
            problems.push(
                style::banner(
                    ui,
                    style::BannerKind::Info,
                    false,
                    format!(
                        "Preview feature, behind the “{}” flag — a release build doesn't have it yet.",
                        flag.key()
                    ),
                )
                .child(
                    Button::new("preview-open-flags")
                        .outline()
                        .small()
                        .label("Feature flags")
                        .on_click(cx.listener(|this, _, _, cx| this.show_section(Section::FeatureFlags, cx))),
                )
                .into_any_element(),
            );
        }
        match store.error.clone() {
            Some(error) if self.section == Section::Advanced => problems.push(
                style::banner(ui, style::BannerKind::Warning, false, error)
                    .child(Button::new("config-error-open").outline().small().label("Open").on_click(|_, _, _| {
                        sections::open_in_editor(&SettingsStore::config_path())
                    }))
                    .into_any_element(),
            ),
            _ if store.config_unreadable() => problems.push(
                style::banner(
                    ui,
                    style::BannerKind::Warning,
                    false,
                    "config.json can't be read, so changes made here aren't saved until it is fixed (see Advanced).",
                )
                .into_any_element(),
            ),
            _ => {}
        }
        if self.section == Section::Profile {
            if let Some(message) = app(cx).profiles_error.clone() {
                problems.push(
                    style::banner(
                        ui,
                        style::BannerKind::Warning,
                        false,
                        format!("The list of profiles couldn't be read, so it was set aside and rebuilt: {message}"),
                    )
                    .into_any_element(),
                );
            }
            if let Some(message) = self.profile_form.as_ref().and_then(|form| form.problem()) {
                problems
                    .push(style::banner(ui, style::BannerKind::Danger, false, message).into_any_element());
            }
        }
        if let Some((section, notice)) = &self.notice
            && *section == self.section
        {
            problems
                .push(style::banner(ui, style::BannerKind::Danger, false, notice.clone()).into_any_element());
        }
        problems
    }

    /// Nothing found: the search icon, what was looked for, and where else the setting
    /// may be.
    fn render_no_results(&self, query: &Query, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        style::card(ui, ui.border)
            .max_w(px(NO_RESULTS_W))
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child(style::icon(IconName::Search).text_color(ui.muted))
            .child(
                div()
                    .text_body()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("Nothing matches “{}”", query.text())),
            )
            .child(
                div().text_caption().text_color(ui.muted).child(
                    "Some settings live only in config.json, and projects can bring their own actions.",
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pt_1()
                    .child(
                        Button::new("no-results-config")
                            .outline()
                            .small()
                            .icon(IconName::CodeXml)
                            .label("Open config.json")
                            .on_click(|_, _, _| sections::open_in_editor(&SettingsStore::config_path())),
                    )
                    .child(
                        Button::new("no-results-clear")
                            .ghost()
                            .small()
                            .label("Clear search")
                            .on_click(cx.listener(|this, _, window, cx| this.clear_search(window, cx))),
                    ),
            )
            .into_any_element()
    }
}

/// Show one of our folders in the Finder (the icon themes, the fonts),
/// creating it first so there's somewhere to drop files.
// GPUI's test platform panics on `open_with_system` (gpui-pre 0.3.9, `platform/test/platform.rs`), so a view
// test that clicks this needs a stand-in first.
fn open_folder(dir: &Path, cx: &App) {
    match wormhole_core::storage::fsutil::create_dir_all_private(dir) {
        Ok(()) => cx.open_with_system(dir),
        Err(err) => tracing::warn!("opening {}: {err}", dir.display()),
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Bounds, Pixels, point, px, size};

    use super::grown_right;

    fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds { origin: point(px(x), px(y)), size: size(px(w), px(h)) }
    }

    #[test]
    fn a_window_grows_to_the_right_and_keeps_its_left_edge() {
        let grown = grown_right(bounds(260.0, 40.0, 920.0, 720.0), 480.0, bounds(0.0, 25.0, 2560.0, 1400.0));
        assert_eq!(grown, bounds(260.0, 40.0, 1400.0, 720.0));
    }

    #[test]
    fn a_screen_too_narrow_moves_it_left_then_narrows_it() {
        let screen = bounds(0.0, 25.0, 1440.0, 875.0);
        // Room for the width but not from x 260: it moves left until its right edge fits.
        assert_eq!(
            grown_right(bounds(260.0, 40.0, 920.0, 720.0), 480.0, screen),
            bounds(40.0, 40.0, 1400.0, 720.0)
        );
        // Not even the width fits: the usable width, from the screen's left edge.
        let small = bounds(0.0, 25.0, 1280.0, 775.0);
        assert_eq!(
            grown_right(bounds(180.0, 40.0, 920.0, 720.0), 480.0, small),
            bounds(0.0, 40.0, 1280.0, 720.0)
        );
    }
}
