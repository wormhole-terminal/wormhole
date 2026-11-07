//! The constructors of a hosted [`Pty`], and what only a hosted terminal does.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Context as _;
use polling::Poller;
use wormhole_wire::{self as wire, StreamMode};

use super::error::AttachError;
use super::launch::{self, Ready};
use super::link::{Hosted, Link};
use super::replica::{ReplicaOptions, grid_of, wire_colors, wire_size};
use super::socket::{self, Joined};
use super::thread::{Client, State};
use super::{READY_TIMEOUT, START_TIMEOUT};
use crate::engine::OwnFeed;
use crate::events::EventProxy;
use crate::hosted::{meta, paths};
use crate::pty::{self, FairLock, Sink};
use crate::sys::detached::Signal;
use crate::sys::socket::Stream;
use crate::{Backend, GridSize, Launch, Pty, PtyEvents, StartOptions};

/// How to start a session host: its binary (inside the app bundle), the folder hosts keep their
/// sockets and meta files in (`<data dir>/hosts`), and the new host's 12-character id.
#[derive(Clone, Copy, Debug)]
pub struct HostCommand<'a> {
    pub binary: &'a Path,
    pub hosts_dir: &'a Path,
    pub id: &'a str,
}

impl Pty {
    /// Start `launch` in a new session host: the host process (see `client::launch`), then the
    /// program in it with `options` and `size`, then the stream from the program's first byte.
    ///
    /// The program's environment is the app's with `launch.env` over it, as a local terminal's
    /// is, all of it sent to the host (which adds nothing). `options.restore`, the text of a
    /// previous session whose host is gone, is the host's seed: it draws it dimmed with the "restored"
    /// marker before the program's first byte. A host that did (protocol 1.4 or newer) has it, and the
    /// replica draws the same so the first stream byte lands under it; an older host skips the seed, and
    /// then the replica does not draw it either, or the two would differ. Blocks until the program runs or could not be
    /// started (a few seconds at most); the error then says why, in the words the "could not
    /// start" view shows.
    pub fn start_hosted(
        host: &HostCommand<'_>,
        size: GridSize,
        options: &StartOptions,
        launch: Launch,
    ) -> anyhow::Result<(Self, PtyEvents)> {
        anyhow::ensure!(meta::valid_id(host.id), "not a session host id: {:?}", host.id);
        let program = launch.shell.program.clone();
        let started = launch::start(host.binary, host.hosts_dir, host.id)
            .with_context(|| format!("could not start the session host {}", host.binary.display()))?;
        let mut started = started;
        match spawn_in(host, &mut started, size, options, &launch) {
            Ok((joined, info)) => {
                tracing::info!(
                    id = host.id,
                    host_pid = started.pid(),
                    pid = info.pid,
                    "hosted terminal started"
                );
                let replica_size = GridSize { cols: info.size.cols, rows: info.size.rows, ..size };
                hosted(
                    joined,
                    host.id,
                    size,
                    replica_size,
                    options,
                    StreamMode::Raw,
                    HostFacts {
                        pid: info.pid,
                        minor: info.proto_minor,
                        features: info.features,
                        replay: seed_drawn_by(options.restore, info.proto_minor),
                    },
                )
            }
            Err(err) => {
                // A host that came up without its program would wait for a `Spawn` until it gives
                // up; it is ours, so it goes now (one that ended already is a zombie until
                // `started` is dropped, so the signal cannot reach anyone else).
                started.signal(Signal::Terminate);
                Err(err.context(format!("could not start {program}")))
            }
        }
    }

