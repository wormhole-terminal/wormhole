//! Walking vttest's menus: choose each entry in turn, capture the screen at every `Push <RETURN>`,
//! press return, and come back out.
//!
//! vttest is driven only by what it prints. A menu is recognised by its prompt and its entries are
//! read off the screen, so an entry vttest adds is walked without a change here; one that needs a
//! person is named in `plan.rs` with the reason, and shows up in the run's notes.

use std::path::Path;
use std::time::Duration;

use crate::capture::{self, Meta};
use crate::menu::{self, Entry};
use crate::plan::{self, Answer};
use crate::session::{Session, State};

/// How many silent screens in a row the walk presses return on before it gives the entry up.
const MAX_WAITS: usize = 3;

/// A menu we are inside of.
struct Level {
    /// How we know we are back at it.
    fingerprint: String,
    entries: Vec<Entry>,
    /// The order to run them in, as indexes of `entries`: each once in order unless the plan says
    /// otherwise.
    order: Vec<usize>,
    next: usize,
    /// The entry running now.
    chosen: Option<Entry>,
}

/// What a walk produced.
#[derive(Default)]
pub struct Walked {
    /// The golden text of each screen, in the order vttest drew them. Its header says `screen @@`
    /// where the number goes: the caller numbers the screens of all the parts of a menu together.
    pub screens: Vec<String>,
    /// What was not run or went wrong, one line each.
    pub notes: Vec<String>,
    /// How long the walk took.
    pub took: Duration,
}

struct Walker<'a> {
    session: Session,
    version: &'a str,
    /// The menu numbers of the part of vttest's menus this walk is for: it goes down this way only.
    unit: &'a [u32],
    dir: String,
    stack: Vec<Level>,
    run: Walked,
    /// How many of the running entry's answers have been typed.
    answered: usize,
    /// The main menu has been left: vttest is saying goodbye and ending, nothing more to walk.
    leaving: bool,
}

/// Walk the part of vttest's menus at `unit` (menu numbers from the main menu: `[11, 3]` is the VT420
/// menu), from a fresh vttest.
pub fn walk(unit: &[u32], vttest: &Path, relay: &Path, version: &str, trace: bool) -> Walked {
    let started = std::time::Instant::now();
    let mut run = Walked::default();
    let name = unit.iter().map(u32::to_string).collect::<Vec<_>>().join("-");
    let scratch = std::env::temp_dir().join(format!("vttest-host-{}-{name}", std::process::id()));
    let session = match Session::start(vttest, relay, &scratch, trace) {
        Ok(session) => session,
        Err(message) => {
            run.notes.push(format!("could not start vttest: {message}"));
            return run;
        }
    };
    let dir = plan::menu_dir(unit.first().copied().unwrap_or(0));
    let mut walker =
        Walker { session, version, unit, dir, stack: Vec::new(), run, answered: 0, leaving: false };
    walker.walk();
    let _ = std::fs::remove_dir_all(&scratch);
    walker.run.took = started.elapsed();
    walker.run
}

