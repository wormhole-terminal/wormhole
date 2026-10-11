//! The welcome's pages as a pure function: the model, the screen's size and the clock in, a grid of
//! styled cells and the lines that can be clicked out. Nothing here writes to a terminal, so every step
//! at every size is a snapshot test.
//!
//! The clock is how long the program has been drawing (the wait for the start animation is over).
//! Titles type out a few milliseconds a character, and the lines of a list appear one by one; under
//! Reduce Motion, after a key, or on a step seen before, the model hands the pages a clock of for ever
//! and everything is simply there.
//!
//! Layout: a left margin of two cells and a blank row on top, then the `>_` mark at [`MARK_CELL`] (or
//! the logo at [`LOGO`]), wherever the window is sized, because the start animation aims the ship at
//! that cell. Colours are the terminal's palette only.

use std::time::Duration;

use super::ansi::{Grid, Style, color, fit, text_width, wrap};
use super::art::{self, LOGO, MARGIN, MARK_CELL, TOP};
use super::keys::Key;
use super::model::Welcome;
use crate::intro::Handover;
use crate::ipc::control::HooksFact;
use crate::keymap::{self, ActionId};
use crate::onboarding::Step;

/// The one line under the name.
pub const TAGLINE: &str = "A terminal for working with AI coding agents.";

/// How fast the name types, then the lines under it, a character; a list's rows each appear this long
/// after the one before; and the pause after a line.
pub(super) const TYPE_NAME: Duration = Duration::from_millis(28);
pub(super) const TYPE_TEXT: Duration = Duration::from_millis(6);
pub(super) const ROW_EVERY: Duration = Duration::from_millis(40);
pub(super) const LINE_PAUSE: Duration = Duration::from_millis(70);

/// The screen's size in cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub cols: usize,
    pub rows: usize,
}

/// What a click on a span of a row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The cursor goes to this line of the list and Return is pressed on it.
    Row(usize),
    /// A key in the footer.
    Key(Key),
}

/// A clickable span: cells `from..to` of `row`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hit {
    pub row: usize,
    pub from: usize,
    pub to: usize,
    pub target: Target,
}

impl Hit {
    pub fn contains(&self, row: usize, col: usize) -> bool {
        self.row == row && (self.from..self.to).contains(&col)
    }
}

/// A drawn page.
#[derive(Debug, PartialEq, Eq)]
pub struct Frame {
    pub grid: Grid,
    pub hits: Vec<Hit>,
}

const TITLE: Style = Style::PLAIN.bold();
const DIM: Style = Style::PLAIN.dim();
const ACCENT: Style = Style::PLAIN.fg(color::BLUE).bold();
const GOOD: Style = Style::PLAIN.fg(color::GREEN);
const BAD: Style = Style::PLAIN.fg(color::RED);
const MARK: Style = Style::PLAIN.fg(color::BRIGHT_WHITE).bold();

/// Draw the welcome as it looks `clock` into the program.
pub fn render(welcome: &Welcome, size: Size, clock: Duration) -> Frame {
    let mut page = Page {
        grid: Grid::new(size.cols, size.rows),
        hits: Vec::new(),
        clock: welcome.page_clock(clock),
        t: Duration::ZERO,
        turn: Duration::ZERO,
        size,
    };
    let hints = match welcome.step() {
        Step::Welcome => page.welcome(welcome),
        Step::Folder => page.folder(welcome),
        Step::Look => page.look(welcome),
        Step::Agents => page.agents(welcome),
        Step::Primer => page.primer(welcome),
    };
    page.problem_and_tour(welcome);
    page.footer(&hints);
    Frame { grid: page.grid, hits: page.hits }
}

/// A footer hint: the key as it is written, what it does, and the key a click on it presses.
type Hint<'a> = (&'a str, &'a str, Option<Key>);

struct Page {
    grid: Grid,
    hits: Vec<Hit>,
    /// How far into the page's entrance we are.
    clock: Duration,
    /// Where the next line starts in the entrance's schedule, and where the last one did.
    t: Duration,
    turn: Duration,
    size: Size,
}

impl Page {
    /// How many of `len` characters are out at the schedule time `start`, `per` a character.
    fn out(&self, start: Duration, per: Duration, len: usize) -> usize {
        if self.clock == Duration::MAX {
            return len;
        }
        if self.clock < start {
            return 0;
        }
        let done = (self.clock - start).as_micros() / per.as_micros().max(1);
        (done as usize + 1).min(len)
    }