    /// Attach to the running host with `id` at `socket` (from its meta file: see
    /// [`client::socket_of`](super::socket_of)), as its controller: with a replica fed from a
    /// repaint when `visible`, headless otherwise ([`set_visible`](Self::set_visible) switches).
    /// `options` must be the ones the terminal was started with (the replica's history and what
    /// the program may do to the window are the host's). The terminal's size is the host's until
    /// the app [`resize`](Self::resize)s it.
    pub fn attach_hosted(
        socket: &Path,
        id: &str,
        options: &StartOptions,
        visible: bool,
    ) -> Result<(Self, PtyEvents), AttachError> {
        if !meta::valid_id(id) {
            return Err(AttachError::Failed(format!("not a session host id: {id:?}")));
        }
        let stream = socket::connect(socket).map_err(|e| AttachError::from_connect(&e))?;
        let mut dec = wire::Decoder::new();
        let until = Instant::now() + READY_TIMEOUT;
        let welcome = socket::hello(&stream, &mut dec, id, until)?;
        let size = grid_of(welcome.size);
        let first = if visible { StreamMode::Sync } else { StreamMode::None };
        tracing::info!(id, pid = welcome.pid, visible, "attaching to a hosted terminal");
        hosted(
            Joined { stream, dec },
            id,
            size,
            size,
            options,
            first,
            // An attach never replays saved text: the host's screen is what it shows, and its repaint has it.
            HostFacts {
                pid: welcome.pid,
                minor: welcome.proto_minor,
                features: welcome.features,
                replay: None,
            },
        )
        .map_err(|e| AttachError::Failed(format!("{e:#}")))
    }

    /// Whether this terminal's program runs in a session host.
    pub fn is_hosted(&self) -> bool {
        matches!(self.io, Backend::Hosted(_))
    }

    /// The session host's id, for a hosted terminal.
    pub fn host_id(&self) -> Option<&str> {
        match &self.io {
            Backend::Hosted(host) => Some(&host.id),
            Backend::Local(_) => None,
        }
    }

    /// The connection's counters and state, for a hosted terminal (the Session Hosts window).
    pub fn link_stats(&self) -> Option<super::LinkStats> {
        match &self.io {
            Backend::Hosted(host) => Some(host.stats()),
            Backend::Local(_) => None,
        }
    }

    /// The protocol minor the host said it speaks, for a hosted terminal: what it understands of
    /// the messages added after the first minor.
    pub fn host_minor(&self) -> Option<u16> {
        match &self.io {
            Backend::Hosted(host) => Some(host.minor()),
            Backend::Local(_) => None,
        }
    }

    /// Ask a hosted terminal's host to keep a checkpoint of the terminal for a restart of the Mac, sealed with
    /// `key`. `false` when the terminal is not hosted or its host is too old to do it; the host writes nothing
    /// until it is asked.
    pub fn start_checkpoints(&self, key: &crate::checkpoint::Key) -> bool {
        match &self.io {
            Backend::Hosted(host) if host.can_checkpoint() => {
                host.checkpoint_key(Some(key));
                true
            }
            _ => false,
        }
    }

    /// Tell the host to stop keeping checkpoints (the feature was turned off): it writes nothing more. `false` as
    /// for [`start_checkpoints`](Self::start_checkpoints).
    pub fn stop_checkpoints(&self) -> bool {
        match &self.io {
            Backend::Hosted(host) if host.can_checkpoint() => {
                host.checkpoint_key(None);
                true
            }
            _ => false,
        }
    }

    /// Ask the host to write a checkpoint now. With `flush` the host says when it is on disk
    /// ([`PtyEvent::CheckpointWritten`](crate::PtyEvent::CheckpointWritten)), whatever the write budget and the
    /// quiet rule say; without it, only when something was printed since the last. `false` as for
    /// [`start_checkpoints`](Self::start_checkpoints).
    pub fn checkpoint_now(&self, flush: bool) -> bool {
        match &self.io {
            Backend::Hosted(host) if host.can_checkpoint() => {
                host.checkpoint(flush);
                true
            }
            _ => false,
        }
    }

    /// Whether the host forgets history when told to (`ClearHistory`, protocol minor 2): `Some(false)`
    /// for an older host, which ignores the message, so the history it keeps comes back at the
    /// next attach. `None` for a terminal that is not hosted.
    pub fn host_forgets_history(&self) -> Option<bool> {
        self.host_minor().map(|minor| minor >= super::CLEAR_HISTORY_MINOR)
    }

    /// Let go of the session host and leave the program running in it, for the next
    /// [`attach_hosted`](Self::attach_hosted) (the app does this for every hosted terminal when it
    /// quits). Dropping the terminal afterwards does not end it. Whether there was a host to let go
    /// of.
    pub fn detach(&self) -> bool {
        match &self.io {
            Backend::Hosted(host) => {
                host.detach();
                true
            }
            Backend::Local(_) => false,
        }
    }