impl Walker<'_> {
    fn walk(&mut self) {
        let mut waits = 0;
        loop {
            // vttest ends a moment after its goodbye; a loaded machine may be slow to say so.
            let quiet = if self.leaving { Duration::from_secs(10) } else { plan::quiet(&self.numbers()) };
            let state = self.session.wait(quiet);
            if state != State::Quiet {
                waits = 0;
            }
            match state {
                State::Hold => {
                    self.capture(None);
                    self.answered = 0;
                    self.session.send("\n");
                }
                State::Menu => {
                    if !self.on_menu() {
                        break;
                    }
                }
                State::BadChoice => {
                    self.note(format!("{}: vttest did not accept the choice", self.path()));
                    self.session.send("0\n");
                }
                State::Quiet if self.leaving => break,
                State::Quiet => {
                    self.capture(Some("silent without a Push <RETURN>, waiting for a key"));
                    if self.answer() {
                        continue;
                    }
                    waits += 1;
                    self.note(format!("{}: silent without a prompt, return pressed", self.path()));
                    if waits >= MAX_WAITS {
                        self.note(format!(
                            "{}: still silent after {MAX_WAITS} returns; the walk stops",
                            self.path()
                        ));
                        break;
                    }
                    self.session.send("\n");
                }
                State::Closed => break,
                State::TimedOut => {
                    self.note(format!("{}: the walk passed its time limit", self.path()));
                    break;
                }
            }
        }
    }

    /// Type the next answer the plan has for the running entry. False when it has none left.
    fn answer(&mut self) -> bool {
        let answers = plan::answers(&self.numbers());
        let Some(answer) = answers.get(self.answered) else { return false };
        self.answered += 1;
        match answer {
            Answer::Key(key) => self.session.press(*key),
            Answer::Text(text) => self.session.send(text),
        }
        true
    }

    fn note(&mut self, text: String) {
        self.run.notes.push(text);
    }

    /// The menu numbers chosen so far, outermost first.
    fn numbers(&self) -> Vec<u32> {
        self.stack.iter().filter_map(|l| l.chosen.as_ref()).map(|e| e.number).collect()
    }

    /// The choices so far with the names vttest gives them: `3 Test of character sets > 8 ...`.
    fn path(&self) -> String {
        let parts: Vec<String> = self
            .stack
            .iter()
            .filter_map(|l| l.chosen.as_ref())
            .map(|e| format!("{} {}", e.number, e.name))
            .collect();
        if parts.is_empty() { "(main menu)".into() } else { parts.join(" > ") }
    }

    /// Keep the screen as the next golden file.
    fn capture(&mut self, state: Option<&str>) {
        let path = self.path();
        let meta = Meta { version: self.version, menu: &self.dir, path: &path, state };
        let text = capture::render(self.session.term(), &meta);
        self.run.screens.push(text);
    }

    /// A menu prompt: enter the next entry of the menu we are in, or leave it. False when the walk
    /// cannot go on.
    fn on_menu(&mut self) -> bool {
        let rows = self.session.rows();
        let fingerprint = menu::fingerprint(&rows);
        // A menu we are already inside of (a test or a one-shot submenu has returned to it), or a new one.
        if let Some(at) = self.stack.iter().rposition(|l| l.fingerprint == fingerprint) {
            self.stack.truncate(at + 1);
            if let Some(level) = self.stack.last_mut() {
                level.chosen = None;
            }
        } else {
            let entries = self.read_entries(&rows);
            if self.stack.is_empty() && !entries.iter().any(|e| Some(&e.number) == self.unit.first()) {
                self.note(format!("the main menu has no entry {:?}", self.unit));
                return false;
            }
            let path = self.numbers();
            let order = self.order(&path, &entries);
            self.note_left_out(&path, &entries, &order);
            self.stack.push(Level { fingerprint, entries, order, next: 0, chosen: None });
        }
        loop {
            let Some(level) = self.stack.last_mut() else { return true };
            let Some(&index) = level.order.get(level.next) else {
                // Done here: leave. At the main menu that ends vttest, and the next wait sees it close.
                self.stack.pop();
                self.leaving = self.stack.is_empty();
                self.session.send("0\n");
                return true;
            };
            level.next += 1;
            let entry = level.entries[index].clone();
            let mut numbers = self.numbers();
            numbers.push(entry.number);
            // Off the way to this walk's part: another walk has it.
            if !(self.unit.starts_with(&numbers) || numbers.starts_with(self.unit)) {
                continue;
            }
            let at = numbers.iter().map(u32::to_string).collect::<Vec<_>>().join(".");
            if entry.unimplemented {
                self.note(format!("{at} {}: vttest has not written it", entry.name));
                continue;
            }
            if let Some(reason) = plan::skip(&numbers, &entry.name) {
                self.note(format!("{at} {}: not run, {reason}", entry.name));
                continue;
            }
            self.answered = 0;
            if let Some(level) = self.stack.last_mut() {
                level.chosen = Some(entry.clone());
            }
            self.session.send(&format!("{}\n", entry.number));
            return true;
        }
    }

    /// The order to run a new menu's entries in, as indexes: the plan's, or each once.
    fn order(&self, path: &[u32], entries: &[Entry]) -> Vec<usize> {
        if let Some(numbers) = plan::order(path) {
            return numbers.iter().filter_map(|n| entries.iter().position(|e| e.number == *n)).collect();
        }
        (0..entries.len()).collect()
    }

    /// Say which entries a menu with its own order leaves out, and why.
    fn note_left_out(&mut self, path: &[u32], entries: &[Entry], order: &[usize]) {
        if plan::order(path).is_none() {
            return;
        }
        for (index, entry) in entries.iter().enumerate() {
            let mut numbers = path.to_vec();
            numbers.push(entry.number);
            if order.contains(&index)
                || entry.unimplemented
                || !(self.unit.starts_with(&numbers) || numbers.starts_with(self.unit))
            {
                continue;
            }
            let at = numbers.iter().map(u32::to_string).collect::<Vec<_>>().join(".");
            let reason = plan::skip(&numbers, &entry.name).unwrap_or(
                "it switches a setting that this menu's order in plan.rs leaves as vttest starts it",
            );
            self.note(format!("{at} {}: not run, {reason}", entry.name));
        }
    }

    /// The entries of the menu on screen, over its pages when it has more than one.
    fn read_entries(&mut self, rows: &[String]) -> Vec<Entry> {
        let quiet = Duration::from_millis(1500);
        let (mut entries, highest) = menu::parse(rows);
        let mut pages = 0;
        while entries.iter().map(|e| e.number).max().unwrap_or(0) < highest && pages < 6 {
            pages += 1;
            self.session.send("n\n");
            if self.session.wait(quiet) != State::Menu {
                break;
            }
            for entry in menu::parse(&self.session.rows()).0 {
                if !entries.iter().any(|e| e.number == entry.number) {
                    entries.push(entry);
                }
            }
        }
        // Back to the first page, which is how the menu is recognised when a test returns to it.
        for _ in 0..pages {
            self.session.send("p\n");
            if self.session.wait(quiet) != State::Menu {
                break;
            }
        }
        entries.retain(|e| e.number != 0);
        entries.sort_by_key(|e| e.number);
        entries
    }
}