    /// Draw the first `n` characters of `parts` from (`row`, `col`); returns where the whole would end.
    fn draw(&mut self, row: usize, col: usize, parts: &[(&str, Style)], n: usize) -> usize {
        let (mut at, mut left) = (col, n);
        for (text, style) in parts {
            let take: String = text.chars().take(left).collect();
            left -= take.chars().count();
            self.grid.put(row, at, &take, *style);
            at += text_width(text);
        }
        at
    }

    /// A line that types out, `per` a character; the next waits for it.
    fn typed(&mut self, row: usize, col: usize, parts: &[(&str, Style)], per: Duration) -> usize {
        let total: usize = parts.iter().map(|(text, _)| text.chars().count()).sum();
        let n = self.out(self.t, per, total);
        self.turn = self.t;
        self.t += per * u32::try_from(total).unwrap_or(u32::MAX) + LINE_PAUSE;
        self.draw(row, col, parts, n)
    }

    /// A line that is there from its turn on, the next a little later.
    fn shown(&mut self, row: usize, col: usize, parts: &[(&str, Style)]) -> usize {
        let end = self.also(row, col, parts, self.t);
        self.turn = self.t;
        self.t += ROW_EVERY;
        end
    }

    /// More of the line whose turn `shown` just took (its second column): out with it, not after it.
    fn also(&mut self, row: usize, col: usize, parts: &[(&str, Style)], turn: Duration) -> usize {
        let total: usize = parts.iter().map(|(text, _)| text.chars().count()).sum();
        let n = self.out(turn, ROW_EVERY, 1) * total;
        self.draw(row, col, parts, n)
    }

    fn hit(&mut self, row: usize, from: usize, to: usize, target: Target) {
        self.hits.push(Hit { row, from, to, target });
    }

    /// The width a page's text may take.
    fn text_width(&self) -> usize {
        self.size.cols.saturating_sub(2 * MARGIN).max(8)
    }

    // ---------------------------------------------------------------- the pages

