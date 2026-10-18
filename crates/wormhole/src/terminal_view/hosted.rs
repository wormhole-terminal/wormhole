//! A terminal whose program runs in a session host (`repo-docs/docs/session-host.md`): starting it
//! in a new host or attaching to one that runs, both off the UI thread while the pane shows blank;
//! falling back to the app's own process when no host will do; on screen or headless; and letting
//! go of the host when the app quits, so the program goes on running.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::{Button, ButtonVariants};

use super::*;
use crate::style::{self, BannerKind};
use wormhole_core::terminal::PtyEvents;

/// How a terminal's program starts.
#[derive(Clone, Debug)]
pub enum Start {
    /// In the app's own process, as without hosts.
    Local,
    /// In a new session host: `binary`, with its files in `<data_dir>/hosts` under the id `id`.
    Host { binary: PathBuf, data_dir: PathBuf, id: String },
    /// Attach to the running host `id` at `socket` (a terminal saved with it, or an orphan), fed a
    /// repaint at once when `visible`, headless otherwise.
    Attach { socket: PathBuf, id: String, visible: bool },
}

/// What a terminal knows about its session host.
#[derive(Default)]
pub(super) struct HostState {
    /// The host it runs in, or is starting in or attaching to; `None` in the app's own process.
    id: Option<String>,
    /// The host went away (crashed, killed) and took the program with it.
    lost: bool,
    /// Let go of (`detach_host`): the program runs on in the host, the view keeps its last screen.
    detached: bool,
    /// The host is being started or attached to (the pane is blank meanwhile).
    connecting: bool,
    /// On screen (fed a repaint, then the stream) or headless, as the workspace last said.
    on_screen: bool,
    /// The options it was started with, for attaching again (`reattach`): the replica must have
    /// the same history and window rules as the host.
    options: Option<SpawnOptions>,
    /// The one-time notice that the host could not start, while it shows.
    notice: Option<SharedString>,
}

/// The notice that a host could not start is shown once per run: when one fails, the next would
/// for the same reason, and every new terminal would say it again.
static FALLBACK_TOLD: AtomicBool = AtomicBool::new(false);

/// The app is quitting and has let go of its hosts (`workspace::hosting::detach_all`). A start or
/// attach still under way then finishes on its background thread with no view to take it, and its
/// `Pty` would be dropped there, which closes the host: it lets go of the host instead.
static APP_QUITTING: AtomicBool = AtomicBool::new(false);

/// Called as the app quits, before the hosts are let go of.
pub fn note_app_quitting() {
    APP_QUITTING.store(true, Ordering::Release);
}

/// `ready`, let go of when the app quit while it was being made (see [`APP_QUITTING`]).
fn kept_past_quit(ready: anyhow::Result<(Pty, PtyEvents)>) -> anyhow::Result<(Pty, PtyEvents)> {
    if APP_QUITTING.load(Ordering::Acquire)
        && let Ok((pty, _)) = &ready
    {
        pty.detach();
    }
    ready
}

impl HostState {
    pub(super) fn of(start: &Start) -> Self {
        match start {
            Start::Local => Self::default(),
            Start::Host { id, .. } => {
                Self { id: Some(id.clone()), connecting: true, on_screen: true, ..Self::default() }
            }
            Start::Attach { id, visible, .. } => {
                Self { id: Some(id.clone()), connecting: true, on_screen: *visible, ..Self::default() }
            }
        }
    }
}

impl TerminalView {
    /// Start what `start` asks for that runs off the UI thread: a new host, or an attach. A local
    /// start has already happened in `new`.
    pub(super) fn begin(
        &mut self,
        start: Start,
        options: SpawnOptions,
        size: GridSize,
        cx: &mut Context<Self>,
    ) {
        if !matches!(start, Start::Local) {
            // What a reattach needs; the saved text only matters to the fallback below.
            self.host.options = Some(SpawnOptions { restore: None, ..options.clone() });
        }
        match start {
            Start::Local => {}
            Start::Host { binary, data_dir, id } => {
                let task = cx.background_spawn({
                    let options = options.clone();
                    async move {
                        let hosts_dir = wormhole_core::hosts::ensure_hosts_dir(&data_dir)?;
                        let host = wormhole_core::terminal::client::HostCommand {
                            binary: &binary,
                            hosts_dir: &hosts_dir,
                            id: &id,
                        };
                        kept_past_quit(wormhole_core::terminal::spawn_hosted(&host, size, &options))
                    }
                });
                self.await_host(task, options, size, false, cx);
            }
            Start::Attach { socket, id, visible } => {
                let task = cx.background_spawn({
                    let options = options.clone();
                    async move {
                        kept_past_quit(wormhole_core::terminal::attach_hosted(
                            &socket, &id, &options, visible,
                        ))
                    }
                });
                self.await_host(task, options, size, true, cx);
            }
        }
    }

