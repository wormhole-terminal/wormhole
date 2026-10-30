//! Project folders: a project picker, then either the defaults for all
//! projects (plus the trusted list) or one project's own switches and trust.

use std::cell::{OnceCell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId as Section;
use wormhole_core::project::Feature;
use wormhole_core::project::{FeatureStanding, TrustStatus, trust_applies};

use super::reveal;
use crate::features::project;
use crate::features::project::LoadedProject;
use crate::select;
use crate::select::{Choice, SearchSelect, SelectTrigger};
use crate::settings::UiColors;
use crate::settings_window::registry::Setting;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, SettingParts, SettingState, TextRole};
use crate::window::window_lifecycle::with_front_workspace;

pub(super) const SETTINGS: &[Setting] = &[Setting::block("projects", |w, cx| w.draw_projects(cx))];

/// The picker's state inside the settings window.
pub(crate) struct ProjectPicker {
    /// `None` = the defaults for all projects.
    selected: Option<PathBuf>,
    known: Rc<Vec<PathBuf>>,
    open: bool,
    search: Entity<InputState>,
    /// The selected project's files as read for this page. Reading them (and the trust
    /// fingerprint) is file I/O, so it is kept between frames and read again about once a
    /// second, and right after a button changes something.
    info: RefCell<Option<(PathBuf, Option<Rc<LoadedProject>>)>>,
    /// Whether the Mac can approve a project (asked once; asking creates a system
    /// authentication context).
    can_approve: OnceCell<bool>,
}

impl ProjectPicker {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> (Self, Vec<Subscription>) {
        let known = Rc::new(project::known_projects(cx));
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(text(SETTINGS_PROJECTS_SEARCH_PROJECTS)));
        let subscriptions =
            vec![cx.subscribe_in(&search, window, |this, _, event: &InputEvent, _, cx| match event {
                InputEvent::Change => cx.notify(),
                InputEvent::PressEnter { .. } => {
                    this.projects.pick_first_match(cx);
                    cx.notify();
                }
                _ => {}
            })];
        (
            Self {
                selected: None,
                known,
                open: false,
                search,
                info: RefCell::new(None),
                can_approve: OnceCell::new(),
            },
            subscriptions,
        )
    }

    /// Jump to a project (e.g. from the trusted list).
    pub(crate) fn select(&mut self, root: PathBuf) {
        if !self.known.contains(&root) {
            Rc::make_mut(&mut self.known).push(root.clone());
        }
        self.selected = Some(root);
    }

    /// devctl: open the menu, optionally with `query` typed into its search box.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_open(&mut self, query: String, window: &mut Window, cx: &mut App) {
        self.open = true;
        self.search.update(cx, |search, cx| search.set_value(query, window, cx));
    }

    /// The project's files as last read, reading them if they are not (or are of another
    /// project).
    fn info(&self, root: &Path, cx: &App) -> Option<Rc<LoadedProject>> {
        let mut info = self.info.borrow_mut();
        if let Some((cached, project)) = info.as_ref()
            && cached == root
        {
            return project.clone();
        }
        let project = project::load_project_info(project::shown_profile(cx), root, cx).map(Rc::new);
        *info = Some((root.to_path_buf(), project.clone()));
        project
    }

    /// Something changed in the project (trust, a feature): read it again at the next draw.
    pub(crate) fn forget_info(&self) {
        self.info.take();
    }

    /// The choice's value is the project root ("" = all projects).
    fn pick(&mut self, value: &str) {
        self.selected = Some(value).filter(|v| !v.is_empty()).map(PathBuf::from);
        self.open = false;
    }

    /// Enter in the search box: take the first project that matches.
    fn pick_first_match(&mut self, cx: &App) {
        let query = self.search.read(cx).value().to_string();
        if let Some(first) = select::filter(&choices(&self.known), &query).into_iter().next() {
            self.pick(&first.value);
        }
    }
}

/// The select's first choice: the defaults every project starts from.
fn all_projects() -> &'static str {
    text(SETTINGS_PROJECTS_ALL_PROJECTS_DEFAULTS)
}

/// A project's name in the select: its folder's name.
fn choice_label(root: &Path) -> SharedString {
    root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default().into()
}

fn choices(known: &[PathBuf]) -> Vec<Choice<String>> {
    let mut items = vec![Choice::new(String::new(), all_projects())];
    items.extend(known.iter().map(|root| {
        Choice::new(root.to_string_lossy().into_owned(), choice_label(root)).detail(short_home(root))
    }));
    items
}

