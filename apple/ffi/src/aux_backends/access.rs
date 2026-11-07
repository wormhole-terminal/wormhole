//! The agent-access approvals' backend (`agent_access/`): core's question queue, the approvals in force and
//! the terminals a program may reach (`ipc::access`), with the answers that settle a question.
//!
//! The control socket's gate puts questions here (`AccessDesk::ask`, Rust-side: the socket thread never waits
//! for the person, its request waits on the channel the queue answers); Swift shows one at a time
//! (`next_question`) and answers it (`answer`). Nothing a caller says counts, and no request can change what
//! is held: only these methods do, and only the person's sheet reaches them.

use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use wormhole_core::ipc::Decision;
use wormhole_core::ipc::access::{
    CapSet, Capability, Full, Grants, Kind, Need, Pending, Queue, Resolved, Scope, Scopes, agent_word,
};
use wormhole_core::ipc::control::{Error, ErrorCode, Reason, Reply};
use wormhole_core::platform::codesign::SignerKind;
use wormhole_core::platform::{paths, process};
use wormhole_core::storage::paths::{DataRoot, ProfileId};
use wormhole_core::storage::store::{AgentGrantRow, Store};

use super::access_records::{
    AccessAnswer, AccessCapability, AccessHeldRecord, AccessKindRecord, AccessProgramRecord,
    AccessQuestionRecord, AccessSignerKind,
};
use super::common::{AuxError, AuxListener, Waker, locked, resolve_data_dir};

/// The scope the one profile of this desk stands for (the app keeps one per profile).
const SCOPE: u64 = 0;

#[derive(Default)]
struct Inner {
    queue: Queue,
    grants: Grants,
    scopes: Scopes,
}

#[derive(uniffi::Object)]
pub struct AccessDesk {
    inner: Mutex<Inner>,
    /// The profile's folder, where "Always" is stored.
    profile_dir: PathBuf,
    waker: Waker,
}

#[uniffi::export]
impl AccessDesk {
    /// A desk for `profile` (the default one when `None`) of the data folder.
    #[uniffi::constructor]
    pub fn new(
        profile: Option<String>,
        data_dir: Option<String>,
        listener: Arc<dyn AuxListener>,
    ) -> Result<Arc<Self>, AuxError> {
        let root = DataRoot::new(resolve_data_dir(data_dir)?);
        let paths = match profile {
            Some(id) => root.profile(&ProfileId::parse(&id).map_err(AuxError::failed)?),
            None => root.default_profile(),
        };
        Ok(Arc::new(Self {
            inner: Mutex::new(Inner::default()),
            profile_dir: paths.dir().to_path_buf(),
            waker: Waker::new(listener),
        }))
    }

    /// The question to put on screen now: the first not shown, and only when none is. Re-arms the wake.
    pub fn next_question(&self) -> Option<AccessQuestionRecord> {
        self.waker.rearm();
        let mut inner = locked(&self.inner);
        let pending = inner.queue.next_to_show()?;
        Some(self.record_of(pending))
    }

    /// Settles question `id` with the person's answer: what it means is recorded, every request that waited
    /// is told, and the next question can be taken.
    pub fn answer(&self, id: u64, answer: AccessAnswer) {
        let mut inner = locked(&self.inner);
        let Some(pending) = inner.queue.take(id) else { return };
        let resolved = pending.resolved.clone();
        let label = resolved.label.clone();
        match (answer, pending.about) {
            (AccessAnswer::Deny, Some(terminal)) => {
                inner.scopes.deny(resolved.identity, terminal, Instant::now());
                pending.answer(|| Decision::Refuse(declined_terminal(&label, terminal)));
            }
            (AccessAnswer::Deny, None) => {
                inner.grants.deny(resolved.identity, Instant::now());
                pending.answer(|| Decision::Refuse(declined(&label)));
            }
            (button, Some(terminal)) => {
                // A terminal is never approved for good: "for this session" while the program runs, else until quit.
                let scope = match (button, resolved.process) {
                    (AccessAnswer::Session, Some(process)) => Scope::Session(process),
                    _ => Scope::UntilQuit,
                };
                inner.scopes.widen(resolved.identity.clone(), scope, terminal, pending.caps);
                pending.answer(|| Decision::Retry);
            }
            (button, None) => {
                self.allow(&mut inner, &resolved, button);
                pending.answer(|| Decision::Retry);
            }
        }
        drop(inner);
        self.waker.notify();
    }

