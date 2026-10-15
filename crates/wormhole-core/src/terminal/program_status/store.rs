//! What the terminal holds of what programs report: a set of records, one per id, and the one
//! thing the UI shows of them.

use super::{Kind, Record, Report, State, agent_for_app};
use crate::terminal::agents::AgentId;
use crate::terminal::clean;

/// Records per terminal. The spec asks for at least 64.
pub const RECORDS_MAX: usize = 256;

/// The most characters of a message or title the UI is given: one line of the sidebar, like the
/// messages agent hooks bring.
const DISPLAY_MAX_CHARS: usize = 200;

/// The records of one terminal, least recently updated first (so eviction takes the front and an
/// update moves a record to the back).
#[derive(Debug, Default)]
pub struct ProgramStatus {
    records: Vec<Stored>,
}

#[derive(Debug)]
struct Stored {
    /// `None` is the root record.
    id: Option<String>,
    record: Record,
}

/// Whether `id` is `ancestor` or something beneath it (`build/test` is beneath `build`).
fn within(id: &str, ancestor: &str) -> bool {
    id.strip_prefix(ancestor).is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

impl ProgramStatus {
    /// Apply a report. Returns whether the records, or the order that says which was updated
    /// last, changed (a program repeating itself changes nothing for anyone who is watching).
    pub fn apply(&mut self, report: Report) -> bool {
        match report {
            Report::Clear { id } => self.clear(id.as_deref()),
            Report::Set { id, record } => self.set(id, record),
        }
    }

    /// A report replaces its record completely, and makes it the most recently updated. One that
    /// would add a record past [`RECORDS_MAX`] removes the least recently updated first.
    fn set(&mut self, id: Option<String>, record: Record) -> bool {
        match self.records.iter().position(|stored| stored.id == id) {
            Some(at) => {
                if at + 1 == self.records.len() && self.records[at].record == record {
                    return false;
                }
                self.records.remove(at);
            }
            None if self.records.len() >= RECORDS_MAX => {
                self.records.remove(0);
            }
            None => {}
        }
        self.records.push(Stored { id, record });
        true
    }

    /// The record at `id` and everything beneath it; with no id, everything.
    fn clear(&mut self, id: Option<&str>) -> bool {
        let before = self.records.len();
        match id {
            None => self.records.clear(),
            Some(id) => self.records.retain(|stored| !stored.id.as_deref().is_some_and(|s| within(s, id))),
        }
        self.records.len() != before
    }

    /// A new shell prompt began (OSC 133 `A`): the program is over. Working and blocked records
    /// are not true any more; idle ones are dropped too (the spec allows it): a program that
    /// stopped at rest and left does not need remembering, and a terminal that kept it would
    /// take its absence for the program still reporting. Done and error stay for the user to
    /// look at. Returns whether anything was dropped.
    pub fn on_prompt(&mut self) -> bool {
        self.keep(|state| state.outlives_program())
    }

    /// The process attached to the terminal exited: the same as a new prompt.
    pub fn on_process_exit(&mut self) -> bool {
        self.on_prompt()
    }

    /// The user has looked at the terminal: finished and failed records have done their job.
    /// (The spec leaves it to the terminal to decide when to stop showing them.) What is still
    /// going on stays. Returns whether anything was dropped.
    pub fn acknowledge(&mut self) -> bool {
        self.keep(|state| !state.outlives_program())
    }

    /// A full reset (RIS) removes every record; a soft reset (DECSTR) is not heard of here.
    pub fn reset(&mut self) -> bool {
        self.clear(None)
    }

    fn keep(&mut self, mut keep: impl FnMut(State) -> bool) -> bool {
        let before = self.records.len();
        self.records.retain(|stored| keep(stored.record.state));
        self.records.len() != before
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// The record's `app`, or the nearest ancestor's that has one (the root last).
    fn app_of<'a>(&'a self, stored: &'a Stored) -> Option<&'a str> {
        if let Some(app) = &stored.record.app {
            return Some(app);
        }
        let mut path = stored.id.as_deref();
        // `a/b/c`: `a/b`, `a`, then the root.
        while let Some(id) = path {
            path = id.rsplit_once('/').map(|(parent, _)| parent);
            let ancestor = self.records.iter().find(|s| s.id.as_deref() == path);
            if let Some(app) = ancestor.and_then(|s| s.record.app.as_deref()) {
                return Some(app);
            }
        }
        None
    }

    /// The record the UI shows: the one that needs the user most (blocked, then error, then done,
    /// then working, then idle), the most recently updated of those that tie. A terminal shows one
    /// state for the program in it, and a question must never hide behind a worker that is still
    /// busy, nor a result behind one at rest. `None` when there are no records.
    pub fn summary(&self) -> Option<Summary> {
        // `max_by_key` keeps the last of equals, and the records are oldest first.
        let top = self.records.iter().max_by_key(|stored| stored.record.state.priority())?;
        let record = &top.record;
        Some(Summary {
            state: record.state,
            kind: record.kind,
            progress: record.progress,
            app: self.app_of(top).map(str::to_string),
            title: record.title.clone(),
            msg: record.msg.clone(),
        })
    }
}

/// The record shown for a terminal (see [`ProgramStatus::summary`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    pub state: State,
    pub kind: Option<Kind>,
    pub progress: Option<u8>,
    /// The record's own, or its nearest ancestor's.
    pub app: Option<String>,
    /// As reported: validated, not cleaned for display.
    pub title: Option<String>,
    pub msg: Option<String>,
}

impl Summary {
    /// The message as the UI may show it outside the terminal grid: one line, without control
    /// characters or the invisible and direction-changing ones that could disguise what it says,
    /// cut to a length a row shows. `None` when nothing is left.
    pub fn message(&self) -> Option<String> {
        display(self.msg.as_deref())
    }

    /// [`Self::message`] for the title.
    pub fn label(&self) -> Option<String> {
        display(self.title.as_deref())
    }

    /// The coding agent `app` names, if it names one.
    pub fn agent(&self) -> Option<AgentId> {
        self.app.as_deref().and_then(agent_for_app)
    }
}

fn display(text: Option<&str>) -> Option<String> {
    Some(clean(text?, DISPLAY_MAX_CHARS)).filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::super::{Kind, Record, Report, State};
    use super::{ProgramStatus, RECORDS_MAX, Summary, within};

    fn record(state: State) -> Record {
        Record { state, kind: None, progress: None, app: None, title: None, msg: None }
    }

    fn set(id: Option<&str>, state: State) -> Report {
        Report::Set { id: id.map(str::to_string), record: record(state) }
    }

    fn with_app(id: Option<&str>, state: State, app: &str) -> Report {
        Report::Set { id: id.map(str::to_string), record: Record { app: Some(app.into()), ..record(state) } }
    }

    fn clear(id: Option<&str>) -> Report {
        Report::Clear { id: id.map(str::to_string) }
    }

    fn store(reports: impl IntoIterator<Item = Report>) -> ProgramStatus {
        let mut store = ProgramStatus::default();
        for report in reports {
            store.apply(report);
        }
        store
    }

    fn state_of(store: &ProgramStatus) -> Option<State> {
        store.summary().map(|summary| summary.state)
    }

    #[test]
    fn each_report_replaces_its_record_entirely() {
        let mut store = ProgramStatus::default();
        store.apply(Report::Set {
            id: None,
            record: Record {
                state: State::Blocked,
                kind: Some(Kind::Auth),
                progress: Some(30),
                app: Some("brew".into()),
                title: Some("Brew".into()),
                msg: Some("Password".into()),
            },
        });
        assert_eq!(store.len(), 1);
        store.apply(set(None, State::Working));
        assert_eq!(store.len(), 1, "same id, same record");
        let summary = store.summary().expect("a record");
        // A key the new report left out is gone: kind, progress, app, title and msg.
        assert_eq!(
            summary,
            Summary { state: State::Working, kind: None, progress: None, app: None, title: None, msg: None }
        );
    }

    #[test]
    fn clearing_removes_the_record_and_everything_beneath_it() {
        let ids = ["a", "a/b", "a/b/c", "ab", "a-b", "b", "b/a"];
        let all =
            || store(ids.iter().map(|id| set(Some(id), State::Working)).chain([set(None, State::Idle)]));
        let left = |store: &ProgramStatus| -> Vec<Option<String>> {
            let mut ids: Vec<_> = store.records.iter().map(|s| s.id.clone()).collect();
            ids.sort();
            ids
        };
        let mut s = all();
        assert!(s.apply(clear(Some("a"))));
        // `a` and `a/…` go; `ab` and `a-b` are not beneath `a`; the root is no descendant.
        let names: Vec<_> = left(&s).into_iter().map(|id| id.unwrap_or_default()).collect();
        assert_eq!(names, ["", "a-b", "ab", "b", "b/a"]);
        let mut s = all();
        s.apply(clear(Some("a/b")));
        let names: Vec<_> = left(&s).into_iter().map(|id| id.unwrap_or_default()).collect();
        assert_eq!(names, ["", "a", "a-b", "ab", "b", "b/a"]);
        // No id: every record on the terminal, the root included.
        let mut s = all();
        assert!(s.apply(clear(None)));
        assert!(s.is_empty());
        assert_eq!(s.summary(), None);
        // A parent that does not exist still clears what is beneath it; clearing nothing is no change.
        let mut s = store([set(Some("x/y"), State::Working)]);
        assert!(s.apply(clear(Some("x"))));
        assert!(!s.apply(clear(Some("x"))));
        assert!(!ProgramStatus::default().apply(clear(None)));
    }

    #[test]
    fn within_means_the_id_itself_or_a_path_beneath_it() {
        assert!(within("a", "a"));
        assert!(within("a/b", "a"));
        assert!(within("a/b/c", "a/b"));
        assert!(!within("ab", "a"));
        assert!(!within("a", "a/b"));
        assert!(!within("b/a", "a"));
    }

    #[test]
    fn the_least_recently_updated_record_makes_room() {
        let id = |n: usize| format!("r{n}");
        let mut store = ProgramStatus::default();
        for n in 0..RECORDS_MAX {
            store.apply(set(Some(&id(n)), State::Idle));
        }
        assert_eq!(store.len(), RECORDS_MAX);
        // Updating r0 makes r1 the oldest; a new record then takes r1's place, not r0's.
        store.apply(set(Some(&id(0)), State::Working));
        assert_eq!(store.len(), RECORDS_MAX, "an update adds nothing");
        store.apply(set(Some("new"), State::Done));
        assert_eq!(store.len(), RECORDS_MAX);
        fn has(store: &ProgramStatus, name: &str) -> bool {
            store.records.iter().any(|s| s.id.as_deref() == Some(name))
        }
        assert!(has(&store, "r0") && has(&store, "new") && has(&store, &id(2)));
        assert!(has(&store, &id(RECORDS_MAX - 1)));
        assert!(!has(&store, "r1"), "r1 was the least recently updated");
        // The root counts as a record like any other.
        store.apply(set(None, State::Idle));
        assert!(!has(&store, "r2"));
        assert_eq!(store.len(), RECORDS_MAX);
        // 64 is the least a terminal must keep.
        const { assert!(RECORDS_MAX >= 64) };
    }

    #[test]
    fn a_report_that_repeats_the_newest_record_changes_nothing() {
        let mut store = ProgramStatus::default();
        assert!(store.apply(set(Some("a"), State::Working)));
        assert!(!store.apply(set(Some("a"), State::Working)));
        assert!(store.apply(set(Some("b"), State::Working)));
        // "a" repeats, but is no longer the last one updated: which of two equals the UI shows can differ.
        assert!(store.apply(set(Some("a"), State::Working)));
        assert!(store.apply(set(Some("a"), State::Done)));
    }

    #[test]
    fn the_record_that_needs_the_user_first_is_shown() {
        let order = [State::Blocked, State::Error, State::Done, State::Working, State::Idle];
        for (i, &top) in order.iter().enumerate() {
            // Whatever the order they arrived in, the first of the states present wins.
            let rest = &order[i..];
            let forward = store(rest.iter().enumerate().map(|(n, s)| set(Some(&format!("r{n}")), *s)));
            let backward = store(rest.iter().enumerate().rev().map(|(n, s)| set(Some(&format!("r{n}")), *s)));
            assert_eq!(state_of(&forward), Some(top), "{rest:?}");
            assert_eq!(state_of(&backward), Some(top), "{rest:?} reversed");
        }
        // A blocked worker shows through a root that says it is working.
        let deploy = store([set(None, State::Working), set(Some("eu"), State::Blocked)]);
        assert_eq!(state_of(&deploy), Some(State::Blocked));
    }

    #[test]
    fn of_equal_states_the_most_recently_updated_is_shown() {
        let one = |msg: &str| Report::Set {
            id: Some(msg.into()),
            record: Record { msg: Some(msg.into()), ..record(State::Working) },
        };
        let mut store = store([one("first"), one("second")]);
        assert_eq!(store.summary().and_then(|s| s.msg).as_deref(), Some("second"));
        store.apply(one("first"));
        assert_eq!(store.summary().and_then(|s| s.msg).as_deref(), Some("first"));
    }

    #[test]
    fn app_comes_from_the_nearest_ancestor_that_has_one() {
        let app = |store: &ProgramStatus, id: &str| {
            let stored = store.records.iter().find(|s| s.id.as_deref() == Some(id)).expect("a record");
            store.app_of(stored).map(str::to_string)
        };
        let s = store([
            with_app(None, State::Working, "deploy"),
            set(Some("a/b/c"), State::Working),
            with_app(Some("a"), State::Working, "mid"),
            set(Some("a/b"), State::Working),
            set(Some("x/y"), State::Working),
            with_app(Some("own"), State::Working, "mine"),
            set(Some("own/child"), State::Working),
        ]);
        assert_eq!(app(&s, "a/b/c"), Some("mid".into()), "a/b has none, a does");
        assert_eq!(app(&s, "a/b"), Some("mid".into()));
        assert_eq!(app(&s, "a"), Some("mid".into()), "its own");
        assert_eq!(app(&s, "x/y"), Some("deploy".into()), "neither x/y nor x exist: the root's");
        assert_eq!(app(&s, "own/child"), Some("mine".into()));
        // No ancestor has one: none.
        let bare = store([set(None, State::Working), set(Some("a"), State::Working)]);
        assert_eq!(app(&bare, "a"), None);
        // It is read through the summary too.
        let only = store([with_app(None, State::Done, "cargo")]);
        assert_eq!(only.summary().and_then(|s| s.app).as_deref(), Some("cargo"));
    }

    #[test]
    fn a_prompt_or_the_programs_exit_ends_what_only_the_running_program_could_say() {
        for event in [ProgramStatus::on_prompt, ProgramStatus::on_process_exit] {
            let mut s = store([
                set(None, State::Working),
                set(Some("a"), State::Blocked),
                set(Some("b"), State::Idle),
                set(Some("c"), State::Done),
                set(Some("d"), State::Error),
            ]);
            assert!(event(&mut s));
            let mut left: Vec<_> = s.records.iter().map(|r| r.record.state).collect();
            left.sort_by_key(|state| state.priority());
            assert_eq!(left, [State::Done, State::Error]);
            assert!(!event(&mut s), "nothing more to drop");
        }
        // Without done or error, the prompt leaves nothing to show.
        let mut s = store([set(None, State::Idle)]);
        s.on_prompt();
        assert!(s.is_empty());
    }

    #[test]
    fn looking_at_the_terminal_ends_done_and_error_but_not_the_work() {
        let mut s = store([
            set(None, State::Working),
            set(Some("a"), State::Done),
            set(Some("b"), State::Error),
            set(Some("c"), State::Blocked),
            set(Some("d"), State::Idle),
        ]);
        assert!(s.acknowledge());
        assert_eq!(s.len(), 3);
        assert_eq!(state_of(&s), Some(State::Blocked));
        assert!(!s.acknowledge());
        let mut done = store([set(None, State::Done)]);
        assert!(done.acknowledge());
        assert!(done.is_empty());
    }

    #[test]
    fn a_full_reset_removes_every_record() {
        let mut s =
            store([set(None, State::Done), set(Some("a"), State::Error), set(Some("b"), State::Working)]);
        assert!(s.reset());
        assert!(s.is_empty());
        assert!(!s.reset());
    }

    #[test]
    fn the_message_is_cleaned_for_display() {
        let summary = |msg: &str| Summary {
            state: State::Blocked,
            kind: None,
            progress: None,
            app: None,
            title: None,
            msg: Some(msg.into()),
        };
        // Direction overrides and invisible formatting go (they could disguise what it says),
        // runs of spaces fold, and a long line is cut.
        assert_eq!(
            summary("Apply\u{202e} the   plan\u{200b}?").message().as_deref(),
            Some("Apply the plan?")
        );
        assert_eq!(summary("   ").message(), None);
        assert_eq!(summary("\u{202e}\u{200b}").message(), None);
        let long = summary(&"x".repeat(500)).message().expect("some text");
        assert_eq!(long.chars().count(), 201);
        assert!(long.ends_with('…'));
        let none = Summary { msg: None, ..summary("x") };
        assert_eq!((none.message(), none.label()), (None, None));
        assert_eq!(Summary { title: Some("EU West".into()), ..none }.label().as_deref(), Some("EU West"));
    }

    #[test]
    fn app_names_an_agent_where_it_is_obvious() {
        let agent = |app: &str| {
            Summary {
                state: State::Working,
                kind: None,
                progress: None,
                app: Some(app.into()),
                title: None,
                msg: None,
            }
            .agent()
        };
        use crate::terminal::agents::AgentId;
        assert_eq!(agent("claude-code"), Some(AgentId::Claude));
        assert_eq!(agent("Claude"), Some(AgentId::Claude));
        assert_eq!(agent("codex"), Some(AgentId::Codex));
        assert_eq!(agent("gemini-cli"), Some(AgentId::Gemini));
        assert_eq!(agent("cursor-agent"), Some(AgentId::Cursor));
        assert_eq!(agent("opencode"), Some(AgentId::OpenCode));
        assert_eq!(agent("kiro"), Some(AgentId::Kiro));
        assert_eq!(agent("terraform"), None);
        assert_eq!(agent("brew"), None);
        let bare = Summary {
            app: None,
            ..Summary { state: State::Idle, kind: None, progress: None, app: None, title: None, msg: None }
        };
        assert_eq!(bare.agent(), None);
    }
}