fn short_home(path: &Path) -> String {
    match crate::settings_window::home() {
        Some(home) if path.starts_with(&home) => {
            format!("~/{}", path.strip_prefix(&home).unwrap_or(path).display())
        }
        _ => path.display().to_string(),
    }
}

impl SettingsWindow {
    /// The project select: "All projects" or one of the known folders, searchable.
    fn project_select(&self, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let value = self.projects.selected.as_ref().map(|p| p.to_string_lossy().into_owned());
        let value = value.unwrap_or_default();
        // The trigger's own text; the options are only built while the menu is open.
        let label = match &self.projects.selected {
            Some(root) if self.projects.known.contains(root) => choice_label(root),
            _ => all_projects().into(),
        };
        let known = self.projects.known.clone();
        let this = cx.weak_entity();
        let on_change = this.clone();
        SearchSelect::new(
            "project-select",
            ui,
            SelectTrigger::new("project-select-trigger", ui, label).width(px(360.0)),
            &self.projects.search,
            cx,
        )
        .options_with(move || choices(&known).into())
        .current(value)
        .open(self.projects.open)
        .menu_width(px(360.0))
        .on_open_change(move |want_open, window, cx| {
            on_change
                .update(cx, |this, cx| {
                    this.projects.open = want_open;
                    if want_open {
                        this.projects.search.update(cx, |search, cx| search.set_value("", window, cx));
                    }
                    cx.notify();
                })
                .ok();
        })
        .on_pick(move |value, _, cx| {
            this.update(cx, |this, cx| {
                this.projects.pick(value);
                cx.notify();
            })
            .ok();
        })
        .into_popover()
        .into_any_element()
    }

    /// Re-list the projects the picker offers, keeping the selection.
    pub(in crate::settings_window) fn refresh_known_projects(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Listing checks the disk, so only while the list is on screen.
        if self.section != Section::Projects {
            return;
        }
        let known = project::known_projects(cx);
        if known != *self.projects.known {
            self.projects.known = Rc::new(known);
        }
        // The files and the trust status can change under us (an approval finishing, an
        // edit in the folder).
        self.projects.forget_info();
        cx.notify();
    }

