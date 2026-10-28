//! The picker as data: the text, the answer being shown, the selection and the window over the list. It asks
//! for work (a query, decorations) as plain requests and takes the results back by generation, so an answer
//! for text that has moved on is dropped; it never runs anything itself.

use std::collections::{HashMap, HashSet};

use super::ghost::ghost;
use super::keys::{Do, Facts, Field, Key, PAGE, decide};
use super::shown::{Item, Shown};
use super::types::{Answer, Context, Crumb, Decoration, Query, Row};

/// The most keys one decoration request carries, so the first rows come back before the rest.
const BATCH: usize = 16;
/// The most rows decorated in one picker's life, however far the list is scrolled.
const DECORATED: usize = 300;
/// The most decorations kept.
const CACHE: usize = 1_000;

/// What a key came to, for whoever opened the picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing to do (the key was taken, or is waiting for the list to settle).
    Nothing,
    /// The key is the field's own: it is left to do what it does.
    Pass,
    Close,
    /// Hand this to whoever opened the picker; the picker closes.
    Insert(String),
    /// The field's text is now this (the panel puts it there, the caret at its end).
    SetText(String),
    /// Forget the row with this key.
    Forget(String),
}

/// The query the model wants run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub generation: u64,
    pub query: Query,
}

/// The rows the model wants decorated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecorationRequest {
    pub generation: u64,
    pub keys: Vec<String>,
}

/// Everything a panel needs to draw the picker.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub text: String,
    /// Changes when the model, not the person, changed the text: the panel then puts `text` in the field.
    pub epoch: u64,
    pub ghost: Option<String>,
    pub crumbs: Vec<Crumb>,
    pub items: Vec<Item>,
    /// The selected row's place in `items`.
    pub selected: Option<usize>,
    /// The first item in view, and how many items are in view.
    pub top: usize,
    pub shown: usize,
    pub facts: Facts,
    /// An answer for the current text is on its way.
    pub busy: bool,
}

pub struct Model {
    context: Context,
    text: String,
    epoch: u64,
    generation: u64,
    asked: bool,
    settled: bool,
    shown: Shown,
    selected: usize,
    top: usize,
    window: usize,
    pending: Option<Do>,
    cache: HashMap<String, Decoration>,
    requested: HashSet<String>,
}

