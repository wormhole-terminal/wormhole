//! The pages after how it looks: what reaches you, the sidebar with the agents, and what else is here.

use super::super::ansi::{Style, fit, text_width, wrap};
use super::super::art::MARGIN;
use super::super::keys::Key;
use super::super::model::{Item, NotifyRow, Welcome};
use super::{ACCENT, ATTENTION, BACK, DIM, ESC_SKIP, GOOD, Hint, Page, TITLE, Target, UP_DOWN, window_start};
use crate::config::NotifySound;
use crate::ipc::control::{HooksFact, NotifyFacts, PermissionFact};
use crate::keymap::{self, ActionId};

/// What the person is told when no coding agent is on PATH.
const PRIMER: &str = "No coding agent found on PATH. Install Claude Code, Codex or another and Wormhole \
                      recognises it in any terminal, then offers its hooks once.";

impl Page {
    /// What reaches you: the same switches as Settings → Notifications, the current ones chosen.
    pub(super) fn notify(&mut self, w: &Welcome) -> Vec<Hint<'static>> {
        let settings = keymap::glyphs(ActionId::Settings);
        let note = format!(
            "A banner and a sound can tell you when a terminal needs you or an agent finishes, so you \
             can look away. The same switches are in Settings ({settings})."
        );
        let mut row = self.heading("Tell me when an agent needs me", &note);
        let Some(notify) = w.notify().copied() else {
            self.shown(row, MARGIN, &[("Nothing changes here: this tour is read-only.", DIM)]);
            return vec![("⏎", "continue", Some(Key::Enter)), BACK, ESC_SKIP];
        };
        let cursor = w.cursor();
        let mut enter = "continue";
        let mut start = None;
        for (line, item) in w.items().iter().enumerate() {
            let selected = line == cursor;
            match item {
                Item::Notify(NotifyRow::NeedsYou) => {
                    let options = [("On", notify.needs_you), ("Off", !notify.needs_you)];
                    let meaning = "an agent asks a question or for permission";
                    self.switch(row, line, selected, "Needs you", &options, meaning);
                }
                Item::Notify(NotifyRow::AgentDone) => {
                    let options = [("On", notify.agent_done), ("Off", !notify.agent_done)];
                    self.switch(row, line, selected, "Finished", &options, "an agent is done");
                }
                Item::Notify(NotifyRow::Sound) => {
                    self.switch(row, line, selected, "Sound", &sound_choices(notify.sound), "");
                }
                Item::Notify(NotifyRow::Allow) => {
                    let style = if selected { ACCENT } else { Style::PLAIN };
                    let end = self.item_row(row, MARGIN, selected, &[("Allow notifications…", style)], line);
                    let turn = self.turn;
                    let hint = "macOS asks once, and only when you press this";
                    if end + 2 + text_width(hint) <= self.size.cols.saturating_sub(MARGIN) {
                        self.also(row, end + 2, &[(hint, DIM)], turn);
                    }
                }
                Item::Start => start = Some(line),
                Item::Theme(_) | Item::Agent(_) => continue,
            }
            if selected {
                enter = match item {
                    Item::Notify(NotifyRow::Allow) => "allow",
                    Item::Notify(_) => "change",
                    _ => "continue",
                };
            }
            if *item != Item::Start {
                row += 1;
            }
        }
        // What macOS says, then the button.
        row += 1;
        let said = self.permission_line(row, &notify);
        let line = start.unwrap_or(0);
        self.button(row + said + usize::from(said > 0), line == cursor, "Continue", line);
        vec![UP_DOWN, ("⏎", enter, Some(Key::Enter)), BACK, ESC_SKIP]
    }

    /// A row of choices: its label, then each option with the one in use filled, and a dim line of
    /// what it is for when there is room. A click anywhere on it presses it.
    fn switch(
        &mut self,
        at: usize,
        line: usize,
        selected: bool,
        label: &str,
        options: &[(&str, bool)],
        meaning: &str,
    ) {
        let style = if selected { ACCENT } else { Style::PLAIN };
        let pad = " ".repeat(LABEL_W.saturating_sub(text_width(label)) + 2);
        let end = self.item_row(at, MARGIN, selected, &[(label, style), (&pad, Style::PLAIN)], line);
        let turn = self.turn;
        let mut col = end;
        for (name, chosen) in options {
            let (dot, name_style) = if *chosen { ("●", TITLE) } else { ("○", DIM) };
            let width = text_width(name) + 2;
            self.also(at, col, &[(dot, name_style), (" ", Style::PLAIN), (name, name_style)], turn);
            col += width + 2;
        }
        let edge = self.size.cols.saturating_sub(MARGIN);
        if !meaning.is_empty() && col + text_width(meaning) <= edge {
            self.also(at, col + 2, &[(meaning, DIM)], turn);
        }
        self.hit(at, MARGIN, col.min(edge).max(MARGIN + 2), Target::Row(line));
    }

    /// What macOS says about banners, under the choices; returns how many rows it took.
    fn permission_line(&mut self, row: usize, notify: &NotifyFacts) -> usize {
        let (mark, text, style): (&str, &str, Style) = match notify.permission {
            PermissionFact::Allowed => ("✓", "macOS lets Wormhole show banners.", GOOD),
            PermissionFact::Denied => (
                "!",
                "Off in System Settings: banners stay away until you turn Wormhole's notifications on there.",
                ATTENTION,
            ),
            PermissionFact::Outside => {
                ("·", "This build runs outside Wormhole.app, so banners come through osascript.", DIM)
            }
            PermissionFact::NotAsked | PermissionFact::Checking => return 0,
        };
        self.shown(row, MARGIN, &[(mark, style)]);
        let turn = self.turn;
        let lines = wrap(text, self.text_width().saturating_sub(2));
        for (n, piece) in lines.iter().enumerate() {
            self.also(row + n, MARGIN + 2, &[(piece, DIM)], turn);
        }
        lines.len()
    }

    /// The sidebar step: what the example rows in the window's sidebar mean, and the agents found.
    pub(super) fn sidebar(&mut self, w: &Welcome) -> Vec<Hint<'static>> {
        let note = "The Example folder in the sidebar is only there while this page is up: three \
                    terminals showing what a row tells you.";
        let mut row = self.heading("Watch the sidebar", note);
        let states = [
            ("working  ", ACCENT, "the agent is busy; nothing for you to do"),
            ("needs you", ATTENTION, "it asked a question or for permission"),
            ("done     ", GOOD, "it finished; time to look at what it did"),
        ];
        for (word, style, meaning) in states {
            self.shown(
                row,
                MARGIN,
                &[
                    ("←", ACCENT),
                    ("  ", Style::PLAIN),
                    (word, style),
                    ("  ", Style::PLAIN),
                    (meaning, Style::PLAIN),
                ],
            );
            row += 1;
        }
        row += 1;
        let cursor = w.cursor();
        let Some(agents) = w.agents() else {
            self.shown(row, MARGIN, &[("Looking for agents…", DIM)]);
            self.button(row + 2, true, "Continue", 0);
            return vec![("⏎", "continue", Some(Key::Enter)), BACK, ESC_SKIP];
        };
        let mut enter = "continue";
        if agents.found.is_empty() {
            row = self.paragraph(row, MARGIN, PRIMER, DIM) + 1;
            self.button(row, true, "Continue", 0);
            return vec![("⏎", "continue", Some(Key::Enter)), BACK, ESC_SKIP];
        }
        self.shown(row, MARGIN, &[("Coding agents", DIM)]);
        row += 1;
        let note = "Hooks tell Wormhole what an agent is doing. Nothing is installed until you press \
                    Install on its row.";
        row = self.paragraph(row, MARGIN, note, DIM) + 1;
        let name_w = agents.found.iter().map(|a| text_width(&a.name)).max().unwrap_or(0).min(20);
        let wide = self.size.cols >= 72;
        // The agents, then (maybe) the count of the others, a blank and the button.
        let below = usize::from(agents.others > 0) + 2;
        let height = self.list_height(row, below);
        let first = window_start(cursor, agents.found.len(), height);
        for (line, agent) in agents.found.iter().enumerate().skip(first).take(height) {
            let at = row + line - first;
            let selected = line == cursor;
            let name = fit(&agent.name, name_w);
            let style = if selected { ACCENT } else { Style::PLAIN };
            let pad = " ".repeat(name_w - text_width(&name) + 2);
            let end = self.item_row(at, MARGIN, selected, &[(&name, style), (&pad, Style::PLAIN)], line);
            if wide && let Some(path) = &agent.path {
                self.grid.put(at, end, &fit(path, 28), DIM);
            }
            let (status, status_style) = match agent.hooks {
                HooksFact::Installed => ("Installed ✓", GOOD),
                HooksFact::Partial => ("[ Repair ]", if selected { ACCENT } else { Style::PLAIN }),
                HooksFact::Missing => ("[ Install ]", if selected { ACCENT } else { Style::PLAIN }),
                HooksFact::Unsupported => ("status from its output", DIM),
            };
            let status = if w.live() || !agent.hooks.installable() { status } else { "not installed" };
            let status_style = if w.live() || !agent.hooks.installable() { status_style } else { DIM };
            let place = self.size.cols.saturating_sub(MARGIN + text_width(status)).max(end);
            self.grid.put(at, place, status, status_style);
            self.hit(at, place, place + text_width(status), Target::Row(line));
            if selected && agent.hooks.installable() && w.live() {
                enter = if agent.hooks == HooksFact::Partial { "repair" } else { "install" };
            }
        }
        row += height.min(agents.found.len());
        if agents.others > 0 {
            let line = format!("{} more recognised, not found on PATH", agents.others);
            self.shown(row, MARGIN + 2, &[(&line, DIM)]);
            row += 1;
        }
        self.button(row + 1, cursor == agents.found.len(), "Continue", agents.found.len());
        vec![UP_DOWN, ("⏎", enter, Some(Key::Enter)), BACK, ESC_SKIP]
    }

    /// What else is here, read-only: the keys come from the keymap.
    pub(super) fn extras(&mut self) -> Vec<Hint<'static>> {
        let palette = keymap::glyphs(ActionId::CommandPalette);
        let note = format!("Everything has a key, and every action is in the command palette ({palette}).");
        let mut row = self.heading("What else is here", &note);
        let worktree =
            keymap::entry(ActionId::NewTerminalInWorktree).map(|entry| entry.title).unwrap_or_default();
        let lines = [
            (
                keymap::glyphs(ActionId::ShowChanges),
                "Changes",
                "what an agent changed. Click a line of a diff to comment on it, then paste the comments \
                 into the terminal for its agent."
                    .to_string(),
            ),
            (
                keymap::glyphs(ActionId::ShowNotes),
                "Notes",
                "notes and next steps for each terminal, kept with it.".to_string(),
            ),
            (palette, "Worktrees", format!("{worktree} gives a branch a folder and a terminal of its own.")),
        ];
        let key_w = lines.iter().map(|(keys, ..)| text_width(keys)).max().unwrap_or(0);
        let label_w = lines.iter().map(|(_, label, _)| text_width(label)).max().unwrap_or(0);
        let text_at = MARGIN + key_w + 2 + label_w + 2;
        let room = self.size.cols.saturating_sub(text_at + MARGIN).max(8);
        for (keys, label, text) in &lines {
            let pad_keys = " ".repeat(key_w - text_width(keys) + 2);
            let pad_label = " ".repeat(label_w - text_width(label) + 2);
            for (n, piece) in wrap(text, room).iter().enumerate() {
                if n == 0 {
                    let head = [
                        (keys.as_str(), TITLE),
                        (&pad_keys, Style::PLAIN),
                        (label, ACCENT),
                        (&pad_label, Style::PLAIN),
                    ];
                    self.shown(row, MARGIN, &[head[0], head[1], head[2], head[3], (piece, Style::PLAIN)]);
                } else {
                    self.shown(row, text_at, &[(piece, Style::PLAIN)]);
                }
                row += 1;
            }
        }
        self.button(row + 1, true, "Start", 0);
        vec![("⏎", "start", Some(Key::Enter)), BACK, ESC_SKIP]
    }
}

/// The width of a switch's label ("Needs you" is the longest), so the choices line up.
const LABEL_W: usize = 9;

/// Settings' three sounds, the one in use chosen.
fn sound_choices(sound: NotifySound) -> [(&'static str, bool); 3] {
    [
        ("Needs you only", sound == NotifySound::NeedsYou),
        ("All", sound == NotifySound::All),
        ("None", sound == NotifySound::None),
    ]
}