    fn draw_projects(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let all_projects = self.projects.selected.is_some().then(|| {
            style::link_button("all-projects", ui, text(SETTINGS_PROJECTS_ALL_PROJECTS), false)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.projects.selected = None;
                    cx.notify();
                }))
                .into_any_element()
        });
        let picker = div()
            .flex()
            .flex_col()
            .child(style::setting_group_head(ui, text(SETTINGS_PROJECTS_PROJECT), None))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .py_2()
                    .rule_top(ui)
                    .child(self.project_select(ui, cx))
                    .children(all_projects),
            );
        let body = match self.projects.selected.clone() {
            None => self.render_project_defaults(cx),
            Some(root) => self.render_project(&root, cx),
        };
        div().flex().flex_col().gap_6().child(picker).child(body).into_any_element()
    }

    fn render_project_defaults(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let mut defaults = div()
            .flex()
            .flex_col()
            .child(style::setting_group_head(ui, text(SETTINGS_PROJECTS_USE_FROM_PROJECTS_BY_DEFAULT), None))
            .child(
                div().pb_2().text_body().text_color(ui.muted).child(text(SETTINGS_PROJECTS_DEFAULTS_NOTE)),
            );
        for feature in Feature::ALL {
            let on = project::api::default_enabled(feature, cx);
            let profile = project::shown_profile(cx).clone();
            let switch = style::toggle(
                SharedString::from(format!("default-{}", feature.key())),
                on,
                move |on, _, cx| project::set_feature_default(&profile, feature, on, cx),
            );
            let description = if feature.requires_trust() {
                settings_projects_feature_needs_trust(feature.description())
            } else {
                feature.description().to_string()
            };
            defaults = defaults.child(feature_row(
                ui,
                feature,
                description,
                SettingState::Default,
                switch.into_any_element(),
                cx,
            ));
        }
        let defaults = defaults.rule_bottom(ui);

        let mut trusted = div().flex().flex_col().child(style::setting_group_head(
            ui,
            text(SETTINGS_PROJECTS_TRUSTED_PROJECTS),
            Some(text(SETTINGS_PROJECTS_PROJECTS_ALLOWED_TO_RUN_COMMANDS)),
        ));
        let list: Vec<PathBuf> = project::api::trusted_roots(cx);
        if list.is_empty() {
            trusted = trusted.child(
                div()
                    .py_3()
                    .rule_top(ui)
                    .text_body()
                    .text_color(ui.faint)
                    .child(text(SETTINGS_PROJECTS_NONE_YET)),
            );
        }
        for (ix, root) in list.into_iter().enumerate() {
            let (open, revoke) = (root.clone(), root.clone());
            trusted = trusted.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .min_h(px(style::ROW_H + 8.0))
                    .rule_top(ui)
                    .child(div().text_body().truncate().child(short_home(&root)))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(
                                Button::new(("trusted-open", ix))
                                    .ghost()
                                    .small()
                                    .label(text(SETTINGS_PROJECTS_SETTINGS))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.projects.select(open.clone());
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new(("revoke", ix))
                                    .ghost()
                                    .small()
                                    .label(text(SETTINGS_PROJECTS_REVOKE))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        let root = revoke.clone();
                                        project::revoke_project(
                                            &project::shown_profile(cx).clone(),
                                            &root,
                                            cx,
                                        );
                                        this.projects.forget_info();
                                    })),
                            ),
                    ),
            );
        }
        if !*self.projects.can_approve.get_or_init(crate::platform::auth::available) {
            trusted = trusted.child(div().pb_2().child(style::banner(
                ui,
                style::BannerKind::Warning,
                false,
                text(SETTINGS_PROJECTS_APPROVAL_ISNT_AVAILABLE_ON_THIS_MAC),
            )));
        }
        div().flex().flex_col().gap_6().child(defaults).child(trusted.rule_bottom(ui)).into_any_element()
    }

    fn render_project(&self, root: &Path, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let Some(project) = self.projects.info(root, cx) else {
            return style::banner(
                ui,
                style::BannerKind::Warning,
                false,
                settings_projects_missing_folder(&root.display().to_string()),
            )
            .into_any_element();
        };
        let (approving, hub_error) = project::api::approval(cx);
        let prefs: Vec<(Feature, FeatureStanding, Option<bool>, bool)> =
            project::api::feature_rows(root, &project, cx);
        let needing = project.snapshot.needing_trust();
        let has_commands = project.request.as_ref().is_some_and(|r| !r.commands.is_empty());

        // Trust: a badge for where it stands, the one action that fits.
        let (status, color) = match project.status {
            TrustStatus::Trusted => (text(SETTINGS_PROJECTS_TRUSTED), ui.success_text),
            TrustStatus::Changed => (text(SETTINGS_PROJECTS_CHANGED_SINCE_YOU_TRUSTED_IT), ui.warning_text),
            TrustStatus::Untrusted => (text(SETTINGS_PROJECTS_NOT_TRUSTED), ui.muted),
        };
        let detail = trust_detail(project.status, has_commands, &needing);
        let trusted = project.status == TrustStatus::Trusted;
        let trust_root = root.to_path_buf();
        let trust_button = if trusted {
            Button::new("project-revoke")
                .outline()
                .small()
                .label(text(SETTINGS_PROJECTS_REVOKE_TRUST))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let root = trust_root.clone();
                    project::revoke_project(&project::shown_profile(cx).clone(), &root, cx);
                    this.projects.forget_info();
                }))
        } else {
            Button::new("project-trust")
                .primary()
                .small()
                .icon(IconName::FingerprintPattern)
                .label(if approving {
                    text(SETTINGS_PROJECTS_WAITING_FOR_APPROVAL)
                } else {
                    text(SETTINGS_PROJECTS_TRUST_WITH_TOUCH_ID)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    let root = trust_root.clone();
                    project::approve_project(&project::shown_profile(cx).clone(), &root, cx);
                    this.projects.forget_info();
                }))
        };
        let mut trust = div()
            .flex()
            .flex_col()
            .child(style::setting_group_head(ui, text(SETTINGS_PROJECTS_TRUST), None))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .min_h(px(style::SETTING_ROW_MIN_H))
                    .py_2()
                    .rule_top(ui)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_body()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(text(SETTINGS_PROJECTS_THIS_PROJECT)),
                                    )
                                    .child(style::status_badge(ui, color, status)),
                            )
                            .child(div().text_caption().text_color(ui.muted).child(detail)),
                    )
                    .child(trust_button),
            );
        if let Some(request) = project.request.as_ref().filter(|r| !trusted && !r.commands.is_empty()) {
            trust = trust.child(
                div().pb_2().child(
                    style::card(ui, ui.border)
                        .flex()
                        .flex_col()
                        .gap_1()
                        .text_caption()
                        .font_family(crate::fonts::code_font(cx).0)
                        .children(request.commands.iter().map(|c| div().child(c.to_string()))),
                ),
            );
        }
        if let Some(error) = hub_error {
            trust =
                trust.child(div().pb_2().child(style::banner(ui, style::BannerKind::Danger, false, error)));
        }

        // Features: default / on / off per project, as the panel's small segmented control.
        let mut features = div()
            .flex()
            .flex_col()
            .child(style::setting_group_head(ui, text(SETTINGS_PROJECTS_USE_FROM_THIS_PROJECT), None))
            .child(
                div()
                    .pb_2()
                    .text_body()
                    .text_color(ui.muted)
                    .child(text(SETTINGS_PROJECTS_ONLY_FOR_YOU_NOTHING_IS_WRITTEN)),
            );
        for (feature, standing, current, default) in prefs {
            let description = feature_description(feature, standing);
            let key = feature.key();
            let options = [
                (
                    None,
                    if default {
                        text(SETTINGS_PROJECTS_DEFAULT_ON)
                    } else {
                        text(SETTINGS_PROJECTS_DEFAULT_OFF)
                    },
                ),
                (Some(true), text(SETTINGS_PROJECTS_ON)),
                (Some(false), text(SETTINGS_PROJECTS_OFF)),
            ];
            let control = style::segmented(ui)
                .children(options.into_iter().enumerate().map(|(ix, (value, label))| {
                    let root = root.to_path_buf();
                    style::segment(
                        SharedString::from(format!("feature-{key}-{ix}")),
                        ui,
                        label,
                        current == value,
                    )
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let root = root.clone();
                        project::set_feature_for(
                            &project::shown_profile(cx).clone(),
                            &root,
                            feature,
                            value,
                            cx,
                        )
                    }))
                }))
                .into_any_element();
            // Notes without a file are still for the person to switch on: that makes the file.
            let state = if standing == FeatureStanding::Missing && feature != Feature::Notes {
                SettingState::Disabled
            } else {
                SettingState::Default
            };
            features = features.child(feature_row(ui, feature, description, state, control, cx));
        }
        let features = features.rule_bottom(ui);

        let errors = (!project.errors.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(style::setting_group_head(ui, text(SETTINGS_PROJECTS_PROBLEMS), None))
                .children(
                    project
                        .errors
                        .iter()
                        .map(|e| style::banner(ui, style::BannerKind::Warning, false, e.clone())),
                )
        });

        let open_root = root.to_path_buf();
        let reveal_root = project.folder.dir.clone();
        let actions = div()
            .flex()
            .gap_2()
            .child(
                Button::new("project-open-session")
                    .small()
                    .icon(IconName::SquareTerminal)
                    .label(text(SETTINGS_PROJECTS_OPEN_IN_A_NEW_TERMINAL))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let root = open_root.clone();
                        with_front_workspace(cx, |ws, window, cx| {
                            crate::background::activate_window(window, cx);
                            ws.spawn_session(None, Some(root), window, cx);
                        });
                    })),
            )
            .child(
                Button::new("project-reveal")
                    .ghost()
                    .small()
                    .icon(IconName::ExternalLink)
                    .label(settings_projects_show_in_finder(&project.folder.dir_name()))
                    .on_click(move |_, _, cx| reveal(&reveal_root, cx)),
            );

        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(trust.rule_bottom(ui))
            .child(features)
            .children(errors)
            .child(actions)
            .into_any_element()
    }
}

