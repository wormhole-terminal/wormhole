//! The generic picker, run: core's [`Model`] (`view::picker`) with its providers asked off the main thread and
//! the native panel (`platform::picker`) showing it. It knows nothing of what is listed and nothing of any
//! terminal: who opens it says where it goes (an anchor), which words it wears and which providers feed it,
//! and hears what came of it (a pick, a forget, a close) as [`PickerEvent`]s. `crate::folder_picker` is the
//! one that opens it for a terminal.
//!
//! What happens, in one line: the panel reports (text, a decided key, click, wheel) into a queue; a task reads
//! it and the model acts; the model asks for a query, which each provider answers on a pool thread of its own,
//! so a slow one holds back only its rows (`gate`: a few reads at a time for the whole app, and a read that
//! has run past the deadline stops counting, so one share that never answers cannot starve every picker); each
//! answer comes back by generation, so text that has moved on drops it; the model's snapshot of what is in view
//! is handed to the panel, when it changed.
//!
//! Without a panel (a test window, a system that has none, a background-mode instance, which never shows the
//! panel) all of it still runs: [`Picker::feed`] takes the events the panel would have sent, which is how the
//! tests and devctl drive it.

mod gate;

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::storage::files::hostile_char;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::panes::Rect;
use wormhole_core::view::picker::{
    Answer, Cancel, Context as Where, Decoration, DecorationRequest, Decorator, Item, Model, Note, Outcome,
    Placed, Provider, Snapshot, place,
};

use crate::platform::picker::{Event, Line, MAX_ROWS, Panel, Segments, View, card};
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
    /// A row requested its secondary action; the key is checked against the current list.
    Context(String),
    /// The person asked to forget the row with this key.
    Forget(String),
    /// The highlighted row changed, or there is no longer a row to preview.
    Highlight(Option<String>),
    /// A provider's status text changed.
    Status(Option<String>),
    /// The panel's optional filter was changed.
    Segment(usize),
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
    pub note: fn(&Note) -> String,
}

/// Where the picker is anchored: the pane it is over and the cursor's cell in it, in the window's space.
pub(crate) type Anchor = Rc<dyn Fn(&Window, &App) -> (Rect, Rect)>;

