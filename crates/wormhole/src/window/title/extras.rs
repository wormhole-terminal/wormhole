//! The title row's extras, the data side: what each one knows about the
//! terminal on screen, and when it is looked up. `window::title::chips` measures and draws them.
//!
//! - **Branch status** is git's own answer for the session, which the Changes tab already refreshes
//!   every few seconds (`RepoStatus::dirty` and `upstream`): no process of its own.
//! - **The agent's model and context window** are what its status line last reported for this
//!   terminal (`Wormhole statusline`, core `usage::Gauge`). Only Claude Code reports them; no other
//!   agent has one, and the row shows nothing for it.
//! - **Open ports** come from libproc for the shell's process tree (core `terminal::ports`), asked
//!   on a background thread every few seconds and only for the terminal on screen.
//! - **The pull request** is `gh`'s (core `git::pr`), asked at most every few minutes per branch
//!   (every minute while its checks run), and only in a project the person trusts: `gh` reads the
//!   repository's own git configuration. Every terminal's branch is asked for, not only the one on
//!   screen: the sidebar's folder bands show theirs too.
//!
//! Each has a Settings switch; one that is off is not looked up at all.

use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::agents::usage::Gauge;
use wormhole_core::git::RepoStatus;
use wormhole_core::git::pr::{self, Branch, PullRequest, Settled};
use wormhole_core::view::title_extras::BranchStatus;
use wormhole_term::Notification;
use wormhole_term::listening_ports;

use crate::app_state::{app, app_mut};
use crate::settings::settings_in;
use crate::window::Workspace;

/// How often the ports under the terminal on screen are looked at.
const PORTS_EVERY: Duration = Duration::from_secs(5);

/// What the extras remember about one session.
#[derive(Default)]
pub(in crate::window) struct SessionExtras {
    /// The model and context window its agent last reported.
    gauge: Option<Gauge>,
    /// The ports its programs listen on as of the last look, lowest first.
    ports: Vec<u16>,
    ports_polled: Option<Instant>,
    /// A look is under way on a background thread.
    ports_polling: bool,
}

#[cfg(test)]
impl SessionExtras {
    /// Ports as a look at the shell's process tree would have found them.
    pub(in crate::window) fn found_ports(&mut self, ports: Vec<u16>) {
        self.ports = ports;
    }
}

/// What the extras say about the terminal on screen, for one frame.
pub(in crate::window) struct ExtraFacts {
    pub(in crate::window) branch: Option<BranchStatus>,
    pub(in crate::window) gauge: Option<Gauge>,
    pub(in crate::window) ports: Vec<u16>,
    pub(in crate::window) pull_request: Option<PullRequest>,
}

impl Workspace {
    /// What the extras show for the active terminal, with every switched-off one left out.
    pub(in crate::window) fn extra_facts(&self, cx: &App) -> ExtraFacts {
        let settings = settings_in(&self.profile, cx);
        let session = self.active_session();
        let terminal = session.terminal().read(cx);
        // Git's answer only while it is about the folder the terminal is in now.
        let status = session.git.status.as_ref().filter(|_| session.git.cwd == terminal.cwd);
        ExtraFacts {
            branch: status.filter(|_| settings.title_branch_status).and_then(RepoStatus::branch_status),
            gauge: self.agent_gauge(cx).filter(|_| settings.title_agent_gauge).cloned(),
            ports: if settings.title_ports { session.extras.ports.clone() } else { Vec::new() },
            pull_request: self.pull_request_branch(cx).and_then(|branch| app(cx).pr.get(&branch).cloned()),
        }
    }

    /// The model and context window the active terminal's agent last reported. Only the agent
    /// that reported it: another one in the same terminal has not (yet).
    pub(in crate::window) fn agent_gauge<'a>(&'a self, cx: &'a App) -> Option<&'a Gauge> {
        let session = self.active_session();
        let agent = session.terminal().read(cx).agent;
        session.extras.gauge.as_ref().filter(|gauge| agent == Some(gauge.agent))
    }

    /// A terminal's agent reported its model and context window (a status line ran in it).
    pub(crate) fn handle_gauge(&mut self, tab: &str, gauge: Gauge, cx: &mut Context<Self>) {
        let Some(ix) = tab.parse().ok().and_then(|id| self.find_session(id)) else { return };
        let extras = &mut self.sessions[ix].extras;
        // A status line runs many times a minute: redraw only for a change.
        if extras.gauge.as_ref() != Some(&gauge) {
            extras.gauge = Some(gauge);
            if ix == self.active {
                cx.notify();
            }
        }
    }

    /// On the tick: forget a report whose agent has gone, and look up what is due for the terminal
    /// on screen.
    pub(in crate::window) fn tick_extras(&mut self, cx: &mut Context<Self>) {
        self.forget_gone_gauges(cx);
        self.poll_ports(cx);
        self.poll_pull_request(cx);
    }