/// What the Trust row says under the badge. Trusting applies more than commands: a `config.json`
/// with just `font_size` is held back too, so a project with no command still says what waits.
fn trust_detail(status: TrustStatus, has_commands: bool, needing: &[Feature]) -> String {
    let applies = trust_applies(needing);
    match (status, has_commands, needing.is_empty()) {
        (TrustStatus::Trusted, true, _) => {
            text(SETTINGS_PROJECTS_ACTIONS_THE_TERMINAL_LAYOUT_AND_ENV).to_string()
        }
        (TrustStatus::Trusted, false, false) => settings_projects_trust_applies(&applies),
        (TrustStatus::Trusted, false, true) => text(SETTINGS_PROJECTS_NOTHING_IN_IT_NEEDS_TRUST).to_string(),
        (TrustStatus::Changed, true, _) => {
            text(SETTINGS_PROJECTS_REVIEW_THE_COMMANDS_BELOW_AND_TRUST).to_string()
        }
        (TrustStatus::Changed, false, false) => settings_projects_trust_again(&applies),
        (TrustStatus::Changed, false, true) => {
            text(SETTINGS_PROJECTS_NOTHING_IN_IT_NEEDS_TRUST_NOW).to_string()
        }
        (TrustStatus::Untrusted, true, _) => {
            text(SETTINGS_PROJECTS_NOTHING_IN_IT_CAN_RUN_COMMANDS).to_string()
        }
        (TrustStatus::Untrusted, false, false) => settings_projects_trust_would_apply(&applies),
        (TrustStatus::Untrusted, false, true) => {
            text(SETTINGS_PROJECTS_NOTHING_IN_IT_NEEDS_TRUST).to_string()
        }
    }
}

