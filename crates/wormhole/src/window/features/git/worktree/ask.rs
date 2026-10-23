//! The dialog that asks for the branch, and what happens when the person confirms it.

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
use wormhole_core::storage::paths::ProfileId;

use super::Worktrees;
use crate::app_state::{app, app_mut};
use crate::settings::{settings, ui};
use crate::style::{self, TextRole};
use crate::terminal_view::TerminalView;
use crate::window::dialog;
use crate::window::feature::{FeatureEvent, TerminalInput, TerminalTarget};
use crate::window::features::project;

/// The dialog's state while it is up: the branch field, what the worktree would be of, what to run in the
/// terminal once it opens and why the last Create was refused.
pub(super) struct Asking {
    pub(super) input: Entity<InputState>,
    repo: PathBuf,
    /// The profile of the window it was asked in, kept up to date if the window moves
    /// ([`Worktrees::follow_profile`]): whose approvals the project's setup follows.
    profile: ProfileId,
    command: Option<String>,
    pub(super) error: Option<SharedString>,
    /// ↩ in the field presses Create.
    _enter: Subscription,
}

/// `worktree_base` as a folder, `~` expanded; `None` puts worktrees beside the repository.
pub(super) fn base(cx: &App) -> Option<PathBuf> {
    let base = settings(cx).worktree_base.trim();
    if base.is_empty() {
        return None;
    }
    let home = crate::style::home_dir();
    Some(match (base.strip_prefix("~/"), home) {
        (Some(rest), Some(home)) => home.join(rest),
        _ if base == "~" => home.map_or_else(|| PathBuf::from(base), Path::to_path_buf),
        _ => PathBuf::from(base),
    })
}

/// A path with the home folder as `~`.
pub(super) fn shown(path: &Path) -> String {
    wormhole_core::view::sidebar::display_path(path, crate::style::home_dir())
}

/// `message` in a dialog of its own, with nothing to do but close it.
pub(super) fn tell(title: &'static str, message: String, window: &mut Window, cx: &mut App) {
    let message = SharedString::from(message);
    dialog::open_alert(window, cx, move |alert, _, _| {
        alert.title(title).description(message.clone()).show_cancel(false).ok_text("OK")
    });
}

impl Worktrees {
    /// The repository a terminal in `band_folder` (its repository's root, else its folder: the window's
    /// cached answer) is in, as the main working tree (a terminal in a worktree makes its siblings of the
    /// same repository); `None` outside any.
    fn repo_for_worktree(band_folder: PathBuf) -> Option<PathBuf> {
        if !band_folder.join(".git").exists() {
            return None;
        }
        Some(worktree_of(&band_folder).unwrap_or(band_folder))
    }

    /// Shell → New Terminal in New Worktree…: ask for the branch, for the terminal in front, which is in
    /// `band_folder` (`SessionFacts::band_folder`).
    pub(in crate::window) fn new_terminal_in_worktree(
        &mut self,
        profile: &ProfileId,
        band_folder: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(repo) = band_folder.and_then(Self::repo_for_worktree) else { return };
        if let Err(why) = project::api::worktree_allowed(profile, &repo, cx) {
            tell("Can't make a worktree here", why, window, cx);
            return;
        }
        self.ask_worktree(profile.clone(), repo, None, None, window, cx);
    }

