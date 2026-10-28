//! The picker as data: the text, the answer being shown, the selection and the window over the list. It asks
//! for work (a query, decorations) as plain requests and takes the results back by generation, so an answer
//! for text that has moved on is dropped; it never runs anything itself.
//!
//! One query is asked of every provider at once and each answers when it can ([`Model::deliver`]): the list
//! is made of the answers that are in, so a provider that is slow holds back only its own rows. When some are
//! still out after the deadline the driver says so ([`Model::slow`]) and the list shows a note that it is
//! still reading, and can be picked from meanwhile.

use std::collections::{HashMap, HashSet};

use super::compose::merge;
use super::ghost::ghost;
use super::keys::{Do, Facts, Field, Key, PAGE, decide};
use super::shown::{Item, Reading, Shown};
use super::types::{Answer, Context, Crumb, Decoration, Query, Row};

/// The most keys one decoration request carries, so the first rows come back before the rest.
const BATCH: usize = 16;
/// The most rows decorated for one answer, however far the list is scrolled.
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

/// The query the model wants run, by each of its providers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub generation: u64,
    pub query: Query,
}

/// A row to find out more about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub key: String,
    /// What kind of row it is (`Row::kind`): a decorator is asked only about the kinds it knows.
    pub kind: String,
}

/// The rows the model wants decorated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecorationRequest {
    pub rows: Vec<Target>,
}

impl DecorationRequest {
    /// The keys, for a decorator that takes no interest in kinds.
    pub fn keys(&self) -> Vec<String> {
        self.rows.iter().map(|t| t.key.clone()).collect()
    }
}

/// Everything a panel needs to draw the picker. Only the lines in view are in it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub text: String,
    /// Changes when the model, not the person, changed the text: the panel then puts `text` in the field.
    pub epoch: u64,
    pub ghost: Option<String>,
    pub crumbs: Vec<Crumb>,
    /// The lines in view, from `top`.
    pub items: Vec<Item>,
    /// The first line in view, in the list of all of them.
    pub top: usize,
    /// How many lines the list has in all.
    pub lines: usize,
    /// The selected row's place in the list of all lines.
    pub selected: Option<usize>,
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
    /// What each provider answered to the current question.
    parts: Vec<Option<Answer>>,
    /// The text the answers shown are for.
    answered: String,
    /// Some providers are still out after the deadline.
    slow: bool,
    shown: Shown,
    selected: usize,
    top: usize,
    window: usize,
    pending: Option<Do>,
    cache: HashMap<String, Decoration>,
    /// Keys that were asked about and not answered yet, and those that were.
    asking: HashSet<String>,
    decorated: HashSet<String>,
    /// How many were asked about for the answer shown.
    asked_for_answer: usize,
}