/// The line under a feature's name: core's words for a file that is missing or sets nothing, what
/// the feature does otherwise, with the wait for trust when something is held back.
fn feature_description(feature: Feature, standing: FeatureStanding) -> String {
    match (feature, standing) {
        // The switch is how the file gets made.
        (Feature::Notes, FeatureStanding::Missing) => {
            text(SETTINGS_PROJECTS_NO_NOTES_MD_YET_ON_CREATES).to_string()
        }
        (_, FeatureStanding::NeedsTrust) => settings_projects_feature_needs_trust(feature.description()),
        (_, standing) => standing.hint(feature).unwrap_or_else(|| feature.description().to_string()),
    }
}

/// A feature's row: what it uses and its control (a switch for the defaults, Default | On |
/// Off for a project), faint while the project has no such file.
fn feature_row(
    ui: UiColors,
    feature: Feature,
    description: String,
    state: SettingState,
    control: AnyElement,
    cx: &App,
) -> AnyElement {
    style::setting(
        SharedString::from(format!("feature-row-{}", feature.key())),
        ui,
        SettingParts {
            label: feature.label().into(),
            description: Some(description.into()),
            visible_hint: state == SettingState::Disabled,
            chip: None,
            key: "",
            state,
            found: None,
            found_description: None,
            control,
            tall: false,
            extra: None,
            on_reset: None,
        },
        cx,
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use wormhole_core::project::{Feature, FeatureStanding, TrustStatus};

    use super::{feature_description, trust_detail};

    /// The page words a feature the way the Project tab does: a file that sets nothing says so
    /// rather than waiting for trust, and what is held back says it waits.
    #[test]
    fn a_feature_row_says_what_the_project_tab_says() {
        assert_eq!(
            feature_description(Feature::Settings, FeatureStanding::Empty),
            "Nothing set yet in config.json"
        );
        assert_eq!(
            feature_description(Feature::Actions, FeatureStanding::Missing),
            "No actions.json in this project"
        );
        assert_eq!(
            feature_description(Feature::Settings, FeatureStanding::NeedsTrust),
            format!("{}. Only after you trust the project", Feature::Settings.description())
        );
        assert_eq!(feature_description(Feature::Theme, FeatureStanding::Ready), Feature::Theme.description());
        // Notes can be switched on without the file: that makes it.
        assert!(feature_description(Feature::Notes, FeatureStanding::Missing).contains("On creates it"));
    }

    #[test]
    fn the_trust_row_names_what_waits_when_nothing_is_a_command() {
        let settings = [Feature::Settings];
        assert_eq!(
            trust_detail(TrustStatus::Untrusted, false, &settings),
            "Trusting it applies its settings in config.json."
        );
        assert_eq!(
            trust_detail(TrustStatus::Changed, false, &settings),
            "Trust it again to apply its settings in config.json."
        );
        assert_eq!(
            trust_detail(TrustStatus::Trusted, false, &settings),
            "Applies its settings in config.json."
        );
        // Commands keep the words they had; a project with nothing to approve says so.
        assert_eq!(
            trust_detail(TrustStatus::Untrusted, true, &settings),
            "Nothing in it can run commands until you trust it."
        );
        assert_eq!(trust_detail(TrustStatus::Untrusted, false, &[]), "Nothing in it needs trust.");
    }
}
