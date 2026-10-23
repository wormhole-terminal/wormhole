//! The right panel's Project tab: the colour of the active terminal's folder, then its
//! `.wormhole/` folder and its trust, bands for the per-feature switches, the actions, the
//! prompts and the terminal layout, with two-line rows like the sidebar's; or, in a folder with
//! no `.wormhole/`, the button that makes one.

use std::path::Path;

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::color_picker::ColorPicker;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::project::{ActionKind, Feature, TrustLine, TrustStatus, trust_applies};
use wormhole_core::theme::folder_color::FolderColor;
use wormhole_core::view::limits::count_grouped;
use wormhole_core::view::sidebar::display_path;
use wormhole_core::view::spoken;

use super::{LoadedProject, hub};
use crate::platform::auth;
use crate::settings::UiColors;
use crate::settings::{store_in, ui_in};
use crate::style::{self, TextRole};
use crate::theme_bridge;
use crate::window::Workspace;
use crate::window::folder_color;
use crate::window::session::home_dir;

/// How many lines the trust notice shows in full; a longer list shows [`TRUST_LINES_SHOWN`]
/// and "View all N…", which opens them in their own window.
const TRUST_LINES_MAX: usize = 6;
const TRUST_LINES_SHOWN: usize = 5;

impl Workspace {
    /// The Project tab: the colour of the terminal's folder, which works with or without a
    /// project folder, then the project folder or the way to make one.
    pub(in crate::window) fn render_project_tab(&self, cx: &Context<Self>) -> AnyElement {
        let folder = self.active_folder(cx);
        div()
            .flex()
            .flex_col()
            .children(folder.as_deref().map(|folder| self.render_folder_color(folder, cx)))
            .child(match self.active_project(cx) {
                Some(project) => self.render_project(project, cx),
                None => self.render_no_project(folder.as_deref(), cx),
            })
            .into_any_element()
    }

