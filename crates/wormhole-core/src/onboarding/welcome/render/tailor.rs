//! The Shown step's page: how much the window shows, asked one question at a time as a shell prompt's
//! configuration wizard does. Each answer is numbered and drawn as the window it would give
//! ([`sketch`](super::sketch)); the one in use has its outline lit, the cursor has a marker, a line says
//! what the answer is, and a button keeps things as they are. After the last question comes a summary
//! of what was chosen.
//!
//! The answers sit side by side as cards when the terminal is wide enough for three; on a narrower or
//! shorter one they are a list with the window the answer under the cursor gives drawn below it, and on
//! one with no room even for that, a list alone. All words are catalog messages (`tailor-*`), the same
//! the intro window's Look step uses.

use wormhole_text::keys::{
    TAILOR_HEADING, TAILOR_LOOKS_GOOD, TAILOR_OWN, TAILOR_SUMMARY, TAILOR_WELCOME_CONTINUE,
    TAILOR_WELCOME_CONTINUE_DETAIL, TAILOR_WELCOME_KEY_CONTINUE, TAILOR_WELCOME_KEY_MOVE,
    TAILOR_WELCOME_KEY_PICK, TAILOR_WELCOME_KEY_START_OVER, TAILOR_WELCOME_NOTE, TAILOR_WELCOME_UNAVAILABLE,
    tailor_counter,
};
use wormhole_text::text;

use super::super::ansi::{Style, fit, text_width, wrap};
use super::super::art::MARGIN;
use super::super::keys::Key;
use super::super::model::Welcome;
use super::super::tailoring::Tailoring;
use super::sketch::{MIN_COLS, Part, Sketch, sketch};
use super::{ACCENT, BACK, DIM, ESC_SKIP, GOOD, Hint, Page, ROW_EVERY, TITLE, Target, UP_DOWN};
use crate::onboarding::tailor::{Question, Step as Asked};

/// The cells between two cards.
const GAP: usize = 2;
/// A card is at least this wide (three of them fit an 80-column terminal with room to spare), and at
/// most this: a wider window than that shows no more.
const CARD_MIN_W: usize = 20;
const CARD_MAX_W: usize = 30;
/// The heights a sketch is drawn at, the tallest first.
const HEIGHTS: [usize; 3] = [8, 7, 6];
/// The rows a page needs under its answers: the line of detail (two, wrapped), a blank, the button, and
/// the three rows at the bottom (what went wrong, the tour's note, the keys).
const DETAIL_ROWS: usize = 2;
const BELOW: usize = DETAIL_ROWS + 1 + 1 + 3;
/// The summary's sketch of the whole window, and the rows under its button that the page keeps free
/// (what went wrong, the tour's note, the keys).
const SUMMARY_W: usize = 24;
const SUMMARY_H: usize = 8;
const BELOW_BUTTON: usize = 3;

/// How a question's answers are laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Plan {
    /// Side by side, each a sketch `w` by `h` cells with its label under it.
    Cards { w: usize, h: usize },
    /// A list, and the window of the answer under the cursor, `w` by `h`, below it.
    Preview { w: usize, h: usize },
    /// A list alone.
    List,
}

/// The layout for `answers` answers on a page `avail` cells wide and `rows` tall whose answers begin at
/// row `top`.
pub(super) fn plan(avail: usize, rows: usize, top: usize, answers: usize) -> Plan {
    let across = avail.saturating_sub(GAP * answers.saturating_sub(1)) / answers.max(1);
    let card = across.min(CARD_MAX_W);
    if card >= CARD_MIN_W
        && let Some(h) = HEIGHTS.into_iter().find(|h| top + h + 1 + BELOW <= rows)
    {
        return Plan::Cards { w: card, h };
    }
    let w = avail.min(CARD_MAX_W);
    if w >= MIN_COLS
        && let Some(h) = HEIGHTS.into_iter().find(|h| top + answers + h + 1 + BELOW <= rows)
    {
        return Plan::Preview { w, h };
    }
    Plan::List
}