    /// The mark, the name and what it is, then the sidebar's three lines and the way on.
    fn welcome(&mut self, w: &Welcome) -> Vec<Hint<'static>> {
        let next = self.header(w.handover());
        let mut row = next;
        let lines = [
            "your terminals and what their agents are doing".to_string(),
            "which is working, which needs you, which is done".to_string(),
            format!("{} opens another; click one to switch", keymap::glyphs(ActionId::NewTerminal)),
        ];
        for line in lines {
            self.shown(row, MARGIN, &[("←", ACCENT), ("  ", Style::PLAIN), (&line, Style::PLAIN)]);
            row += 1;
        }
        row += 1;
        let label = if w.is_replay() { "Back to Wormhole" } else { "Get started" };
        self.item_row(row, MARGIN, true, &[(label, TITLE)], 0);
        vec![
            ("⏎", if w.is_replay() { "back to Wormhole" } else { "get started" }, Some(Key::Enter)),
            ESC_SKIP,
        ]
    }

    /// The top of the welcome page, which the start animation lands on: the mark's `>` is at
    /// [`MARK_CELL`], or the logo at [`LOGO`]. Returns the row the page continues from.
    fn header(&mut self, handover: Handover) -> usize {
        let (row, col) = MARK_CELL;
        match handover {
            Handover::Logo => {
                // The ship has docked on the chevron: the logo is there, the name types beside it.
                if self.out(self.t, ROW_EVERY, 1) > 0 {
                    art::draw_logo(&mut self.grid, LOGO.row, LOGO.col);
                }
                self.t += ROW_EVERY;
                let text_col = LOGO.col + LOGO.cols + 2;
                self.typed(LOGO.row + 1, text_col, &[("Wormhole", TITLE)], TYPE_NAME);
                self.typed(LOGO.row + 2, text_col, &[(TAGLINE, Style::PLAIN)], TYPE_TEXT);
                LOGO.row + LOGO.rows + 1
            }
            Handover::Cursor => {
                // The ship has docked on the `>`: it is there, the `_` a beat after, then the name.
                self.shown(row, col, &[(">", MARK)]);
                self.shown(row, col + 1, &[("_", MARK)]);
                self.typed(row, col + 3, &[("Wormhole", TITLE)], TYPE_NAME);
                self.typed(row + 1, col + 3, &[(TAGLINE, Style::PLAIN)], TYPE_TEXT);
                row + 3
            }
            Handover::Portal => {
                // No ship: the mark types with the name.
                self.typed(row, col, &[(">_", MARK), (" ", Style::PLAIN), ("Wormhole", TITLE)], TYPE_NAME);
                self.typed(row + 1, col + 3, &[(TAGLINE, Style::PLAIN)], TYPE_TEXT);
                row + 3
            }
        }
    }

    /// A step's heading and the line under it; returns the row the step continues from.
    fn heading(&mut self, title: &str, note: &str) -> usize {
        self.typed(TOP, MARGIN, &[(title, TITLE)], TYPE_NAME);
        let mut row = TOP + 1;
        for line in wrap(note, self.text_width()) {
            self.typed(row, MARGIN, &[(&line, DIM)], TYPE_TEXT);
            row += 1;
        }
        row + 1
    }

    fn folder(&mut self, w: &Welcome) -> Vec<Hint<'static>> {
        let row = self.heading("Where do you work?", "Your shell starts in the folder you pick.");
        let places = w.places();
        let name_w = places.iter().map(|p| text_width(&p.name)).max().unwrap_or(0).min(24);
        let height = self.list_height(row, 0);
        let cursor = w.cursor();
        let first = window_start(cursor, places.len(), height);
        for (line, place) in places.iter().enumerate().skip(first).take(height) {
            let at = row + line - first;
            let name = fit(&place.name, name_w);
            let pad = " ".repeat(name_w - text_width(&name) + 2);
            let detail = fit(&place.detail, self.text_width().saturating_sub(name_w + 4));
            let name_style = if line == cursor { ACCENT } else { Style::PLAIN };
            let end =
                self.item_row(at, MARGIN, line == cursor, &[(&name, name_style), (&pad, Style::PLAIN)], line);
            self.also(at, end, &[(&detail, DIM)], self.turn);
        }
        vec![UP_DOWN, ("⏎", "continue", Some(Key::Enter)), BACK, ESC_SKIP]
    }

    fn look(&mut self, w: &Welcome) -> Vec<Hint<'static>> {
        let look = w.look();
        if look.themes.is_empty() {
            let row = self.heading(
                "How should it look?",
                &format!(
                    "Themes, colours and the font are in Settings ({}).",
                    keymap::glyphs(ActionId::Settings)
                ),
            );
            self.shown(row, MARGIN, &[("Nothing changes here: this tour is read-only.", DIM)]);
            return vec![("⏎", "continue", Some(Key::Enter)), BACK, ESC_SKIP];
        }
        let row =
            self.heading("How should it look?", "Themes apply as you move. The rest of it is in Settings.");
        self.shown(row, MARGIN, &[("Theme", DIM)]);
        // The rows below the list: a blank, the colour, the font and the ligatures.
        let below = 4;
        let height = self.list_height(row + 1, below);
        let cursor = w.cursor();
        let first = window_start(cursor, look.themes.len(), height);
        for (line, theme) in look.themes.iter().enumerate().skip(first).take(height) {
            let at = row + 1 + line - first;
            let style = if line == cursor { ACCENT } else { Style::PLAIN };
            let end = self.item_row(at, MARGIN, line == cursor, &[(&theme.name, style)], line);
            if theme.light {
                self.also(at, end + 2, &[("light", DIM)], self.turn);
            }
        }
        let mut at = row + 1 + height.min(look.themes.len()) + 1;
        // The primary colour: a dot in its own colour before each name, the one in use filled and bold.
        self.shown(at, MARGIN, &[("Colour", DIM)]);
        let turn = self.turn;
        let mut col = MARGIN + VALUE_AT;
        for primary in &look.primaries {
            let current = primary.hex == look.primary;
            let dot = Style { fg: primary.ansi, ..Style::PLAIN };
            let name = if current { TITLE } else { Style::PLAIN };
            let parts = [(if current { "●" } else { "○" }, dot), (" ", Style::PLAIN), (&primary.name, name)];
            let width = text_width(&primary.name) + 2;
            if col + width > self.size.cols.saturating_sub(MARGIN) {
                break;
            }
            self.also(at, col, &parts, turn);
            col += width + 2;
        }
        at += 1;
        let font =
            fit(&format!("{} {:.0}", look.font, look.font_size), self.text_width().saturating_sub(VALUE_AT));
        self.shown(at, MARGIN, &[("Font", DIM)]);
        self.also(at, MARGIN + VALUE_AT, &[(&font, Style::PLAIN)], self.turn);
        at += 1;
        self.shown(at, MARGIN, &[("Ligatures", DIM)]);
        let state = if look.ligatures { "on " } else { "off" };
        self.also(
            at,
            MARGIN + VALUE_AT,
            &[(state, TITLE), ("  ", Style::PLAIN), ("=> != >=", DIM)],
            self.turn,
        );
        vec![
            UP_DOWN,
            ("p", "colour", Some(Key::Char('p'))),
            ("l", "ligatures", Some(Key::Char('l'))),
            ("+ -", "size", None),
            ("⏎", "continue", Some(Key::Enter)),
            BACK,
            ESC_SKIP,
        ]
    }

    fn agents(&mut self, w: &Welcome) -> Vec<Hint<'static>> {
        let note = "Hooks tell Wormhole what an agent is doing. Nothing is installed until you press \
                    Install on its row.";
        let mut row = self.heading("Coding agents", note);
        let cursor = w.cursor();
        let Some(agents) = w.agents() else {
            self.shown(row, MARGIN, &[("Looking for agents…", DIM)]);
            return vec![("⏎", "start", Some(Key::Enter)), BACK, ESC_SKIP];
        };
        let name_w = agents.found.iter().map(|a| text_width(&a.name)).max().unwrap_or(0).min(20);
        let wide = self.size.cols >= 72;
        let mut enter = "start";
        for (line, agent) in agents.found.iter().enumerate() {
            let selected = line == cursor;
            let name = fit(&agent.name, name_w);
            let style = if selected { ACCENT } else { Style::PLAIN };
            let pad = " ".repeat(name_w - text_width(&name) + 2);
            let end = self.item_row(row, MARGIN, selected, &[(&name, style), (&pad, Style::PLAIN)], line);
            if wide && let Some(path) = &agent.path {
                self.grid.put(row, end, &fit(path, 28), DIM);
            }
            let (status, status_style) = match agent.hooks {
                HooksFact::Installed => ("Installed ✓", GOOD),
                HooksFact::Partial => ("[ Repair ]", if selected { ACCENT } else { Style::PLAIN }),
                HooksFact::Missing => ("[ Install ]", if selected { ACCENT } else { Style::PLAIN }),
                HooksFact::Unsupported => ("status from its output", DIM),
            };
            let status = if w.live() || !agent.hooks.installable() { status } else { "not installed" };
            let status_style = if w.live() || !agent.hooks.installable() { status_style } else { DIM };
            let at = self.size.cols.saturating_sub(MARGIN + text_width(status)).max(end);
            self.grid.put(row, at, status, status_style);
            self.hit(row, at, at + text_width(status), Target::Row(line));
            if selected && agent.hooks.installable() && w.live() {
                enter = if agent.hooks == HooksFact::Partial { "repair" } else { "install" };
            }
            row += 1;
        }
        if agents.found.is_empty() {
            self.shown(row, MARGIN, &[("None found on PATH.", DIM)]);
            row += 1;
        } else if agents.others > 0 {
            let line = format!("{} more recognised, not found on PATH", agents.others);
            self.shown(row, MARGIN + 2, &[(&line, DIM)]);
            row += 1;
        }
        row += 1;
        let mut at_item = agents.found.len();
        if let Some(trust) = w.trust() {
            let title = format!("{}/ has a {}/ folder", trust.name, trust.dir);
            self.shown(row, MARGIN, &[(&fit(&title, self.text_width()), TITLE)]);
            row += 1;
            let waits = match trust.commands {
                0 => "Nothing in it runs commands yet.".to_string(),
                1 => "Its action runs a command, so it waits until you trust the folder.".to_string(),
                n => format!("Its actions run {n} commands, so they wait until you trust the folder."),
            };
            for line in wrap(&format!("Its theme and notes work now. {waits}"), self.text_width()) {
                self.shown(row, MARGIN, &[(&line, DIM)]);
                row += 1;
            }
            let selected = cursor == at_item;
            let label =
                if trust.asking { "Waiting for approval…" } else { "Trust this folder (Touch ID)…" };
            self.item_row(
                row,
                MARGIN,
                selected,
                &[(label, if selected { ACCENT } else { Style::PLAIN })],
                at_item,
            );
            if selected && !trust.asking {
                enter = "trust";
            }
            row += 2;
            at_item += 1;
        }
        let selected = cursor == at_item;
        self.item_row(row, MARGIN, selected, &[("Start", if selected { ACCENT } else { TITLE })], at_item);
        vec![UP_DOWN, ("⏎", enter, Some(Key::Enter)), BACK, ESC_SKIP]
    }

    /// No agent on PATH: Wormhole as a terminal, and the keys worth knowing.
    fn primer(&mut self, w: &Welcome) -> Vec<Hint<'static>> {
        let _ = w;
        let note = "No coding agent found on PATH. Install Claude Code, Codex or another and Wormhole \
                    recognises it in any terminal, then offers its hooks once.";
        let mut row = self.heading("Ready when you are", note);
        self.shown(row, MARGIN, &[("Things to try first", DIM)]);
        row += 1;
        let keys: [(&str, ActionId); 4] = [
            ("Jump to anything", ActionId::CommandPalette),
            ("See every shortcut", ActionId::KeyboardShortcuts),
            ("Show what changed in git", ActionId::ShowChanges),
            ("Hide the side panels", ActionId::ToggleSidebar),
        ];
        let label_w = keys.iter().map(|(label, _)| text_width(label)).max().unwrap_or(0);
        for (label, action) in keys {
            let glyphs = keymap::glyphs(action);
            let pad = " ".repeat(label_w - text_width(label) + 3);
            self.shown(row, MARGIN + 2, &[(label, Style::PLAIN), (&pad, Style::PLAIN), (&glyphs, TITLE)]);
            row += 1;
        }
        row += 1;
        self.item_row(row, MARGIN, true, &[("Start", ACCENT)], 0);
        vec![("⏎", "start", Some(Key::Enter)), BACK, ESC_SKIP]
    }

    // ---------------------------------------------------------------- the pieces

    /// A line of a list: the marker, the parts, and a click anywhere on them. Returns where the
    /// parts end.
    fn item_row(
        &mut self,
        row: usize,
        col: usize,
        selected: bool,
        parts: &[(&str, Style)],
        line: usize,
    ) -> usize {
        let marker = if selected { "❯" } else { " " };
        let mut all = vec![(marker, ACCENT), (" ", Style::PLAIN)];
        all.extend_from_slice(parts);
        let end = self.shown(row, col, &all);
        self.hit(row, col, end.max(col + 2), Target::Row(line));
        end
    }

    /// How many lines a list that starts at `row` may take, leaving `below` lines after it and the
    /// footer (and a tour's note) at the bottom.
    fn list_height(&self, row: usize, below: usize) -> usize {
        self.size.rows.saturating_sub(row + below + 2).max(1)
    }

    /// The lines at the bottom: what went wrong with the last Install, and a tour's one dim note.
    fn problem_and_tour(&mut self, w: &Welcome) {
        let last = self.size.rows.saturating_sub(1);
        if let Some(problem) = w.problem()
            && last >= 3
        {
            let line = fit(problem, self.text_width());
            self.grid.put(last - 2, MARGIN, &line, BAD);
        }
        // With the footer: the page is whole when the last line of it is out.
        if !w.live() && last >= 2 && self.out(self.t, ROW_EVERY, 1) > 0 {
            let note = "Read-only tour: not connected to Wormhole, so nothing here changes anything.";
            self.grid.put(last - 1, MARGIN, &fit(note, self.text_width()), DIM);
        }
    }

    /// The key hints along the bottom row; each is clickable.
    fn footer(&mut self, hints: &[Hint<'_>]) {
        let row = self.size.rows.saturating_sub(1);
        let mut col = MARGIN;
        // It comes in last, when the page above it is out.
        let out = self.out(self.t, ROW_EVERY, 1) > 0;
        for (key, label, press) in hints {
            let width = text_width(key) + 1 + text_width(label);
            if col + width > self.size.cols.saturating_sub(MARGIN) {
                break;
            }
            if out {
                self.grid.put(row, col, key, TITLE);
                self.grid.put(row, col + text_width(key) + 1, label, DIM);
            }
            if let Some(press) = press {
                self.hit(row, col, col + width, Target::Key(*press));
            }
            col += width + 2;
        }
    }
}

/// Where the value of a look option starts, after its label.
const VALUE_AT: usize = 11;

const UP_DOWN: Hint<'static> = ("↑↓", "choose", None);
const BACK: Hint<'static> = ("←", "back", Some(Key::Left));
const ESC_SKIP: Hint<'static> = ("esc", "skip", Some(Key::Esc));

/// The first line of a window of `height` lines onto a list of `len`, the cursor kept inside it.
fn window_start(cursor: usize, len: usize, height: usize) -> usize {
    if len <= height {
        return 0;
    }
    cursor.saturating_sub(height / 2).min(len - height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_onto_a_list_keeps_the_cursor_inside() {
        assert_eq!(window_start(0, 3, 5), 0, "it all fits");
        assert_eq!(window_start(0, 10, 4), 0);
        assert_eq!(window_start(5, 10, 4), 3);
        assert_eq!(window_start(9, 10, 4), 6, "at the end the window ends there");
    }
}
