//! The generic picker, run: core's [`Model`] (`view::picker`) with its providers asked off the main thread and
//! the native panel (`platform::picker`) showing it. It knows nothing of what is listed and nothing of any
//! terminal: who opens it says where it goes (an anchor), which words it wears and which providers feed it,
//! and hears what came of it (a pick, a forget, a close) as [`PickerEvent`]s. `crate::folder_picker` is the
//! one that opens it for a terminal.
//!
//! What happens, in one line: the panel reports (text, key, click, wheel) into a queue; a task reads it and
//! lets the model decide; the model asks for a query, which runs on a pool thread, one disk job at a time for
//! the whole app (a share that stopped answering holds one thread, not one for each keystroke); the answer
//! comes back by generation, so text that has moved on drops it; the model's snapshot is handed to the panel.
//!
//! Without a panel (a test window, a system that has none) all of it still runs: [`Picker::feed`] takes the
//! events the panel would have sent, which is how the tests and devctl drive it.

use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::panes::Rect;
use wormhole_core::view::picker::{
    Answer, Cancel, Context as Where, Decoration, DecorationRequest, Decorator, Item, Model, Note, Outcome,
    Placed, Provider, Request, place,
};

use crate::platform::picker::{Event, Line, MAX_ROWS, Panel, View, card};
use crate::settings::store_in;

#[cfg(feature = "devctl")]
mod devctl;

#[cfg(feature = "devctl")]
pub(crate) use devctl::summary;

/// What came of the picker, for whoever opened it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PickerEvent {
    /// A row was picked: this is what it hands over. The picker is done.
    Insert(String),
    /// The person asked to forget the row with this key.
    Forget(String),
    /// The picker is done without a pick: Esc, a click away.
    Close,
}

/// The words the panel wears (the catalog's, found by whoever opens the picker).
#[derive(Clone)]
pub(crate) struct Words {
    pub placeholder: String,
    /// What VoiceOver calls the panel and its list.
    pub name: String,
    pub hint: String,
    /// What a note under the rows says.
    pub note: fn(Note) -> String,
}

/// Where the picker is anchored: the pane it is over and the cursor's cell in it, in the window's space.
pub(crate) type Anchor = Rc<dyn Fn(&Window, &App) -> (Rect, Rect)>;

/// What feeds the picker.
#[derive(Clone)]
pub(crate) struct Sources {
    pub provider: Arc<dyn Provider>,
    pub decorators: Vec<Arc<dyn Decorator>>,
}