    /// "Folder colour": the eight colours and None in a row, the folder's marked. It is the same
    /// setting as the band's menu. Where it is kept depends on the folder: the person's own, or
    /// the project's `config.json` when it has one; the line under the row says which. A project
    /// that locks its colour (or whose file can't be read) shows the row dimmed, and a hex colour
    /// it sets by hand is shown as a swatch of its own, matching none of the names.
    fn render_folder_color(&self, folder: &Path, cx: &Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let paint = self.folder_paint(folder, cx);
        let (current, custom) = (paint.checked(), paint.custom());
        let enabled = paint.closed().is_none();
        let choice =
            |id: SharedString, name: &'static str, shown: Option<Hsla>, color: Option<FolderColor>| {
                let (target, paint) = (folder.to_path_buf(), paint.clone());
                let chosen = current == color && (color.is_some() || custom.is_none());
                style::color_choice(id, ui, name, shown, chosen, enabled)
                    .tooltip(move |window, cx| Tooltip::new(name).build(window, cx))
                    .when(enabled, |d| {
                        d.on_click(move |_, _, cx| folder_color::set_folder_color(&paint, &target, color, cx))
                    })
            };
        let row = div()
            .flex()
            .flex_wrap()
            // The first dot lines up with the panel's inset, inside its hit area.
            .px(px(style::CARD_INSET - (style::HIT_MIN - style::COLOR_DOT) / 2.0))
            .child(choice("folder-color-none".into(), "None", None, None))
            .children(custom.map(|color| {
                let hex = color.color().to_hex();
                style::color_choice(
                    "folder-color-custom",
                    ui,
                    format!("Custom colour {hex}"),
                    Some(theme_bridge::hsla(color.color())),
                    true,
                    false,
                )
                .tooltip(move |window, cx| {
                    Tooltip::new(format!("Custom colour {hex}, set in config.json")).build(window, cx)
                })
            }))
            .children(folder_color::choices(cx).into_iter().map(|(color, shown)| {
                choice(
                    format!("folder-color-{}", color.key()).into(),
                    color.label(),
                    Some(shown),
                    Some(color),
                )
            }))
            // Any other colour: the picker, which shows the folder's own and saves a hex colour.
            .children((enabled && self.picking_folder.is_none()).then(|| {
                let featured = folder_color::choices(cx).into_iter().map(|(_, shown)| shown).collect();
                div().flex().items_center().h(px(style::HIT_MIN)).pl_1().child(
                    ColorPicker::new(&self.folder_picker)
                        .icon(IconName::Plus)
                        .featured_colors(featured)
                        .anchor(Anchor::TopRight)
                        .accessibility_label("Pick a colour"),
                )
            }));
        style::Cards::new(ui)
            .head(section_band("project-folder-color", "Folder colour", None, ui))
            .child(row)
            .child(
                div()
                    .px(px(style::CARD_INSET))
                    .pb_2()
                    .text_caption()
                    .text_color(ui.muted)
                    .child(paint.note()),
            )
            .into_any_element()
    }

    /// A folder with no `.wormhole/`: what one is, and the button that makes it.
    fn render_no_project(&self, folder: Option<&Path>, cx: &Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let target = folder.map(|folder| {
            format!("Creates {}/.wormhole/", display_path(folder, home_dir()).trim_end_matches('/'))
        });
        style::Cards::new(ui)
            .head(section_band("project-folder", "Project folder", None, ui))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .px(px(style::CARD_INSET))
                    .py_2()
                    .text_body()
                    .text_color(ui.muted)
                    .child("This terminal's folder has no .wormhole/ project folder.")
                    .child(
                        "A committed .wormhole/ can hold project settings, a theme, actions, prompts, \
                         a terminal layout and shared notes. Every part is optional, and you choose which \
                         ones to use.",
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .items_start()
                            .child(
                                Button::new("create-project")
                                    .primary()
                                    .small()
                                    .label("Create project folder")
                                    .disabled(folder.is_none())
                                    .on_click(cx.listener(|this, _, _, cx| this.create_project_folder(cx))),
                            )
                            .child(
                                div().text_caption().text_color(ui.faint).child(target.unwrap_or_else(
                                    || "This terminal's folder isn't known yet.".to_string(),
                                )),
                            ),
                    )
                    .children(hub(cx).error.clone().map(|err| problem_line(ui, err))),
            )
            .into_any_element()
    }

    /// The project at the terminal's folder: its trust, then a band for each part.
    fn render_project(&self, project: &LoadedProject, cx: &Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let root = project.folder.root.clone();
        let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();

        // The folder and its trust, the header the sections hang from.
        let mut body = style::Cards::new(ui).child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .h(px(style::BAND_H))
                .px(px(style::CARD_INSET))
                .child(style::lead(style::icon(IconName::Folder).text_color(ui.muted)))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_body()
                        .child(format!("{name}/{}", project.folder.dir_name())),
                )
                .child(div().flex_1())
                .child(trust_badge(ui, project.status)),
        );

        // Trust: only matters if something here can run commands or is held back until the
        // project is trusted (settings too: `font_size` alone needs it).
        let commands = project.request.as_ref().map(|r| r.commands.clone()).unwrap_or_default();
        let needing = project.snapshot.needing_trust();
        if project.request.is_some() && (!commands.is_empty() || !needing.is_empty()) {
            body = body.child(self.render_trust(project.status, &commands, &needing, cx));
        }
        if let Some(err) = &hub(cx).error {
            body = body.child(problem_line(ui, err.clone()));
        }
        let notes_error = hub(cx).notes_errors.get(&root).map(|err| format!("Notes sync: {err}"));
        for err in project.errors.iter().chain(notes_error.as_ref()) {
            body = body.child(problem_line(ui, err.clone()));
        }
        // What loaded but was set aside in part: an action's key that is taken.
        for warning in &project.warnings {
            body = body.child(problem_line(ui, warning.clone()).text_color(ui.warning_text));
        }

        // Feature switches (per project, for you only).
        body = body.head(section_band("project-use", "Use from this project", None, ui));
        for feature in Feature::ALL {
            let present = project.snapshot.has(feature);
            if !present && feature != Feature::Notes {
                continue;
            }
            let prefs = &hub(cx).prefs;
            // Notes sync where the file is, not wherever the switch defaults to.
            let on = match feature {
                Feature::Notes => prefs.notes_sync(&root, present),
                _ => prefs.enabled(&root, feature),
            };
            let (hint, warns) = feature_hint(project, feature);
            body = body.child(style::two_line_row(
                SharedString::from(format!("feature-row-{}", feature.key())),
                ui,
                style::TwoLineRow {
                    lead: style::icon(feature_icon(feature)).text_color(ui.muted).into_any_element(),
                    title: feature.label().into(),
                    detail: Some(hint.into()),
                    detail_color: warns.then_some(ui.warning_text),
                    trailing: Some(
                        Switch::new(SharedString::from(format!("feature-{}", feature.key())))
                            .checked(on)
                            .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                this.set_project_feature(feature, *checked, cx)
                            }))
                            .into_any_element(),
                    ),
                    actions: Vec::new(),
                },
            ));
        }

        // Actions: a click runs one.
        let actions = project.actions();
        if hub(cx).enabled(project, Feature::Actions) && !actions.is_empty() {
            body = body.head(section_band("project-actions", "Actions", Some(actions.len()), ui));
            for action in actions {
                let id = action.id.clone();
                body = body.child(
                    style::two_line_row(
                        SharedString::from(format!("action-{}", action.id)),
                        ui,
                        style::TwoLineRow {
                            lead: style::icon(IconName::Play).text_color(ui.primary).into_any_element(),
                            title: action.title.clone().into(),
                            detail: action_detail(action).map(SharedString::from),
                            detail_color: None,
                            trailing: action.key_spec().map(|key| {
                                div()
                                    .text_caption()
                                    .text_color(ui.faint)
                                    .child(key.glyphs())
                                    .into_any_element()
                            }),
                            actions: Vec::new(),
                        },
                    )
                    .cursor_pointer()
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.run_project_action(&id, window, cx)),
                    ),
                );
            }
        }

        // Prompts: paste one, or copy it into a note to edit first.
        let prompts = project.snapshot.prompts();
        if hub(cx).enabled(project, Feature::Prompts) && !prompts.is_empty() {
            body = body.head(section_band("project-prompts", "Prompts", Some(prompts.len()), ui));
            for prompt in prompts {
                let (send_id, note_id) = (prompt.id.clone(), prompt.id.clone());
                let actions = vec![
                    style::item_action(
                        SharedString::from(format!("prompt-send-{}", prompt.id)),
                        ui,
                        IconName::ArrowUpRight,
                        "Paste into the terminal",
                        cx.listener(move |this, _, window, cx| this.send_prompt(&send_id, false, window, cx)),
                    ),
                    style::item_action(
                        SharedString::from(format!("prompt-note-{}", prompt.id)),
                        ui,
                        IconName::FilePlus,
                        "Copy into a new note",
                        cx.listener(move |this, _, window, cx| this.prompt_to_note(&note_id, window, cx)),
                    ),
                ];
                body = body.child(style::two_line_row(
                    SharedString::from(format!("prompt-{}", prompt.id)),
                    ui,
                    style::TwoLineRow {
                        lead: style::icon(IconName::FileText).text_color(ui.muted).into_any_element(),
                        title: prompt.title.clone().into(),
                        detail: prompt.description.clone().map(SharedString::from),
                        detail_color: None,
                        trailing: None,
                        actions,
                    },
                ));
            }
        }

        // Terminal layout: what it opens, and the button that opens it.
        if hub(cx).enabled(project, Feature::Layout)
            && let Some(layout) = project.snapshot.layout().filter(|l| !l.sessions.is_empty())
        {
            let names: Vec<String> = layout.sessions.iter().map(|c| c.name.clone()).collect();
            body = body.head(section_band("project-layout", "Terminal layout", Some(names.len()), ui)).child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .h(px(style::TITLE_ROW_H))
                    .pl(px(style::CARD_INSET + style::INDENT))
                    .pr(px(style::CARD_INSET))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_caption()
                            .text_color(ui.muted)
                            .child(names.join(" · ")),
                    )
                    .child(
                        Button::new("open-layout").primary().xsmall().label("Open terminals").on_click(
                            cx.listener(|this, _, window, cx| this.open_project_layout(window, cx)),
                        ),
                    ),
            );
        }

        body.into_any_element()
    }

    /// Trusted: one quiet line with Revoke. Otherwise the amber callout: what the project
    /// wants to run, each line in full with what runs in its own colour, and the one button that
    /// approves it. A long list shows its first lines and "View all N…", which opens every line
    /// in a window of its own. A project with no command to list (only settings, say) gets the
    /// callout all the same, saying what trusting applies: that is held back too.
    fn render_trust(
        &self,
        status: TrustStatus,
        commands: &[TrustLine],
        needing: &[Feature],
        cx: &Context<Self>,
    ) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let applies = trust_applies(needing);
        if status == TrustStatus::Trusted {
            let line = if commands.is_empty() {
                format!("Applies {applies}.")
            } else {
                "Actions, the terminal layout and project env may run.".to_string()
            };
            return div()
                .flex()
                .items_center()
                .gap_2()
                .h(px(style::ROW_H))
                .px(px(style::CARD_INSET))
                .child(style::lead(style::icon(IconName::ShieldCheck).text_color(ui.success_text)))
                .child(div().flex_1().min_w_0().truncate().text_caption().text_color(ui.muted).child(line))
                .child(
                    Button::new("revoke-trust")
                        .ghost()
                        .xsmall()
                        .label("Revoke trust")
                        .on_click(cx.listener(|this, _, _, cx| this.revoke_active_project(cx))),
                )
                .into_any_element();
        }
        let (title, text) = trust_notice(status, !commands.is_empty(), &applies);
        let approve_label = if hub(cx).approving {
            "Waiting for approval…"
        } else if auth::biometrics_available() {
            "Trust with Touch ID"
        } else {
            "Trust (approve with password)"
        };
        // Every line wrapped, never cut off: a cut-off line is where a payload would hide. Past
        // a few lines the rest are a window away, with the same approval there.
        let command_color = crate::theme_bridge::token(&store_in(&self.profile, cx).tokens, "trust.command");
        let shown = if commands.len() > TRUST_LINES_MAX { TRUST_LINES_SHOWN } else { commands.len() };
        let lines = commands.iter().take(shown).map(|line| trust_line(ui, command_color, line));
        let view_all = (shown < commands.len()).then(|| {
            let root = self.active_session().project.clone();
            Button::new("trust-view-all")
                .ghost()
                .small()
                .label(format!("View all {}…", commands.len()))
                .on_click(move |_, _, cx| {
                    if let Some(root) = root.clone() {
                        super::open_trust_window(root, cx);
                    }
                })
        });
        let mut body = vec![div().text_caption().text_color(ui.muted).child(text).into_any_element()];
        if !commands.is_empty() {
            body.push(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_2()
                    .rounded(ui.radius)
                    .bg(ui.hover)
                    .text_caption()
                    .font_family(crate::fonts::code_font_in(&self.profile, cx).0)
                    .font_features(crate::fonts::code_font_features_in(&self.profile, cx))
                    .children(lines)
                    .into_any_element(),
            );
        }
        body.push(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    Button::new("trust-project")
                        .primary()
                        .small()
                        .label(approve_label)
                        .on_click(cx.listener(|this, _, _, cx| this.approve_active_project(cx))),
                )
                .children(view_all)
                .into_any_element(),
        );
        div()
            .pt_1()
            .pb_2()
            .child(style::callout(ui, style::BannerKind::Warning, title, body))
            .into_any_element()
    }
}