/// What feeds the picker: each provider is asked on its own.
#[derive(Clone)]
pub(crate) struct Sources {
    pub providers: Vec<Arc<dyn Provider>>,
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
    /// The panel, until the picker is closed.
    panel: Option<Panel>,
    shown: bool,
    placed: Option<Placed>,
    /// What the panel was last handed, so a push of the same is skipped.
    last: Option<(Snapshot, Rect)>,
    highlighted: Option<String>,
    /// A caller's initial selection, retained while its asynchronously loaded row is absent.
    preferred: Option<String>,
    status: Option<String>,
    segments: Option<Segments>,
    /// The token of the queries asked for the text now, cancelled when the text moves on. Decorations have
    /// their own: a text that moved on makes a query worthless, not the badge of a folder.
    cancel: Cancel,
    decorate_cancel: Cancel,
    /// Queries of providers that are out, for the tests and devctl to know the picker is at rest.
    out: usize,
    decorating: bool,
    /// Which decoration request is out, so one that was given up on doesn't undo the next.
    decoration: u64,
    /// An answer has come: from then on a new query waits for the text to rest.
    answered: bool,
    /// The wait before a query for text that moved on, and which question it is for: replaced (and so
    /// cancelled) by the next key, but not by a push that changed nothing the text.
    pause: Option<(u64, Task<()>)>,
    _tasks: Vec<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PickerEvent> for Picker {}

/// How long the text must rest before a query is asked again for it (the first one is asked at once).
const PAUSE: Duration = Duration::from_millis(60);

/// Whether `text` may be handed to whoever opened the picker as it is: a control character in it is a key,
/// not text, and an invisible formatting one makes it say something it doesn't. Whatever the provider gave,
/// the picker does not pass it on.
fn insertable(text: &str) -> bool {
    !text.chars().any(hostile_char)
}

/// A recycled native row may report a press after the answer has changed.
fn context_key<'a>(items: &'a [Item], at: usize, key: &str) -> Option<&'a str> {
    match items.get(at) {
        Some(Item::Row(row)) if row.key == key && row.context_menu => Some(&row.key),
        _ => None,
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
        let panel = Panel::new(window, opaque, tx, cx);
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
        let providers = sources.providers.len();
        let mut this = Self {
            model: Model::new(context, MAX_ROWS, providers),
            sources,
            words,
            profile,
            anchor,
            spot,
            panel,
            shown: false,
            placed: None,
            last: None,
            highlighted: None,
            preferred: None,
            status: None,
            segments: None,
            cancel: Cancel::new(),
            decorate_cancel: Cancel::new(),
            out: 0,
            decorating: false,
            decoration: 0,
            answered: false,
            pause: None,
            _tasks: tasks,
            _subscriptions: vec![moved],
        };
        this.go(cx);
        this
    }

    /// The window moved or changed size, or the terminal under the panel did: measure the anchor again, and
    /// place the panel again if it moved.
    pub(crate) fn reanchor(&mut self, window: &Window, cx: &mut Context<Self>) {
        let spot = (self.anchor)(window, cx);
        if spot == self.spot {
            return;
        }
        self.spot = spot;
        self.push(cx);
    }

    /// What the person did in the panel (or what a test says they did).
    pub(crate) fn feed(&mut self, event: Event, cx: &mut Context<Self>) {
        // Once the person interacts, their choice takes precedence over an initial selection.
        self.preferred = None;
        let before = self.model.generation();
        let outcome = match event {
            Event::Typed(field, epoch) => {
                self.model.typed_at(epoch, &field.text);
                Outcome::Nothing
            }
            Event::Do(what) => self.model.perform(what),
            Event::Key(key, field) => self.model.key(key, &field),
            #[cfg(any(test, feature = "devctl"))]
            Event::Row(at) => self.model.click(at),
            Event::RowKey { at, key } => self.model.click_key(at, &key),
            Event::ContextKey { at, key } => {
                if let Some(key) = context_key(self.model.items(), at, &key) {
                    cx.emit(PickerEvent::Context(key.to_owned()));
                }
                Outcome::Nothing
            }
            Event::Hover { at, key } => {
                self.model.hover_item(at, &key);
                Outcome::Nothing
            }
            Event::Segment(index) => {
                if self.segments.as_ref().is_some_and(|segments| index < segments.labels.len()) {
                    cx.emit(PickerEvent::Segment(index));
                }
                Outcome::Nothing
            }
            Event::Crumb(at) => self.model.crumb(at),
            Event::Scroll(by) => {
                self.model.scroll(by);
                Outcome::Nothing
            }
            Event::Resigned => Outcome::Close,
        };
        self.apply(outcome, cx);
        // Text that moved on makes the queries asked for it worthless: they are told to stop early.
        if self.model.generation() != before {
            self.cancel.cancel();
        }
        self.go(cx);
    }

    /// The text is `text`, put there by a script (devctl): the field in the panel shows it too.
    #[cfg(feature = "devctl")]
    pub(crate) fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if self.model.text() != text {
            self.preferred = None;
        }
        let before = self.model.generation();
        self.model.replace_text(text);
        if self.model.generation() != before {
            self.cancel.cancel();
        }
        self.go(cx);
    }

    /// Ask the model again for the same text (a row was forgotten).
    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        self.model.refresh();
        self.cancel.cancel();
        self.go(cx);
    }

    /// Select this row when it first appears, unless the person interacts first.
    pub(crate) fn prefer_initial(&mut self, key: String, cx: &mut Context<Self>) {
        self.preferred = Some(key);
        self.go(cx);
    }

    /// Set the optional filter displayed by the platform panel.
    pub(crate) fn set_segments(&mut self, segments: Option<Segments>, cx: &mut Context<Self>) {
        let segments = segments
            .filter(|segments| !segments.labels.is_empty() && segments.selected < segments.labels.len());
        if self.segments != segments {
            self.segments = segments;
            self.last = None;
            self.push(cx);
        }
    }

    /// Take the panel down. From a task, not from here: see the panel's own words. Whatever is still being read
    /// is told it is not wanted.
    pub(crate) fn close(&mut self, cx: &mut Context<Self>) {
        self.cancel.cancel();
        self.decorate_cancel.cancel();
        if let Some(panel) = self.panel.take() {
            panel.close(cx);
        }
    }

    fn apply(&mut self, outcome: Outcome, cx: &mut Context<Self>) {
        match outcome {
            Outcome::Insert(text) if insertable(&text) => cx.emit(PickerEvent::Insert(text)),
            // A provider's text with a key in it is not handed on: the pick does nothing.
            Outcome::Insert(_) => {}
            Outcome::Close => cx.emit(PickerEvent::Close),
            Outcome::Forget(key) => cx.emit(PickerEvent::Forget(key)),
            Outcome::Nothing | Outcome::Pass | Outcome::SetText(_) => {}
        }
    }

    /// Whatever has changed: ask for what the model wants, and show what it has.
    fn go(&mut self, cx: &mut Context<Self>) {
        self.query(cx);
        self.decorate(cx);
        if let Some(key) = self.preferred.as_deref()
            && let Some(index) =
                self.model.items().iter().position(|item| matches!(item, Item::Row(row) if row.key == key))
        {
            self.model.select_item(index);
            self.preferred = None;
        }
        self.push(cx);
        let highlighted = self.model.selected_row().map(|row| row.key.clone());
        if highlighted != self.highlighted {
            self.highlighted.clone_from(&highlighted);
            cx.emit(PickerEvent::Highlight(highlighted));
        }
        let status = self.model.items().iter().find_map(|item| match item {
            Item::Note(Note::Custom(text)) => Some(text.as_str()),
            _ => None,
        });
        if status != self.status.as_deref() {
            let status = status.map(str::to_owned);
            self.status.clone_from(&status);
            cx.emit(PickerEvent::Status(status));
        }
        cx.notify();
    }

    /// Ask every provider the question the model has, if it has one not asked: at once the first time, after
    /// the text has rested for the next ones.
    fn query(&mut self, cx: &mut Context<Self>) {
        if !self.model.wants_query() {
            return;
        }
        if !self.answered {
            self.start(cx);
            return;
        }
        // The wait is for this question: whatever else made the picker go (a scroll, a decoration) leaves it
        // running, or a busy list would never be asked.
        let generation = self.model.generation();
        if self.pause.as_ref().is_some_and(|(waiting, _)| *waiting == generation) {
            return;
        }
        // Replacing the task cancels the wait of the last key: only the text that rests is asked.
        self.pause = Some((
            generation,
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(PAUSE).await;
                this.update(cx, |this, cx| {
                    this.start(cx);
                    this.go(cx);
                })
                .ok();
            }),
        ));
    }

    /// One job for each provider, each on its own: an answer is merged when it comes, and the deadline names
    /// the ones that are still out.
    fn start(&mut self, cx: &mut Context<Self>) {
        let Some(request) = self.model.request() else { return };
        self.cancel = Cancel::new();
        let generation = request.generation;
        for (index, provider) in self.sources.providers.iter().cloned().enumerate() {
            let (query, cancel) = (request.query.clone(), self.cancel.clone());
            self.out += 1;
            cx.spawn(async move |this, cx| {
                let right = gate::enter(cx.background_executor()).await;
                let answer = cx
                    .background_spawn(async move {
                        let _right = right;
                        provider.query(&query, &cancel)
                    })
                    .await;
                this.update(cx, |this, cx| this.answered(generation, index, answer, cx)).ok();
            })
            .detach();
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(gate::DEADLINE).await;
            this.update(cx, |this, cx| this.overdue(generation, cx)).ok();
        })
        .detach();
    }

    fn answered(&mut self, generation: u64, index: usize, answer: Answer, cx: &mut Context<Self>) {
        self.out = self.out.saturating_sub(1);
        self.answered = true;
        if let Some(outcome) = self.model.deliver(generation, index, answer) {
            self.apply(outcome, cx);
        }
        self.go(cx);
    }

    /// The deadline of the query of `generation`: the providers still out are named, and a key that waited for
    /// them is done with what is in.
    fn overdue(&mut self, generation: u64, cx: &mut Context<Self>) {
        if let Some(outcome) = self.model.slow(generation) {
            self.apply(outcome, cx);
        }
        self.go(cx);
    }

    /// Ask the decorators about the rows in view, one request at a time.
    fn decorate(&mut self, cx: &mut Context<Self>) {
        if self.decorating || self.sources.decorators.is_empty() {
            return;
        }
        let Some(request) = self.model.decorations_wanted() else { return };
        self.decorating = true;
        self.decoration += 1;
        let (serial, decorators, cancel) =
            (self.decoration, self.sources.decorators.clone(), self.decorate_cancel.clone());
        let (asked, lost) = (request.clone(), request.clone());
        cx.spawn(async move |this, cx| {
            let right = gate::enter(cx.background_executor()).await;
            let found = cx
                .background_spawn(async move {
                    let _right = right;
                    decorate(&decorators, &request, &cancel)
                })
                .await;
            this.update(cx, |this, cx| this.decorated(serial, &asked, found, cx)).ok();
        })
        .detach();
        // A decoration that doesn't come (a share that doesn't answer) is given up on: its rows are open again.
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(gate::DEADLINE).await;
            this.update(cx, |this, cx| this.give_up(serial, &lost, cx)).ok();
        })
        .detach();
    }

    fn decorated(
        &mut self,
        serial: u64,
        asked: &DecorationRequest,
        found: Vec<Decoration>,
        cx: &mut Context<Self>,
    ) {
        // What was found is kept whenever it comes, for the rows as they are listed now.
        self.model.decorated(asked, found);
        if self.decoration == serial {
            self.decorating = false;
        }
        self.go(cx);
    }

    fn give_up(&mut self, serial: u64, lost: &DecorationRequest, cx: &mut Context<Self>) {
        if self.decoration == serial && self.decorating {
            self.decorating = false;
            self.model.decorations_lost(lost);
            self.go(cx);
        }
    }

    /// Hand the model's state to the panel, placed against the anchor, if it is not what it was handed last. The
    /// list is as tall as the pane has room for, which the model is told, so what is selected is always in view.
    fn push(&mut self, cx: &mut Context<Self>) {
        let (pane, cursor) = self.spot;
        let placed = place(
            pane,
            cursor,
            card(self.model.has_crumbs(), self.segments.is_some()),
            self.model.lines().max(1),
        );
        self.model.set_window(placed.rows);
        self.placed = Some(placed);
        let Some(panel) = self.panel.as_mut() else { return };
        let snapshot = self.model.snapshot();
        if self.last.as_ref().is_some_and(|(was, rect)| *was == snapshot && *rect == placed.rect) {
            return;
        }
        let notes: Vec<String> = snapshot
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Note(note) => Some((self.words.note)(note)),
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
            segments: self.segments.as_ref(),
            top: snapshot.top,
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
        drop(lines);
        self.last = Some((snapshot, placed.rect));
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

    /// Whether the picker has nothing more to do: every provider has answered, nothing is waiting to be asked
    /// and no decoration is out or wanted.
    #[cfg(test)]
    pub(crate) fn idle(&self) -> bool {
        self.model.settled() && self.out == 0 && !self.decorating && !self.model.wants_decorations()
    }

    /// Whether the panel is still there to show things (it is not once the picker is closed).
    pub(crate) fn has_panel(&self) -> bool {
        self.panel.is_some()
    }
}