    /// The command line's `new-terminal --worktree <branch>`: the dialog, filled in, in this window;
    /// the terminal opens (running `command`) once the person confirms. `window_id` is the window's, for
    /// the answer.
    #[expect(clippy::too_many_arguments, reason = "what the command line asked, and where")]
    pub(super) fn control_ask_worktree(
        &mut self,
        profile: &ProfileId,
        terminal: &Entity<TerminalView>,
        window_id: i64,
        folder: Option<String>,
        branch: &str,
        command: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Asked, Error> {
        let refuse = |code, message: String| Error { code, message };
        let start = folder.map(PathBuf::from).or_else(|| terminal.read(cx).cwd.clone());
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
        project::api::worktree_allowed(profile, &repo, cx).map_err(|why| refuse(ErrorCode::Refused, why))?;
        let plan = worktree::plan(&repo, branch, base(cx).as_deref())
            .map_err(|why| refuse(ErrorCode::BadRequest, why))?;
        if self.asking.is_some() {
            return Err(refuse(ErrorCode::Failed, "a worktree dialog is already up in that window".into()));
        }
        self.ask_worktree(profile.clone(), repo, Some(plan.branch.clone()), command, window, cx);
        Ok(Asked {
            asked: true,
            window: window_id,
            branch: plan.branch,
            folder: plan.folder.to_string_lossy().into_owned(),
        })
    }

    /// The dialog: a branch field (filled with `branch` when given), the folder it would get under it, and
    /// Create. The folder follows the field as it is typed; a name git won't take keeps the dialog up with
    /// the reason under the field.
    fn ask_worktree(
        &mut self,
        profile: ProfileId,
        repo: PathBuf,
        branch: Option<String>,
        command: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.asking.is_some() {
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
        self.asking = Some(Asking {
            input: input.clone(),
            repo: repo.clone(),
            profile,
            command,
            error: None,
            _enter: enter,
        });
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
                    this.asking
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
                        this.asking = None;
                        cx.notify();
                        // The dialog had the keys; once it is gone (deferred: it is still closing)
                        // they go to the terminal in front, or nothing has them: a Create that git
                        // refuses opens no terminal to take them.
                        cx.defer_in(window, |_, _, cx| cx.emit(FeatureEvent::FocusTerminal));
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
            if let Some(ask) = &this.asking {
                let field = ask.input.read(cx).focus_handle(cx);
                window.focus(&field, cx);
            }
        });
    }

    /// The window now lives in `profile` (it was moved to another): the approvals the dialog's Create
    /// follows are that profile's, not those of the one it was opened in.
    pub(super) fn follow_profile(&mut self, profile: &ProfileId) {
        if let Some(ask) = &mut self.asking {
            ask.profile = profile.clone();
        }
    }

    /// The profile whose approvals Create would follow.
    #[cfg(test)]
    pub(super) fn asked_as(&self) -> Option<&ProfileId> {
        self.asking.as_ref().map(|ask| &ask.profile)
    }

    /// Create (or ↩): make the worktree the field names. Returns whether the dialog closes: not while
    /// the name is one git won't take, which the dialog then says.
    pub(super) fn confirm_worktree(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(ask) = &self.asking else { return true };
        let typed = ask.input.read(cx).value().to_string();
        match worktree::plan(&ask.repo, &typed, base(cx).as_deref()) {
            Ok(plan) => {
                let (command, profile) = (ask.command.clone(), ask.profile.clone());
                self.asking = None;
                self.create_worktree(plan, command, profile, window, cx);
                true
            }
            Err(why) => {
                if let Some(ask) = &mut self.asking {
                    ask.error = Some(why.into());
                }
                cx.notify();
                false
            }
        }
    }

    /// Make `plan`'s worktree off the UI thread, then open a terminal in it (running `command`), and
    /// remember it as one of Wormhole's. Git's refusal is shown in its words.
    fn create_worktree(
        &mut self,
        plan: Worktree,
        command: Option<String>,
        profile: ProfileId,
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
            this.update_in(cx, |_, window, cx| match result {
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
                    let setup = project::api::worktree_setup(&profile, &plan.repo, cx);
                    app_mut(cx).record_worktree(plan.clone());
                    // The project's setup first, then what was asked for, as one line: the command
                    // starts once the setup has finished and only if it worked, and a setup that
                    // asks a question doesn't get the command typed ahead as its answer.
                    let command = command.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
                    let line = match (setup, command) {
                        (Some(setup), Some(command)) => Some(format!("{setup} && {command}")),
                        (setup, command) => setup.or(command),
                    };
                    // The window opens the terminal beside the selected one, in the new folder, and
                    // types the line into it; the person's confirmation is what got us here.
                    let input = match line {
                        Some(text) => TerminalInput::Paste { text, submit: true },
                        None => TerminalInput::None,
                    };
                    let target = TerminalTarget::Beside { cwd: Some(plan.folder.clone()) };
                    cx.emit(FeatureEvent::Terminal { target, input });
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
}
