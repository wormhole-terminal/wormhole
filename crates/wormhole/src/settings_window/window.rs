//! The settings window as a view: making it, the page it draws and what its controls do.

use std::time::Duration;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::*;
use wormhole_core::config::settings_index::{self, Query};
use wormhole_core::theme::fonts::FontSlot;

use super::font_picker::FontPickerState;
use crate::app_state::app;
use crate::platform::vibrancy::Blur;
use crate::select::ColorPickState;
use crate::settings::SettingsStore;
use crate::settings::{UiColors, settings};
use crate::style::TitleDrag as _;
use crate::style::{self, TextRole};
use crate::ui::{Look, PageTitleProps};

use super::open::REVEAL_INSET;
use super::{
    COLUMN_W, FrontWindow, INTRO_W, NO_RESULTS_W, PAGE_GAP, PAGE_TITLE_H, PAGE_TOP, Section, SettingsWindow,
    sections, ui,
};

impl SettingsWindow {
    pub(super) fn new(section: Option<Section>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(text(SETTINGS_WINDOW_SEARCH_SETTINGS)));
        let font_picker = FontPickerState::new(window, cx);
        let primary_pick = cx.new(|cx| ColorPickState::new(window, cx));
        let branch_pick = cx.new(|cx| ColorPickState::new(window, cx));
        let (projects, project_subscriptions) = sections::ProjectPicker::new(window, cx);
        let mut subscriptions = vec![
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
            cx.subscribe_in(&font_picker.search, window, |_, _, event: &InputEvent, _, cx| {
                if let InputEvent::Change = event {
                    cx.notify();
                }
            }),
        ];
        subscriptions.extend(project_subscriptions);
        // Closed while its field records: the hotkey comes back with the window gone.
        subscriptions.push(cx.on_release(|_, cx| {
            crate::hotkey::pause(false, cx);
        }));
        let theme_picker = sections::ThemePicker::default();
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
            motion_previews: None,
            recording: None,
            primary_pick,
            branch_pick,
            specimen: FontSlot::Terminal,
            themes_all: false,
            #[cfg(feature = "devctl")]
            components_sheet: None,
            themes_separate: None,
            sidebar_preview: None,
            changed_copied: false,
            others_open: false,
            status_open: false,
            updates_details_open: false,
            terminal_rows: Default::default(),
            profile_rows: Default::default(),
            profile_form: None,
            confirm_delete_profile: false,
            new_profile: None,
            notice: None,
            content_scroll: ScrollHandle::new(),
            group: super::nav::GroupPick::default(),
            revealed: None,
            reveal_pending: false,
            projects,
            theme_picker,
            agents: None,
            agents_task: None,
            import: sections::ImportPage::default(),
            cli_tool: sections::CliToolPanel::read(),
            extensions: sections::Extensions::default(),
            rem: window.rem_size(),
            blur,
            focus_handle,
            _subscriptions: subscriptions,
            _refresh: refresh,
        };
        // The apps Open folders in lists, looked for when Settings opens, when its Terminal page is shown and
        // when the app comes forward again (an app may have been installed meanwhile): never while it draws.
        crate::open_in::OpenIn::refresh(cx);
        // The Probes folder is read again when the window comes forward with its page showing: a folder
        // may have been added or edited in the meantime. There is no timer.
        cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                crate::open_in::OpenIn::refresh(cx);
                if this.section == Section::Extensions {
                    this.scan_probes(cx);
                }
            }
        })
        .detach();
        if this.section == Section::Extensions {
            this.scan_probes(cx);
        }
        // The nav marks an agent whose hooks need repair, so the agents are read up front.
        this.load_agents(cx);
        this.load_cli_tool(cx);
        if this.section == Section::Motion {
            this.show_section(Section::Motion, cx);
        }
        if this.section == Section::Import {
            this.import_shown(cx);
        }
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
            self.group = super::nav::GroupPick::default();
            self.content_scroll.set_offset(point(px(0.0), px(0.0)));
        }
        self.section = section;
        // The previews run a clock each: they are there while their page is, and not otherwise.
        self.motion_previews = (section == Section::Motion)
            .then(|| self.motion_previews.take().unwrap_or_else(|| sections::MotionPreviews::new(cx)));
        if section == Section::Terminal {
            crate::open_in::OpenIn::refresh(cx);
        }
        // The first time the page is shown it scans (never at launch: nothing reads another terminal's
        // files until a person opens Import), and from here, not `render`, which must not notify.
        if section == Section::Import {
            self.import_shown(cx);
        }
        if section == Section::Extensions {
            self.scan_probes(cx);
        }
        self.show_results = false;
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

    /// Esc: whatever is open first (a sheet, the reset question), then the search, then the
    /// window.
    pub(super) fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.new_profile.take().is_some()
            || self.close_probe_sheet(cx)
            || self.close_import_sheet(cx)
            || self.font_picker.open.take().is_some()
            || self.end_recording(cx)
            || std::mem::take(&mut self.confirm_delete_profile)
        {
            cx.notify();
        } else if !self.search.read(cx).value().is_empty() {
            self.clear_search(window, cx);
        } else {
            window.remove_window();
        }
    }

    /// Point the colour controls' editors at the colours in force (the primary's, the branch's), unless a
    /// change of their own put them there.
    pub(super) fn sync_color_picks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (primary, branch) = sections::colors_in_force(cx.global::<SettingsStore>());
        self.primary_pick.update(cx, |pick, cx| pick.follow(primary, window, cx));
        self.branch_pick.update(cx, |pick, cx| pick.follow(branch, window, cx));
    }

    /// Whether the primary's field says that what was typed is not a colour.
    #[cfg(test)]
    pub(super) fn primary_pick_says_bad(&self, cx: &App) -> bool {
        self.primary_pick.read(cx).says_bad()
    }

    /// The page's title row and the line under it; in a search, the results' count instead.
    pub(super) fn render_header(
        &self,
        query: &Query,
        hits: usize,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let title: SharedString = if self.show_results {
            text(SETTINGS_WINDOW_RESULTS).into()
        } else {
            self.section.title().into()
        };
        // The design has no sentence under a title; in a search the count says what was found.
        let found = self.show_results.then(|| {
            let sections = settings_index::search_visible(query, settings(cx))
                .iter()
                .map(|hit| hit.section)
                .collect::<std::collections::HashSet<_>>()
                .len();
            let intro = match hits {
                0 => text(SETTINGS_WINDOW_NO_RESULTS).to_string(),
                _ => settings_window_results_summary(whole(hits), whole(sections), &query.text()),
            };
            div().max_w(px(INTRO_W)).text_body().text_color(ui.muted).child(SharedString::from(intro))
        });
        let action = self.show_results.then(|| {
            style::key_hint(ui, "esc", text(SETTINGS_WINDOW_CLEARS_THE_SEARCH_THEN_CLOSES)).into_any_element()
        });
        let badge = (!self.show_results && self.section.is_preview()).then(|| self.preview_badge(ui, cx));
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
                    .h(px(PAGE_TITLE_H))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(PageTitleProps { title, badge: None }.render(&Look::of(cx)))
                            .children(badge),
                    )
                    .children(action),
            )
            .children(found)
            .into_any_element()
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(feature = "devctl")]
        if let Some(from) = self.components_sheet {
            return div()
                .id("settings-window")
                .size_full()
                .child(super::components_sheet::render(from, cx))
                .into_any_element();
        }
        self.sync_color_picks(window, cx);
        self.sync_profile_form(window, cx);
        self.rem = window.rem_size();
        style::sync_option(window.modifiers().alt, cx);
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
        let changed = super::changed::differing(cx);

        let body = if self.show_results && hits == 0 {
            self.render_no_results(&query, ui, cx)
        } else if self.show_results {
            self.render_results(&query, cx)
        } else {
            self.render_section(self.section, cx)
        };
        self.scroll_to_revealed(window, cx);
        self.scroll_to_group(window, cx);
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
                    .w_full()
                    .max_w(px(COLUMN_W))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .gap(px(PAGE_GAP))
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
                } else if sections::is_unlock_key(keys) {
                    this.toggle_flags_unlocked(cx);
                    cx.stop_propagation();
                } else if keys.modifiers.platform && keys.key == "w" {
                    window.remove_window();
                } else if keys.key == "escape" {
                    this.escape(window, cx);
                }
            }))
            .on_modifiers_changed(cx.listener(|this, event: &ModifiersChangedEvent, _, cx| {
                this.record_modifiers(&event.modifiers, cx);
                // ⌥ shows every row's key: the page is drawn again when it goes down or up, not for ⇧ or ⌘.
                if style::option_held(cx) != event.modifiers.alt {
                    cx.notify();
                }
            }))
            .size_full()
            .relative()
            .flex()
            .text_color(ui.text)
            // Drag the window by its top strip (the title bar is transparent); first, so it is
            // behind what is drawn over it (`style::TitleDrag`).
            .child(div().absolute().top_0().left_0().right_0().h(px(style::TITLE_ROW_H)).title_drag())
            .child(self.render_nav(&query, changed, surfaces.panel, cx))
            .child(content)
            .children(self.render_new_profile(cx))
            .children(self.render_probe_sheet(window, cx))
            .children(self.render_import_sheet(window, cx))
            .into_any_element()
    }
}