    /// Take the terminal `task` makes when it is ready, or start one in the app's process when it
    /// fails (`attaching`: the host had gone, which is today's restore with the saved text).
    fn await_host(
        &mut self,
        task: Task<anyhow::Result<(Pty, PtyEvents)>>,
        options: SpawnOptions,
        size: GridSize,
        attaching: bool,
        cx: &mut Context<Self>,
    ) {
        // Kept with the view: a terminal closed before its host is ready drops this task, and the
        // terminal the host's start makes is dropped with its result, which ends that host.
        self._tasks.push(cx.spawn(async move |this, cx| {
            let ready = task.await;
            let _ = this.update(cx, |this, cx| this.host_ready(ready, &options, size, attaching, cx));
        }));
    }

    fn host_ready(
        &mut self,
        ready: anyhow::Result<(Pty, PtyEvents)>,
        options: &SpawnOptions,
        size: GridSize,
        attaching: bool,
        cx: &mut Context<Self>,
    ) {
        self.host.connecting = false;
        match ready {
            Ok((pty, events)) => {
                // Attached headless while the workspace has put it on screen since (or the other
                // way round): the stream follows what is shown now.
                if attaching {
                    pty.set_visible(self.host.on_screen);
                }
                self.install(pty, events, cx);
            }
            Err(err) => {
                let id = self.host.id.take().unwrap_or_default();
                if attaching {
                    // Gone between the scan and the attach: the saved text comes back as it would
                    // have without a host.
                    tracing::warn!(id, "couldn't attach to a session host, restoring the text: {err:#}");
                } else {
                    tracing::warn!(
                        id,
                        "couldn't start a session host, the terminal runs in the app: {err:#}"
                    );
                    if !FALLBACK_TOLD.swap(true, Ordering::Relaxed) {
                        self.host.notice = Some(
                            "This terminal runs inside Wormhole and ends when it quits: \
                             its session host couldn't start."
                                .into(),
                        );
                    }
                }
                match wormhole_core::terminal::spawn(self.painted_size().unwrap_or(size), options) {
                    Ok((pty, events)) => self.install(pty, events, cx),
                    Err(err) => tracing::error!("failed to spawn shell: {err:#}"),
                }
                cx.emit(TerminalEvent::HostChanged);
            }
        }
        cx.notify();
    }

    /// The terminal's `Pty` is ready: listen to it, and give it the pane's size when a frame has
    /// shown how big that is (a terminal attached on screen waited for its host's size first, so
    /// this is its one resize before the first frame that shows it).
    fn install(&mut self, mut pty: Pty, events: PtyEvents, cx: &mut Context<Self>) {
        if let Some(size) = self.painted_size()
            && pty.size() != size
        {
            pty.resize(size);
        }
        self.pty = Some(pty);
        // The window's place goes to the new engine on the next frame.
        self.window_given = None;
        self.listen(events, cx);
    }

    /// The grid's size as the last frame laid it out, if one did.
    fn painted_size(&self) -> Option<GridSize> {
        let layout = self.layout.get();
        (layout.cols > 0 && layout.rows > 0).then(|| GridSize {
            cols: layout.cols as u16,
            rows: layout.rows as u16,
            cell_width: f32::from(layout.cell.width),
            cell_height: f32::from(layout.cell.height),
        })
    }

    /// The host this terminal runs in (or is starting in, or attaching to), and whether it was lost.
    pub fn host_id(&self) -> Option<&str> {
        self.host.id.as_deref()
    }

    pub fn host_lost(&self) -> bool {
        self.host.lost
    }

