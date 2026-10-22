//! What devctl (`crate::devctl`) asks of a window that nothing else does.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use gpui_kit::*;
use wormhole_core::agents::{AgentId, integrations};

use crate::workspace::{Workspace, folder_color};

/// What runs at the end of every window tick in a devctl build: devctl's command reader, which
/// the app installs at startup (`crate::devctl::install`). devctl drives the window from above,
/// so the window holds a hook rather than naming devctl.
pub(crate) struct TickHook(pub(crate) fn(&mut Workspace, &mut Window, &mut Context<Workspace>));

impl Global for TickHook {}

impl Workspace {
    /// Run the installed [`TickHook`], if there is one (the view tests install none).
    pub(super) fn run_tick_hook(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(hook) = cx.try_global::<TickHook>().map(|hook| hook.0) {
            hook(self, window, cx);
        }
    }

    /// How many of this window's terminals run in a session host, and how many in the app
    /// (`perf-profile` reports it: the two are read differently).
    pub(crate) fn devctl_hosting(&self, cx: &App) -> (usize, usize) {
        let hosted = self.sessions.iter().filter(|session| session.is_hosted(cx)).count();
        (hosted, self.sessions.len() - hosted)
    }

    /// Git work this window still waits for: a status or history refresh in flight, or a session
    /// whose status was read for another folder than the one it is in now (the next tick asks
    /// again). A diff being read has no flag here; `wait-idle` waits for the picture instead.
    pub(crate) fn devctl_git_busy(&self, cx: &App) -> bool {
        self.sessions.iter().any(|session| {
            session.git.loading()
                || session.history.loading()
                || session.git.cwd != session.terminal().read(cx).cwd
        })
    }

    /// The selected terminal's id (devctl `pane-close` without an id): the focused pane.
    pub(crate) fn devctl_focused_pane(&self) -> i64 {
        self.active_session().id
    }

    /// devctl `panes`: the selected terminal's split with its panes' rectangles in the panes' area,
    /// one a line, then its dividers. A terminal on its own is one pane over the area.
    pub(crate) fn devctl_panes(&self) -> String {
        let (w, h) = (f32::from(self.pane_area.size.width), f32::from(self.pane_area.size.height));
        let single;
        let layout = match self.active_split() {
            Some(split) => &split.layout,
            None => {
                single = wormhole_core::view::panes::TabPanes::single(self.active_session().id);
                &single
            }
        };
        let mut out = format!(
            "area {}x{} at {},{} split={} layout={:?}\n",
            w,
            h,
            f32::from(self.pane_area.origin.x),
            f32::from(self.pane_area.origin.y),
            layout.is_split(),
            layout.preset()
        );
        for (id, rect) in layout.shown(w, h) {
            let focused = if layout.focused() == id { " focused" } else { "" };
            let zoomed = if layout.zoomed() == Some(id) { " zoomed" } else { "" };
            out.push_str(&format!(
                "pane {id} {} {} {} {}{focused}{zoomed}\n",
                rect.x, rect.y, rect.w, rect.h
            ));
        }
        for divider in layout.dividers(w, h) {
            out.push_str(&format!(
                "divider {:?}/{} {:?} at {}\n",
                divider.id.path,
                divider.id.index,
                divider.axis,
                divider.position()
            ));
        }
        out
    }

    /// How many terminals the window has, and the id of the `ix`th (devctl `row-drop`).
    pub(crate) fn devctl_session_count(&self) -> usize {
        self.sessions.len()
    }