    /// On screen (the replica is fed: a repaint now, then the stream) or not (headless: events,
    /// the size and a revision that says the program printed; the screen keeps what it last
    /// showed). Nothing for a local terminal.
    pub fn set_visible(&self, visible: bool) {
        if let Backend::Hosted(host) = &self.io {
            host.set_visible(visible);
        }
    }

    /// The terminal ended because its session host went away (it crashed or was killed) and not
    /// because its program exited: the screen is the last the host sent. Set before the
    /// `Exited(None)` that reports it.
    pub fn host_lost(&self) -> bool {
        match &self.io {
            Backend::Hosted(host) => host.link.lost(),
            Backend::Local(_) => false,
        }
    }
}

/// Wait for the new host to listen, connect, say `Hello` and `Spawn`; the connection and the
/// host's `Info`.
fn spawn_in(
    host: &HostCommand<'_>,
    started: &mut launch::Started,
    size: GridSize,
    options: &StartOptions,
    launch: &Launch,
) -> anyhow::Result<(Joined, wire::Welcome)> {
    let stream = connect_new(host, started)?;
    let mut dec = wire::Decoder::new();
    // Nothing runs yet: the `Welcome` says only that this is the host for `id`.
    socket::hello(&stream, &mut dec, host.id, Instant::now() + READY_TIMEOUT)
        .context("the session host's handshake")?;
    let env = environment(launch);
    let cwd = launch.cwd.clone().or_else(|| std::env::current_dir().ok());
    let cwd = cwd.as_deref().and_then(Path::to_str).unwrap_or("");
    let args: Vec<&str> = launch.shell.args.iter().map(String::as_str).collect();
    let pairs: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let colors = options.colors.map(wire_colors);
    let flag = |on: bool, bit: u32| if on { bit } else { 0 };
    let flags = flag(options.allow_window_ops, wire::option_flags::WINDOW_OPS)
        | flag(options.allow_screen_checksum, wire::option_flags::CHECKSUM);
    let seed = options.restore.map(wire::seed_chunks).unwrap_or_default();
    let spawn = wire::Spawn {
        size: wire_size(size),
        options: wire::Options {
            scrollback_lines: options.scrollback_lines.min(u32::MAX as usize) as u32,
            flags,
        },
        program: &launch.shell.program,
        cwd,
        args: wire::List::Borrowed(&args),
        env: wire::List::Borrowed(&pairs),
        colors,
        seed: wire::List::Borrowed(&seed),
    };
    let until = Instant::now() + READY_TIMEOUT;
    let info = match socket::spawn(&stream, &mut dec, &spawn, until)? {
        Ok(info) => info,
        Err(text) => anyhow::bail!("{text}"),
    };
    // The pipe says the same; it is the host's word if the connection lied.
    match started.ready(until) {
        Ready::Ok(pid) if pid == info.pid => {}
        Ready::Ok(pid) => anyhow::bail!("the session host started {pid}, but said {}", info.pid),
        Ready::Err(text) => anyhow::bail!("{text}"),
        Ready::Closed => anyhow::bail!("the session host ended before its program ran"),
        Ready::Silent => {
            tracing::warn!(id = host.id, "the session host's readiness pipe said nothing in time")
        }
    }
    Ok((Joined { stream, dec }, info))
}

/// Connect to the host just started, once it listens: its meta file names the socket and appears
/// once it does. The readiness pipe is watched meanwhile, for a host that failed first.
fn connect_new(host: &HostCommand<'_>, started: &mut launch::Started) -> anyhow::Result<Stream> {
    let meta_path = paths::meta_path(host.hosts_dir, host.id);
    let until = Instant::now() + START_TIMEOUT;
    loop {
        match started.ready(Instant::now() + std::time::Duration::from_millis(5)) {
            Ready::Silent => {}
            Ready::Err(text) => anyhow::bail!("{text}"),
            Ready::Closed => anyhow::bail!("the session host ended before it listened"),
            Ready::Ok(_) => anyhow::bail!("the session host said it ran a program before it was asked to"),
        }
        if let Ok(found) = meta::read(&meta_path)
            && found.id == host.id
            && found.pid == started.pid()
        {
            match socket::connect(&found.socket) {
                Ok(stream) => return Ok(stream),
                Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                    return Err(anyhow::Error::new(err).context("the session host's socket"));
                }
                Err(_) => {}
            }
        }
        anyhow::ensure!(Instant::now() < until, "the session host did not listen in time");
    }
}