/// A section of the Project tab: the same card head as Notes and Changes, open, with its count.
fn section_band(id: &'static str, title: &'static str, count: Option<usize>, ui: UiColors) -> AnyElement {
    style::section_card(
        id,
        ui,
        style::BandParts {
            kind: style::BandKind::Section,
            section: style::Section::Open,
            title: title.into(),
            detail: None,
            count: count.map(|n| count_grouped(n).into()),
            waiting: None,
            add: None,
            on_toggle: None,
            drop_target: false,
            sticky: false,
            joined: false,
        },
        Vec::new(),
    )
    .when_some(count, |d, n| d.aria_label(spoken::section_band(title, n, spoken::Counted::Items)))
    .into_any_element()
}

/// A line about something the project couldn't load, in the danger color.
fn problem_line(ui: UiColors, text: impl Into<SharedString>) -> Div {
    div().px(px(style::CARD_INSET)).py_1().text_caption().text_color(ui.danger_text).child(text.into())
}

/// One line of what a project wants to run, as the trust notice and its window draw it: what
/// runs it muted, what runs in `command_color` (`trust.command`), where after it, as one wrapped
/// paragraph. The caller sets the code font.
pub(super) fn trust_line(ui: UiColors, command_color: Hsla, line: &TrustLine) -> AnyElement {
    let mut text = line.what.clone();
    let runs = if line.runs.is_empty() {
        None
    } else {
        text.push(' ');
        let start = text.len();
        text.push_str(&line.runs);
        Some(start..text.len())
    };
    if !line.note.is_empty() {
        text.push(' ');
        text.push_str(&line.note);
    }
    let highlight = HighlightStyle { color: Some(command_color), ..Default::default() };
    div()
        .text_color(ui.muted)
        .child(StyledText::new(text).with_highlights(runs.map(|range| (range, highlight))))
        .into_any_element()
}