impl Page {
    /// The page: the question asked, or the summary once they are all answered.
    pub(super) fn tailor(&mut self, w: &Welcome) -> Vec<Hint<'static>> {
        let hints = match w.tailoring() {
            Some(tailoring) => match tailoring.asking() {
                Some(step) => self.tailor_question(tailoring, step),
                None => self.tailor_summary(tailoring),
            },
            // The walk begins as the step is reached, so this is a page nobody sees; it still has its way on.
            None => {
                let row = self.heading(text(TAILOR_HEADING), text(TAILOR_WELCOME_NOTE));
                self.button(row, true, text(TAILOR_WELCOME_CONTINUE), 0);
                vec![("⏎", text(TAILOR_WELCOME_KEY_CONTINUE), Some(Key::Enter)), ESC_SKIP]
            }
        };
        // Where the answers reach nothing (an app that can't change its window, or can't be asked) the
        // page says so, in the row a tour's note takes.
        let last = self.size.rows.saturating_sub(1);
        if w.live() && !w.tailor_live() && last >= 2 && self.out(self.t, ROW_EVERY, 1) > 0 {
            let note = fit(text(TAILOR_WELCOME_UNAVAILABLE), self.text_width());
            self.grid.put(last - 1, MARGIN, &note, DIM);
        }
        hints
    }

    fn tailor_question(&mut self, tailoring: &Tailoring, step: Asked) -> Vec<Hint<'static>> {
        let question = step.question;
        let row = self.heading(text(TAILOR_HEADING), text(TAILOR_WELCOME_NOTE));
        // The question, and where the walk is at the right end of its row.
        let counter = tailor_counter(&step.number.to_string(), &step.of.to_string());
        let room = self.text_width().saturating_sub(text_width(&counter) + 2);
        self.shown(row, MARGIN, &[(&fit(text(question.title()), room), TITLE)]);
        let turn = self.turn;
        let edge = self.size.cols.saturating_sub(MARGIN);
        self.also(row, edge.saturating_sub(text_width(&counter)).max(MARGIN), &[(&counter, DIM)], turn);

        let top = row + 2;
        let count = question.answers().len();
        let cursor = tailoring.cursor();
        let after = match plan(self.text_width(), self.size.rows, top, count) {
            Plan::Cards { w, h } => self.cards(tailoring, question, top, w, h),
            Plan::Preview { w, h } => {
                let list_end = self.answer_list(tailoring, question, top);
                let shown = tailoring.highlighted().or_else(|| question.picked(&tailoring.walk().shown));
                let window = shown.map_or_else(
                    || tailoring.walk().shown.clone(),
                    |ix| question.preview(ix, &tailoring.walk().shown),
                );
                let at = self.detail(tailoring, question, list_end) + 1;
                self.sketch_at(sketch(&window, tailoring.offered(), w, h).as_ref(), at, MARGIN, DIM);
                at + h
            }
            Plan::List => {
                let list_end = self.answer_list(tailoring, question, top);
                self.detail(tailoring, question, list_end)
            }
        };
        // The button keeps things as they are, however far the walk got.
        let button = after + 1;
        self.button(button, cursor == count, text(TAILOR_WELCOME_CONTINUE), count);
        let detail_at = MARGIN + 2 + text_width(text(TAILOR_WELCOME_CONTINUE)) + 2;
        let detail = text(TAILOR_WELCOME_CONTINUE_DETAIL);
        let turn = self.turn;
        if button < self.size.rows.saturating_sub(2) && detail_at + text_width(detail) <= edge {
            self.also(button, detail_at, &[(detail, DIM)], turn);
        }
        question_hints(count, cursor < count)
    }

    /// The answers as cards side by side, each with its number and name under it; returns the row after
    /// the labels. A click anywhere on a card gives its answer.
    fn cards(&mut self, tailoring: &Tailoring, question: Question, top: usize, w: usize, h: usize) -> usize {
        let walk = tailoring.walk();
        let picked = question.picked(&walk.shown);
        let cursor = tailoring.cursor();
        let answers = question.answers();
        let sketches: Vec<Option<Sketch>> = (0..answers.len())
            .map(|ix| sketch(&question.preview(ix, &walk.shown), tailoring.offered(), w, h))
            .collect();
        for r in 0..h {
            for (ix, drawn) in sketches.iter().enumerate() {
                let col = MARGIN + ix * (w + GAP);
                let outline = outline(picked == Some(ix), cursor == ix);
                self.sketch_row(drawn.as_ref(), r, top + r, col, outline, ix == 0);
                self.hit(top + r, col, col + w, Target::Row(ix));
            }
        }
        let labels = top + h;
        for (ix, answer) in answers.iter().enumerate() {
            let col = MARGIN + ix * (w + GAP);
            let style = if picked == Some(ix) { ACCENT } else { Style::PLAIN };
            let label = fit(text(answer.label), w.saturating_sub(4));
            let marker = if cursor == ix { "❯" } else { " " };
            let number = (ix + 1).to_string();
            let parts = [(marker, ACCENT), (&number[..], TITLE), (" ", Style::PLAIN), (&label[..], style)];
            if ix == 0 {
                self.shown(labels, col, &parts);
            } else {
                let turn = self.turn;
                self.also(labels, col, &parts, turn);
            }
            self.hit(labels, col, col + w, Target::Row(ix));
        }
        // Each card says what its answer is, under its own label.
        let details: Vec<Vec<String>> =
            answers.iter().map(|answer| two_lines(text(answer.detail), w)).collect();
        for line in 0..DETAIL_ROWS {
            let mut first = true;
            for (ix, lines) in details.iter().enumerate() {
                let Some(words) = lines.get(line) else { continue };
                let col = MARGIN + ix * (w + GAP);
                if first {
                    self.shown(labels + 1 + line, col, &[(words, DIM)]);
                } else {
                    let turn = self.turn;
                    self.also(labels + 1 + line, col, &[(words, DIM)], turn);
                }
                first = false;
            }
        }
        labels + 1 + DETAIL_ROWS
    }

    /// The answers as lines: the marker, the number, the name, and a dot after the one in use. Returns
    /// the row after them.
    fn answer_list(&mut self, tailoring: &Tailoring, question: Question, top: usize) -> usize {
        let picked = question.picked(&tailoring.walk().shown);
        let cursor = tailoring.cursor();
        let width = self.text_width().saturating_sub(4);
        for (ix, answer) in question.answers().iter().enumerate() {
            let style = if picked == Some(ix) { ACCENT } else { Style::PLAIN };
            let number = (ix + 1).to_string();
            let label = fit(text(answer.label), width.saturating_sub(4));
            let mut parts = vec![(&number[..], TITLE), ("  ", Style::PLAIN), (&label[..], style)];
            if picked == Some(ix) {
                parts.push((" ●", ACCENT));
            }
            self.item_row(top + ix, MARGIN, cursor == ix, &parts, ix);
        }
        top + question.answers().len()
    }

    /// What the answer under the cursor (else the one in use) is, in a line or two under the answers.
    /// Returns the row after the lines it has room for, drawn or not.
    fn detail(&mut self, tailoring: &Tailoring, question: Question, row: usize) -> usize {
        if let Some(ix) = tailoring.highlighted() {
            for (line, words) in wrap(text(question.answers()[ix].detail), self.text_width())
                .into_iter()
                .take(DETAIL_ROWS)
                .enumerate()
            {
                self.shown(row + line, MARGIN, &[(&words, DIM)]);
            }
        }
        row + DETAIL_ROWS
    }

    // ---------------------------------------------------------------------------------- the summary

    fn tailor_summary(&mut self, tailoring: &Tailoring) -> Vec<Hint<'static>> {
        let row = self.heading(text(TAILOR_SUMMARY), "");
        let lines = tailoring.walk().summary();
        let width = self.text_width();
        let given = lines
            .iter()
            .map(|(_, answer)| text_width(answer.map_or(text(TAILOR_OWN), |answer| text(answer.label))))
            .max()
            .unwrap_or(0);
        let asked = lines.iter().map(|(question, _)| text_width(text(question.title()))).max().unwrap_or(0);
        let asked = asked.min(width.saturating_sub(given + 2)).max(8);
        for (line, (question, answer)) in lines.iter().enumerate() {
            let title = fit(text(question.title()), asked);
            let pad = " ".repeat(asked - text_width(&title) + 2);
            let (given, style) = match answer {
                Some(answer) => (text(answer.label), TITLE),
                None => (text(TAILOR_OWN), DIM),
            };
            self.shown(row + line, MARGIN, &[(&title, DIM), (&pad, Style::PLAIN), (given, style)]);
        }
        // The whole window as it is now: at the right of the lines when they leave it room, else under
        // them when the terminal is tall enough, else not at all.
        let window = sketch(&tailoring.walk().shown, tailoring.offered(), SUMMARY_W, SUMMARY_H);
        let right = self.size.cols.saturating_sub(MARGIN).saturating_sub(SUMMARY_W);
        let end = row + lines.len();
        let fits = |after: usize| after + 2 + BELOW_BUTTON <= self.size.rows;
        let beside = MARGIN + asked + 2 + given + 2 <= right && fits(end.max(row + SUMMARY_H));
        let end = if beside {
            self.sketch_at(window.as_ref(), row, right, GOOD);
            end.max(row + SUMMARY_H)
        } else if fits(end + 1 + SUMMARY_H) {
            self.sketch_at(window.as_ref(), end + 1, MARGIN, GOOD);
            end + 1 + SUMMARY_H
        } else {
            end
        };
        self.button(end + 1, true, text(TAILOR_LOOKS_GOOD), 0);
        vec![("⏎", text(TAILOR_WELCOME_KEY_CONTINUE), Some(Key::Enter)), BACK, start_over_hint(), ESC_SKIP]
    }

    // ---------------------------------------------------------------------------------- sketches

    /// A sketch from `row` at `col`, its outline `outline`. A sketch the room is too small for is not drawn.
    fn sketch_at(&mut self, sketch: Option<&Sketch>, row: usize, col: usize, outline: Style) {
        let Some(drawn) = sketch else { return };
        for r in 0..drawn.rows() {
            self.sketch_row(Some(drawn), r, row + r, col, outline, true);
        }
    }

    /// Row `r` of a sketch on screen row `at`. `first` says it is the first of the cards in this row,
    /// which takes the row's turn in the page's entrance; the rest come in with it.
    fn sketch_row(
        &mut self,
        sketch: Option<&Sketch>,
        r: usize,
        at: usize,
        col: usize,
        outline: Style,
        first: bool,
    ) {
        let Some(sketch) = sketch else { return };
        let runs = runs(sketch.row(r), outline);
        let parts: Vec<(&str, Style)> = runs.iter().map(|(text, style)| (text.as_str(), *style)).collect();
        if first {
            self.shown(at, col, &parts);
        } else {
            let turn = self.turn;
            self.also(at, col, &parts, turn);
        }
    }
}

