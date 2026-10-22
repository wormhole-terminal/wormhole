//! A terminal in a new git worktree (Shell → New Terminal in New Worktree…, the palette, and the
//! command line's `new-terminal --worktree <branch>`): the dialog that asks for the branch and
//! names the folder, what happens once the user confirms, the question before closing such a
//! terminal (keep the worktree, or remove it) and the removal. The rules and the git calls are
//! core's `git::worktree`; here is only what needs the window and the app's state.
//!
//! Nothing here runs on its own: the dialog is the one way a worktree gets made, so a script
//! asking through the command line only gets the dialog opened. A folder with an `.wormhole/`
//! project needs that project trusted and its settings on, as a terminal opened there would
//! take its env and shell; a repository without one needs nothing.

use std::path::{Path, PathBuf};

use gpui_kit::component::Sizable;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::button::{Button, ButtonVariant};
use gpui_kit::component::dialog::{Cancel, Confirm, DialogFooter};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;
use wormhole_core::git::worktree::{self, Worktree};
use wormhole_core::git::{find_root, worktree_of};
use wormhole_core::ipc::control::{Asked, Error, ErrorCode};
use wormhole_core::project::Feature;

use super::app::{AppState, app, app_mut};
use super::{Workspace, dialog, project, session};
use crate::settings::{settings, ui};
use crate::style::{self, TextRole};
use crate::terminal_view::Status;

/// The dialog's state while it is up: the branch field, what the worktree would be of, what to
/// run in the terminal once it opens, and why the last Create was refused.
pub(super) struct Asking {
    input: Entity<InputState>,
    repo: PathBuf,
    command: Option<String>,
    error: Option<SharedString>,
    /// ↩ in the field presses Create.
    _enter: Subscription,
}

/// `worktree_base` as a folder, `~` expanded; `None` puts worktrees beside the repository.
fn base(cx: &App) -> Option<PathBuf> {
    let base = settings(cx).worktree_base.trim();
    if base.is_empty() {
        return None;
    }
    let home = session::home_dir();
    Some(match (base.strip_prefix("~/"), home) {
        (Some(rest), Some(home)) => home.join(rest),
        _ if base == "~" => home.map_or_else(|| PathBuf::from(base), Path::to_path_buf),
        _ => PathBuf::from(base),
    })
}

/// Whether a worktree may be made of `repo`: a project folder there must be trusted with its
/// settings on, since the new terminal takes the project's env and shell; no project, no condition.
fn allowed(repo: &Path, cx: &mut App) -> Result<(), String> {
    // With the project-folder feature off, `.wormhole/` isn't read at all, so a worktree carries
    // no project settings and there's nothing to trust.
    if !project::project_folders_on(cx) {
        return Ok(());
    }
    let Some(folder) = wormhole_core::project::find_project(repo) else { return Ok(()) };
    let Some(project) = project::load_project_info(&folder.root, cx) else {
        return Err("The project's .wormhole folder can't be read".into());
    };
    if app(cx).hub.enabled(&project, Feature::Settings) {
        Ok(())
    } else {
        Err("Trust the project first: a terminal in its worktree would run with the project's settings"
            .into())
    }
}

/// What a new worktree of `repo` runs first: its trusted project's `worktree.setup`, under the
/// same conditions [`allowed`] checks (a project that isn't trusted with its settings on runs
/// nothing). It is the repository's project that is asked, not the copy of `.wormhole/` the new
/// worktree checks out: that one has not been trusted in its new folder.
fn setup_command(repo: &Path, cx: &mut App) -> Option<String> {
    if !project::project_folders_on(cx) {
        return None;
    }
    let folder = wormhole_core::project::find_project(repo)?;
    let project = project::load_project_info(&folder.root, cx)?;
    if !app(cx).hub.enabled(&project, Feature::Settings) {
        return None;
    }
    project.snapshot.config()?.worktree_setup.clone()
}

/// A path with the home folder as `~`.
fn shown(path: &Path) -> String {
    wormhole_core::sidebar::display_path(path, session::home_dir())
}