    /// Its program runs in a session host that is still there: quitting leaves it running.
    pub fn is_hosted(&self) -> bool {
        self.host.id.is_some() && !self.host.lost
    }

    /// The exit just heard came from the host going away rather than from the program.
    pub(super) fn exit_was_host_lost(&mut self, cx: &mut Context<Self>) {
        if self.host.id.is_some() && self.pty.as_ref().is_some_and(Pty::host_lost) {
            tracing::warn!(id = self.host.id.as_deref(), "a terminal's session host went away");
            self.host.lost = true;
            cx.emit(TerminalEvent::HostChanged);
        }
    }

    /// On screen or not: a hosted terminal on screen is fed a repaint and then the stream, one off
    /// screen only hears that something printed (its screen keeps what it last showed, which is
    /// what a switch to it shows at once).
    pub fn set_on_screen(&mut self, on_screen: bool) {
        if self.host.on_screen == on_screen {
            return;
        }
        self.host.on_screen = on_screen;
        if let Some(pty) = &self.pty {
            pty.set_visible(on_screen);
        }
    }

    /// Let go of the session host: its program goes on running for the next attach, and dropping
    /// this view afterwards doesn't end it. What every hosted terminal does as the app quits.
    /// Whether there was a host to let go of.
    pub fn detach_host(&mut self) -> bool {
        let detached = self.pty.as_ref().is_some_and(Pty::detach);
        self.host.detached |= detached;
        detached
    }

    /// Attach again to the host let go of with [`detach_host`](Self::detach_host), as at launch.
    /// Whether an attach started.
    #[cfg(feature = "devctl")]
    pub fn reattach(&mut self, cx: &mut Context<Self>) -> Option<String> {
        if !self.host.detached || self.host.lost || self.host.connecting {
            return None;
        }
        let id = self.host.id.clone()?;
        let options = self.host.options.clone()?;
        let hosts_dir = wormhole_core::hosts::hosts_dir(&wormhole_core::store::data_dir());
        let socket = match wormhole_core::terminal::client::socket_of(&hosts_dir, &id) {
            Ok(socket) => socket,
            Err(err) => {
                tracing::warn!(id, "no socket to attach to again: {err:#}");
                return None;
            }
        };
        self.host.detached = false;
        self.host.connecting = true;
        let size = self.painted_size().unwrap_or(GridSize {
            cols: 100,
            rows: 30,
            cell_width: 8.0,
            cell_height: 18.0,
        });
        let visible = self.host.on_screen;
        self.begin(Start::Attach { socket, id: id.clone(), visible }, options, size, cx);
        Some(id)
    }

    /// `hosted <id>` or `local`, for devctl's `state`, with what the host is doing when it is not
    /// simply running.
    #[cfg(feature = "devctl")]
    pub fn host_label(&self) -> String {
        let Some(id) = &self.host.id else { return "local".into() };
        let state = if self.host.lost {
            " lost"
        } else if self.host.connecting {
            " connecting"
        } else if self.host.detached {
            " detached"
        } else if !self.host.on_screen {
            " headless"
        } else {
            ""
        };
        format!("hosted {id}{state}")
    }

    /// What the exited bar says first: an ended program's exit, or a host that went away with it.
    pub(super) fn exited_words(&self) -> String {
        if self.host.lost {
            return "This terminal ended unexpectedly".to_string();
        }
        match self.exit_code {
            Some(code) => format!("{} exited with code {code}", self.shell),
            None => format!("{} exited", self.shell),
        }
    }

    /// The notice over the pane that its host couldn't start, until it is dismissed.
    pub(super) fn render_host_notice(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let text = self.host.notice.clone()?;
        let ui = store_in(&self.profile, cx).ui;
        let banner = style::banner(ui, BannerKind::Warning, false, text).child(
            Button::new("host-notice-dismiss").small().ghost().label("Dismiss").on_click(cx.listener(
                |this, _, _, cx| {
                    this.host.notice = None;
                    cx.notify();
                },
            )),
        );
        Some(
            div()
                .id("host-notice")
                .absolute()
                .top_2()
                .left_4()
                .right_4()
                // The pane below it would start a selection.
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(banner)
                .into_any_element(),
        )
    }
}
