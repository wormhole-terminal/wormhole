//! The Session Hosts report: every host in the hosts folder as one row, classified, with what is
//! known about it from its files, from asking it (`probe`), from the system and from the app's own
//! terminals. This is the data of the Session Hosts window and of its "copy" text; nothing here
//! reads a file, a socket or a clock (`survey` does), so the rules are tested with made-up inputs.
//!
//! A row is **attached** when a terminal of this app holds a live connection to the host,
//! **detached** when the host runs and a terminal row names it (or the app let go of it) but no
//! connection is held, an **orphan** when it runs and nothing names it, and **stale** when its files
//! are left over without a running host (`StaleReason` says which kind).

mod plain;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use wormhole_term::client::LinkStats;
pub use wormhole_wire::version::HostFit;
use wormhole_wire::version::Range;
use wormhole_wire::{AppVersion, HostState};

use super::probe::Probed;
use super::{HostMeta, Scan, StaleReason, meta_path};

pub use plain::{bytes_words, duration_words, range_words, stale_word, time_words};

/// What this build is, for the header and for judging each host against it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppFacts {
    /// `1.6.2`: the same text a host's meta file carries as `host_version`.
    pub version: String,
    pub proto_major: u16,
    pub proto_minor: u16,
}

impl AppFacts {
    /// This build: the workspace's version (the app's and core's are one) and the wire's protocol.
    pub fn this_build() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            proto_major: wormhole_wire::version::PROTO_MAJOR,
            proto_minor: wormhole_wire::version::PROTO_MINOR,
        }
    }
}

/// A terminal of this app that runs in a host: where it is, what it is called, how its link stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub host_id: String,
    pub window: i64,
    pub title: String,
    /// `None` for a terminal whose connection is not up yet.
    pub link: Option<LinkStats>,
}

/// What looking at one host found besides its files. All of it is optional: a pid may be gone,
/// the socket may not answer, a reading may not be allowed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Observed {
    pub probed: Option<Probed>,
    /// The host process's resident memory, in bytes.
    pub rss: Option<u64>,
    /// The program in the terminal's foreground, by name.
    pub program: Option<String>,
    pub cwd: Option<PathBuf>,
}

/// Everything [`Report::build`] needs.
#[derive(Debug)]
pub struct Inputs {
    pub app: AppFacts,
    pub hosts_dir: PathBuf,
    /// Whether `FeatureFlag::PersistentTerminals` is on.
    pub flag_on: bool,
    pub host_binary: Option<PathBuf>,
    pub scan: Scan,
    pub observed: HashMap<String, Observed>,
    pub attachments: Vec<Attachment>,
    /// Host ids some terminal row names (any profile's database).
    pub named: HashSet<String>,
    /// Unix seconds now.
    pub now: u64,
}

/// Where a host stands relative to this app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    Attached { window: i64, title: String },
    Detached,
    Orphan,
    Stale(StaleReason),
}

impl Standing {
    pub fn is_live(&self) -> bool {
        !matches!(self, Self::Stale(_))
    }
}

/// What asking the host came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Not asked (a stale host has nobody to ask).
    NotAsked,
    /// It answered `GetInfo`.
    Yes(HostInfo),
    /// It answered that it speaks no protocol this build does.
    Incompatible,
    /// It did not answer; the text says why.
    No(String),
}

/// A host's own account of itself (`Info`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostInfo {
    pub version: String,
    pub proto: u16,
    pub proto_minor: u16,
    /// The program has ended and the host waits to be asked to go.
    pub exited: bool,
    pub cols: u16,
    pub rows: u16,
    pub history_lines: u32,
    pub alt_screen: bool,
    pub revision: u64,
}

/// One host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRow {
    pub id: String,
    pub standing: Standing,
    pub host_pid: Option<u32>,
    pub shell_pid: Option<u32>,
    pub program: Option<String>,
    pub cwd: Option<PathBuf>,
    /// Unix seconds.
    pub started_at: Option<u64>,
    pub uptime_secs: Option<u64>,
    /// As the meta file says.
    pub host_version: Option<String>,
    pub proto: Option<(u16, u16)>,
    /// `None` for a host that is not running.
    pub fit: Option<HostFit>,
    pub socket: Option<PathBuf>,
    pub meta_path: PathBuf,
    pub answer: Answer,
    pub rss: Option<u64>,
    pub link: Option<LinkStats>,
}

impl HostRow {
    /// The host is not the build that runs the app (another version).
    pub fn version_differs(&self, app: &AppFacts) -> bool {
        self.host_version.as_deref().is_some_and(|v| v != app.version)
    }

    /// This build cannot attach to the host (its protocol is too old or too new).
    pub fn fit_problem(&self) -> bool {
        matches!(self.fit, Some(HostFit::TooOld | HostFit::TooNew))
    }

    /// Anything a person should look at: a mismatch, a live host that does not answer, a link that
    /// was lost, a host whose program has ended.
    pub fn needs_attention(&self, app: &AppFacts) -> bool {
        self.version_differs(app)
            || self.fit_problem()
            || (self.standing.is_live() && matches!(self.answer, Answer::No(_) | Answer::Incompatible))
            || self.link.is_some_and(|l| l.lost)
            || matches!(&self.answer, Answer::Yes(info) if info.exited)
    }

    /// May the host be ended: it runs (stale files are forgotten instead).
    pub fn can_end(&self) -> bool {
        self.standing.is_live()
    }

