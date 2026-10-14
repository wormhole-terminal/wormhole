//! The status bar along the bottom of the window. It is optional and hidden by default (`status_bar` in config.json, one switch
//! for every window); then nothing of it is drawn and the panes reach the window's bottom.
//!
//! Shown, it has the branch and its changes (opens Changes), the agents of every window by what
//! they do (working · needs you · done · failed; each opens the first such terminal, needs you the
//! one that has waited longest), how full the selected terminal's agent context window is (what its
//! status line last reported, as the title row's gauge) and the terminal's size. What it says, and how much fits the window's
//! width, is core's (`wormhole_core::layout::status_bar`).
use std::time::Instant;

use gpui_kit::component::tooltip::Tooltip;
use wormhole_core::focus::Area;
use wormhole_core::layout::status_bar::{self as rules, Activity, Detail, Entry, Summary};
use wormhole_core::spoken;

use super::app::{app, open_tab};
use super::*;
use crate::style::StatusGlyph;

/// One terminal (a sidebar row) as the bar counts it.
#[derive(Clone)]
pub(super) struct Counted {
    tab: i64,
    activity: Activity,
    since: Instant,
    /// The agent's short name, or the name of the program that reports its status.
    agent: Option<SharedString>,
    name: SharedString,
}

/// What the bar shows, worked out once per frame and compared on the tick, so a change in
/// another window (or a minute more of work) redraws this one.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct StatusBar {
    /// The selected terminal's branch and how many files changed; `None` outside a repository.
    branch: Option<(SharedString, usize)>,
    /// Each agent item's words and the terminal a click goes to.
    working: Option<(String, i64)>,
    needs_you: Option<(String, String, i64)>,
    done: Option<(String, i64)>,
    failed: Option<(String, i64)>,
    /// The selected terminal's agent context, while the window is wide enough.
    context: Option<ContextItem>,
    /// The terminal's shell and grid, while the window is wide enough.
    size: Option<String>,
    /// What VoiceOver says for each item, whole whatever the width.
    spoken: Spoken,
}

/// The context meter as the bar draws it.
#[derive(Clone, Debug, PartialEq)]
struct ContextItem {
    /// The meter's track: narrower with the other items' words gone.
    width: f32,
    words: bool,
    percent: u32,
    warn: bool,
}

/// The bar's items in words.
#[derive(Clone, Debug, Default, PartialEq)]
struct Spoken {
    branch: Option<String>,
    working: Option<String>,
    needs_you: Option<String>,
    done: Option<String>,
    failed: Option<String>,
    context: Option<String>,
    size: Option<String>,
}

/// An item of the bar the keys can be on (a toolbar; the size is read, not pressed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StatusItem {
    Branch,
    Working(i64),
    NeedsYou,
    Done(i64),
    Failed(i64),
}

impl Workspace {
    /// This window's terminals.
    pub(super) fn status_terminals(&self, cx: &App) -> Vec<Counted> {
        self.sessions
            .iter()
            .map(|session| {
                let terminal = session.terminal().read(cx);
                Counted {
                    tab: session.id,
                    activity: match terminal.status() {
                        Status::Busy => Activity::Working,
                        Status::Attention => Activity::NeedsYou,
                        Status::Done => Activity::Done,
                        Status::Failed => Activity::Failed,
                        Status::Idle | Status::Exited => Activity::Quiet,
                    },
                    since: terminal.status_since(),
                    agent: terminal
                        .agent
                        .map(|id| SharedString::from(wormhole_core::agents::agent(id).short_name()))
                        .or_else(|| terminal.program_name().map(SharedString::from)),
                    name: session.name(cx),
                }
            })
            .collect()
    }

    /// Every other window's terminals, for the bar. Only ever from the tick, never while
    /// drawing: GPUI makes an entity read during a window's draw that window's, so another
    /// window's workspace read in this frame would run its tick (and its deferred work) here, and
    /// lose it when this window closes.
    fn read_other_windows(&mut self, cx: &App) {
        self.status_others = app(cx)
            .windows
            .iter()
            .filter(|window| window.id != self.window_id)
            .filter_map(|window| {
                window.workspace.read_with(cx, |ws, cx| (window.id, ws.status_terminals(cx))).ok()
            })
            .collect();
    }

