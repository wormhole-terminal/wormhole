//! The Session Hosts window's backend: core's survey of the hosts folder, classified (`ipc::hosts::report`),
//! said in the catalog's words, with End and Forget (`ipc::hosts::{end_host, forget}`) and a watch of the
//! folder that wakes Swift when a host's files change. The report blocks (files, sockets: each host gets
//! 400 ms to answer), so Swift calls it off the main thread.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use wormhole_core::config::profile::ProfileList;
use wormhole_core::ipc::host_binary::{HostProgram, host_program};
use wormhole_core::ipc::hosts::report::{Answer, AppFacts, Attachment, HostRow, Inputs, Report, Standing};
use wormhole_core::ipc::hosts::{self, Survey};
use wormhole_core::platform::watch::{self as os_watch, Watch};
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::storage::store::Store;
use wormhole_text::keys::{HOSTS_UNKNOWN, hosts_protocol_minor, hosts_version_differs};
use wormhole_text::text;

use super::common::{AuxError, AuxListener, Waker, locked, resolve_data_dir};
use super::hosts_words as words;

/// How long one host gets to answer `GetInfo` (`hosts_window/gather.rs`).
const ANSWER_WITHIN: Duration = Duration::from_millis(400);
/// How long ending a host waits for it to hang up (`hosts_window/mod.rs`).
const END_WAIT: Duration = Duration::from_secs(5);

