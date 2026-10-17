//! The status bar along the bottom of the window. It is optional and hidden by default (`status_bar` in config.json, one switch
//! for every window); then nothing of it is drawn and the panes reach the window's bottom.
//!
//! Shown, it has the branch and its changes (opens Changes), the terminal of any window that has
//! waited longest for you as a button (its tile, its name, how long: one click goes there), the
//! other agents by what they do as numbers ("2 working · 1 done"; each opens the first such
//! terminal), how full the selected terminal's agent context window is (what its status line last
//! reported, as the title row's gauge) and the terminal's size. What it says, which terminal is
//! the longest waiting and how much fits the window's width, is core's
//! (`wormhole_core::layout::status_bar`).
use std::time::Instant;

use gpui_kit::component::tooltip::Tooltip;
use wormhole_core::focus::Area;
use wormhole_core::layout::status_bar::{self as rules, Activity, Detail, Entry, Summary};
use wormhole_core::spoken;

use super::app::{app, open_tab};
use super::*;
use crate::style::{StatusGlyph, StatusTile, TileFace};

/// One terminal (a sidebar row) as the bar counts it.
#[derive(Clone)]
pub(super) struct Counted {
    tab: i64,
    activity: Activity,
    since: Instant,
    /// The agent's short name, or the name of the program that reports its status.
    agent: Option<SharedString>,
    name: SharedString,
    /// What its tile shows inside: the agent's logo, or the terminal symbol.
    face: TileFace,
}

/// What the bar shows, worked out once per frame and compared on the tick, so a change in
/// another window (or a minute more of waiting) redraws this one.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct StatusBar {
    /// The selected terminal's branch and how many files changed; `None` outside a repository.
    branch: Option<(SharedString, usize)>,
    /// The terminal that has waited longest, as the bar's button.
    attention: Option<Attention>,
    /// The rest, as numbers, in the order core gives them.
    rest: Vec<Rest>,
    /// Whether the window is wide enough for the words ("2 working") rather than glyphs and numbers.
    words: bool,
    /// The selected terminal's agent context, while the window is wide enough.
    context: Option<ContextItem>,
    /// The terminal's shell and grid, while the window is wide enough.
    size: Option<String>,
    /// What VoiceOver says for each item, whole whatever the width.
    spoken: Spoken,
    /// The profile of a window that is not in the profile Wormhole is in, while the window is
    /// wide enough: the bar's lowest-priority item, at its left.
    profile: Option<ProfileTag>,
}

/// The name of the profile an odd window is in, with its colour.
#[derive(Clone, Debug, PartialEq)]
struct ProfileTag {
    name: SharedString,
    color: Hsla,
    spoken: String,
    tooltip: String,
}

/// The longest-waiting terminal's button.
#[derive(Clone, Debug, PartialEq)]
struct Attention {
    tab: i64,
    name: SharedString,
    face: TileFace,
    /// "needs you · 2m", or "2m" in a narrow window.
    words: String,
    /// Who else needs you, and what a click does.
    tooltip: String,
    spoken: String,
}

/// One count after the button, and the terminal a click on it goes to.
#[derive(Clone, Debug, PartialEq)]
struct Rest {
    activity: Activity,
    count: usize,
    words: String,
    tab: i64,
    spoken: String,
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
    context: Option<String>,
    size: Option<String>,
}

/// An item of the bar the keys can be on (a toolbar; the size is read, not pressed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StatusItem {
    Branch,
    /// The longest-waiting terminal's button.
    Attention(i64),
    Working(i64),
    /// Another terminal that needs you.
    NeedsYou(i64),
    Done(i64),
    Failed(i64),
}