/// What the decorators know of the rows of `request`, each asked only about the kinds of row it names.
fn decorate(
    decorators: &[Arc<dyn Decorator>],
    request: &DecorationRequest,
    cancel: &Cancel,
) -> Vec<Decoration> {
    let mut found = Vec::new();
    for decorator in decorators {
        if cancel.is_cancelled() {
            break;
        }
        let kinds = decorator.kinds();
        let keys: Vec<String> = request
            .rows
            .iter()
            .filter(|row| kinds.is_empty() || kinds.contains(&row.kind.as_str()))
            .map(|row| row.key.clone())
            .collect();
        if !keys.is_empty() {
            found.extend(decorator.decorate(&keys, cancel));
        }
    }
    found
}

impl Drop for Picker {
    fn drop(&mut self) {
        // Whatever still runs is told it is not wanted; the panel is taken down by its own drop.
        self.cancel.cancel();
        self.decorate_cancel.cancel();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use wormhole_core::view::picker::{Cancel, Decoration, DecorationRequest, Decorator, Item, Row, Target};

    use super::{context_key, decorate, insertable};

    #[test]
    fn context_actions_ignore_stale_rows_and_rows_without_a_menu() {
        let mut enabled = Row::new("group-1", "A group");
        enabled.context_menu = true;
        let rows = vec![Item::Row(enabled), Item::Row(Row::new("terminal-1", "A terminal"))];
        assert_eq!(context_key(&rows, 0, "group-1"), Some("group-1"));
        assert_eq!(context_key(&rows, 0, "old-group"), None);
        assert_eq!(context_key(&rows, 1, "group-1"), None);
        assert_eq!(context_key(&rows, 1, "terminal-1"), None);
        assert_eq!(context_key(&rows, 2, "group-1"), None);
    }

    struct Only(&'static str, &'static [&'static str]);

    impl Decorator for Only {
        fn kinds(&self) -> &[&'static str] {
            self.1
        }

        fn decorate(&self, keys: &[String], _: &Cancel) -> Vec<Decoration> {
            keys.iter()
                .map(|k| Decoration { key: k.clone(), badge: Some(self.0.into()), chevron: None })
                .collect()
        }
    }

    #[test]
    fn a_decorator_is_asked_only_about_the_kinds_of_row_it_names() {
        let request = DecorationRequest {
            rows: vec![
                Target { key: "/a".into(), kind: "folder".into() },
                Target { key: "plum".into(), kind: "fruit".into() },
            ],
        };
        let decorators: Vec<Arc<dyn Decorator>> =
            vec![Arc::new(Only("path", &["folder"])), Arc::new(Only("any", &[]))];
        let found = decorate(&decorators, &request, &Cancel::new());
        let said: Vec<_> = found.iter().map(|d| (d.key.as_str(), d.badge.as_deref().unwrap_or(""))).collect();
        assert_eq!(said, [("/a", "path"), ("/a", "any"), ("plum", "any")]);
    }

    #[test]
    fn what_a_provider_says_to_insert_is_never_a_key() {
        assert!(insertable("~/My Folder/"));
        assert!(!insertable("a\nb"), "a line break would run the line");
        assert!(!insertable("a\u{15}b"), "Ctrl-U clears it");
        assert!(!insertable("exe\u{202e}txt"), "a bidi override makes it a lie");
    }
}