/// `words` in at most two lines of `width` cells, an ellipsis ending the second if it has to be cut.
fn two_lines(words: &str, width: usize) -> Vec<String> {
    let mut lines = wrap(words, width);
    if lines.len() > DETAIL_ROWS {
        let rest = lines.split_off(DETAIL_ROWS - 1).join(" ");
        // The ellipsis follows the last word, not the space before it.
        lines.push(fit(&rest, width).replace(" …", "…"));
    }
    lines
}

/// How a card's outline is drawn: lit for the answer in use, bold under the cursor, else dim.
fn outline(picked: bool, cursor: bool) -> Style {
    match (picked, cursor) {
        (true, _) => ACCENT,
        (false, true) => TITLE,
        (false, false) => DIM,
    }
}

/// How each part of a sketch is drawn: the palette's colours only, so the theme applies.
fn style_of(part: Part, outline: Style) -> Style {
    match part {
        Part::Blank => Style::PLAIN,
        Part::Frame => outline,
        Part::Rule | Part::Row | Part::Mark | Part::Text | Part::Minimap | Part::Tab => DIM,
        Part::CurrentRow | Part::Crumb => TITLE,
        Part::Prompt => GOOD,
        Part::CurrentTab => ACCENT,
        Part::Status => Style { reverse: true, dim: true, ..Style::PLAIN },
    }
}

