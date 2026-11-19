//! The agent-access approvals' backend (`agent_access/`): core's question queue, the approvals in force and
//! the terminals a program may reach (`ipc::access`), with the answers that settle a question.
//!
//! The control socket's gate puts questions here (`AccessDesk::ask`, Rust-side: the socket thread never waits
//! for the person, its request waits on the channel the queue answers); Swift shows one at a time
//! (`next_question`) and answers it (`answer`). Nothing a caller says counts, and no request can change what
//! is held: only these methods do, and only the person's sheet reaches them.
//!
//! The desk and its questions are here; a mode widened outside Wormhole is `modes` (the rule) and `widening` (the
//! desk's looks, the question and its answer), a question as the sheet shows it `record`, what a request that is
//! not let in is told `replies`, the list in Settings `approvals`, the control folder's answers `scripted`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use wormhole_core::config::AgentAccess;
use wormhole_core::ipc::Decision;
use wormhole_core::ipc::access::{CapSet, Full, Grants, Need, Queue, Resolved, Scope, Scopes};
use wormhole_core::ipc::control::ASK_WAIT_S;
use wormhole_core::storage::paths::{DataRoot, ProfileId, ProfilePaths};
use wormhole_core::storage::store::{AgentGrantRow, Store};

mod approvals;
mod modes;
mod record;
mod replies;
mod scripted;
mod widening;

pub use approvals::{AccessApprovalRecord, AccessHow};
use replies::{access_off, declined, declined_terminal, not_answered, timed_out};

use super::access_records::{AccessAnswer, AccessHeldRecord, AccessQuestionRecord};
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
    /// The profile this desk holds approvals for: a request is judged by the desk of the window it reaches.
    profile: ProfilePaths,
    waker: Waker,
    /// The mode in force, and a widening nobody in Wormhole made (`modes`).
    modes: Mutex<modes::Modes>,
    /// This desk, for the threads that time its questions out.
    me: Weak<Self>,
    /// How long a question waits, in milliseconds: a minute, shorter in a test.
    wait_ms: AtomicU64,
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
        Ok(Arc::new_cyclic(|me| Self {
            inner: Mutex::new(Inner::default()),
            profile_dir: paths.dir().to_path_buf(),
            profile: paths,
            waker: Waker::new(listener),
            modes: Mutex::default(),
            me: me.clone(),
            wait_ms: AtomicU64::new(ASK_WAIT_S * 1000),
        }))
    }

    /// The question to put on screen now: the first not shown, and only when none is. Re-arms the wake.
    pub fn next_question(&self) -> Option<AccessQuestionRecord> {
        self.waker.rearm();
        let record = {
            let mut inner = locked(&self.inner);
            let pending = inner.queue.next_to_show()?;
            self.record_of(pending)
        };
        // A question nobody answers is no after a minute, and the sheet goes (the next wake finds it gone).
        let id = record.id;
        self.after_wait(move |desk| desk.time_out(id));
        Some(record)
    }

    /// Whether question `id` is still waiting for the person: false once it was answered, withdrawn or timed out.
    pub fn is_waiting(&self, id: u64) -> bool {
        // As `is_waiting_mode`: the read the window makes while this question is up re-arms the wake.
        self.waker.rearm();
        locked(&self.inner).queue.iter().any(|pending| pending.id == id)
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
    pub(crate) fn profile(&self) -> &ProfilePaths {
        &self.profile
    }

    /// What a mode that changed means for the questions waiting here (`agent_access/ask.rs` `settle_mode`): Off, or the
    /// Automation flag off (`None`), refuses every one with `agent_access_off`; Allowed while open lets every one in at once
    /// (there is nothing left to ask). Asking keeps them. The sheet on the screen goes with its question.
    pub(crate) fn settle_questions(&self, mode: Option<AgentAccess>) {
        let waiting = locked(&self.inner).queue.take_scope(SCOPE);
        for pending in waiting {
            match mode {
                Some(AgentAccess::Open) => pending.answer(|| Decision::Retry),
                _ => pending.answer(|| Decision::Refuse(access_off())),
            }
        }
        self.waker.notify();
    }

    /// The ids of the approvals in force for a while and of the stored ones, which a mode of Off ends the streams of.
    pub(crate) fn approval_ids(&self) -> (Vec<u64>, Vec<u64>) {
        let held = locked(&self.inner).grants.held().iter().map(|held| held.id).collect();
        let stored = Store::open_at(&self.profile_dir)
            .ok()
            .and_then(|store| store.agent_grants().ok())
            .map(|rows| rows.into_iter().filter_map(|row| u64::try_from(row.id).ok()).collect())
            .unwrap_or_default();
        (held, stored)
    }

    /// Question `id` was not answered in time: no, and not remembered.
    fn time_out(&self, id: u64) {
        let Some(pending) = locked(&self.inner).queue.take(id) else { return };
        let label = pending.resolved.label.clone();
        pending.answer(|| Decision::Refuse(timed_out(&label)));
        self.waker.notify();
    }

    /// A question waits this long from now on, not a minute.
    #[cfg(test)]
    pub(crate) fn wait_only(&self, wait: Duration) {
        self.wait_ms.store(u64::try_from(wait.as_millis()).unwrap_or(u64::MAX), Ordering::Relaxed);
    }

    /// Runs `then` on a thread of its own once the wait is over.
    fn after_wait(&self, then: impl FnOnce(&Self) + Send + 'static) {
        let (desk, wait) = (self.me.clone(), Duration::from_millis(self.wait_ms.load(Ordering::Relaxed)));
        let spawned = std::thread::Builder::new().name("access-wait".into()).spawn(move || {
            std::thread::sleep(wait);
            if let Some(desk) = desk.upgrade() {
                then(&desk);
            }
        });
        if let Err(err) = spawned {
            tracing::warn!("a question will not time out: {err}");
        }
    }

    /// Queues a question for `resolved` (or joins the one its program has open); `waiter` hears the decision.
    /// The control socket's gate calls this; the question's id.
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

    /// What the control socket's gate reads and updates of the desk: the approvals in force and the
    /// terminals programs may reach. One lock, so a decision sees both as one moment.
    pub(crate) fn with_state<R>(&self, read: impl FnOnce(&mut Grants, &mut Scopes) -> R) -> R {
        let mut inner = locked(&self.inner);
        let Inner { grants, scopes, .. } = &mut *inner;
        read(grants, scopes)
    }

    /// What a stored ("Always") approval of `key` allows, if the profile has one.
    pub(crate) fn stored_caps(&self, key: &wormhole_core::storage::store::AgentGrantKey) -> Option<CapSet> {
        let row = Store::open_at(&self.profile_dir).ok()?.agent_grant(key).ok().flatten()?;
        Some(CapSet::from_bits(row.caps))
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
}

fn stored_rows(profile_dir: &Path) -> Option<Vec<AgentGrantRow>> {
    // Not created by a look: a profile without a database has nothing stored.
    profile_dir.join("wormhole.db").is_file().then_some(())?;
    Store::open_at(profile_dir).ok()?.agent_grants().ok()
}