/// The picker as an entity of the window. It draws nothing in GPUI: the panel is the system's.
pub(crate) struct Picker {
    pub(crate) model: Model,
    sources: Sources,
    words: Words,
    profile: ProfileId,
    anchor: Anchor,
    /// The pane and the cursor as last measured, so a result that arrives between frames places the panel.
    spot: (Rect, Rect),
    panel: Option<Panel>,
    shown: bool,
    placed: Option<Placed>,
    /// The token of the disk job that is running, cancelled when the text moves on.
    cancel: Cancel,
    working: bool,
    worker: Option<Task<()>>,
    /// An answer has come: from then on a new query waits for the text to rest.
    answered: bool,
    _tasks: Vec<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PickerEvent> for Picker {}

/// One disk job at a time for the whole app: a read of a share that has stopped answering never comes back, and
/// must not be joined by another thread for each key. While one runs the next waits for it.
static READING: AtomicBool = AtomicBool::new(false);

struct Reading;

impl Reading {
    fn take() -> Option<Self> {
        (!READING.swap(true, Ordering::AcqRel)).then_some(Reading)
    }
}

impl Drop for Reading {
    fn drop(&mut self) {
        READING.store(false, Ordering::Release);
    }
}

/// How long the text must rest before a query is run again for it (the first one is run at once).
const PAUSE: Duration = Duration::from_millis(60);
/// How often a job that waits for another to end looks again.
const WAIT: Duration = Duration::from_millis(40);

enum Job {
    Query(Request),
    Decorate(DecorationRequest),
}

enum Done {
    Answer(u64, Answer),
    Decorated(Vec<Decoration>),
}

struct Work {
    job: Job,
    cancel: Cancel,
    sources: Sources,
}

impl Work {
    fn run(self) -> Done {
        match self.job {
            Job::Query(Request { generation, query }) => {
                Done::Answer(generation, self.sources.provider.query(&query, &self.cancel))
            }
            Job::Decorate(DecorationRequest { keys, .. }) => Done::Decorated(
                self.sources.decorators.iter().flat_map(|d| d.decorate(&keys, &self.cancel)).collect(),
            ),
        }
    }
}

impl Picker {
    /// A picker over `context`, fed by `sources`, anchored where `anchor` says, in `window`'s profile `profile`.
    /// It is up as soon as this returns: its first query is on its way.
    pub(crate) fn new(
        context: Where,
        sources: Sources,
        words: Words,
        profile: ProfileId,
        anchor: Anchor,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (tx, rx) = smol::channel::unbounded::<Event>();
        let opaque = store_in(&profile, cx).system().opaque();
        let panel = Panel::new(window, opaque, tx);
        let mut tasks = Vec::new();
        if panel.is_some() {
            tasks.push(cx.spawn(async move |this, cx| {
                while let Ok(event) = rx.recv().await {
                    if this.update(cx, |this, cx| this.feed(event, cx)).is_err() {
                        break;
                    }
                }
            }));
        }
        let moved = cx.observe_window_bounds(window, |this, window, cx| this.reanchor(window, cx));
        let spot = anchor(window, cx);
        let mut this = Self {
            model: Model::new(context, MAX_ROWS),
            sources,
            words,
            profile,
            anchor,
            spot,
            panel,
            shown: false,
            placed: None,
            cancel: Cancel::new(),
            working: false,
            worker: None,
            answered: false,
            _tasks: tasks,
            _subscriptions: vec![moved],
        };
        this.run_wanted(cx);
        this.push(cx);
        this
    }

    /// The window moved or changed size, or the terminal under the panel did: measure the anchor again.
    pub(crate) fn reanchor(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.spot = (self.anchor)(window, cx);
        self.push(cx);
    }

    /// What the person did in the panel (or what a test says they did).
    pub(crate) fn feed(&mut self, event: Event, cx: &mut Context<Self>) {
        let before = self.model.generation();
        let outcome = match event {
            Event::Typed(field) => {
                self.model.typed(&field.text);
                Outcome::Nothing
            }
            Event::Key(key, field) => self.model.key(key, &field),
            Event::Row(at) => self.model.click(at),
            Event::Crumb(at) => self.model.crumb(at),
            Event::Scroll(by) => {
                self.model.scroll(by);
                Outcome::Nothing
            }
            Event::Resigned => Outcome::Close,
        };
        self.apply(outcome, cx);
        // Text that moved on makes the running job's answer worthless: it is told to stop early.
        if self.model.generation() != before {
            self.cancel.cancel();
        }
        self.run_wanted(cx);
        self.push(cx);
        cx.notify();
    }

    /// The text is `text`, put there by a script (devctl): the field in the panel shows it too.
    #[cfg(feature = "devctl")]
    pub(crate) fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let before = self.model.generation();
        self.model.replace_text(text);
        if self.model.generation() != before {
            self.cancel.cancel();
        }
        self.run_wanted(cx);
        self.push(cx);
        cx.notify();
    }