/// `message` in a dialog of its own, with nothing to do but close it.
fn tell(title: &'static str, message: String, window: &mut Window, cx: &mut App) {
    let message = SharedString::from(message);
    dialog::open_alert(window, cx, move |alert, _, _| {
        alert.title(title).description(message.clone()).show_cancel(false).ok_text("OK")
    });
}

impl Workspace {
    /// The repository the active terminal is in, as the main working tree (a terminal in a
    /// worktree makes its siblings of the same repository); `None` outside any.
    fn repo_for_worktree(&self, cx: &App) -> Option<PathBuf> {
        let cwd = self.active_terminal().read(cx).cwd.clone()?;
        let root = self.active_session().repo_root.of(&cwd);
        if !root.join(".git").exists() {
            return None;
        }
        Some(worktree_of(&root).unwrap_or(root))
    }

    /// Shell → New Terminal in New Worktree…: ask for the branch.
    pub(super) fn new_terminal_in_worktree(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repo) = self.repo_for_worktree(cx) else { return };
        if let Err(why) = allowed(&repo, cx) {
            tell("Can't make a worktree here", why, window, cx);
            return;
        }
        self.ask_worktree(repo, None, None, window, cx);
    }

    /// The command line's `new-terminal --worktree <branch>`: the dialog, filled in, in this
    /// window; the terminal opens (running `command`) once the user confirms.
    pub(super) fn control_ask_worktree(
        &mut self,
        folder: Option<String>,
        branch: &str,
        command: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Asked, Error> {
        let refuse = |code, message: String| Error { code, message };
        let start = folder.map(PathBuf::from).or_else(|| self.active_terminal().read(cx).cwd.clone());
        let Some(start) = start else {
            return Err(refuse(ErrorCode::NotFound, "no folder to make a worktree of".into()));
        };
        let Some(root) = find_root(&start) else {
            return Err(refuse(
                ErrorCode::NotFound,
                format!("{} is not in a git repository", start.display()),
            ));
        };
        let repo = worktree_of(&root).unwrap_or(root);
        allowed(&repo, cx).map_err(|why| refuse(ErrorCode::Refused, why))?;
        let plan = worktree::plan(&repo, branch, base(cx).as_deref())
            .map_err(|why| refuse(ErrorCode::BadRequest, why))?;
        if self.worktree_ask.is_some() {
            return Err(refuse(ErrorCode::Failed, "a worktree dialog is already up in that window".into()));
        }
        self.ask_worktree(repo, Some(plan.branch.clone()), command, window, cx);
        Ok(Asked {
            asked: true,
            window: self.window_id,
            branch: plan.branch,
            folder: plan.folder.to_string_lossy().into_owned(),
        })
    }

    /// The dialog: a branch field (filled with `branch` when given), the folder it would get
    /// under it, and Create. The folder follows the field as it is typed; a name git won't take
    /// keeps the dialog up with the reason under the field.
    fn ask_worktree(
        &mut self,
        repo: PathBuf,
        branch: Option<String>,
        command: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.worktree_ask.is_some() {
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("fix/login"));
        if let Some(branch) = branch {
            input.update(cx, |input, cx| input.set_value(branch, window, cx));
        }
        let enter = cx.subscribe_in(&input, window, |_, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                window.dispatch_action(Box::new(Confirm { secondary: false }), cx);
            }
        });
        self.worktree_ask =
            Some(Asking { input: input.clone(), repo: repo.clone(), command, error: None, _enter: enter });
        let repo_name = repo.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let weak = cx.weak_entity();
        // The Kit gives the keys back to whatever had them when the dialog opened, a quarter
        // second after it closes: by then Create has opened and selected the new terminal, and
        // the old one, not on screen any more, would take them. With nothing focused at the open
        // there is nothing to give back; `on_close` below hands them to the terminal in front.
        window.blur(cx);
        dialog::open_alert(window, cx, move |alert, _, cx| {
            let ui = ui(cx);
            // What the field says now, and why the last Create was refused.
            let (typed, error) = weak
                .read_with(cx, |this, cx| {
                    this.worktree_ask
                        .as_ref()
                        .map(|ask| (ask.input.read(cx).value().to_string(), ask.error.clone()))
                })
                .ok()
                .flatten()
                .unwrap_or_default();
            let branch = worktree::branch_name(&typed).unwrap_or_else(|_| "<branch>".into());
            let folder = shown(&worktree::folder_for(&repo, &branch, base(cx).as_deref()));
            let (confirm, dismiss) = (weak.clone(), weak.clone());
            alert
                .title("New terminal in a new worktree")
                .description(format!(
                    "A worktree of {repo_name} on a branch of its own, in a folder of its own, so work there \
                     touches nothing in the repository's folder."
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .pt_2()
                        .child(
                            div()
                                .text_caption()
                                .text_color(ui.muted)
                                .child("Branch (new, or one the repository has)"),
                        )
                        .child(Input::new(&input).small())
                        .child(div().text_caption().text_color(ui.muted).child(format!("Folder: {folder}")))
                        .children(
                            error.map(|why| div().text_caption().text_color(ui.warning_text).child(why)),
                        ),
                )
                .on_ok(move |_, window, cx| {
                    confirm.update(cx, |this, cx| this.confirm_worktree(window, cx)).unwrap_or(true)
                })
                .on_close(move |_, window, cx| {
                    let _ = dismiss.update(cx, |this, cx| {
                        this.worktree_ask = None;
                        cx.notify();
                        // The dialog had the keys; once it is gone (deferred: it is still
                        // closing), they go back to the terminal, or nothing has them.
                        cx.defer_in(window, |this, window, cx| {
                            let terminal = this.active_terminal().focus_handle(cx);
                            window.focus(&terminal, cx);
                        });
                    });
                })
                .footer(
                    DialogFooter::new()
                        .child(style::key_hint(ui, "esc", "cancels"))
                        .child(div().flex_1())
                        .child(Button::new("worktree-cancel").label("Cancel").on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(Cancel), cx);
                        }))
                        .child(
                            Button::new("worktree-create")
                                .label("Create")
                                .with_variant(ButtonVariant::Primary)
                                .on_click(|_, window, cx| {
                                    window.dispatch_action(Box::new(Confirm { secondary: false }), cx);
                                }),
                        ),
                )
        });
        // The keys go to the field once the dialog is up (it takes them itself as it opens).
        cx.on_next_frame(window, |this, window, cx| {
            if let Some(ask) = &this.worktree_ask {
                let field = ask.input.read(cx).focus_handle(cx);
                window.focus(&field, cx);
            }
        });
    }

    /// Create (or ↩): make the worktree the field names. Returns whether the dialog closes: not
    /// while the name is one git won't take, which the dialog then says.
    fn confirm_worktree(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(ask) = &self.worktree_ask else { return true };
        let typed = ask.input.read(cx).value().to_string();
        match worktree::plan(&ask.repo, &typed, base(cx).as_deref()) {
            Ok(plan) => {
                let command = ask.command.clone();
                self.worktree_ask = None;
                self.create_worktree(plan, command, window, cx);
                true
            }
            Err(why) => {
                if let Some(ask) = &mut self.worktree_ask {
                    ask.error = Some(why.into());
                }
                cx.notify();
                false
            }
        }
    }

    /// Make `plan`'s worktree off the UI thread, then open a terminal in it (running `command`),
    /// and remember it as one of Wormhole's. Git's refusal is shown in its words.
    fn create_worktree(
        &mut self,
        plan: Worktree,
        command: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The ports the other worktrees have, as they are now: the free block is found off the
        // UI thread (up to a few thousand binds), next to the git work.
        let others = app(cx).worktrees.clone();
        cx.spawn_in(window, async move |this, cx| {
            let made = plan.clone();
            let result = cx
                .background_spawn(async move {
                    worktree::add(&made).map(|()| others.free_ports(worktree::port_free))
                })
                .await;
            this.update_in(cx, |ws, window, cx| match result {
                Ok(port) => {
                    let mut plan = plan;
                    // Its own ports, before its first terminal starts with them in its env; a
                    // worktree made meanwhile may have taken that block, and then this one asks
                    // again here.
                    let taken = |port: u16| {
                        app(cx).worktrees.iter().any(|w| {
                            w.port.is_some_and(|p| (p..p + worktree::PORT_BLOCK).contains(&port))
                        })
                    };
                    plan.port = match port {
                        Some(port) if !taken(port) => Some(port),
                        _ => app(cx).worktrees.free_ports(worktree::port_free),
                    };
                    tracing::info!(branch = plan.branch, folder = %plan.folder.display(), port = ?plan.port, "worktree made");
                    let setup = setup_command(&plan.repo, cx);
                    app_mut(cx).record_worktree(plan.clone());
                    let folder = plan.folder.to_string_lossy().into_owned();
                    let ix = ws.open_session(None, Some(folder), window, cx);
                    let terminal = ws.sessions[ix].id;
                    // The project's setup first, then what was asked for, as one line: the command
                    // starts once the setup has finished and only if it worked, and a setup that
                    // asks a question doesn't get the command typed ahead as its answer.
                    let command = command.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
                    let line = match (setup, command) {
                        (Some(setup), Some(command)) => Some(format!("{setup} && {command}")),
                        (setup, command) => setup.or(command),
                    };
                    if let Some(line) = line {
                        ws.control_send_text(terminal, &line, true, cx);
                    }
                }
                Err(err) => {
                    tracing::warn!("making a worktree: {err:#}");
                    tell("Couldn't create the worktree", format!("{err:#}"), window, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// The worktree Wormhole made that session `ix`'s terminal is in, if any.
    pub(super) fn made_worktree_of(&self, ix: usize, cx: &App) -> Option<Worktree> {
        let session = self.sessions.get(ix)?;
        let cwd = session.terminal().read(cx).cwd.clone()?;
        app(cx).worktrees.find(&cwd).cloned()
    }

    /// Closing a terminal in a worktree Wormhole made: Close keeps the worktree, Close and Remove
    /// Worktree deletes its folder too (the branch stays); not offered while the folder has
    /// changes as last read, and refused again by git itself if it has some by then.
    pub(super) fn request_close_worktree_session(
        &mut self,
        ix: usize,
        worktree: Worktree,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.sessions.get(ix) else { return };
        if self.closing_session_prompt {
            return;
        }
        self.closing_session_prompt = true;
        cx.notify();
        let id = session.id;
        let needs = session.needs_close_confirmation(cx);
        let (status, command) = (session.status(cx), session.command_running(cx));
        let dirty = session.git.status.as_ref().is_some_and(|status| !status.files.is_empty());
        let mut detail = if needs {
            let agent = session.told_name(cx);
            let programs = session.running_programs(matches!(status, Status::Busy | Status::Attention), cx);
            session::close_session_detail(
                agent.as_deref(),
                session.open_items(cx),
                status,
                command,
                &programs,
            ) + " "
        } else {
            String::new()
        };
        detail.push_str(&format!(
            "Its worktree {} at {} was made by Wormhole. {}",
            worktree.branch,
            shown(&worktree.folder),
            if dirty {
                "It has uncommitted changes, so it stays."
            } else {
                "Close keeps it; Close and Remove Worktree deletes its folder (the branch stays)."
            }
        ));
        let question = dialog::CloseQuestion {
            title: session::close_session_title(&session.name(cx)).into(),
            detail: detail.into(),
            confirm: "Close",
        };
        let (confirm, remove, dismiss) = (cx.weak_entity(), cx.weak_entity(), cx.weak_entity());
        let close_by_id = move |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
            if let Some(ix) = this.sessions.iter().position(|session| session.id == id) {
                this.close_session(ix, window, cx);
            }
        };
        let also: Option<dialog::Also> = (!dirty).then(|| {
            let on_also: dialog::OnConfirm = std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let _ = remove.update(cx, |this, cx| close_by_id(this, window, cx));
                remove_worktree(worktree.clone(), cx);
            });
            ("Close and Remove Worktree", on_also)
        });
        dialog::ask_before_closing_with(
            window,
            cx,
            question,
            also,
            move |window, cx| {
                let _ = confirm.update(cx, |this, cx| close_by_id(this, window, cx));
            },
            move |cx| {
                let _ = dismiss.update(cx, |this, cx| {
                    this.closing_session_prompt = false;
                    cx.notify();
                });
            },
        );
    }
}

/// The worktrees Wormhole made that a group's terminals are in, for the question before closing the
/// group: each once, and whether it may be offered for removal (no changes as last read, and no
/// terminal outside the group still in it).
pub(super) struct GroupWorktrees(Vec<(Worktree, bool)>);

impl GroupWorktrees {
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// What the question adds: which worktrees, and what Close and Close and Remove do with them.
    pub(super) fn note(&self) -> String {
        let names = self
            .0
            .iter()
            .map(|(w, _)| format!("{} at {}", w.branch, shown(&w.folder)))
            .collect::<Vec<_>>()
            .join(", ");
        let kept = self.0.iter().filter(|(_, removable)| !removable).count();
        let removable = self.0.len() - kept;
        let many = if self.0.len() == 1 { "worktree" } else { "worktrees" };
        let mut note = format!("Wormhole made the {many} {names}. ");
        match (removable, kept) {
            (0, _) => note.push_str("They have changes or other terminals in them, so they stay."),
            (_, 0) => note.push_str(
                "Close keeps them; Close and Remove Worktrees deletes their folders (the branches stay).",
            ),
            _ => note.push_str(
                "Close keeps them; Close and Remove Worktrees deletes the folders without changes or other \
                 terminals (the branches stay).",
            ),
        }
        note
    }

    /// The worktrees "Close and Remove Worktrees" removes.
    pub(super) fn removable(&self) -> Vec<Worktree> {
        self.0.iter().filter(|(_, removable)| *removable).map(|(w, _)| w.clone()).collect()
    }
}

impl Workspace {
    /// The worktrees Wormhole made that the terminals `members` are in.
    pub(super) fn made_worktrees_of(&self, members: &[i64], cx: &App) -> GroupWorktrees {
        let mut found: Vec<(Worktree, bool)> = Vec::new();
        for session in members.iter().filter_map(|&id| self.find_session(id)).map(|ix| &self.sessions[ix]) {
            let Some(cwd) = session.terminal().read(cx).cwd.clone() else { continue };
            let Some(made) = app(cx).worktrees.find(&cwd).cloned() else { continue };
            if found.iter().any(|(w, _)| w.folder == made.folder) {
                continue;
            }
            let dirty = session.git.status.as_ref().is_some_and(|status| !status.files.is_empty());
            let shared = self.sessions.iter().any(|other| {
                !members.contains(&other.id)
                    && other.terminal().read(cx).cwd.as_ref().is_some_and(|c| c.starts_with(&made.folder))
            });
            found.push((made, !dirty && !shared));
        }
        GroupWorktrees(found)
    }
}

/// Remove `worktree` off the UI thread, once its terminal is closed; forgotten when it is gone,
/// and git's refusal (changes made in the meantime) shown in the front window.
pub(super) fn remove_worktree(worktree: Worktree, cx: &mut App) {
    cx.spawn(async move |cx| {
        let gone = worktree.clone();
        let result = cx.background_spawn(async move { worktree::remove(&gone) }).await;
        cx.update(|cx| match result {
            Ok(()) => {
                tracing::info!(folder = %worktree.folder.display(), "worktree removed");
                app_mut(cx).forget_worktree(&worktree.folder);
            }
            Err(err) => {
                tracing::warn!("removing a worktree: {err:#}");
                if let Some(front) = AppState::front_window(cx) {
                    let words = format!("{err:#}");
                    let _ = front.handle.update(cx, |_, window, cx| {
                        tell("Couldn't remove the worktree", words, window, cx);
                    });
                }
            }
        });
    })
    .detach();
}