    /// What the bar shows now, or `None` while it is switched off.
    pub(super) fn status_bar(&self, cx: &App) -> Option<StatusBar> {
        if !settings(cx).status_bar {
            return None;
        }
        // Every window's terminals, in window order: this one's now, the others' as the tick
        // last read them.
        let mut counted = Vec::new();
        let mut seen_self = false;
        for window in &app(cx).windows {
            if window.id == self.window_id {
                counted.extend(self.status_terminals(cx));
                seen_self = true;
            } else if let Some((_, theirs)) = self.status_others.iter().find(|(id, _)| *id == window.id) {
                counted.extend(theirs.iter().cloned());
            }
        }
        if !seen_self {
            counted.extend(self.status_terminals(cx));
        }
        let entries: Vec<Entry> = counted
            .iter()
            .map(|c| Entry { activity: c.activity, since: c.since, agent: c.agent.as_deref(), name: &c.name })
            .collect();
        let summary = Summary::of(&entries);
        let detail = Detail::at(self.window_width);
        let tab = |ixs: &[usize]| ixs.first().map(|&ix| counted[ix].tab);

        let session = self.active_session();
        let terminal = session.terminal().read(cx);
        // Only while the status is of the folder the terminal is in now.
        let branch = session.git.status.as_ref().filter(|_| session.git.cwd == terminal.cwd).map(|status| {
            let name = status.branch.clone().unwrap_or_else(|| "HEAD".into());
            (SharedString::from(name), status.files.len())
        });
        let (cols, rows) = terminal.grid_size();
        let size = detail.size().then(|| rules::size(&terminal.shell(), cols, rows));
        let gauge = self.agent_gauge(cx);
        let context =
            detail.context_meter().zip(gauge.and_then(|gauge| gauge.percent())).map(|(width, percent)| {
                ContextItem {
                    width,
                    words: detail.words(),
                    percent,
                    warn: gauge.is_some_and(|gauge| gauge.warns()),
                }
            });
        let now = Instant::now();
        let spoken = Spoken {
            branch: branch.as_ref().map(|(name, changes)| spoken::status_branch(name, *changes)),
            working: spoken::status_working(&summary, &entries, now),
            needs_you: spoken::status_needs_you(&summary),
            done: spoken::status_done(&summary),
            failed: spoken::status_failed(&summary),
            context: context.as_ref().map(|item| spoken::status_context(item.percent)),
            size: size.as_ref().map(|_| spoken::status_size(&terminal.shell(), cols, rows)),
        };
        Some(StatusBar {
            spoken,
            branch,
            working: summary.working(&entries, detail, now).zip(tab(&summary.working)),
            needs_you: summary
                .needs_you()
                .zip(summary.needs_you_tooltip(&entries))
                .zip(tab(&summary.needs_you))
                .map(|((words, tooltip), tab)| (words, tooltip, tab)),
            done: summary.done(detail).zip(tab(&summary.done)),
            failed: summary.failed(detail).zip(tab(&summary.failed)),
            context,
            size,
        })
    }

    /// On the tick: read the other windows, and redraw when what the bar would show has changed
    /// since it was drawn (a terminal in another window, or the working time).
    pub(super) fn refresh_status_bar(&mut self, cx: &mut Context<Self>) {
        if settings(cx).status_bar {
            self.read_other_windows(cx);
        } else {
            self.status_others.clear();
        }
        if self.status_bar(cx) != self.status_bar_drawn {
            cx.notify();
        }
    }

    /// A click on an agent item: that terminal, in whichever window it is.
    fn go_to_tab(&mut self, tab: i64, window: &mut Window, cx: &mut Context<Self>) {
        if self.find_session(tab).is_some() {
            self.reveal_terminal(tab, window, cx);
        } else {
            // Another window's: it is reached through the app, after this update.
            cx.defer(move |cx| open_tab(tab, cx));
        }
    }

