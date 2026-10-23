//! `wormhole-host`: one small process per terminal that owns its PTY, its program and the
//! authoritative screen, so the terminal outlives the app (`../repo-docs/docs/session-host.md`).
//!
//! The app starts it as `wormhole-host --id <id> --dir <hosts dir>` with `posix_spawn` and its own
//! session, and a readiness pipe on descriptor 3. The host binds `<hosts dir>/<id>.sock` (or the
//! fallback `wormhole_term::hosted::paths` decides), writes `<id>.meta`, and waits for a
//! controller to `Spawn` the program; then it serves up to four connections until it is closed,
//! killed, sent SIGTERM, or its program exited and nobody came for a week.
//!
//! - `session`: everything the host decides, as a state machine without I/O or a clock of its
//!   own (tested in its own tests with a scripted terminal and a virtual clock);
//! - `program`: the seam between the session and the terminal (`wormhole_term::hosted`);
//! - `serve`: the socket, the connections and the clock around the session;
//! - `listen`: the socket file and its directory; `sys`: start-up hygiene, signals, the peer's
//!   code signature, QoS; `log`: the log file; `args`: the command line.
//!
//! **What a client says, in order.** Spawning (the app, right after starting the host): connect
//! (retry while the socket is not there yet; the meta file appears once it listens), `Hello{role:
//! Controller}` → `Welcome` (pid 0: nothing runs yet), `Spawn` → `Info` (the program's pid and
//! size) or `Error{SPAWN_FAILED}`, and the readiness pipe says the same (`ok <pid>` / `err
//! <text>`). Then `Attach{Raw}` streams `Out` from offset 0, the program's first byte. Attaching
//! later: `Hello` → `Welcome` (pid, size, counters, `Exited` state if so), then `Attach{Sync}`:
//! `ResizeApplied` (the size to give the replica), `Event{RepaintSeeds}`, the `Repaint` parts
//! (`first` .. `last`), then `Out` from the repaint's offset; `Attach{Raw}` from a connection that
//! did not spawn streams from now on, with no repaint; `Attach{None}` (headless) gets events,
//! resizes and `Counters` only. All along: `Ack{offset}` opens the window, `Input` →
//! `InputAck`, `Resize` → `ResizeApplied` in the stream, `Detached{TooSlow}` means attach again.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::disallowed_methods, reason = "tests build their own files and processes"))]

pub mod args;
mod listen;
pub mod log;
mod peer;
pub mod program;
mod serve;
pub mod session;
mod sys;

use std::ffi::OsString;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use polling::Poller;
use wormhole_term::hosted::{
    self,
    meta::{self, Meta},
};
use wormhole_wire::{AppVersion, version::Range};

use listen::Listener;
use peer::Policy;
use program::PtySpawner;
use serve::{Files, Wakes};
use session::{Config, Session};
use sys::ready::ReadyPipe;

/// The host's version, as `Welcome` carries it.
pub fn host_version() -> AppVersion {
    let mut parts = env!("CARGO_PKG_VERSION").split('.').map(|p| p.parse::<u16>().unwrap_or(0));
    AppVersion {
        major: parts.next().unwrap_or(0),
        minor: parts.next().unwrap_or(0),
        patch: parts.next().unwrap_or(0),
    }
}

/// The whole program: `main` is this.
pub fn main(argv: impl IntoIterator<Item = OsString>) -> ExitCode {
    let args = match args::parse(argv.into_iter().skip(1)) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("wormhole-host: {err}");
            return ExitCode::from(2);
        }
    };
    // First, before anything opens a descriptor or starts a thread.
    let mut ready = ReadyPipe::take();
    let socket = hosted::paths::socket_path(&args.dir, &args.id);
    let level = log::level_from(std::env::var("WORMHOLE_LOG").ok().as_deref());
    sys::process::clear_environment();
    sys::process::hygiene();
    sys::signals::block_ending_signals();
    log::init(&log::path(&args.data_dir(), &args.id), level);
    std::panic::set_hook(Box::new(|info| {
        // `panic = abort` for the host: a half-broken host must not keep a terminal half-served.
        // The location only: a message may hold what the program printed.
        let at = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        tracing::error!(at, "panicked, aborting");
        std::process::abort();
    }));
    match run(&args, &socket, &mut ready) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!("ending: {err:#}");
            ready.say(&format!("err {err:#}"));
            ExitCode::FAILURE
        }
    }
}

fn run(args: &args::Args, socket: &std::path::Path, ready: &mut ReadyPipe) -> anyhow::Result<()> {
    use anyhow::Context as _;
    let started = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    tracing::info!(id = args.id, version = env!("CARGO_PKG_VERSION"), "starting");
    listen::prepare_dir(&args.dir).context("the hosts directory")?;
    if socket.parent() != Some(args.dir.as_path()) {
        tracing::info!(
            len = socket.as_os_str().len(),
            "the hosts dir is too deep for a socket, using the fallback"
        );
    }
    let mut listener = Listener::bind(socket).context("binding the socket")?;
    let range = Range::current();
    let mut files = Files {
        hosts_dir: &args.dir,
        meta: Meta {
            id: args.id.clone(),
            pid: std::process::id(),
            child_pid: None,
            started_at: started.as_secs(),
            host_version: env!("CARGO_PKG_VERSION").to_owned(),
            proto_min: range.min,
            proto_max: range.max,
            socket: socket.to_owned(),
            data_dir: args.data_dir(),
        },
    };
    let result = serve_files(args, &mut listener, ready, &mut files, started.as_millis() as u64);
    listener.remove();
    meta::remove(&args.dir, &args.id);
    tracing::info!("ended");
    result
}

fn serve_files(
    args: &args::Args,
    listener: &mut Listener,
    ready: &mut ReadyPipe,
    files: &mut Files<'_>,
    started_ms: u64,
) -> anyhow::Result<()> {
    meta::write(&args.dir, &files.meta)?;
    // Once, from the host's own signature; the socket is bound but nobody has been accepted yet.
    let peers = Policy::detect();
    let poller = Arc::new(Poller::new()?);
    let wakes = Wakes::new(poller);
    let signals = wakes.clone();
    std::thread::Builder::new().name("signals".into()).spawn(move || {
        loop {
            if let Some(ending) = sys::signals::wait_ending_signal() {
                tracing::info!(?ending, "signal");
                signals.terminate();
            }
        }
    })?;
    let wake_from = wakes.clone();
    let wake: hosted::Wake = Arc::new(move || wake_from.wake());
    let config = Config::new(args.id.clone(), host_version(), started_ms);
    let spawner = PtySpawner { on_reader_start: sys::process::set_user_interactive };
    let mut session = Session::new(config, spawner, wake, Instant::now());
    serve::serve(&mut session, listener, &wakes, ready, files, &peers)?;
    if !ready.is_spent() {
        ready.say("err the host ended before a program was started");
    }
    // The program's thread finishes its hangup before the host exits (`Session` waited for it).
    drop(session);
    Ok(())
}
