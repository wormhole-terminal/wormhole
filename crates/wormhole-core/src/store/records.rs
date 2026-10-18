//! The plain data the store saves and loads.
//!
//! Every terminal is a session of its own (a row in the sidebar), and a split is a layout that
//! groups several of them: a session names its split in `split_id`, the window keeps the splits.
//! A terminal's id is its session's id, and it is what agent hooks report (`WORMHOLE_TAB_ID`).

#[derive(Debug, Default, Clone)]
pub struct WindowRecord {
    pub id: i64,
    /// Index of the selected session (a split's members count one each).
    pub active: usize,
    /// Panel layout of the window, as JSON (owned by the app).
    pub layout: Option<String>,
    /// Where the window was, as JSON (owned by the app).
    pub bounds: Option<String>,
    pub sessions: Vec<SessionRecord>,
    /// The window's splits. A split nobody names in `split_id` is forgotten when the window is saved.
    pub splits: Vec<SplitRecord>,
}

/// Several terminals shown side by side as one: the members are the sessions that name it, next to
/// each other in the window's order.
#[derive(Debug, Default, Clone)]
pub struct SplitRecord {
    /// Never a session's id: ids for splits and sessions come from one counter.
    pub id: i64,
    /// The pane tree (which member is where, focused, zoomed) as JSON, owned by the app and keyed
    /// by the members' ids. `None`: the members side by side, in their order.
    pub layout: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct SessionRecord {
    pub id: i64,
    pub name: Option<String>,
    pub tasks_collapsed: bool,
    /// The Notes section of the right panel is folded up (its notes are kept).
    pub notes_collapsed: bool,
    pub notes: Vec<NoteRecord>,
    pub tasks: Vec<TaskRecord>,
    /// Pinned to the top of its group in the sidebar. Every member of a split carries the flag.
    pub pinned: bool,
    /// The terminal's working folder.
    pub cwd: Option<String>,
    /// The shell profile it was opened with (`shells`' key), `None` for the default: a restart of
    /// its shell and a restored session run the same thing.
    pub shell: Option<String>,
    /// Only filled when loading.
    pub scrollback: Option<String>,
    /// The split this terminal is a member of (`WindowRecord::splits`), `None` for a terminal alone.
    /// A name no split has is dropped the next time the database is opened.
    pub split_id: Option<i64>,
    /// The session host that runs this terminal (`hosts::is_valid_host_id`'s 12 characters), `None`
    /// for a terminal that lives in the app's own process. Stored on the terminal's `tabs` row.
    pub host_id: Option<String>,
}

impl SessionRecord {
    /// A session with a single fresh terminal (whose id is the session's).
    pub fn new(id: i64, name: Option<String>, cwd: Option<String>) -> Self {
        Self { id, name, cwd, ..Default::default() }
    }
}

#[derive(Debug, Clone)]
pub struct NoteRecord {
    pub body: String,
    pub collapsed: bool,
}

#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub text: String,
    pub done: bool,
}

/// A note or task with nothing in it but whitespace: an entry that isn't one.
pub fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}