    /// The question went away without an answer (its window closed): it counts as no for now, not remembered.
    pub fn withdraw(&self, id: u64) {
        let Some(pending) = locked(&self.inner).queue.take(id) else { return };
        let label = pending.resolved.label.clone();
        pending.answer(|| Decision::Refuse(not_answered(&label)));
        self.waker.notify();
    }

    /// The approvals in force for a while (the stored ones are Settings').
    pub fn held(&self) -> Vec<AccessHeldRecord> {
        locked(&self.inner)
            .grants
            .held()
            .iter()
            .map(|h| AccessHeldRecord {
                id: h.id,
                label: h.label.clone(),
                until_quit: matches!(h.scope, Scope::UntilQuit),
            })
            .collect()
    }

    /// Takes approval `id` back. Whether there was one.
    pub fn revoke(&self, id: u64) -> bool {
        let revoked = locked(&self.inner).grants.revoke(id);
        if revoked {
            self.waker.notify();
        }
        revoked
    }

    /// How many questions wait (the one on the screen included).
    pub fn waiting(&self) -> u32 {
        u32::try_from(locked(&self.inner).queue.len()).unwrap_or(u32::MAX)
    }
}

impl AccessDesk {
    /// Queues a question for `resolved` (or joins the one its program has open); `waiter` hears the decision.
    /// The control socket's gate calls this; the question's id.
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "the control socket gate calls it once it is in this crate")
    )]
    pub(crate) fn ask(
        &self,
        resolved: Resolved,
        need: Need,
        about: Option<i64>,
        waiter: Option<SyncSender<Decision>>,
    ) -> Result<u64, Full> {
        let id = locked(&self.inner).queue.push(resolved, need, SCOPE, about, waiter)?;
        self.waker.notify();
        Ok(id)
    }

    /// Remembers that `resolved` was let in, as long as `button` says.
    fn allow(&self, inner: &mut Inner, resolved: &Resolved, button: AccessAnswer) {
        let caps = CapSet::STANDARD;
        let (identity, label) = (resolved.identity.clone(), resolved.label.clone());
        match (button, resolved.process) {
            (AccessAnswer::Session, Some(process)) => {
                inner.grants.grant(identity, label, Scope::Session(process), caps);
            }
            (AccessAnswer::Always, _) if identity.can_be_remembered() => {
                if !self.store_always(resolved, caps) {
                    // The database would not take it: it holds until Wormhole quits, which the person can see.
                    inner.grants.grant(identity, label, Scope::UntilQuit, caps);
                }
            }
            _ => {
                inner.grants.grant(identity, label, Scope::UntilQuit, caps);
            }
        }
    }

    fn store_always(&self, resolved: &Resolved, caps: CapSet) -> bool {
        let program = resolved.program.as_ref();
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0));
        let row = AgentGrantRow {
            key: resolved.identity.key(),
            label: resolved.label.clone(),
            location: program.map(|p| p.path.to_string_lossy().into_owned()).unwrap_or_default(),
            cdhash: program.and_then(|p| p.signature.cdhash.clone()).unwrap_or_default(),
            caps: caps.bits(),
            created,
            ..AgentGrantRow::default()
        };
        match Store::open_at(&self.profile_dir).and_then(|mut store| store.save_agent_grant(&row)) {
            Ok(_) => true,
            Err(err) => {
                tracing::error!("couldn't store an approval: {err:#}");
                false
            }
        }
    }

    fn record_of(&self, pending: &Pending) -> AccessQuestionRecord {
        let resolved = &pending.resolved;
        let capability = match pending.need {
            Need::Capability(Capability::Read) => AccessCapability::Read,
            Need::Capability(Capability::Type) => AccessCapability::Type,
            Need::Capability(Capability::Run) => AccessCapability::Run,
            Need::Capability(Capability::Open) => AccessCapability::Open,
            Need::Capability(Capability::Close) | Need::Approved | Need::Nothing => AccessCapability::Close,
        };
        let kind = match resolved.identity.kind {
            Kind::Scripts => AccessKindRecord::Scripts,
            Kind::Script => AccessKindRecord::Script,
            Kind::Unsigned => AccessKindRecord::Unsigned,
            Kind::AdHoc => AccessKindRecord::AdHoc,
            Kind::DeveloperId => AccessKindRecord::DeveloperId,
            Kind::Apple => AccessKindRecord::Apple,
        };
        let program = resolved.program.as_ref().map(|program| AccessProgramRecord {
            path: shown_path(&program.path),
            pid: program.pid,
            signer: match program.signature.kind {
                SignerKind::DeveloperId => AccessSignerKind::DeveloperId,
                SignerKind::AppleSystem => AccessSignerKind::AppleSystem,
                SignerKind::AdHoc => AccessSignerKind::AdHoc,
                SignerKind::Other => AccessSignerKind::Other,
                SignerKind::Unsigned => AccessSignerKind::Unsigned,
                SignerKind::Unreadable => AccessSignerKind::Unreadable,
            },
            subject: program.signature.subject.clone(),
            team: program.signature.team_id.clone(),
            interpreter: program.interpreter.clone(),
            started_by: resolved.started_by.clone().or_else(|| parent_name(program.pid)),
        });
        AccessQuestionRecord {
            id: pending.id,
            label: resolved.label.clone(),
            capability,
            kind,
            program,
            about_terminal: pending.about,
            unusual_signer: resolved.unusual_signer,
            changed_since_allowed: self.changed_since_allowed(resolved),
            elevated: resolved.elevated,
            can_be_remembered: resolved.identity.can_be_remembered() && pending.about.is_none(),
        }
    }

    /// Whether the same agent was allowed before as something else: another signer, another build, another
    /// install (stored approvals only: what "for this session" allowed ended with the process).
    fn changed_since_allowed(&self, resolved: &Resolved) -> bool {
        let Some(agent) = resolved.identity.agent else { return false };
        let key = resolved.identity.key();
        let word = agent_word(agent);
        stored_rows(&self.profile_dir)
            .is_some_and(|rows| rows.iter().any(|r| r.key.agent == word && r.key != key))
    }
}