impl Model {
    /// A picker with nothing typed, fed by `providers` of them, whose list shows `window` lines.
    pub fn new(context: Context, window: usize, providers: usize) -> Self {
        Self {
            context,
            text: String::new(),
            epoch: 0,
            generation: 0,
            asked: false,
            parts: vec![None; providers.max(1)],
            answered: String::new(),
            slow: false,
            shown: Shown::default(),
            selected: 0,
            top: 0,
            window: window.max(1),
            pending: None,
            cache: HashMap::new(),
            asking: HashSet::new(),
            decorated: HashSet::new(),
            asked_for_answer: 0,
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

    /// Every provider has answered the current question.
    pub fn settled(&self) -> bool {
        self.parts.iter().all(Option::is_some)
    }

    /// The list can be acted on: it is complete, or complete enough that waiting longer is the person's call.
    fn ready(&self) -> bool {
        self.settled() || self.slow
    }

    /// The text was edited by the person, in a field that showed the picker's text of `epoch`: an edit made
    /// before the picker replaced the text (a ⇥ still on its way) is dropped, so the two never disagree.
    pub fn typed_at(&mut self, epoch: u64, text: &str) {
        if epoch == self.epoch {
            self.typed(text);
        }
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

    /// The field shows the picker's text of this epoch.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    fn set(&mut self, text: &str) {
        self.text = text.to_string();
        self.generation += 1;
        self.asked = false;
        self.parts.iter_mut().for_each(|part| *part = None);
        self.slow = false;
        self.selected = 0;
        self.top = 0;
    }

    /// Ask again for the same text (a row was forgotten): the selection stays where it was.
    pub fn refresh(&mut self) {
        self.generation += 1;
        self.asked = false;
        self.parts.iter_mut().for_each(|part| *part = None);
        self.slow = false;
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

    /// The answer of provider `index` to the query of `generation`. One for text that has moved on is dropped
    /// (`None`); otherwise the list is made again of the answers that are in, and a ↩ or ⇥ that was waiting for
    /// a complete list is done now if this completes it: its outcome.
    pub fn deliver(&mut self, generation: u64, index: usize, answer: Answer) -> Option<Outcome> {
        if generation != self.generation || index >= self.parts.len() {
            return None;
        }
        self.parts[index] = Some(answer);
        self.show();
        if !self.ready() {
            return None;
        }
        let waiting = self.pending.take()?;
        Some(self.act(waiting))
    }

    /// The deadline of the query of `generation` passed with providers still out: the list says it is still
    /// reading, and a key that waited for it is done with what is in.
    pub fn slow(&mut self, generation: u64) -> Option<Outcome> {
        if generation != self.generation || self.settled() {
            return None;
        }
        self.slow = true;
        self.show();
        let waiting = self.pending.take()?;
        Some(self.act(waiting))
    }

    /// The list, made of the answers that are in.
    fn show(&mut self) {
        let merged = merge(self.parts.iter().flatten().cloned().collect());
        self.answered = self.text.clone();
        self.shown = Shown::of(
            merged,
            if self.settled() {
                Reading::Done
            } else if self.slow {
                Reading::Slow
            } else {
                Reading::Waiting
            },
        );
        for row in 0..self.shown.rows() {
            if let Some(found) = self.shown.row(row).and_then(|r| self.cache.get(&r.key)).cloned()
                && let Some(row) = self.shown.row_mut(row)
            {
                apply(row, &found);
            }
        }
        self.selected = self.selected.min(self.shown.rows().saturating_sub(1));
        self.asked_for_answer = 0;
        self.reveal();
    }

    /// The rows in view that nobody has asked about, a few of them and never more than the answer's budget.
    fn undecorated(&self) -> Vec<Target> {
        if self.shown.items.is_empty() || self.asked_for_answer >= DECORATED {
            return Vec::new();
        }
        let end = (self.top + self.window).min(self.shown.items.len());
        self.shown.items[self.top.min(end)..end]
            .iter()
            .filter_map(|item| match item {
                Item::Row(row) if !self.asking.contains(&row.key) && !self.decorated.contains(&row.key) => {
                    Some(Target { key: row.key.clone(), kind: row.kind.clone() })
                }
                _ => None,
            })
            .take(BATCH.min(DECORATED - self.asked_for_answer))
            .collect()
    }

    /// Whether [`decorations_wanted`](Self::decorations_wanted) has something to give.
    pub fn wants_decorations(&self) -> bool {
        !self.undecorated().is_empty()
    }

    /// The rows in view that nobody has asked about yet, a few at a time and never more than a budget for the
    /// answer shown. Asked about is not answered: the rows stay open to be asked again if the answer is lost
    /// ([`decorations_lost`](Self::decorations_lost)).
    pub fn decorations_wanted(&mut self) -> Option<DecorationRequest> {
        let rows = self.undecorated();
        if rows.is_empty() {
            return None;
        }
        self.asked_for_answer += rows.len();
        self.asking.extend(rows.iter().map(|t| t.key.clone()));
        Some(DecorationRequest { rows })
    }

    /// What decorators found for `request`'s rows: kept for the rows as they are listed now and whenever they
    /// are listed again, and the rows are done with.
    pub fn decorated(&mut self, request: &DecorationRequest, found: Vec<Decoration>) {
        if self.cache.len() >= CACHE {
            self.cache.clear();
            self.decorated.clear();
        }
        for decoration in found {
            let listed = (0..self.shown.rows())
                .find(|&at| self.shown.row(at).is_some_and(|row| row.key == decoration.key));
            if let Some(at) = listed
                && let Some(row) = self.shown.row_mut(at)
            {
                apply(row, &decoration);
            }
            // Two decorators may know different things about one row: what is kept is both.
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
        for target in &request.rows {
            self.asking.remove(&target.key);
            self.decorated.insert(target.key.clone());
        }
    }

    /// The answer to `request` did not come in time (a folder on a share that never answers): its rows are done
    /// with, with nothing found. They are not asked about again: asking is what costs a thread that stays
    /// blocked, and a chevron is not worth one every few seconds.
    pub fn decorations_lost(&mut self, request: &DecorationRequest) {
        for target in &request.rows {
            self.asking.remove(&target.key);
            self.decorated.insert(target.key.clone());
        }
    }

    pub fn rows(&self) -> usize {
        self.shown.rows()
    }

    /// Every line of the list (headers, rows and notes), as they are now.
    pub fn items(&self) -> &[Item] {
        &self.shown.items
    }

    /// How many lines the list has.
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
            // Only for the text the answer is for: a text typed since says nothing about where it ends.
            up_at_end: self.shown.up_at_end && self.answered == self.text,
            descends: row.is_some_and(|r| r.descend.is_some()),
            ghost: self.ghost().is_some(),
            forgettable: row.is_some_and(|r| r.forgettable),
        }
    }

    /// The grey completion: the rest of the selected row's text past what is typed.
    pub fn ghost(&self) -> Option<String> {
        ghost(&self.text, self.selected_row()?.descend.as_deref()?)
    }

    /// What `key` does with the field as it is: see [`decide`]. A panel that must answer before the key returns
    /// asks it with the facts it has, and tells the model what it decided ([`perform`](Self::perform)).
    pub fn decide(&self, key: Key, field: &Field) -> Do {
        decide(key, field, &self.facts())
    }

    /// A key, with the field as it is, decided and done here.
    pub fn key(&mut self, key: Key, field: &Field) -> Outcome {
        let what = self.decide(key, field);
        self.perform(what)
    }

    /// Do what a key was decided to do. ↩ and ⇥ wait for the list to be the one for what is typed, so ⇥ at once
    /// after a letter acts on the answer to that letter, not on the last one's.
    pub fn perform(&mut self, what: Do) -> Outcome {
        match what {
            Do::Pass => Outcome::Pass,
            Do::Close => Outcome::Close,
            waiting @ (Do::Pick | Do::Descend) if !self.ready() => {
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

    /// What a panel draws: only the lines in view, so a long list costs a few clones, not all of them.
    pub fn snapshot(&self) -> Snapshot {
        let end = (self.top + self.window).min(self.shown.items.len());
        let top = self.top.min(end);
        Snapshot {
            text: self.text.clone(),
            epoch: self.epoch,
            ghost: self.ghost(),
            crumbs: self.shown.crumbs.clone(),
            items: self.shown.items[top..end].to_vec(),
            top,
            lines: self.shown.items.len(),
            selected: self.shown.row_items.get(self.selected).copied(),
            facts: self.facts(),
            busy: !self.settled() && !self.slow,
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
