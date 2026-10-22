//! The window behind the trust notice's "View all N…": every line a project wants to
//! run, in full, with what runs set apart in its own colour, and the same approval the notice
//! offers, so a long list is reviewed and approved in one place. Built like the About window
//! (`aux_window`): opaque, one at a time, closed with Esc or ⌘W; it follows the hub's copy of
//! the project, so a `git pull` while it is open shows what would run now.

use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::project::{TrustLine, TrustStatus};

use super::{LoadedProject, approve_project};
use crate::app_state::{app, app_mut};
use crate::platform::auth;
use crate::settings::{SettingsStore, ui};
use crate::style::{self, Edges, TextRole};
use crate::workspace::Workspace;
use crate::workspace::aux_window;

const SIZE: (f32, f32) = (640.0, 480.0);
const MIN: (f32, f32) = (480.0, 320.0);
/// Lines laid out above and below the list's view, so scrolling doesn't pop them in.
const LIST_OVERDRAW: f32 = 200.0;

pub(crate) struct TrustWindow {
    root: PathBuf,
    list: ListState,
    focus_handle: FocusHandle,
}

/// Open the window for the project at `root`, or bring it to the front showing that project.
pub(crate) fn open_trust_window(root: PathBuf, cx: &mut App) {
    if let Some((handle, view)) = app(cx).trust_window.clone() {
        let shown = handle.update(cx, |_, window, cx| {
            if let Some(view) = view.upgrade().and_then(|view| view.downcast::<TrustWindow>().ok()) {
                view.update(cx, |view, cx| view.show(root.clone(), cx));
            }
            crate::background::activate_window(window, cx);
        });
        if shown.is_ok() {
            return;
        }
    }
    let options = aux_window::window_options(
        "Project commands",
        size(px(SIZE.0), px(SIZE.1)),
        size(px(MIN.0), px(MIN.1)),
        cx,
    );
    // Deferred: the new window renders right away, and this may run inside another window's
    // update (the Project tab's button).
    cx.defer(move |cx| {
        let opened = crate::background::open_window(options, cx, |window, cx| {
            cx.new(|cx| TrustWindow::new(root, window, cx))
        });
        match opened {
            Ok((handle, view)) => app_mut(cx).trust_window = Some((handle, view.downgrade().into())),
            Err(err) => tracing::error!("opening the project commands window failed: {err:#}"),
        }
    });
}

/// What the window shows of a project, read from the hub before anything borrows `cx` again.
struct Shown {
    name: String,
    /// The project folder's name on disk (`.wormhole`, or an older project's `.andromeda`).
    dir: String,
    status: TrustStatus,
    lines: Vec<TrustLine>,
    /// Why the project can't be described (a file that doesn't load).
    problem: Option<String>,
}

impl Shown {
    fn of(project: Option<&LoadedProject>, root: &Path) -> Self {
        let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let dir = wormhole_core::project::dir_name_at(root);
        let Some(project) = project else {
            return Self {
                name,
                dir,
                status: TrustStatus::Untrusted,
                lines: Vec::new(),
                problem: Some("This project is no longer open in a terminal.".into()),
            };
        };
        let (lines, problem) = match &project.request {
            Some(request) => (request.commands.clone(), None),
            None => (Vec::new(), Some(project.errors.first().cloned().unwrap_or_default())),
        };
        Self { name, dir, status: project.status, lines, problem }
    }
}

impl TrustWindow {
    fn new(root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        Self { root, list: ListState::new(0, ListAlignment::Top, px(LIST_OVERDRAW)), focus_handle }
    }

    fn show(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        self.root = root;
        cx.notify();
    }
}

