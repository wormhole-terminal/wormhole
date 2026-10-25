//! What devctl (`crate::devctl`) asks of a window that nothing else does.

use anyhow::{Result, bail};
use gpui_kit::*;
use wormhole_core::agents::{AgentId, integrations};

use crate::app_state::app;
use crate::window::Workspace;

/// What runs at the end of every window tick in a devctl build: devctl's command reader, which
/// the app installs at startup (`crate::devctl::install`). devctl drives the window from above,
/// so the window holds a hook rather than naming devctl.
pub(crate) struct TickHook(pub(crate) fn(&mut Workspace, &mut Window, &mut Context<Workspace>));

impl Global for TickHook {}

impl Workspace {
    /// `scroll-panel`: the shown tab's list scrolled by `y` points.
    pub(crate) fn devctl_panel_scroll(&mut self, y: f32, cx: &mut Context<Self>) {
        let Some(list) = self.devctl_panel_list(cx) else { return };
        list.scroll_by(px(y));
        cx.notify();
    }

    /// The shown tab's list for the terminal on screen, if the tab has one.
    pub(crate) fn devctl_panel_list(&self, cx: &App) -> Option<ListState> {
        let (at, tab) = self.tab_entry(self.right_tab)?;
        let (walk, entity) = (tab.walk.as_ref()?, self.feature_entity(at)?);
        (walk.list)(entity, &self.facts(cx), cx)
    }

    /// Run the installed [`TickHook`], if there is one (the view tests install none).
    pub(super) fn run_tick_hook(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(hook) = cx.try_global::<TickHook>().map(|hook| hook.0) {
            hook(self, window, cx);
        }
    }

    /// Whether the accessibility tree has been built in this window (`perf-profile` reports it: the chrome
    /// is drawn in every frame then, not replayed).
    pub(crate) fn devctl_a11y_tree(&self) -> bool {
        self.regions.tree_seen()
    }

    /// How many of this window's terminals run in a session host, and how many in the app
    /// (`perf-profile` reports it: the two are read differently).
    pub(crate) fn devctl_hosting(&self, cx: &App) -> (usize, usize) {
        let hosted = self.sessions.iter().filter(|session| session.is_hosted(cx)).count();
        (hosted, self.sessions.len() - hosted)
    }

    /// Repository reads this window still waits for: a status refresh in flight, or a session whose
    /// status was read for another folder than the one it is in now (the next tick asks again). A
    /// feature's own reads (History's log) are its, and a diff being read has no flag here;
    /// `wait-idle` waits for the picture instead.
    pub(crate) fn devctl_repo_busy(&self, cx: &App) -> bool {
        self.sessions
            .iter()
            .any(|session| session.git.loading() || session.git.cwd != session.terminal().read(cx).cwd)
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
                        .sidebar_view(cx)
                        .groups
                        .get(band.group)
                        .and_then(|group| group.paint())
                        .map_or(String::new(), |color| format!(" colour={}", color.key()));
                    let folder = self
                        .sidebar_view(cx)
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
                        own.name(),
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
        let apps: Vec<String> = crate::open_in::OpenIn::apps(cx).into_iter().map(|app| app.bundle).collect();
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
            crate::window::title::chips::describe(&plan),
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

impl Workspace {
    /// Quit the app (every window is saved on the way out).
    #[cfg(feature = "devctl")]
    pub(crate) fn quit(&mut self, cx: &mut Context<Self>) {
        cx.quit();
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn debug_state(&self, cx: &App) -> String {
        let mut out = format!(
            "window {} of {} (id {}) close_dot={} closing_terminal_prompt={} profile={}\nlayout sidebar={} notes={} width={} sidebar_shown={} notes_wanted={} notes_visible={} resizing={} status_bar={} groups={} folded={:?} listed={:?} numbered={:?}\n",
            self.window_number(cx),
            app(cx).windows.len(),
            self.window_id,
            self.close_dot,
            self.closing_session_prompt,
            self.profile,
            self.panes.sidebar_width,
            self.panes.panel_width,
            self.window_width,
            self.panes.sidebar_open,
            self.panes.panel_wanted,
            self.notes_visible(),
            self.resizing.is_some(),
            self.regions.drawn.status_bar.is_some(),
            self.sidebar_view(cx).groups.iter().count(),
            self.sidebar_view(cx).folded,
            self.session_order(cx).into_iter().map(|ix| ix + 1).collect::<Vec<_>>(),
            self.session_listing(cx).numbered().into_iter().map(|ix| ix + 1).collect::<Vec<_>>(),
        );
        out.push_str(&format!(
            "regions built {:?} (sidebar header, panel, title row, status bar); sidebar list built {}\n",
            self.regions.builds(cx),
            self.sidebar.read(cx).builds()
        ));
        out.push_str(&format!(
            "a11y tree built={} (every frame builds it once it is; the chrome is still replayed)\n",
            self.devctl_a11y_tree()
        ));
        let glass = self.glass.placed();
        out.push_str(&format!(
            "glass available={} on={} placed={} {:?}\n",
            self.glass.available(),
            self.glass_on(cx),
            glass.len(),
            glass.iter().map(|g| (g.x as i32, g.y as i32, g.w as i32, g.h as i32)).collect::<Vec<_>>()
        ));
        for (ix, session) in self.sessions.iter().enumerate() {
            let terminal = session.terminal().read(cx);
            let marker = if ix == self.active {
                "*"
            } else if self.on_screen(ix) {
                "+"
            } else {
                " "
            };
            out.push_str(&format!(
                "{marker}{} id={} {:?} agent={:?}{}{} name={:?} cwd={:?}{}{} command={:?} shell={:?} runs={:?} {}\n",
                ix + 1,
                session.id,
                terminal.status(),
                terminal.agent,
                // A program that reports its own status and is no agent, and a failed command's
                // exit code (the row's cue).
                terminal.program_name().map(|name| format!(" program={name:?}")).unwrap_or_default(),
                terminal.failure().map(|failure| format!(" failed=exit {}", failure.code)).unwrap_or_default(),
                session.name(),
                terminal.cwd,
                if session.pinned { " pinned" } else { "" },
                session.split.map(|id| format!(" split={id}")).unwrap_or_default(),
                terminal.foreground().map(|f| f.command.clone()),
                session.pane.shell,
                terminal.shell_name(),
                terminal.host_label(),
            ));
            // The features' parts of the saved record say what the terminal holds.
            let saved = self.session_record(ix, cx);
            for task in &saved.tasks {
                out.push_str(&format!("    [{}] {}\n", if task.done { "x" } else { " " }, task.text));
            }
            for note in &saved.notes {
                out.push_str(&format!(
                    "    note{}: {:?}\n",
                    if note.collapsed { " (collapsed)" } else { "" },
                    note.body
                ));
            }
        }
        for split in &self.splits {
            out.push_str(&format!(
                "split {} members={:?} panes={:?} focused={} zoomed={:?} layout={:?}\n",
                split.id,
                self.members(split.id).into_iter().map(|ix| ix + 1).collect::<Vec<_>>(),
                split.layout.panes(),
                split.layout.focused(),
                split.layout.zoomed(),
                split.layout.preset(),
            ));
        }
        out
    }
}