/// The program's whole environment: the app's (what a local terminal's program inherits), with
/// the launch's on top. Entries that are not UTF-8, or too long for the wire, are left out.
fn environment(launch: &Launch) -> Vec<(String, String)> {
    let mut env: HashMap<String, String> = std::env::vars_os()
        .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
        .collect();
    env.extend(launch.env.iter().map(|(k, v)| (k.clone(), v.clone())));
    let fits = |s: &str| s.len() <= usize::from(u16::MAX);
    env.into_iter().filter(|(k, v)| fits(k) && fits(v) && !k.is_empty()).collect()
}

/// The first protocol minor whose host draws a `Spawn`'s seed.
const SEED_MINOR: u16 = 4;

/// The saved text the replica draws as a host of protocol `minor` did: none for a host that skipped
/// the seed (it would show what the host does not have).
pub(super) fn seed_drawn_by(restore: Option<&str>, minor: u16) -> Option<&str> {
    restore.filter(|_| minor >= SEED_MINOR)
}

/// What the host's `Welcome` said that the terminal keeps, and what the replica draws first.
#[derive(Clone, Copy)]
pub(crate) struct HostFacts<'a> {
    /// The pid of the terminal's program.
    pub pid: u32,
    /// The protocol minor the host speaks.
    pub minor: u16,
    /// What the host announced it does beyond the first minor.
    pub features: wire::Features,
    /// The saved text a host that was just started drew before the program's first byte, which the
    /// replica draws too: `None` for an attach (the host has its screen already) and for a host that
    /// skipped the seed.
    pub replay: Option<&'a str>,
}

/// The `Pty` around a connection whose handshake is done, and its thread. `size` is the app's;
/// `replica` the size the host has (the replica's until the host says otherwise).
pub(crate) fn hosted(
    joined: Joined,
    id: &str,
    size: GridSize,
    replica: GridSize,
    options: &StartOptions,
    first: StreamMode,
    host: HostFacts<'_>,
) -> anyhow::Result<(Pty, PtyEvents)> {
    let HostFacts { pid, minor, features, replay } = host;
    let (proxy, events) = EventProxy::new();
    let shared = proxy.shared().clone();
    // The replica decodes the pictures it draws itself. The host keeps its own store, at the default
    // budget: the wire carries no image budget yet (a new minor would).
    if let Some(bytes) = options.image_budget {
        shared.images().set_budget(bytes);
    }
    let replica_options = ReplicaOptions::of(options);
    let mut screen = replica_options.screen(replica);
    // What the host draws before the program's first byte (see `start_hosted`): the same here, so the
    // stream's bytes land under it as they do there.
    if let Some(text) = replay {
        crate::checkpoint::replay::replay_seed(&mut screen, &wire::seed_chunks(text).concat());
    }
    if let Some(colors) = options.colors {
        screen.set_default_colors(colors);
    }
    let screen = Arc::new(FairLock::new(screen));
    let feed = OwnFeed::replica(proxy.clone(), screen.clone());
    let poller = Arc::new(Poller::new()?);
    let link = Arc::new(Link::new(poller.clone()));
    // What the replica itself sends the program (the locator's reports) is input like typing.
    let input = link.clone();
    shared.set_writer(Box::new(move |bytes| input.send(super::link::Cmd::Input(bytes.into()))));
    let sink = Sink::new(screen.clone(), feed, proxy.clone());
    // A host learns the scheme only from this message, so a program that asks before it comes in
    // goes unanswered, as in a local terminal before its first draw; one attached again kept it.
    if let Some(scheme) = options.color_scheme {
        link.send(super::link::Cmd::ColorScheme(scheme));
    }
    let state = State::new(link.clone(), proxy, sink, replica, replica_options, first);
    let client = Client::new(joined.stream, joined.dec, poller, state)?;
    std::thread::Builder::new().name("host client".into()).spawn(move || client.run())?;
    let pty = Pty {
        term: screen,
        shared,
        io: Backend::Hosted(Hosted::new(link, id.to_owned(), minor, features)),
        size,
        pid,
        hangup_grace: pty::HANGUP_GRACE,
        reported_cwd: Mutex::new(None),
    };
    Ok((pty, events))
}