    /// The items the keys move along, left to right.
    pub(super) fn status_items(&self) -> Vec<StatusItem> {
        let Some(bar) = &self.status_bar_drawn else { return Vec::new() };
        [
            bar.branch.as_ref().map(|_| StatusItem::Branch),
            bar.working.as_ref().map(|(_, tab)| StatusItem::Working(*tab)),
            bar.needs_you.as_ref().map(|_| StatusItem::NeedsYou),
            bar.done.as_ref().map(|(_, tab)| StatusItem::Done(*tab)),
            bar.failed.as_ref().map(|(_, tab)| StatusItem::Failed(*tab)),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// What a click on `item` does, from the keyboard.
    pub(super) fn press_status_item(
        &mut self,
        item: StatusItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match item {
            StatusItem::Branch => self.show_tab(RightTab::Changes, cx),
            StatusItem::Working(tab) | StatusItem::Done(tab) | StatusItem::Failed(tab) => {
                self.go_to_tab(tab, window, cx)
            }
            StatusItem::NeedsYou => cx.defer(super::ambient::jump_to_waiting),
        }
    }

    pub(super) fn render_status_bar(
        &self,
        bar: &StatusBar,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        // The item the keys are on, while the bar has the keyboard.
        let cursor = self.status_items().get(self.areas.status).copied();
        let ring = self.ring(Area::StatusBar, window);
        let keyed = |item: StatusItem| cursor == Some(item);
        // A toolbar's button: its words whole, and VoiceOver on the one the keys are on.
        let spoken = |d: Stateful<Div>, label: &Option<String>, item: StatusItem| {
            d.role(Role::Button)
                .when_some(label.clone(), |d, label| d.aria_label(label))
                .when(keyed(item), |d| d.aria_active_descendant())
        };
        let branch = bar.branch.clone().map(|(name, changes)| {
            let item = StatusItem::Branch;
            spoken(
                style::status_item("status-branch", ui, false, ring && keyed(item)),
                &bar.spoken.branch,
                item,
            )
            .child(div().text_color(ui.branch).child(style::icon(IconName::GitBranch)))
            .child(div().text_color(ui.branch).child(name))
            .when(changes > 0, |d| d.child(div().ml_1().child(format!("±{changes}"))))
            .tooltip(|window, cx| Tooltip::new("Branch and changes").build(window, cx))
            .on_click(cx.listener(|this, _, _, cx| this.show_tab(RightTab::Changes, cx)))
        });
        let agents =
            bar.working.is_some() || bar.needs_you.is_some() || bar.done.is_some() || bar.failed.is_some();
        let working = bar.working.clone().map(|(words, tab)| {
            let item = StatusItem::Working(tab);
            spoken(
                style::status_item("status-working", ui, false, ring && keyed(item)),
                &bar.spoken.working,
                item,
            )
            .child(style::status_glyph(("status-working", tab as u64), ui, StatusGlyph::Working, cx))
            .child(words)
            .on_click(cx.listener(move |this, _, window, cx| this.go_to_tab(tab, window, cx)))
        });
        // ⌥⌘J in the bar: the longest waiting first, then the next one on every click.
        let jump = settings(cx).jump_waiting_key.parse::<wormhole_core::keyspec::KeySpec>().ok();
        let needs_you = bar.needs_you.clone().map(|(words, tooltip, _)| {
            let tooltip = SharedString::from(match &jump {
                Some(key) => format!("{tooltip} ({})", key.glyphs()),
                None => tooltip,
            });
            let item = StatusItem::NeedsYou;
            spoken(
                style::status_item("status-needs-you", ui, true, ring && keyed(item)),
                &bar.spoken.needs_you,
                item,
            )
            .child(style::icon(IconName::CircleAlert))
            .child(words)
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .on_click(|_, _, cx| cx.defer(super::ambient::jump_to_waiting))
        });
        let done = bar.done.clone().map(|(words, tab)| {
            let item = StatusItem::Done(tab);
            spoken(style::status_item("status-done", ui, false, ring && keyed(item)), &bar.spoken.done, item)
                .child(div().text_color(ui.agent_done).child(style::icon(IconName::CircleCheck)))
                .child(words)
                .on_click(cx.listener(move |this, _, window, cx| this.go_to_tab(tab, window, cx)))
        });
        let failed = bar.failed.clone().map(|(words, tab)| {
            let item = StatusItem::Failed(tab);
            spoken(
                style::status_item("status-failed", ui, false, ring && keyed(item)),
                &bar.spoken.failed,
                item,
            )
            .child(div().text_color(ui.danger_text).child(style::icon(IconName::CircleX)))
            .child(words)
            .on_click(cx.listener(move |this, _, window, cx| this.go_to_tab(tab, window, cx)))
        });
        let bar_element = style::status_bar(ui)
            .id("status-bar")
            // A toolbar, there only while the bar is shown.
            .role(Role::Toolbar)
            .aria_label("Status bar")
            .children(branch.map(|branch| {
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(branch)
                    .when(agents, |d| d.child(style::status_separator(ui)))
            }))
            .children(working)
            .children(needs_you)
            .children(done)
            .children(failed)
            .child(div().flex_1())
            .children(bar.context.clone().map(|item| {
                let tooltip = SharedString::from(rules::context_tooltip(item.percent));
                let share = item.percent as f32 / 100.0;
                style::status_context(
                    ui,
                    item.width,
                    item.words,
                    share,
                    item.warn,
                    format!("{}%", item.percent),
                )
                .id("status-context")
                .role(Role::Meter)
                .when_some(bar.spoken.context.clone(), |d, label| d.aria_label(label))
                .aria_numeric_value(item.percent as f64)
                .aria_min_numeric_value(0.0)
                .aria_max_numeric_value(100.0)
                .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            }))
            .children(bar.size.clone().map(|size| {
                div().flex().items_center().gap_1().child(style::status_separator(ui)).child(
                    style::status_text(ui)
                        .id("status-size")
                        .role(Role::Label)
                        .when_some(bar.spoken.size.clone(), |d, label| d.aria_label(label))
                        .child(size),
                )
            }));
        self.area_container(Area::StatusBar, bar_element, cx).into_any_element()
    }
}