    /// devctl `folder-color`: colour `folder` (the active terminal's when none is named), or take
    /// its colour away, where its colour lives (the project's `config.json`, else the person's
    /// settings), as the band's menu does. Says which folder it was.
    pub(crate) fn devctl_folder_color(
        &mut self,
        folder: Option<&Path>,
        color: Option<wormhole_core::project::ProjectColorChoice>,
        cx: &mut Context<Self>,
    ) -> Result<PathBuf, &'static str> {
        let folder = match folder {
            Some(folder) => folder.to_path_buf(),
            None => self.active_folder(cx).ok_or("the active terminal has no folder yet")?,
        };
        let paint = self.folder_paint(&folder, cx);
        folder_color::set_folder_choice(&paint, &folder, color, cx);
        Ok(folder)
    }

    /// devctl `project-init`: what the Project tab's Create project folder button does; says which
    /// folder it makes the `.wormhole/` in.
    pub(crate) fn devctl_project_init(&mut self, cx: &mut Context<Self>) -> Result<PathBuf, &'static str> {
        let folder = self.active_folder(cx).ok_or("the active terminal has no folder yet")?;
        self.create_project_folder(cx);
        Ok(folder)
    }

    /// devctl `project-trust`: what the Project tab's trust button does for the active terminal's
    /// project (Touch ID is answered by `WORMHOLE_AUTH`); says which folder it asked for.
    pub(crate) fn devctl_project_trust(&mut self, cx: &mut Context<Self>) -> Result<PathBuf, &'static str> {
        let root = self.active_session().project.clone().ok_or("the active terminal has no project")?;
        self.approve_active_project(cx);
        Ok(root)
    }

    pub(crate) fn devctl_session_id(&self, ix: usize) -> i64 {
        self.sessions[ix].id
    }

    /// devctl `new <dir>`: a new terminal in `dir`.
    pub(crate) fn devctl_new_session_in(&mut self, dir: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open_session(None, Some(dir), window, cx);
    }

    /// devctl `rows`: the sidebar's list as drawn, one a line: groups and terminals with what the
    /// rows show of their splits.
    pub(crate) fn devctl_rows(&self, cx: &App) -> String {
        use wormhole_core::view::sidebar::{Bracket, Entry};
        let listing = self.session_listing(cx);
        let mut out = String::new();
        for entry in &listing.entries {
            match *entry {
                Entry::Band { band, position, of } => {
                    let band = &listing.bands[band];
                    let folded = if band.folded { " folded" } else { "" };
                    let pinned = if band.pinned { " pinned" } else { "" };
                    let colored = self
                        .sidebar
                        .view
                        .groups
                        .get(band.group)
                        .and_then(|group| group.paint())
                        .map_or(String::new(), |color| format!(" colour={}", color.key()));
                    let folder = self
                        .sidebar
                        .view
                        .groups
                        .get(band.group)
                        .and_then(|group| group.folder.as_deref())
                        .map_or(String::new(), |folder| format!(" folder={folder}"));
                    out.push_str(&format!(
                        "group {:?} id={} {position}/{of}{folded}{pinned}{colored}{folder} terminals={}\n",
                        band.label,
                        band.group,
                        band.members.len()
                    ));
                }
                Entry::Row { session, position, of, number, marks, .. } => {
                    let own = &self.sessions[session];
                    let bracket = match marks.bracket {
                        Some(Bracket::Start) => " bracket=start",
                        Some(Bracket::Mid) => " bracket=mid",
                        Some(Bracket::End) => " bracket=end",
                        None => "",
                    };
                    out.push_str(&format!(
                        "row {} id={} {:?} {position}/{of}{}{bracket}{}{}{}{}\n",
                        session + 1,
                        own.id,
                        own.name(cx),
                        number.map(|n| format!(" cmd={n}")).unwrap_or_default(),
                        if marks.chain { " chain" } else { "" },
                        if marks.on_screen { " on_screen" } else { "" },
                        if session == self.active { " selected" } else { "" },
                        own.split
                            .map(|id| format!(" split={id} members={}", marks.members))
                            .unwrap_or_default(),
                    ));
                }
            }
        }
        out
    }

    /// devctl `new-shell <key> [dir]`: a new terminal running the shell `key` names, in `dir` or
    /// where the selected one is.
    pub(crate) fn devctl_new_shell(
        &mut self,
        key: &str,
        dir: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cwd = dir.or_else(|| self.default_cwd(cx));
        self.open_session(Some(key.to_string()), cwd, window, cx);
    }

    /// An agent's hook event for tab `tab`, as the hook socket delivers it; an error, instead of
    /// nothing, for a tab that isn't in this window or an event the agent doesn't send.
    pub(crate) fn devctl_hook(
        &mut self,
        tab: i64,
        agent: AgentId,
        event: &str,
        message: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if self.find_session(tab).is_none() {
            bail!("no terminal {tab} in this window (`state` lists them)");
        }
        if integrations::signal(agent, event).is_none() {
            bail!("{event:?} is not a status event of {agent:?}");
        }
        self.handle_hook(&tab.to_string(), agent, event, message, window, cx);
        Ok(())
    }

    /// `title`: what the title row draws, as text.
    pub(crate) fn devctl_title(&self, window: &Window, cx: &mut Context<Self>) -> String {
        use wormhole_core::view::layout::title_row::PathShown;
        let plan = self.title_plan(window, cx);
        let (path, steps) = match plan.fit.path {
            PathShown::None => ("none".to_string(), 0),
            PathShown::Icon => ("icon".to_string(), plan.steps.len()),
            PathShown::Step(ix) => (
                format!("{} (step {} of {})", plan.steps[ix].text.full(), ix + 1, plan.steps.len()),
                plan.steps.len(),
            ),
        };
        let tint = match &plan.tint {
            Some(tint) => format!(
                "{} at {:.0}% (line {})",
                tint.color.color().to_hex(),
                tint.peak * 100.0,
                crate::theme_bridge::color(tint.line).to_hex()
            ),
            None => "none".to_string(),
        };
        let apps: Vec<String> =
            crate::workspace::open_in::OpenIn::apps(cx).into_iter().map(|app| app.bundle).collect();
        let chip = plan.fit.chip.map_or("none".to_string(), |w| format!("{w:.0}"));
        format!(
            "title row: path={path} steps={steps} words={} chip={chip} folded={} name={:.0} tint={tint} indicator={:?}\n\
             open-in: {} missing={:?} found={}\n\
             border={:?} card={}\n\
             extras: {}",
            plan.fit.words,
            plan.fit.folded,
            plan.fit.name,
            plan.facts.status,
            plan.app.app.bundle,
            plan.app.missing,
            apps.join(","),
            crate::settings::settings(cx).title_border,
            self.path_card.is_shown(),
            crate::workspace::title_chips::describe(&plan),
        )
    }

    /// `path-card`: open or close the path's hover card at once; `path-menu`: the folder's menu.
    pub(crate) fn devctl_path_card(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.path_card.reset(cx);
        if open {
            self.open_path_card(window, cx);
        }
        cx.notify();
    }

    pub(crate) fn devctl_path_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_path_menu(window, cx);
    }
}