/// The project's trust as a badge: trusted, changed, not trusted.
pub(super) fn trust_badge(ui: UiColors, status: TrustStatus) -> impl IntoElement {
    let (text, color) = match status {
        TrustStatus::Trusted => ("trusted", ui.success_text),
        TrustStatus::Changed => ("changed", ui.warning_text),
        TrustStatus::Untrusted => ("not trusted", ui.muted),
    };
    style::status_badge(ui, color, text)
}

/// The callout's title and text for a project that isn't trusted: what it wants to run, or, with
/// no command to list, what trusting applies (`applies`, from `trust_applies`).
fn trust_notice(status: TrustStatus, has_commands: bool, applies: &str) -> (&'static str, String) {
    match (status, has_commands) {
        (TrustStatus::Changed, true) => (
            "Files changed since you trusted this project",
            "Review what it would run now and approve again.".to_string(),
        ),
        (TrustStatus::Changed, false) => (
            "Files changed since you trusted this project",
            format!("Trusting it applies {applies}. Review it and approve again."),
        ),
        (_, true) => (
            "Trust this project?",
            "It wants to run these commands. Nothing runs until you approve.".to_string(),
        ),
        (_, false) => (
            "Trust this project?",
            format!("Trusting it applies {applies}. Nothing applies until you approve."),
        ),
    }
}