    pub fn can_forget(&self) -> bool {
        !self.standing.is_live()
    }
}

/// How many hosts stand each way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub attached: usize,
    pub detached: usize,
    pub orphan: usize,
    pub stale: usize,
}

impl Counts {
    pub fn live(&self) -> usize {
        self.attached + self.detached + self.orphan
    }
}

/// The whole window's data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub app: AppFacts,
    pub hosts_dir: PathBuf,
    pub flag_on: bool,
    pub host_binary: Option<PathBuf>,
    pub now: u64,
    /// Live hosts first (oldest first), then the stale ones by id.
    pub rows: Vec<HostRow>,
}

impl Report {
    pub fn build(inputs: Inputs) -> Self {
        let Inputs { app, hosts_dir, flag_on, host_binary, scan, observed, attachments, named, now } = inputs;
        let held: HashMap<&str, &Attachment> = attachments.iter().map(|a| (a.host_id.as_str(), a)).collect();
        let none = Observed::default();
        let mut rows = Vec::with_capacity(scan.live.len() + scan.stale.len());
        for meta in &scan.live {
            let seen = observed.get(&meta.id).unwrap_or(&none);
            let hold = held.get(meta.id.as_str()).copied();
            let standing = live_standing(&meta.id, hold, &named);
            rows.push(row_of(meta, standing, seen, hold.and_then(|a| a.link), &hosts_dir, now));
        }
        for stale in &scan.stale {
            let seen = observed.get(&stale.id).unwrap_or(&none);
            let standing = Standing::Stale(stale.reason.clone());
            let link = held.get(stale.id.as_str()).and_then(|a| a.link);
            rows.push(match &stale.meta {
                Some(meta) => row_of(meta, standing, seen, link, &hosts_dir, now),
                None => HostRow::bare(&stale.id, standing, &hosts_dir, link),
            });
        }
        Self { app, hosts_dir, flag_on, host_binary, now, rows }
    }

    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for row in &self.rows {
            match row.standing {
                Standing::Attached { .. } => counts.attached += 1,
                Standing::Detached => counts.detached += 1,
                Standing::Orphan => counts.orphan += 1,
                Standing::Stale(_) => counts.stale += 1,
            }
        }
        counts
    }

    pub fn row(&self, id: &str) -> Option<&HostRow> {
        self.rows.iter().find(|r| r.id == id)
    }
}

impl HostRow {
    /// A host of which only a name is left (a socket with no meta file, or a meta file that does not read).
    fn bare(id: &str, standing: Standing, hosts_dir: &Path, link: Option<LinkStats>) -> Self {
        Self {
            id: id.to_string(),
            standing,
            host_pid: None,
            shell_pid: None,
            program: None,
            cwd: None,
            started_at: None,
            uptime_secs: None,
            host_version: None,
            proto: None,
            fit: None,
            socket: None,
            meta_path: meta_path(hosts_dir, id),
            answer: Answer::NotAsked,
            rss: None,
            link,
        }
    }
}

/// How a live host stands: held by a terminal of this app, named by a row but not held, or nobody's.
fn live_standing(id: &str, held: Option<&Attachment>, named: &HashSet<String>) -> Standing {
    match held {
        Some(a) if a.link.is_none_or(|l| !l.detached && !l.ended) => {
            Standing::Attached { window: a.window, title: a.title.clone() }
        }
        Some(_) => Standing::Detached,
        None if named.contains(id) => Standing::Detached,
        None => Standing::Orphan,
    }
}

fn row_of(
    meta: &HostMeta,
    standing: Standing,
    seen: &Observed,
    link: Option<LinkStats>,
    hosts_dir: &Path,
    now: u64,
) -> HostRow {
    let live = standing.is_live();
    let answer = match (&seen.probed, live) {
        (_, false) | (None, _) => Answer::NotAsked,
        (Some(Probed::Answered(info)), _) => Answer::Yes(HostInfo {
            version: version_text(info.host_version),
            proto: info.proto,
            proto_minor: info.proto_minor,
            exited: info.state == HostState::Exited,
            cols: info.size.cols,
            rows: info.size.rows,
            history_lines: info.counters.history_lines,
            alt_screen: info.counters.alt_screen,
            revision: info.counters.revision,
        }),
        (Some(Probed::Incompatible { .. }), _) => Answer::Incompatible,
        (Some(Probed::Failed(why)), _) => Answer::No(why.clone()),
    };
    HostRow {
        id: meta.id.clone(),
        standing,
        host_pid: Some(meta.pid),
        shell_pid: meta.child_pid,
        program: seen.program.clone(),
        cwd: seen.cwd.clone(),
        started_at: Some(meta.started_at),
        uptime_secs: live.then(|| now.saturating_sub(meta.started_at)),
        host_version: Some(meta.host_version.clone()),
        proto: Some((meta.proto_min, meta.proto_max)),
        // A host that is not running fits nothing: it cannot be attached to.
        fit: live.then(|| Range::current().fit(Range::new(meta.proto_min, meta.proto_max))),
        socket: Some(meta.socket.clone()),
        meta_path: meta_path(hosts_dir, &meta.id),
        answer,
        rss: seen.rss,
        link,
    }
}

/// `1.6.2`, the way a meta file writes a version.
pub fn version_text(v: AppVersion) -> String {
    format!("{}.{}.{}", v.major, v.minor, v.patch)
}
