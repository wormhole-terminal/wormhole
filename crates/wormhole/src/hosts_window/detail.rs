//! A host's facts: what its row shows when it is opened, and which of them are wrong (those are also said
//! under the folded row, so a problem is never hidden behind a click).

use gpui_kit::Hsla;
use wormhole_core::ipc::hosts::report::{Answer, AppFacts, HostRow, Standing};
use wormhole_term::client::LinkMode;
use wormhole_text::keys::{
    HOSTS_FIELD_ANSWER, HOSTS_FIELD_FOLDER, HOSTS_FIELD_HEARD, HOSTS_FIELD_HOST_PID, HOSTS_FIELD_LINK,
    HOSTS_FIELD_MEMORY, HOSTS_FIELD_META, HOSTS_FIELD_PROGRAM, HOSTS_FIELD_PROTOCOL, HOSTS_FIELD_QUIET,
    HOSTS_FIELD_SHELL_PID, HOSTS_FIELD_SOCKET, HOSTS_FIELD_STARTED, HOSTS_FIELD_VERSION, HOSTS_UNKNOWN,
    hosts_version_differs,
};
use wormhole_text::text;

use super::words;
use crate::settings::UiColors;
use crate::terminal_view::unavailable_text;

/// How a fact's value is coloured: the text, or quieter for what only helps to find a file, or in the
/// colour of the trouble it shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tone {
    Normal,
    Quiet,
    Warning,
    Danger,
}

impl Tone {
    pub(super) fn color(self, ui: UiColors) -> Hsla {
        match self {
            Tone::Normal => ui.text,
            Tone::Quiet => ui.muted,
            Tone::Warning => ui.warning_text,
            Tone::Danger => ui.danger_text,
        }
    }

    /// The value tells of trouble.
    pub(super) fn is_problem(self) -> bool {
        matches!(self, Tone::Warning | Tone::Danger)
    }
}

pub(super) struct Fact {
    pub label: &'static str,
    pub value: String,
    pub tone: Tone,
    /// A path: set in the code font.
    pub code: bool,
}

impl Fact {
    fn new(label: &'static str, value: impl Into<String>, tone: Tone) -> Self {
        Self { label, value: value.into(), tone, code: false }
    }

    fn path(label: &'static str, value: impl Into<String>, tone: Tone) -> Self {
        Self { code: true, ..Self::new(label, value, tone) }
    }
}

/// The facts of one host in groups, each a run of lines that belong together: its processes, what it
/// runs and how it answers, the connection to this app, and its files.
pub(super) fn groups(row: &HostRow, app: &AppFacts) -> Vec<Vec<Fact>> {
    let unknown = || text(HOSTS_UNKNOWN).to_string();
    let number = |n: Option<u32>| n.map_or_else(unknown, |n| n.to_string());
    let path = |p: &Option<std::path::PathBuf>| p.as_ref().map_or_else(unknown, |p| words::path(p));
    let mismatch = row.version_differs(app);
    let version = match (&row.host_version, mismatch) {
        (Some(v), true) => format!("{v} ({})", hosts_version_differs(&app.version)),
        (Some(v), false) => v.clone(),
        (None, _) => unknown(),
    };
    let bad_link = row.link.is_some_and(|l| l.lost || l.not_responding_for.is_some());
    let bad_answer = row.standing.is_live() && matches!(row.answer, Answer::No(_) | Answer::Incompatible);
    let tone = |bad: bool, trouble: Tone| if bad { trouble } else { Tone::Normal };

    let processes = vec![
        Fact::new(text(HOSTS_FIELD_HOST_PID), number(row.host_pid), Tone::Normal),
        Fact::new(text(HOSTS_FIELD_SHELL_PID), number(row.shell_pid), Tone::Normal),
        Fact::new(text(HOSTS_FIELD_PROGRAM), row.program.clone().unwrap_or_else(unknown), Tone::Normal),
        Fact::new(text(HOSTS_FIELD_MEMORY), row.rss.map_or_else(unknown, words::size_text), Tone::Normal),
    ];
    let host = vec![
        Fact::new(text(HOSTS_FIELD_VERSION), version, tone(mismatch, Tone::Warning)),
        Fact::new(
            text(HOSTS_FIELD_PROTOCOL),
            words::protocol(row).unwrap_or_else(unknown),
            tone(row.fit_problem(), Tone::Danger),
        ),
        Fact::new(text(HOSTS_FIELD_STARTED), words::started(row).unwrap_or_else(unknown), Tone::Normal),
        Fact::new(
            text(HOSTS_FIELD_ANSWER),
            words::answer(&row.answer),
            if bad_answer {
                Tone::Danger
            } else {
                tone(matches!(&row.answer, Answer::Yes(info) if info.exited), Tone::Warning)
            },
        ),
    ];
    let mut connection = Vec::new();
    if row.link.is_some() || matches!(row.standing, Standing::Attached { .. }) {
        connection.push(Fact::new(
            text(HOSTS_FIELD_LINK),
            row.unavailable
                .as_ref()
                .map_or_else(|| words::link(row.link.as_ref()), |u| unavailable_text(u.kind).to_string()),
            tone(bad_link || row.unavailable.is_some(), Tone::Danger),
        ));
    }
    if let Some(link) = row.link.filter(|l| !l.detached && !l.ended) {
        // A link on screen is pinged, so the last answer is news; one that is not only knows how long it has
        // been quiet.
        connection.push(if link.mode == LinkMode::Headless {
            Fact::new(text(HOSTS_FIELD_QUIET), words::quiet_text(link.quiet_for), Tone::Normal)
        } else {
            Fact::new(text(HOSTS_FIELD_HEARD), words::ago_text(link.quiet_for), Tone::Normal)
        });
    }
    let files = vec![
        Fact::path(text(HOSTS_FIELD_FOLDER), path(&row.cwd), Tone::Normal),
        Fact::path(text(HOSTS_FIELD_SOCKET), path(&row.socket), Tone::Quiet),
        Fact::path(text(HOSTS_FIELD_META), words::path(&row.meta_path), Tone::Quiet),
    ];
    [processes, host, connection, files].into_iter().filter(|group| !group.is_empty()).collect()
}