fn stored_rows(profile_dir: &Path) -> Option<Vec<AgentGrantRow>> {
    // Not created by a look: a profile without a database has nothing stored.
    profile_dir.join("wormhole.db").is_file().then_some(())?;
    Store::open_at(profile_dir).ok()?.agent_grants().ok()
}

/// A path as the title row writes it: the home folder as `~`.
fn shown_path(path: &Path) -> String {
    match paths::home().as_deref().and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// The program that started `pid`, by name.
fn parent_name(pid: u32) -> Option<String> {
    let path = process::executable(process::parent(pid)?)?;
    path.file_name().map(|name| name.to_string_lossy().into_owned())
}

// What a request that waited is told (`agent_access/ask.rs`): agents read these, so they say not to retry.
fn declined(label: &str) -> Reply {
    Error::new(
        ErrorCode::NotAllowed,
        format!("the user did not allow {label} to control Wormhole: do not retry; tell the user what you wanted to do and ask them"),
    )
    .because(Reason::Declined)
    .into()
}

fn declined_terminal(label: &str, terminal: i64) -> Reply {
    Error::new(
        ErrorCode::NotAllowed,
        format!("the user did not allow {label} to use terminal {terminal}, which it did not open: do not retry; tell the user what you wanted to do there and ask them"),
    )
    .because(Reason::Declined)
    .into()
}

fn not_answered(label: &str) -> Reply {
    Error::new(
        ErrorCode::NotAllowed,
        format!("the question about {label} went away without an answer (its window closed, or none could be shown); it counts as no for now, and is not remembered"),
    )
    .because(Reason::NotAnswered)
    .into()
}