/// A terminal of this app that runs in a host.
#[derive(Debug, Clone, uniffi::Record)]
pub struct HostAttachmentRecord {
    pub host_id: String,
    pub window: i64,
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum HostStandingRecord {
    Attached,
    Detached,
    Orphan,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum HostFieldRecord {
    HostPid,
    ShellPid,
    Program,
    Memory,
    Version,
    ProtocolRange,
    Started,
    Answer,
    Link,
    Folder,
    Socket,
    Meta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum HostToneRecord {
    Normal,
    Muted,
    Warning,
    Danger,
}

/// One fact of a card, its value already said.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct HostFactRecord {
    pub field: HostFieldRecord,
    pub value: String,
    pub tone: HostToneRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct HostCardRecord {
    pub id: String,
    pub standing: HostStandingRecord,
    pub note: String,
    pub facts: Vec<HostFactRecord>,
    pub attached_title: Option<String>,
    pub needs_attention: bool,
}

/// What the host program is: found at `program_path`, or why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum HostProgramKind {
    Found,
    DisposableCopy,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct HostsReportRecord {
    pub app_version: String,
    pub protocol_text: String,
    pub folder: String,
    pub flag_on: bool,
    pub program: HostProgramKind,
    pub program_path: Option<String>,
    pub cards: Vec<HostCardRecord>,
}

struct Facts {
    flag_on: bool,
    attachments: Vec<HostAttachmentRecord>,
}

#[derive(uniffi::Object)]
pub struct HostsDesk {
    data_dir: PathBuf,
    hosts_dir: PathBuf,
    facts: Mutex<Facts>,
    waker: Arc<Waker>,
    watch: Mutex<Option<Watch>>,
}

#[uniffi::export]
impl HostsDesk {
    /// `data_dir`: a folder named `Wormhole-swift*` (tests), or `None` for core's own, which must be the
    /// experiment's.
    #[uniffi::constructor]
    pub fn new(data_dir: Option<String>, listener: Arc<dyn AuxListener>) -> Result<Arc<Self>, AuxError> {
        let data_dir = resolve_data_dir(data_dir)?;
        Ok(Arc::new(Self {
            hosts_dir: hosts::hosts_dir(&data_dir),
            data_dir,
            facts: Mutex::new(Facts { flag_on: false, attachments: Vec::new() }),
            waker: Arc::new(Waker::new(listener)),
            watch: Mutex::new(None),
        }))
    }

    /// The terminals of the app's windows that hold a host (the window knows; the folder does not).
    pub fn set_attachments(&self, attachments: Vec<HostAttachmentRecord>) {
        locked(&self.facts).attachments = attachments;
    }

    /// Whether the persistent-terminals feature is on (only said in the header).
    pub fn set_persistent_terminals(&self, on: bool) {
        locked(&self.facts).flag_on = on;
    }

    /// Follows the hosts folder: a wake when a host's files change. `false` where it cannot (a missing folder,
    /// a network volume): the window keeps its slow look.
    pub fn follow(&self) -> bool {
        if let Err(err) = hosts::ensure_hosts_dir(&self.data_dir) {
            tracing::warn!("hosts folder: {err}");
            return false;
        }
        let Ok(base) = self.hosts_dir.canonicalize() else { return false };
        let waker = Arc::clone(&self.waker);
        match os_watch::watch(&[base.as_path()], move |_| waker.notify()) {
            Ok(watch) => {
                *locked(&self.watch) = Some(watch);
                true
            }
            Err(err) => {
                tracing::info!("hosts folder not followed: {err}");
                false
            }
        }
    }

    pub fn unfollow(&self) {
        *locked(&self.watch) = None;
    }

    /// The whole window's data now; the next change wakes again. Blocks: off the main thread.
    pub fn report(&self) -> HostsReportRecord {
        self.waker.rearm();
        let facts = locked(&self.facts);
        let (flag_on, attachments) = (facts.flag_on, facts.attachments.clone());
        drop(facts);
        let survey = hosts::survey(&self.hosts_dir, ANSWER_WITHIN).unwrap_or_else(|err| {
            tracing::warn!("couldn't look at the session hosts: {err}");
            Survey::default()
        });
        let report = Report::build(Inputs {
            app: AppFacts::this_build(),
            hosts_dir: self.hosts_dir.clone(),
            flag_on,
            host_program: host_program(),
            scan: survey.scan,
            observed: survey.observed,
            attachments: attachments
                .into_iter()
                .map(|a| Attachment {
                    host_id: a.host_id,
                    window: a.window,
                    title: a.title,
                    link: None,
                    unavailable: None,
                })
                .collect(),
            named: self.named_hosts(),
            now: SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()),
        });
        record_of(&report)
    }

    /// Ends host `id`: asks it over its socket, signals its process when that fails. The way it ended.
    pub fn end(&self, id: String) -> Result<String, AuxError> {
        let meta = hosts::find(&self.hosts_dir, &id).map_err(AuxError::failed)?;
        let how = hosts::end_host(&meta, END_WAIT).map_err(AuxError::failed)?;
        self.waker.notify();
        Ok(how.word().to_string())
    }

    /// Removes the files of a stale host `id`; core refuses one that may be alive.
    pub fn forget(&self, id: String) -> Result<(), AuxError> {
        hosts::forget(&self.hosts_dir, &id).map_err(AuxError::failed)?;
        self.waker.notify();
        Ok(())
    }
}

impl HostsDesk {
    /// Which hosts a saved terminal names, in every profile: an unnamed host is an orphan.
    fn named_hosts(&self) -> std::collections::HashSet<String> {
        let root = DataRoot::new(&self.data_dir);
        let (list, _) = ProfileList::load_or_recover(&root);
        list.profiles().iter().flat_map(|p| Store::host_ids_read_only(&root.profile(&p.id))).collect()
    }
}

fn record_of(report: &Report) -> HostsReportRecord {
    let (program, program_path) = match &report.host_program {
        HostProgram::Found(path) => (HostProgramKind::Found, Some(path.display().to_string())),
        HostProgram::DisposableCopy => (HostProgramKind::DisposableCopy, None),
        HostProgram::Missing => (HostProgramKind::Missing, None),
    };
    HostsReportRecord {
        app_version: report.app.version.clone(),
        protocol_text: hosts_protocol_minor(
            &report.app.proto_major.to_string(),
            &report.app.proto_minor.to_string(),
        ),
        folder: report.hosts_dir.display().to_string(),
        flag_on: report.flag_on,
        program,
        program_path,
        cards: report.rows.iter().map(|row| card_of(row, &report.app)).collect(),
    }
}

fn card_of(row: &HostRow, app: &AppFacts) -> HostCardRecord {
    use HostFieldRecord as F;
    use HostToneRecord::{Danger, Muted, Normal, Warning};
    let unknown = || text(HOSTS_UNKNOWN).to_string();
    let number = |n: Option<u32>| n.map_or_else(unknown, |n| n.to_string());
    let path = |p: &Option<PathBuf>| p.as_ref().map_or_else(unknown, |p| words::path(p));
    let mismatch = row.version_differs(app);
    let version = match (&row.host_version, mismatch) {
        (Some(v), true) => format!("{v} ({})", hosts_version_differs(&app.version)),
        (Some(v), false) => v.clone(),
        (None, _) => unknown(),
    };
    let bad_answer = row.standing.is_live() && matches!(row.answer, Answer::No(_) | Answer::Incompatible);
    let exited = matches!(&row.answer, Answer::Yes(info) if info.exited);
    let fact = |field, value: String, tone| HostFactRecord { field, value, tone };
    let mut facts = vec![
        fact(F::HostPid, number(row.host_pid), Normal),
        fact(F::ShellPid, number(row.shell_pid), Normal),
        fact(F::Program, row.program.clone().unwrap_or_else(unknown), Normal),
        fact(F::Memory, row.rss.map_or_else(unknown, words::size_text), Normal),
        fact(F::Version, version, if mismatch { Warning } else { Normal }),
        fact(
            F::ProtocolRange,
            words::protocol(row).unwrap_or_else(unknown),
            if row.fit_problem() { Danger } else { Normal },
        ),
        fact(F::Started, words::started(row).unwrap_or_else(unknown), Normal),
        fact(
            F::Answer,
            words::answer(&row.answer),
            if bad_answer {
                Danger
            } else if exited {
                Warning
            } else {
                Normal
            },
        ),
    ];
    if matches!(row.standing, Standing::Attached { .. }) {
        facts.push(fact(F::Link, words::no_link(), Normal));
    }
    facts.push(fact(F::Folder, path(&row.cwd), Normal));
    facts.push(fact(F::Socket, path(&row.socket), Muted));
    facts.push(fact(F::Meta, words::path(&row.meta_path), Muted));
    let (standing, attached_title) = match &row.standing {
        Standing::Attached { title, .. } => (HostStandingRecord::Attached, Some(title.clone())),
        Standing::Detached => (HostStandingRecord::Detached, None),
        Standing::Orphan => (HostStandingRecord::Orphan, None),
        Standing::Stale(_) => (HostStandingRecord::Stale, None),
    };
    HostCardRecord {
        id: row.id.clone(),
        standing,
        note: words::standing_note(&row.standing),
        facts,
        attached_title,
        needs_attention: row.needs_attention(app),
    }
}
