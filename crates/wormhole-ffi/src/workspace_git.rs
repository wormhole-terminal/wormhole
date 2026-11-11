//! The I/O of the git feed: what `WorkspaceModel::git_poll` asks for is run here, and what comes back
//! goes into the model. The rules (when to read, which answer is stale, whose changes are whose) are
//! core's `workspace::git_feed`; git itself is only ever spoken to through core's hardened read-only
//! access (`git::status_summary`, whose `GitCmd` scrubs the environment, takes no locks and times out).
//!
//! Reads run on two worker threads, so one slow repository cannot hold up another; the pump thread
//! asks the model what is due (`step`) and follows the repositories' files (`workspace_watch`).

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};
use std::time::Instant;

use wormhole_core::git::attribution::Snapshot;
use wormhole_core::git::{self, discover};
use wormhole_core::workspace::GitJob;

use crate::workspace::Shared;
use crate::workspace_watch::Watches;

/// How many git processes run at once, at most, for one workspace.
const WORKERS: usize = 2;

/// What the workspace holds of the git feed's I/O.
#[derive(Default)]
pub(crate) struct GitJobs {
    pool: OnceLock<Sender<GitJob>>,
    watches: Mutex<Watches>,
}

impl GitJobs {
    fn run(&self, shared: &Arc<Shared>, job: GitJob) {
        let sender = self.pool.get_or_init(|| spawn_pool(Arc::downgrade(shared)));
        // A closed pool (no thread could be made) leaves the read owed: it is asked for again.
        let _ = sender.send(job);
    }
}

fn spawn_pool(shared: Weak<Shared>) -> Sender<GitJob> {
    let (sender, receiver) = channel();
    let receiver = Arc::new(Mutex::new(receiver));
    for n in 0..WORKERS {
        let (shared, receiver) = (shared.clone(), receiver.clone());
        let spawned = std::thread::Builder::new()
            .name(format!("wormhole-ffi-git-{n}"))
            .spawn(move || work(&shared, &receiver));
        if let Err(err) = spawned {
            eprintln!("wormhole-ffi: no git worker: {err}");
        }
    }
    sender
}

fn work(shared: &Weak<Shared>, jobs: &Mutex<Receiver<GitJob>>) {
    loop {
        let job = jobs.lock().unwrap_or_else(PoisonError::into_inner).recv();
        let (Ok(job), Some(shared)) = (job, shared.upgrade()) else { return };
        match job {
            GitJob::Read { id, cwd, wanted } => {
                // Superseded while it waited its turn: the newer one is the read to make.
                if !wanted.still_wanted() {
                    continue;
                }
                let status = git::status_summary(&cwd, None);
                // The picture of the disk is dated by when it was read, in the same hop as the status.
                let snapshot = status
                    .as_ref()
                    .ok()
                    .and_then(Option::as_ref)
                    .and_then(|status| Snapshot::take(status, Instant::now()));
                let events = shared.with_model(|model| model.git_read(id, &wanted, status, snapshot));
                shared.post(events);
            }
            GitJob::Probe { id, cwd } => {
                let found = discover::probe(&cwd);
                let now = shared.now();
                shared.with_model(|model| model.git_probed(id, &cwd, found, now));
            }
        }
        // The answer may have started another read (a `.git` confirmed) or moved the next deadline.
        shared.pump.kick();
    }
}

/// What is due for git at this moment: follow the repositories' files, start the reads and looks the
/// model asks for. When the next thing is due, on the workspace's clock.
pub(crate) fn step(shared: &Arc<Shared>) -> Option<u64> {
    let roots = shared.with_model(|model| model.git_roots());
    let synced = shared.git.watches.lock().unwrap_or_else(PoisonError::into_inner).sync(
        &roots,
        &shared.pump,
        Instant::now(),
    );
    shared.with_model(|model| {
        for (root, live) in &synced.live {
            model.git_watched(root, *live);
        }
        for root in &synced.changed {
            model.git_repo_changed(root);
        }
    });
    let now = shared.now();
    let jobs = shared.with_model(|model| model.git_poll(now, Instant::now()));
    for job in jobs {
        shared.git.run(shared, job);
    }
    let debounce = synced.due.map(|due| {
        now + u64::try_from(due.saturating_duration_since(Instant::now()).as_millis()).unwrap_or(0)
    });
    [shared.with_model(|model| model.git_next_due(now)), debounce].into_iter().flatten().min()
}