impl SettingsWindow {
    /// "Preview" beside the title of a page that isn't finished. Its tooltip says why, and on a page tied
    /// to a flag it is the way to the flags, which a release build doesn't show.
    fn preview_badge(&self, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let badge = style::solid_chip(ui, style::ChipKind::Info, text(SETTINGS_NAV_PREVIEW));
        let Some(flag) = self.section.feature_flag() else { return badge.into_any_element() };
        let note = SharedString::from(settings_window_preview_feature_note(flag.key()));
        div()
            .child(badge)
            .id("preview-open-flags")
            .role(gpui_kit::accesskit::Role::Button)
            .aria_label(note.clone())
            .aria_description(text(SETTINGS_WINDOW_FEATURE_FLAGS))
            .cursor_pointer()
            .tooltip(move |window, cx| Tooltip::new(note.clone()).build(window, cx))
            .on_click(cx.listener(|this, _, _, cx| this.show_section(Section::FeatureFlags, cx)))
            .into_any_element()
    }

    /// Problems, at the top of the page they concern: what is wrong in config.json on
    /// Advanced (elsewhere only that changes aren't saved meanwhile), and the page's own.
    pub(super) fn render_problems(&self, ui: UiColors, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let store = cx.global::<SettingsStore>();
        let mut problems = Vec::new();
        if self.show_results {
            return problems;
        }
        match store.error.clone() {
            Some(error) if self.section == Section::Advanced => problems.push(
                style::notice(ui, style::NoticeKind::Warning, error)
                    .child(
                        style::notice_button("config-error-open", ui, style::NoticeKind::Warning, cx)
                            .label(text(SETTINGS_WINDOW_OPEN))
                            .on_click(|_, _, _| sections::open_in_editor(&SettingsStore::config_path())),
                    )
                    .into_any_element(),
            ),
            _ if store.config_unreadable() => problems.push(
                style::notice(
                    ui,
                    style::NoticeKind::Warning,
                    text(SETTINGS_WINDOW_CONFIG_JSON_CANT_BE_READ_SO),
                )
                .into_any_element(),
            ),
            _ => {}
        }
        if self.section == Section::Profile {
            if let Some(message) = app(cx).profiles_error.clone() {
                problems.push(
                    style::notice(
                        ui,
                        style::NoticeKind::Warning,
                        settings_window_profiles_unreadable(&message),
                    )
                    .into_any_element(),
                );
            }
            if let Some(message) = self.profile_form.as_ref().and_then(|form| form.problem()) {
                problems.push(style::notice(ui, style::NoticeKind::Danger, message).into_any_element());
            }
        }
        if let Some((section, notice)) = &self.notice
            && *section == self.section
        {
            problems.push(style::notice(ui, style::NoticeKind::Danger, notice.clone()).into_any_element());
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
                        style::form_button("no-results-config")
                            .label(text(SETTINGS_WINDOW_OPEN_CONFIG_JSON))
                            .on_click(|_, _, _| sections::open_in_editor(&SettingsStore::config_path())),
                    )
                    .child(
                        style::form_button("no-results-clear")
                            .label(text(SETTINGS_WINDOW_CLEAR_SEARCH))
                            .on_click(cx.listener(|this, _, window, cx| this.clear_search(window, cx))),
                    ),
            )
            .into_any_element()
    }
}