impl Render for TrustWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        // Opaque: a window of text to read, not a place for the blur.
        let surfaces = aux_window::surfaces(cx, false);
        let shown = Shown::of(app(cx).hub.projects.get(&self.root), &self.root);
        let approving = app(cx).hub.approving;
        if self.list.item_count() != shown.lines.len() {
            self.list.reset(shown.lines.len());
        }
        let (code_font, _) = crate::fonts::code_font(cx);
        let command_color = crate::theme_bridge::token(&cx.global::<SettingsStore>().tokens, "trust.command");
        let lines = Rc::new(shown.lines);
        let rows = {
            let lines = lines.clone();
            list(self.list.clone(), move |ix, _, _| match lines.get(ix) {
                Some(line) => div()
                    .px_6()
                    .py_1()
                    .child(super::panel::trust_line(ui, command_color, line))
                    .into_any_element(),
                None => div().into_any_element(),
            })
            .size_full()
        };
        let text = match shown.status {
            TrustStatus::Trusted => "Trusted: actions, the terminal layout and project env may run.",
            TrustStatus::Changed => {
                "Files changed since you trusted this project. Review what it would run now and approve again."
            }
            TrustStatus::Untrusted => "It wants to run these commands. Nothing runs until you approve.",
        };
        let root = self.root.clone();
        let approve = (shown.status != TrustStatus::Trusted && shown.problem.is_none()).then(|| {
            let label = if approving {
                "Waiting for approval…"
            } else if auth::biometrics_available() {
                "Trust with Touch ID"
            } else {
                "Trust (approve with password)"
            };
            Button::new("trust-window-approve").primary().small().label(label).on_click(move |_, _, cx| {
                approve_project(&root, cx);
            })
        });

        div()
            .id("trust-window")
            .track_focus(&self.focus_handle)
            .key_context("TrustWindow")
            .on_key_down(cx.listener(|_, event: &KeyDownEvent, window, _| {
                aux_window::close_on_esc_or_cmd_w(event, window);
            }))
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(surfaces.background)
            .text_color(ui.text)
            // First, so it is behind what is drawn over it (`chrome::TitleDrag`).
            .child(aux_window::drag_strip())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .px_6()
                    .pt(px(style::TITLE_ROW_H))
                    .pb_3()
                    .rule_bottom(ui)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(style::lead(style::icon(IconName::Folder).text_color(ui.muted)))
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_heading()
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!("{}/{}", shown.name, shown.dir)),
                            )
                            .child(div().flex_1())
                            .child(super::panel::trust_badge(ui, shown.status)),
                    )
                    .child(div().text_body().text_color(ui.muted).child(text)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .py_2()
                    .text_caption()
                    .font_family(code_font)
                    .font_features(crate::fonts::code_font_features(cx))
                    .map(|d| match &shown.problem {
                        Some(problem) => d.child(
                            div().px_6().py_1().text_body().text_color(ui.danger_text).child(problem.clone()),
                        ),
                        None if lines.is_empty() => d.child(
                            div()
                                .px_6()
                                .py_1()
                                .text_body()
                                .text_color(ui.muted)
                                .child("Nothing here would run."),
                        ),
                        None => d.child(rows),
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_2()
                    .h(px(style::TITLE_ROW_H))
                    .px_6()
                    .rule_top(ui)
                    .child(div().text_caption().text_color(ui.faint).child(match lines.len() {
                        1 => "1 line".to_string(),
                        n => format!("{n} lines"),
                    }))
                    .child(div().flex_1())
                    .children(approve),
            )
    }
}

impl Workspace {
    /// devctl `trust-window`: the window for the active terminal's project.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_trust_window(&self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let root = self.active_session().project.clone().ok_or_else(|| anyhow::anyhow!("no project here"))?;
        cx.defer(move |cx| open_trust_window(root, cx));
        Ok(())
    }
}

/// devctl `trust-shot [name]`: the window rendered to `<dir>/<name>.png`.
#[cfg(feature = "devctl")]
pub(crate) fn devctl_trust_shot(name: &str, dir: &Path, cx: &mut App) -> anyhow::Result<String> {
    let (handle, _) =
        app(cx).trust_window.clone().ok_or_else(|| anyhow::anyhow!("that window is not open"))?;
    let path = dir.join(format!("{name}.png"));
    // Draw first: a window behind others may not have redrawn since its last change.
    let image = handle.update(cx, |_, window, cx| {
        window.draw(cx).clear(cx);
        window.render_to_image()
    })??;
    image.save(&path)?;
    Ok(format!("saved {}\n", path.display()))
}