/// The most the button gives its name before cutting it.
const ATTENTION_NAME_MAX: f32 = 220.0;

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
                    face: match terminal.agent.map(wormhole_core::agents::agent) {
                        Some(agent) => match agent.icon() {
                            Some(path) => TileFace::Logo(path),
                            None => TileFace::Monogram(agent.monogram.into()),
                        },
                        None => TileFace::Terminal,
                    },
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
            .filter(|window| window.id != self.window_id && !window.hidden)
            .filter_map(|window| {
                window.workspace.read_with(cx, |ws, cx| (window.id, ws.status_terminals(cx))).ok()
            })
            .collect();
    }

    /// What the bar shows now, or `None` while it is switched off.
    pub(super) fn status_bar(&self, cx: &App) -> Option<StatusBar> {
        if !settings_in(&self.profile, cx).status_bar {
            return None;
        }
        // Every window's terminals, in window order: this one's now, the others' as the tick
        // last read them.
        let mut counted = Vec::new();
        let mut seen_self = false;
        for window in app(cx).windows.iter().filter(|w| !w.hidden) {
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

        let session = self.active_session();
        let terminal = session.terminal().read(cx);
        // Only while the status is of the folder the terminal is in now.
        let branch = session.git.status.as_ref().filter(|_| session.git.cwd == terminal.cwd).map(|status| {
            let name = status.branch.clone().unwrap_or_else(|| "HEAD".into());
            (SharedString::from(name), status.files.len())
        });
        let (cols, rows) = terminal.grid_size();
        // What runs in front (the agent, or the command), as the title row names it, not the shell.
        let program = session.pane.program(cx);
        let size = detail.size().then(|| rules::size(&program, cols, rows));
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
        let attention = summary.attention().map(|ix| {
            let (entry, counted) = (&entries[ix], &counted[ix]);
            Attention {
                tab: counted.tab,
                name: counted.name.clone(),
                face: counted.face.clone(),
                words: summary.attention_words(&entries, detail, now).unwrap_or_default(),
                tooltip: summary.needs_you_tooltip(&entries).unwrap_or_default(),
                spoken: spoken::status_attention(&summary, &entries, now)
                    .unwrap_or_else(|| entry.name.to_string()),
            }
        });
        let rest = summary
            .rest()
            .into_iter()
            .map(|c| Rest {
                activity: c.activity,
                count: c.count,
                words: c.words(),
                tab: counted[c.terminal].tab,
                spoken: spoken::status_counted(&c),
            })
            .collect();
        let spoken = Spoken {
            branch: branch.as_ref().map(|(name, changes)| spoken::status_branch(name, *changes)),
            context: context.as_ref().map(|item| spoken::status_context(item.percent)),
            size: size.as_ref().map(|_| spoken::status_size(&program, cols, rows)),
        };
        let profile = self.odd_profile(cx).filter(|_| detail.profile()).map(|(profile, color)| ProfileTag {
            spoken: spoken::status_profile(&profile.name),
            tooltip: rules::profile_tooltip(&profile.name),
            name: profile.name.clone().into(),
            color,
        });
        Some(StatusBar { spoken, branch, attention, rest, words: detail.words(), context, size, profile })
    }

    /// On the tick: read the other windows, and redraw when what the bar would show has changed
    /// since it was drawn (a terminal in another window, or the working time).
    pub(super) fn refresh_status_bar(&mut self, cx: &mut Context<Self>) {
        if settings_in(&self.profile, cx).status_bar {
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
        bar.branch
            .as_ref()
            .map(|_| StatusItem::Branch)
            .into_iter()
            .chain(bar.attention.as_ref().map(|a| StatusItem::Attention(a.tab)))
            .chain(bar.rest.iter().map(|r| match r.activity {
                Activity::NeedsYou => StatusItem::NeedsYou(r.tab),
                Activity::Done => StatusItem::Done(r.tab),
                Activity::Failed => StatusItem::Failed(r.tab),
                Activity::Working | Activity::Quiet => StatusItem::Working(r.tab),
            }))
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
            StatusItem::Attention(tab)
            | StatusItem::Working(tab)
            | StatusItem::NeedsYou(tab)
            | StatusItem::Done(tab)
            | StatusItem::Failed(tab) => self.go_to_tab(tab, window, cx),
        }
    }

    pub(super) fn render_status_bar(
        &self,
        bar: &StatusBar,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
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
        let jump =
            settings_in(&self.profile, cx).jump_waiting_key.parse::<wormhole_core::keyspec::KeySpec>().ok();
        // The longest waiting first: its tile, its name, how long it has waited. One click goes there.
        let attention = bar.attention.clone().map(|a| {
            let tab = a.tab;
            let item = StatusItem::Attention(tab);
            let tooltip = SharedString::from(match &jump {
                Some(key) => format!("{} ({})", a.tooltip, key.glyphs()),
                None => a.tooltip.clone(),
            });
            spoken(
                style::status_attention("status-attention", ui, ring && keyed(item)),
                &Some(a.spoken.clone()),
                item,
            )
            .child(style::status_tile_at(
                ("status-attention-tile", tab as u64),
                ui,
                StatusTile {
                    face: a.face.clone(),
                    glyph: StatusGlyph::NeedsYou,
                    selected: false,
                    mini: true,
                },
                style::TILE_SMALL,
                cx,
            ))
            .child(div().min_w_0().max_w(px(ATTENTION_NAME_MAX)).truncate().child(a.name.clone()))
            .child(div().flex_none().text_color(ui.attention_text).child(a.words.clone()))
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .on_click(cx.listener(move |this, _, window, cx| this.go_to_tab(tab, window, cx)))
        });
        // The rest as numbers: "2 working · 1 done" while there is room for the words, each a
        // glyph and its count after that.
        let rest = bar.rest.iter().enumerate().map(|(ix, r)| {
            let tab = r.tab;
            let item = match r.activity {
                Activity::NeedsYou => StatusItem::NeedsYou(tab),
                Activity::Done => StatusItem::Done(tab),
                Activity::Failed => StatusItem::Failed(tab),
                Activity::Working | Activity::Quiet => StatusItem::Working(tab),
            };
            let glyph = match r.activity {
                Activity::NeedsYou => {
                    div().text_color(ui.attention_text).child(style::icon(IconName::CircleAlert))
                }
                Activity::Done => div().text_color(ui.agent_done).child(style::icon(IconName::CircleCheck)),
                Activity::Failed => div().text_color(ui.danger_text).child(style::icon(IconName::CircleX)),
                Activity::Working | Activity::Quiet => div().child(style::status_glyph(
                    ("status-working", tab as u64),
                    ui,
                    StatusGlyph::Working,
                    cx,
                )),
            };
            let button = spoken(
                style::status_item(("status-rest", ix), ui, false, ring && keyed(item)),
                &Some(r.spoken.clone()),
                item,
            )
            .when(!bar.words, |d| d.child(glyph))
            .child(if bar.words { r.words.clone() } else { r.count.to_string() })
            .on_click(cx.listener(move |this, _, window, cx| this.go_to_tab(tab, window, cx)));
            // "2 working · 1 done": a dot between the words.
            div()
                .flex()
                .items_center()
                .when(ix > 0 && bar.words, |d| d.child(div().text_caption().text_color(ui.faint).child("·")))
                .child(button)
        });
        // The odd window's profile: its colour and name, before everything else.
        let profile = bar.profile.clone().map(|tag| {
            let tooltip = SharedString::from(tag.tooltip.clone());
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    style::status_text(ui)
                        .id("status-profile")
                        .role(Role::Label)
                        .aria_label(tag.spoken.clone())
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(style::color_dot(ui, tag.color, style::COLOR_DOT_MENU))
                        .child(tag.name.clone())
                        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx)),
                )
                .child(style::status_separator(ui))
        });
        let bar_element = style::status_bar(ui, self.glass_on(cx))
            .id("status-bar")
            // A toolbar, there only while the bar is shown.
            .role(Role::Toolbar)
            .aria_label("Status bar")
            .children(profile)
            .children(branch)
            .children(
                bar.branch
                    .as_ref()
                    .filter(|_| attention.is_some() || !bar.rest.is_empty())
                    .map(|_| style::status_separator(ui)),
            )
            .children(attention)
            .child(div().flex().items_center().children(rest))
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
