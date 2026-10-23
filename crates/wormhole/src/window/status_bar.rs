//! The status bar along the bottom of the window. It is optional and hidden by default (`status_bar` in config.json, one switch
//! for every window); then nothing of it is drawn and the panes reach the window's bottom.
//!
//! Shown, it has the terminal of any window that has waited longest for you as a button (its
//! tile, its name, how long: one click goes there), the other agents by what they do as numbers
//! ("2 working · 1 done"; each opens the first such terminal), how the terminal on screen is doing
//! ("working · 4m", which the title row leaves to the bar then) and its size. The branch,
//! its changes and the agent's context are the title row's, so the bar doesn't repeat them. What it
//! says, which terminal is
//! the longest waiting and how much fits the window's width, is core's
//! (`wormhole_core::view::layout::status_bar`).
use std::time::Instant;

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::focus::Area;
use wormhole_core::view::layout::status_bar::{self as rules, Activity, Detail, Entry, Summary};
use wormhole_core::view::spoken;

use crate::app_state::app;
use crate::settings::{settings_in, ui_in};
use crate::style::{self, TextRole};
use crate::style::{StatusGlyph, StatusTile, TileFace};
use crate::terminal_view::Status;
use crate::window::Workspace;
use crate::window::window_lifecycle::open_tab;

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
    /// The terminal that has waited longest, as the bar's button.
    attention: Option<Attention>,
    /// The rest, as numbers, in the order core gives them.
    rest: Vec<Rest>,
    /// Whether the window is wide enough for the words ("2 working") rather than glyphs and numbers.
    words: bool,
    /// How the terminal on screen is doing ("working · 4m"), which the title row says while the bar
    /// is hidden; nothing while it is idle.
    state: Option<String>,
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

/// The bar's items in words.
#[derive(Clone, Debug, Default, PartialEq)]
struct Spoken {
    size: Option<String>,
}

/// An item of the bar the keys can be on (a toolbar; the size is read, not pressed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StatusItem {
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
    pub(super) fn status_terminals(&self) -> Vec<Counted> {
        self.sessions
            .iter()
            .map(|session| {
                let summary = &session.pane.summary;
                Counted {
                    tab: session.id,
                    activity: match summary.status {
                        Status::Busy => Activity::Working,
                        Status::Attention => Activity::NeedsYou,
                        Status::Done => Activity::Done,
                        Status::Failed => Activity::Failed,
                        Status::Idle | Status::Exited => Activity::Quiet,
                    },
                    since: summary.status_since,
                    agent: summary
                        .agent
                        .map(|id| SharedString::from(wormhole_core::agents::agent(id).short_name()))
                        .or_else(|| summary.program.clone().map(SharedString::from)),
                    name: session.name(),
                    face: match summary.agent.map(wormhole_core::agents::agent) {
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
                window.workspace.read_with(cx, |ws, _| (window.id, ws.status_terminals())).ok()
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
                counted.extend(self.status_terminals());
                seen_self = true;
            } else if let Some((_, theirs)) = self.status_others.iter().find(|(id, _)| *id == window.id) {
                counted.extend(theirs.iter().cloned());
            }
        }
        if !seen_self {
            counted.extend(self.status_terminals());
        }
        let entries: Vec<Entry> = counted
            .iter()
            .map(|c| Entry { activity: c.activity, since: c.since, agent: c.agent.as_deref(), name: &c.name })
            .collect();
        let summary = Summary::of(&entries);
        let detail = Detail::at(self.window_width);

        let session = self.active_session();
        let (cols, rows) = session.pane.grid.size();
        // What runs in front (the agent, or the command), as the title row names it, not the shell.
        let program = session.pane.program();
        let size = detail.size().then(|| rules::size(&program, cols, rows));
        let state = crate::window::title::breadcrumb::crumb_words(&session.pane.summary);
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
        let spoken = Spoken { size: size.as_ref().map(|_| spoken::status_size(&program, cols, rows)) };
        let profile = self.odd_profile(cx).filter(|_| detail.profile()).map(|(profile, color)| ProfileTag {
            spoken: spoken::status_profile(&profile.name),
            tooltip: rules::profile_tooltip(&profile.name),
            name: profile.name.clone().into(),
            color,
        });
        Some(StatusBar { spoken, attention, rest, words: detail.words(), state, size, profile })
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
        bar.attention
            .as_ref()
            .map(|a| StatusItem::Attention(a.tab))
            .into_iter()
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
        let jump = settings_in(&self.profile, cx)
            .jump_waiting_key
            .parse::<wormhole_core::keys::keyspec::KeySpec>()
            .ok();
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
            .children(attention)
            .child(div().flex().items_center().children(rest))
            .child(div().flex_1())
            .children(bar.state.clone().map(|state| {
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(style::status_separator(ui))
                    .child(style::status_text(ui).id("status-state").role(Role::Label).child(state))
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