    /// A report belongs to the agent that made it. Once the shell has its terminal back (no agent,
    /// nothing else in front either) the next agent in it starts without the last one's numbers;
    /// while a program is merely not recognised yet (an agent just started) it is kept.
    fn forget_gone_gauges(&mut self, cx: &mut Context<Self>) {
        let active = self.active;
        for (ix, session) in self.sessions.iter_mut().enumerate() {
            let terminal = session.pane.terminal.read(cx);
            if session.extras.gauge.is_some() && terminal.agent.is_none() && !terminal.program_in_foreground()
            {
                session.extras.gauge = None;
                if ix == active {
                    cx.notify();
                }
            }
        }
    }

    /// Ask for the ports under the terminal on screen, off the main thread, when the last look
    /// is a few seconds old.
    fn poll_ports(&mut self, cx: &mut Context<Self>) {
        let wanted = settings_in(&self.profile, cx).title_ports;
        let session = &mut self.sessions[self.active];
        let pid = session.pane.terminal.read(cx).shell_pid();
        let Some(pid) = pid.filter(|_| wanted) else {
            // Nothing to look at (or nothing to show it in): no chips either.
            if !session.extras.ports.is_empty() {
                session.extras.ports.clear();
                cx.notify();
            }
            return;
        };
        let extras = &mut session.extras;
        let due = extras.ports_polled.is_none_or(|at| at.elapsed() >= PORTS_EVERY);
        if extras.ports_polling || !due {
            return;
        }
        extras.ports_polling = true;
        extras.ports_polled = Some(Instant::now());
        let id = session.id;
        let task = cx.background_executor().spawn(async move { listening_ports(pid) });
        cx.spawn(async move |this, cx| {
            let found = task.await;
            let _ = this.update(cx, |this, cx| this.finish_ports(id, found, cx));
        })
        .detach();
    }

    fn finish_ports(&mut self, id: i64, found: Vec<u16>, cx: &mut Context<Self>) {
        let Some(ix) = self.find_session(id) else { return };
        let extras = &mut self.sessions[ix].extras;
        extras.ports_polling = false;
        if extras.ports != found {
            extras.ports = found;
            if ix == self.active {
                cx.notify();
            }
        }
    }

    /// The branch whose pull request the row may show and ask for: the switch is on, the terminal's
    /// folder is in a repository on a branch, and its project is trusted. `gh` reads the
    /// repository's git configuration and remotes, which a repository the person has not approved
    /// may have filled with anything.
    fn pull_request_branch(&self, cx: &App) -> Option<Branch> {
        self.session_pr_branch(self.active, cx)
    }

    /// [`Self::pull_request_branch`] for any of the window's terminals.
    pub(in crate::window) fn session_pr_branch(&self, ix: usize, cx: &App) -> Option<Branch> {
        if !settings_in(&self.profile, cx).title_pull_request
            || !crate::window::features::project::project_folders_on(cx)
        {
            return None;
        }
        let session = self.sessions.get(ix)?;
        let status =
            session.git.status.as_ref().filter(|_| session.git.cwd == session.terminal().read(cx).cwd)?;
        let hub = &app(cx).hub;
        let project = hub.projects.get(session.project.as_ref()?)?;
        let name = status.branch.clone()?;
        hub.trusted(project).then(|| Branch { root: status.root.clone(), name })
    }

    /// Ask `gh` for each terminal's branch's pull request when the cache says it is time (a branch
    /// not asked yet, or not for a few minutes), off the main thread.
    fn poll_pull_request(&mut self, cx: &mut Context<Self>) {
        // Terminals on the same branch ask once: the cache lets one lookup out per branch.
        let branches: Vec<Branch> =
            (0..self.sessions.len()).filter_map(|ix| self.session_pr_branch(ix, cx)).collect();
        for branch in branches {
            Self::ask_for_pull_request(branch, cx);
        }
    }

    fn ask_for_pull_request(branch: Branch, cx: &mut Context<Self>) {
        if !app_mut(cx).pr.begin(&branch, Instant::now()) {
            return;
        }
        let task = {
            let root = branch.root.clone();
            cx.background_executor().spawn(async move { pr::pull_request(&root) })
        };
        // The answer belongs to the app, not to this window: it is kept even if the window closed.
        cx.spawn(async move |this, cx| {
            let found = task.await;
            let settled = cx.update(|cx| {
                let settled = app_mut(cx).pr.finish(&branch, found, Instant::now());
                // Every window may show it.
                cx.refresh_windows();
                settled
            });
            if let Some(settled) = settled {
                this.update(cx, |this, cx| this.tell_checks_settled(&branch, &settled, cx)).ok();
            }
        })
        .detach();
    }

    /// A pull request's checks finished: the first terminal on its branch tells it the way a
    /// program's own notification is told (`Wormhole notify`'s path), so the notification settings
    /// and the sidebar's needs-you rules apply as they do to those.
    fn tell_checks_settled(&mut self, branch: &Branch, settled: &Settled, cx: &mut Context<Self>) {
        let Some(ix) =
            (0..self.sessions.len()).find(|&ix| self.session_pr_branch(ix, cx).as_ref() == Some(branch))
        else {
            return;
        };
        let (title, body) = settled.notification(&branch.name);
        self.sessions[ix]
            .terminal()
            .update(cx, |terminal, _| terminal.notice_from_control(Notification { title, body }));
    }
}
