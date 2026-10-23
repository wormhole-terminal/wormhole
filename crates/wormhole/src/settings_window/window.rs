//! The settings window as a view: making it, the page it draws and what its controls do.

use std::time::Duration;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::color_picker::{ColorPickerEvent, ColorPickerState};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::slider::SliderEvent;
use gpui_kit::*;
use wormhole_core::config::settings_index::{self, Query};
use wormhole_core::theme::fonts::FontSlot;

use super::font_picker::FontPickerState;
use super::preview_column::COLUMN_SWITCH_W;
use crate::app_state::app;
use crate::platform::vibrancy::Blur;
use crate::settings::{SettingsStore, update_settings};
use crate::settings::{UiColors, settings};
use crate::style::TitleDrag as _;
use crate::style::{self, TextRole};
use crate::theme_bridge;

use super::open::REVEAL_INSET;
use super::{
    COLUMN_W, FrontWindow, INTRO_W, NO_RESULTS_W, PAGE_TOP, Section, SettingsWindow, sections, slider,
    slider_value, ui,
};

impl SettingsWindow {
    pub(super) fn new(section: Option<Section>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let current = settings(cx).clone();
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(text(SETTINGS_WINDOW_SEARCH_SETTINGS)));
        let menu_search = cx.new(|cx| InputState::new(window, cx).placeholder(text(SETTINGS_WINDOW_SEARCH)));
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
            crate::hotkey::pause(false, cx);
        }));
        let theme_picker = sections::ThemePicker::new(window, cx);
        let translucent = settings(cx).settings_translucent;
        let blur = Blur::install(window, style::wants_blur(settings(cx), translucent));
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
    pub(super) fn read_front(&mut self, cx: &mut Context<Self>) {
        let front = FrontWindow::read(cx);
        if front != self.front {
            self.front = front;
            cx.notify();
        }
    }

    pub(super) fn show_section(&mut self, section: Section, cx: &mut Context<Self>) {
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
    pub(super) fn revealed_row(&self) -> Option<(&'static str, ScrollAnchor)> {
        self.revealed.clone().filter(|_| !self.show_results)
    }

    /// Scroll to the revealed row, registered from the draw that laid it out: the anchor knows
    /// where the row is only after that draw, and the scroll takes effect a frame later.
    pub(super) fn scroll_to_revealed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.update(cx, |search, cx| search.set_value("", window, cx));
        self.show_results = false;
        cx.notify();
    }

    /// Esc: whatever is open first (a menu, the reset question), then the search, then the
    /// window.
    pub(super) fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
    pub(super) fn sync_sliders(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
    pub(super) fn sync_primary_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(primary) = cx.global::<SettingsStore>().tokens.get("primary") else { return };
        let shown = self.primary_picker.read(cx).value().map(|c| theme_bridge::color(c).to_hex());
        if shown.as_deref() != Some(primary.to_hex().as_str()) {
            self.primary_picker
                .update(cx, |picker, cx| picker.set_value(theme_bridge::hsla(primary), window, cx));
        }
    }

    /// The page's title row and the line under it; in a search, the results' count instead.
    pub(super) fn render_header(
        &self,
        query: &Query,
        hits: usize,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (title, intro): (SharedString, SharedString) = if self.show_results {
            let sections = settings_index::search_visible(query, settings(cx))
                .iter()
                .map(|hit| hit.section)
                .collect::<std::collections::HashSet<_>>()
                .len();
            let intro = match hits {
                0 => text(SETTINGS_WINDOW_NO_RESULTS).to_string(),
                _ => settings_window_results_summary(whole(hits), whole(sections), &query.text()),
            };
            (text(SETTINGS_WINDOW_RESULTS).into(), intro.into())
        } else {
            (self.section.title().into(), self.section.info().intro().into())
        };
        let action = if self.show_results {
            Some(
                style::key_hint(ui, "esc", text(SETTINGS_WINDOW_CLEARS_THE_SEARCH_THEN_CLOSES))
                    .into_any_element(),
            )
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
        self.blur.set_enabled(style::wants_blur(settings(cx), translucent));
        let surfaces = style::window_surfaces(cx, translucent);
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
            .key_context("SettingsWindow") // text: not shown (a key context)
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
            // behind what is drawn over it (`style::TitleDrag`).
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
    pub(super) fn render_problems(&self, ui: UiColors, cx: &mut Context<Self>) -> Vec<AnyElement> {
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
                    settings_window_preview_feature_note(flag.key()),
                )
                .child(
                    Button::new("preview-open-flags")
                        .outline()
                        .small()
                        .label(text(SETTINGS_WINDOW_FEATURE_FLAGS))
                        .on_click(cx.listener(|this, _, _, cx| this.show_section(Section::FeatureFlags, cx))),
                )
                .into_any_element(),
            );
        }
        match store.error.clone() {
            Some(error) if self.section == Section::Advanced => problems.push(
                style::banner(ui, style::BannerKind::Warning, false, error)
                    .child(
                        Button::new("config-error-open")
                            .outline()
                            .small()
                            .label(text(SETTINGS_WINDOW_OPEN))
                            .on_click(|_, _, _| sections::open_in_editor(&SettingsStore::config_path())),
                    )
                    .into_any_element(),
            ),
            _ if store.config_unreadable() => problems.push(
                style::banner(
                    ui,
                    style::BannerKind::Warning,
                    false,
                    text(SETTINGS_WINDOW_CONFIG_JSON_CANT_BE_READ_SO),
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
                        settings_window_profiles_unreadable(&message),
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
    pub(super) fn render_no_results(
        &self,
        query: &Query,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
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
                    .child(settings_window_nothing_matches(&query.text())),
            )
            .child(
                div()
                    .text_caption()
                    .text_color(ui.muted)
                    .child(text(SETTINGS_WINDOW_SOME_SETTINGS_LIVE_ONLY_IN_CONFIG)),
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
                            .label(text(SETTINGS_WINDOW_OPEN_CONFIG_JSON))
                            .on_click(|_, _, _| sections::open_in_editor(&SettingsStore::config_path())),
                    )
                    .child(
                        Button::new("no-results-clear")
                            .ghost()
                            .small()
                            .label(text(SETTINGS_WINDOW_CLEAR_SEARCH))
                            .on_click(cx.listener(|this, _, window, cx| this.clear_search(window, cx))),
                    ),
            )
            .into_any_element()
    }
}