/// The line under a feature's switch, and whether it is a warning: what is held back by trust is,
/// a file that sets nothing yet is not (a new project's `config.json` has every key commented out).
/// Notes have their own line: the switch is how the file gets made.
fn feature_hint(project: &LoadedProject, feature: Feature) -> (String, bool) {
    if feature == Feature::Notes {
        let hint = if project.snapshot.has(feature) {
            feature.description()
        } else {
            "On creates notes.md and syncs notes"
        };
        return (hint.to_string(), false);
    }
    let standing = project.snapshot.standing(feature, project.status);
    (standing.hint(feature).unwrap_or_else(|| feature.description().to_string()), standing.warns())
}

/// The glyph in front of a feature's switch.
fn feature_icon(feature: Feature) -> IconName {
    match feature {
        Feature::Settings => IconName::Settings,
        Feature::Theme => IconName::Palette,
        Feature::Actions => IconName::Play,
        Feature::Prompts => IconName::FileText,
        Feature::Layout => IconName::SquareTerminal,
        Feature::Notes => IconName::FilePlus,
    }
}

/// An action's second line: its description, or else what it does.
fn action_detail(action: &wormhole_core::project::Action) -> Option<String> {
    if let Some(description) = &action.description {
        return Some(description.clone());
    }
    match &action.kind {
        ActionKind::Run { command, .. } => Some(command.clone()),
        ActionKind::Prompt { file: Some(file), .. } => Some(format!("prompts/{file}")),
        ActionKind::Prompt { text, .. } => text.as_ref().and_then(|t| t.lines().next()).map(str::to_string),
        ActionKind::Open { url } => Some(url.clone()),
        ActionKind::Chain { steps } => Some(format!("{} steps", steps.len())),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use wormhole_core::project::{self as proj, Feature, ProjectFolder, TrustStatus, TrustStore};

    use super::{feature_hint, trust_notice};
    use crate::window::features::project::{LoadedProject, load_project};

    /// A fresh directory under the system temp dir, removed when dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let path = std::env::temp_dir().join(format!("hd-{name}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&path).expect("mkdir");
            Self(path.canonicalize().expect("canonical"))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn loaded(repo: &Scratch, store: &TrustStore) -> LoadedProject {
        load_project(ProjectFolder::open(repo.path()).expect("a project"), store)
    }

    /// A project made the way the Project tab makes one, with `config` in place of the template when given.
    fn project(config: Option<&str>) -> (Scratch, Scratch, TrustStore) {
        let (repo, data) = (Scratch::new("hint-repo"), Scratch::new("hint-data"));
        proj::init(repo.path()).expect("created");
        if let Some(config) = config {
            std::fs::write(repo.path().join(".wormhole/config.json"), config).expect("write");
        }
        let (store, err) = TrustStore::load_or_recover(data.path());
        assert!(err.is_none());
        (repo, data, store)
    }

    /// A new project's `config.json` sets nothing: its row says so in the neutral colour instead of
    /// warning that it needs trust, which was the first thing a new project showed.
    #[test]
    fn a_new_projects_settings_row_is_not_a_warning() {
        let (repo, _data, store) = project(None);
        let project = loaded(&repo, &store);
        assert_eq!(project.status, TrustStatus::Untrusted);
        assert_eq!(
            feature_hint(&project, Feature::Settings),
            ("Nothing set yet in config.json".into(), false)
        );
        assert!(project.snapshot.needing_trust().is_empty(), "so there is no trust section either");
    }

    #[test]
    fn what_is_set_waits_for_trust_and_says_so_until_it_has_it() {
        let (repo, _data, mut store) = project(Some(r#"{ "font_size": 15 }"#));
        let project = loaded(&repo, &store);
        let held = ("Needs trust before it takes effect".to_string(), true);
        assert_eq!(feature_hint(&project, Feature::Settings), held);
        assert_eq!(project.snapshot.needing_trust(), [Feature::Settings], "the trust section shows");
        assert!(project.request.as_ref().is_some_and(|r| r.commands.is_empty()), "with no command to list");

        let request = project.request.clone().expect("request");
        store.trust_request(&request);
        let trusted = LoadedProject::new(Arc::clone(&project.snapshot), &store);
        assert_eq!(trusted.status, TrustStatus::Trusted);
        assert_eq!(
            feature_hint(&trusted, Feature::Settings),
            (Feature::Settings.description().to_string(), false)
        );

        // Edited since: held back again, until it is approved again.
        std::fs::write(repo.path().join(".wormhole/config.json"), r#"{ "font_size": 16 }"#).expect("write");
        let changed = loaded(&repo, &store);
        assert_eq!(changed.status, TrustStatus::Changed);
        assert_eq!(feature_hint(&changed, Feature::Settings), held);
    }

    #[test]
    fn the_notes_row_says_what_the_switch_does() {
        let (repo, _data, store) = project(None);
        let with_file = loaded(&repo, &store);
        assert_eq!(
            feature_hint(&with_file, Feature::Notes),
            (Feature::Notes.description().to_string(), false)
        );
        std::fs::remove_file(repo.path().join(".wormhole/notes.md")).expect("rm");
        let without = loaded(&repo, &store);
        assert_eq!(
            feature_hint(&without, Feature::Notes),
            ("On creates notes.md and syncs notes".to_string(), false)
        );
    }

    #[test]
    fn the_notice_says_what_trusting_applies_when_there_is_no_command() {
        let applies = "its settings in config.json";
        let (title, text) = trust_notice(TrustStatus::Untrusted, false, applies);
        assert_eq!(title, "Trust this project?");
        assert_eq!(
            text,
            "Trusting it applies its settings in config.json. Nothing applies until you approve."
        );
        let (title, text) = trust_notice(TrustStatus::Changed, false, applies);
        assert_eq!(title, "Files changed since you trusted this project");
        assert!(text.starts_with("Trusting it applies its settings in config.json."), "{text}");
        // With commands it is the same words as before.
        assert_eq!(
            trust_notice(TrustStatus::Untrusted, true, applies).1,
            "It wants to run these commands. Nothing runs until you approve."
        );
        assert_eq!(
            trust_notice(TrustStatus::Changed, true, applies).1,
            "Review what it would run now and approve again."
        );
    }
}