/// A row of a sketch as runs of text, a run being the cells in a row that are drawn alike.
fn runs(cells: &[(char, Part)], outline: Style) -> Vec<(String, Style)> {
    let mut out: Vec<(String, Style)> = Vec::new();
    for (ch, part) in cells {
        let style = style_of(*part, outline);
        match out.last_mut() {
            Some((text, last)) if *last == style => text.push(*ch),
            _ => out.push((ch.to_string(), style)),
        }
    }
    out
}

fn start_over_hint() -> Hint<'static> {
    ("r", text(TAILOR_WELCOME_KEY_START_OVER), Some(Key::Char('r')))
}

/// The keys along the bottom of a question: its numbers, the arrows, Return (which presses the answer
/// under the cursor, or goes on from the button), Back and Start over.
fn question_hints(answers: usize, on_answer: bool) -> Vec<Hint<'static>> {
    let numbers = if answers == 2 { "1-2" } else { "1-3" };
    let enter = if on_answer { text(TAILOR_WELCOME_KEY_PICK) } else { text(TAILOR_WELCOME_KEY_CONTINUE) };
    vec![
        (numbers, text(TAILOR_WELCOME_KEY_PICK), None),
        (UP_DOWN.0, text(TAILOR_WELCOME_KEY_MOVE), None),
        ("⏎", enter, Some(Key::Enter)),
        BACK,
        start_over_hint(),
        ESC_SKIP,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_detail_is_two_lines_at_most_and_the_second_ends_in_an_ellipsis_when_cut() {
        assert_eq!(two_lines("Wormhole as it comes", 24), ["Wormhole as it comes"]);
        assert_eq!(two_lines("Closed or open as you like it", 16), ["Closed or open", "as you like it"]);
        let cut =
            two_lines("The terminal list and the terminal: its name and the splits on top, nothing else", 24);
        assert_eq!(cut, ["The terminal list and", "the terminal: its name…"]);
    }

    #[test]
    fn three_cards_fit_a_normal_terminal_and_a_narrow_one_has_a_list() {
        assert_eq!(plan(76, 24, 6, 3), Plan::Cards { w: 24, h: 8 });
        assert_eq!(plan(120, 40, 6, 3), Plan::Cards { w: 30, h: 8 }, "no wider than a window shows");
        assert_eq!(plan(76, 24, 6, 2), Plan::Cards { w: 30, h: 8 });
        assert_eq!(plan(60, 24, 6, 3), Plan::Preview { w: 30, h: 7 }, "too narrow for three");
        assert_eq!(plan(76, 21, 6, 3), Plan::Cards { w: 24, h: 7 }, "a short window shrinks the cards");
        assert_eq!(plan(76, 20, 6, 3), Plan::Cards { w: 24, h: 6 });
        assert_eq!(plan(76, 19, 6, 3), Plan::List, "and then there is room for a list only");
        assert_eq!(plan(10, 24, 6, 3), Plan::List);
        assert_eq!(plan(76, 10, 6, 3), Plan::List);
    }
}