impl Model {
    /// A picker with nothing typed whose list shows `window` lines.
    pub fn new(context: Context, window: usize) -> Self {
        Self {
            context,
            text: String::new(),
            epoch: 0,
            generation: 0,
            asked: false,
            settled: false,
            shown: Shown::default(),
            selected: 0,
            top: 0,
            window: window.max(1),
            pending: None,
            cache: HashMap::new(),
            requested: HashSet::new(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Which question is being asked: it changes with the text, and an answer to an earlier one is dropped.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The current question has not been handed out yet ([`request`](Self::request) would give it).
    pub fn wants_query(&self) -> bool {
        !self.asked
    }

    /// An answer for the current text is in.
    pub fn settled(&self) -> bool {
        self.settled
    }

    /// The text was edited by the person.
    pub fn typed(&mut self, text: &str) {
        if text != self.text {
            self.set(text);
        }
    }

    /// The text is replaced by the picker itself (a completion, a step up, a crumb), or by a script: the
    /// panel is told to put it in the field.
    pub fn replace_text(&mut self, text: &str) {
        self.epoch += 1;
        if text != self.text {
            self.set(text);
        }
    }

    fn set(&mut self, text: &str) {
        self.text = text.to_string();
        self.generation += 1;
        self.asked = false;
        self.settled = false;
        self.selected = 0;
        self.top = 0;
    }

    /// Ask again for the same text (a row was forgotten): the selection stays where it was.
    pub fn refresh(&mut self) {
        self.generation += 1;
        self.asked = false;
        self.settled = false;
    }

    /// How many lines the list shows: fewer in a small pane.
    pub fn set_window(&mut self, window: usize) {
        self.window = window.max(1);
        self.reveal();
    }

    /// The query to run now, once for each text.
    pub fn request(&mut self) -> Option<Request> {
        if self.asked {
            return None;
        }
        self.asked = true;
        Some(Request {
            generation: self.generation,
            query: Query { text: self.text.clone(), context: self.context.clone() },
        })
    }

    /// The answer to the query of `generation`. One for text that has moved on is dropped (`None`); otherwise
    /// it is shown, and a ↩ or ⇥ that was waiting for it is done now: its outcome.
    pub fn deliver(&mut self, generation: u64, answer: Answer) -> Option<Outcome> {
        if generation != self.generation {
            return None;
        }
        self.shown = Shown::of(answer);
        for row in 0..self.shown.rows() {
            if let Some(found) = self.shown.row(row).and_then(|r| self.cache.get(&r.key)).cloned()
                && let Some(row) = self.shown.row_mut(row)
            {
                apply(row, &found);
            }
        }
        self.selected = self.selected.min(self.shown.rows().saturating_sub(1));
        self.settled = true;
        self.reveal();
        let waiting = self.pending.take()?;
        Some(self.act(waiting))
    }

    /// The rows in view that nobody has asked about yet, a few at a time and never more than a budget.
    pub fn decorations_wanted(&mut self) -> Option<DecorationRequest> {
        if !self.settled || self.requested.len() >= DECORATED {
            return None;
        }
        let end = (self.top + self.window).min(self.shown.items.len());
        let keys: Vec<String> = self.shown.items[self.top.min(end)..end]
            .iter()
            .filter_map(|item| match item {
                Item::Row(row) if !self.requested.contains(&row.key) => Some(row.key.clone()),
                _ => None,
            })
            .take(BATCH.min(DECORATED - self.requested.len()))
            .collect();
        if keys.is_empty() {
            return None;
        }
        self.requested.extend(keys.iter().cloned());
        Some(DecorationRequest { generation: self.generation, keys })
    }

    /// What decorators found, for the rows that are listed (kept for when they are listed again).
    pub fn decorated(&mut self, found: Vec<Decoration>) {
        for decoration in found {
            let listed = (0..self.shown.rows())
                .find(|&at| self.shown.row(at).is_some_and(|row| row.key == decoration.key));
            if let Some(at) = listed
                && let Some(row) = self.shown.row_mut(at)
            {
                apply(row, &decoration);
            }
            // Two decorators may know different things about one row: what is kept is both.
            if self.cache.len() < CACHE || self.cache.contains_key(&decoration.key) {
                let slot = self
                    .cache
                    .entry(decoration.key.clone())
                    .or_insert_with(|| Decoration { key: decoration.key.clone(), ..Decoration::default() });
                if decoration.badge.is_some() {
                    slot.badge = decoration.badge;
                }
                if decoration.chevron.is_some() {
                    slot.chevron = decoration.chevron;
                }
            }
        }
    }

    pub fn rows(&self) -> usize {
        self.shown.rows()
    }

    /// How many lines the list has (headers, rows and notes).
    pub fn lines(&self) -> usize {
        self.shown.items.len()
    }

    /// Whether there is a breadcrumb to show.
    pub fn has_crumbs(&self) -> bool {
        !self.shown.crumbs.is_empty()
    }

    pub fn selected_row(&self) -> Option<&Row> {
        self.shown.row(self.selected)
    }

    /// Move the selection `by` rows, wrapping round the ends.
    pub fn step(&mut self, by: isize) {
        let n = self.shown.rows() as isize;
        if n == 0 {
            return;
        }
        let page = by.abs() >= PAGE;
        let next = self.selected as isize + by;
        // A page key stops at the ends; the arrows wrap.
        self.selected = if page { next.clamp(0, n - 1) } else { next.rem_euclid(n) } as usize;
        self.reveal();
    }

    /// Select the row at `item` in the list, if it is a row.
    pub fn select_item(&mut self, item: usize) -> bool {
        match self.shown.row_items.iter().position(|&at| at == item) {
            Some(row) => {
                self.selected = row;
                self.reveal();
                true
            }
            None => false,
        }
    }

    /// Scroll the list `by` lines (the selection stays where it is, so it may leave the view).
    pub fn scroll(&mut self, by: isize) {
        let last = self.shown.items.len().saturating_sub(self.window);
        self.top = (self.top as isize + by).clamp(0, last as isize) as usize;
    }

    /// Bring the selected row into view, with its section's header when it is the first of one.
    fn reveal(&mut self) {
        let last = self.shown.items.len().saturating_sub(self.window);
        let Some(&at) = self.shown.row_items.get(self.selected) else {
            self.top = self.top.min(last);
            return;
        };
        let at_header =
            if at > 0 && matches!(self.shown.items[at - 1], Item::Header(_)) { at - 1 } else { at };
        if at_header < self.top {
            self.top = at_header;
        } else if at >= self.top + self.window {
            self.top = at + 1 - self.window;
        }
        self.top = self.top.min(last);
    }

    /// What the keys depend on, for a panel that must answer before the key returns.
    pub fn facts(&self) -> Facts {
        let row = self.selected_row();
        Facts {
            up: self.shown.up.is_some(),
            descends: row.is_some_and(|r| r.descend.is_some()),
            ghost: self.ghost().is_some(),
            forgettable: row.is_some_and(|r| r.forgettable),
        }
    }

    /// The grey completion: the rest of the selected row's text past what is typed.
    pub fn ghost(&self) -> Option<String> {
        ghost(&self.text, self.selected_row()?.descend.as_deref()?)
    }

    /// A key, with the field as it is. See [`decide`] for what each does.
    pub fn key(&mut self, key: Key, field: &Field) -> Outcome {
        match decide(key, field, &self.facts()) {
            Do::Pass => Outcome::Pass,
            Do::Close => Outcome::Close,
            Do::Step(by) => {
                self.step(by);
                Outcome::Nothing
            }
            // Waits for the list to be the one for what is typed: ⇥ at once after a letter acts on the answer
            // to that letter, not on the last one's.
            waiting @ (Do::Pick | Do::Descend) if !self.settled => {
                self.pending = Some(waiting);
                Outcome::Nothing
            }
            other => self.act(other),
        }
    }

    /// A click on the row at `item`: it is picked.
    pub fn click(&mut self, item: usize) -> Outcome {
        if self.select_item(item) { self.act(Do::Pick) } else { Outcome::Nothing }
    }

    /// A click on a step of the breadcrumb.
    pub fn crumb(&mut self, at: usize) -> Outcome {
        match self.shown.crumbs.get(at).map(|c| c.text.clone()) {
            Some(text) => self.go(text),
            None => Outcome::Nothing,
        }
    }

    fn go(&mut self, text: String) -> Outcome {
        self.replace_text(&text);
        Outcome::SetText(text)
    }

    fn act(&mut self, what: Do) -> Outcome {
        match what {
            Do::Pick => match self.selected_row().and_then(|r| r.insert.clone()) {
                Some(text) => Outcome::Insert(text),
                None => Outcome::Nothing,
            },
            Do::Descend => match self.selected_row().and_then(|r| r.descend.clone()) {
                Some(text) => self.go(text),
                None => Outcome::Nothing,
            },
            Do::Up => match self.shown.up.clone() {
                Some(text) => self.go(text),
                None => Outcome::Nothing,
            },
            Do::Forget => match self.selected_row().filter(|r| r.forgettable) {
                Some(row) => Outcome::Forget(row.key.clone()),
                None => Outcome::Nothing,
            },
            Do::Pass => Outcome::Pass,
            Do::Close => Outcome::Close,
            Do::Step(by) => {
                self.step(by);
                Outcome::Nothing
            }
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            epoch: self.epoch,
            ghost: self.ghost(),
            crumbs: self.shown.crumbs.clone(),
            items: self.shown.items.clone(),
            selected: self.shown.row_items.get(self.selected).copied(),
            top: self.top,
            shown: self.window.min(self.shown.items.len()),
            facts: self.facts(),
            busy: !self.settled,
        }
    }
}

fn apply(row: &mut Row, found: &Decoration) {
    if found.badge.is_some() {
        row.badge.clone_from(&found.badge);
    }
    if let Some(chevron) = found.chevron {
        row.chevron = chevron;
    }
}