    /// Ask the model again for the same text (a row was forgotten).
    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        self.model.refresh();
        self.cancel.cancel();
        self.run_wanted(cx);
        self.push(cx);
        cx.notify();
    }

    fn apply(&mut self, outcome: Outcome, cx: &mut Context<Self>) {
        match outcome {
            Outcome::Insert(text) => cx.emit(PickerEvent::Insert(text)),
            Outcome::Close => cx.emit(PickerEvent::Close),
            Outcome::Forget(key) => cx.emit(PickerEvent::Forget(key)),
            Outcome::Nothing | Outcome::Pass | Outcome::SetText(_) => {}
        }
    }

    /// Start the worker if there is work and none is running. It runs jobs until there are none: a query, then
    /// decorations for the rows in view, and again for whatever the text or the scrolling asked meanwhile.
    fn run_wanted(&mut self, cx: &mut Context<Self>) {
        if self.working {
            return;
        }
        self.working = true;
        self.worker = Some(cx.spawn(async move |this, cx| {
            loop {
                let pause = this.update(cx, |this, _| this.pause()).unwrap_or_default();
                if !pause.is_zero() {
                    cx.background_executor().timer(pause).await;
                }
                let Some(work) = this.update(cx, |this, _| this.next_work()).ok().flatten() else { break };
                let right = loop {
                    if let Some(right) = Reading::take() {
                        break right;
                    }
                    cx.background_executor().timer(WAIT).await;
                };
                let done = cx
                    .background_spawn(async move {
                        let _right = right;
                        work.run()
                    })
                    .await;
                if this.update(cx, |this, cx| this.finish(done, cx)).is_err() {
                    break;
                }
            }
        }));
    }

    /// How long to wait before the next job: the text must rest before a query that follows an answer.
    fn pause(&self) -> Duration {
        if self.answered && self.model.wants_query() { PAUSE } else { Duration::ZERO }
    }

    fn next_work(&mut self) -> Option<Work> {
        let job = if let Some(request) = self.model.request() {
            Job::Query(request)
        } else if let Some(request) = self.model.decorations_wanted() {
            Job::Decorate(request)
        } else {
            self.working = false;
            return None;
        };
        self.cancel = Cancel::new();
        Some(Work { job, cancel: self.cancel.clone(), sources: self.sources.clone() })
    }

    fn finish(&mut self, done: Done, cx: &mut Context<Self>) {
        match done {
            Done::Answer(generation, answer) => {
                self.answered = true;
                if let Some(outcome) = self.model.deliver(generation, answer) {
                    self.apply(outcome, cx);
                }
            }
            Done::Decorated(found) => self.model.decorated(found),
        }
        self.push(cx);
        cx.notify();
    }

    /// Hand the model's state to the panel, placed against the anchor. The list is as tall as the pane has room
    /// for, which the model is told, so what is selected is always in view.
    fn push(&mut self, cx: &mut Context<Self>) {
        let (pane, cursor) = self.spot;
        let placed = place(pane, cursor, card(self.model.has_crumbs()), self.model.lines().max(1));
        self.model.set_window(placed.rows);
        self.placed = Some(placed);
        let Some(panel) = self.panel.as_mut() else { return };
        let snapshot = self.model.snapshot();
        let notes: Vec<String> = snapshot
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Note(note) => Some((self.words.note)(*note)),
                _ => None,
            })
            .collect();
        let mut next = 0;
        let lines: Vec<Line<'_>> = snapshot
            .items
            .iter()
            .map(|item| match item {
                Item::Header(title) => Line::Header(title),
                Item::Row(row) => Line::Row(row),
                Item::Note(_) => {
                    next += 1;
                    Line::Note(&notes[next - 1])
                }
            })
            .collect();
        let view = View {
            lines: &lines,
            top: snapshot.top,
            shown: snapshot.shown,
            selected: snapshot.selected,
            crumbs: &snapshot.crumbs,
            text: &snapshot.text,
            epoch: snapshot.epoch,
            ghost: snapshot.ghost.as_deref(),
            facts: snapshot.facts,
            placeholder: &self.words.placeholder,
            name: &self.words.name,
            hint: &self.words.hint,
            appearance: store_in(&self.profile, cx).tokens.appearance,
        };
        if self.shown {
            panel.update(&view, placed.rect);
        } else {
            self.shown = true;
            panel.show(&view, placed.rect, cx);
        }
    }

    /// Where the panel was last placed, for devctl and the tests.
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn placed(&self) -> Option<Placed> {
        self.placed
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn panel_info(&self) -> Option<crate::platform::picker::Info> {
        self.panel.as_ref().and_then(Panel::info)
    }

    /// Whether the picker has nothing more to do: an answer for the text is in and no job is running.
    #[cfg(test)]
    pub(crate) fn idle(&self) -> bool {
        self.model.settled() && !self.working
    }
}

impl Drop for Picker {
    fn drop(&mut self) {
        // Whatever still runs is told it is not wanted; the panel closes with the picker (`Panel`'s drop).
        self.cancel.cancel();
    }
}
