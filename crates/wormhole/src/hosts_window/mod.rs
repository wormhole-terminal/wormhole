//! The Session Hosts window (Help → Session Hosts, while persistent terminals are on): what the
//! processes that keep terminals running behind the app are doing, so a person can see whether
//! things are right. Every host in the hosts folder is a card: its state and the terminal that holds it, what
//! is wrong with it; opened, its ids and processes, its version and protocol against this app's, whether it
//! answers, and for an attached one the connection's counters. The data is core's `ipc::hosts::report` (collected off the main thread when the hosts folder
//! changes, and on a slow look while the window is open: `follow`); this module only draws it and carries
//! out End, Forget and Copy.

mod detail;
#[cfg(feature = "devctl")]
mod devctl;
mod follow;
mod gather;
mod header;
mod row;
mod words;

use std::collections::HashSet;
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::ipc::hosts::report::Report;
use wormhole_core::ipc::hosts::{self, EndHow};
use wormhole_text::keys::{
    HOSTS_END_CONFIRM, HOSTS_END_DETAIL, HOSTS_LOADING, HOSTS_WINDOW_TITLE, hosts_empty,
    hosts_end_detail_attached, hosts_end_title,
};
use wormhole_text::text;

use crate::aux_window;
use crate::dialog::{self, CloseQuestion};
use crate::style;
use gather::Gather;

#[cfg(feature = "devctl")]
pub(crate) use devctl::command as devctl_command;

const SIZE: (f32, f32) = (940.0, 720.0);
const MIN_SIZE: (f32, f32) = (640.0, 420.0);
/// How often the window looks again while it is open and the hosts folder is followed. The folder's events
/// say when a host comes, goes or rewrites its meta; nothing says when a host stops answering, how much memory
/// it holds or what its connection to this app has counted, so those are asked on this slow look.
const SLOW_LOOK: Duration = Duration::from_secs(3);
/// The look where the folder cannot be followed (no watcher here, a network volume, or the folder not made
/// yet): the old pace, since the timer is all there is.
const FALLBACK_LOOK: Duration = Duration::from_secs(1);
/// How long a copy button says "Copied".
const COPIED_FOR: Duration = Duration::from_millis(1500);
/// How long End waits for the host to take the hangup.
const END_WAIT: Duration = Duration::from_secs(5);

/// The window while open, so a second command brings it forward instead of opening another.
#[derive(Default)]
struct OpenWindow(crate::app_state::Slot);

impl Global for OpenWindow {}

/// What was last copied, for the button to say so.
#[derive(Clone, PartialEq, Eq)]
enum Copied {
    All,
    Host(String),
}

pub(crate) struct HostsWindow {
    report: Option<Report>,
    scroll: ScrollHandle,
    focus_handle: FocusHandle,
    copied: Option<Copied>,
    /// The hosts whose cards are opened, by id.
    open: HashSet<String>,
    _refresh: Option<Task<()>>,
    _copied_reset: Option<Task<()>>,
}

/// Open the window, or bring it to the front.
pub(crate) fn open(cx: &mut App) {
    if let Some((handle, _)) = cx.try_global::<OpenWindow>().and_then(|open| open.0.clone())
        && handle.update(cx, |_, window, cx| crate::background::activate_window(window, cx)).is_ok()
    {
        return;
    }
    let options = aux_window::window_options(
        text(HOSTS_WINDOW_TITLE),
        size(px(SIZE.0), px(SIZE.1)),
        size(px(MIN_SIZE.0), px(MIN_SIZE.1)),
        cx,
    );
    // Deferred: the new window renders right away, and this may run inside another window's update.
    cx.defer(move |cx| {
        match crate::window_open::open(options, cx, |window, cx| cx.new(|cx| HostsWindow::new(window, cx))) {
            Ok((handle, view)) => cx.set_global(OpenWindow(Some((handle, view.downgrade().into())))),
            Err(err) => tracing::error!("opening the Session Hosts window failed: {err:#}"),
        }
    });
}

/// The window's view while it is open (devctl).
#[cfg(feature = "devctl")]
fn open_view(cx: &App) -> Option<Entity<HostsWindow>> {
    let (_, view) = cx.try_global::<OpenWindow>()?.0.clone()?;
    view.upgrade()?.downcast::<HostsWindow>().ok()
}

/// The window's handle while it is open (devctl).
#[cfg(feature = "devctl")]
fn open_handle(cx: &App) -> Option<AnyWindowHandle> {
    cx.try_global::<OpenWindow>().and_then(|open| open.0.clone()).map(|(handle, _)| handle)
}

impl HostsWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        let mut this = Self {
            report: None,
            scroll: ScrollHandle::new(),
            focus_handle,
            copied: None,
            open: HashSet::new(),
            _refresh: None,
            _copied_reset: None,
        };
        this.start(cx);
        this
    }

    /// (Re)start the loop that looks when the hosts folder changes and on a slow timer: the app's part on the
    /// main thread, the files and sockets off it. It ends with the window (the task and the watch it holds
    /// are dropped).
    fn start(&mut self, cx: &mut Context<Self>) {
        let dir = hosts::hosts_dir(&wormhole_core::storage::paths::data_dir());
        self._refresh = Some(cx.spawn(async move |this, cx| {
            let followed = cx.background_executor().spawn(async move { follow::follow(&dir) }).await.ok();
            loop {
                let Ok(gather) = this.update(cx, |_, cx| Gather::collect(cx)) else {
                    return;
                };
                let report = cx.background_executor().spawn(async move { gather.run() }).await;
                let shown = this.update(cx, |this, cx| {
                    // Draw again only when something shows differently.
                    if !this.report.as_ref().is_some_and(|shown| shown.same_as(&report)) {
                        this.report = Some(report);
                        cx.notify();
                    }
                });
                if shown.is_err() {
                    return;
                }
                let look = if followed.is_some() { SLOW_LOOK } else { FALLBACK_LOOK };
                let timer = cx.background_executor().timer(look);
                match &followed {
                    Some(followed) => {
                        let event = async {
                            if followed.woken.recv().await.is_ok() {
                                // The writer finishes; and what came meanwhile is this one look's.
                                cx.background_executor().timer(follow::SETTLE).await;
                                while followed.woken.try_recv().is_ok() {}
                            }
                        };
                        smol::future::or(event, timer).await;
                    }
                    None => timer.await,
                }
            }
        }));
    }

    fn copy(&mut self, what: Copied, cx: &mut Context<Self>) {
        let Some(report) = &self.report else { return };
        let text = match &what {
            Copied::All => report.to_text(),
            Copied::Host(id) => match report.row(id) {
                Some(row) => row.to_text(&report.app),
                None => return,
            },
        };
        crate::platform::clipboard::write(text, cx);
        self.copied = Some(what);
        self._copied_reset = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            this.update(cx, |this, cx| {
                this.copied = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// Ask before ending a host: its program gets a hangup, and an attached terminal ends with it.
    fn ask_to_end(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.report.as_ref().and_then(|r| r.row(id)) else { return };
        let detail = match &row.standing {
            hosts::report::Standing::Attached { title, .. } => hosts_end_detail_attached(title),
            _ => text(HOSTS_END_DETAIL).to_string(),
        };
        let question = CloseQuestion {
            title: hosts_end_title(id).into(),
            detail: detail.into(),
            confirm: text(HOSTS_END_CONFIRM),
        };
        let (this, id) = (cx.weak_entity(), id.to_string());
        dialog::ask_before_closing(
            window,
            cx,
            question,
            move |_, cx| {
                this.update(cx, |this, cx| this.end(id, cx)).ok();
            },
            |_| {},
        );
    }

    fn end(&mut self, id: String, cx: &mut Context<Self>) {
        let dir = hosts::hosts_dir(&wormhole_core::storage::paths::data_dir());
        cx.spawn(async move |this, cx| {
            let done = cx
                .background_executor()
                .spawn(async move {
                    let meta = hosts::find(&dir, &id)?;
                    hosts::end_host(&meta, END_WAIT).map(|how: EndHow| (id, how))
                })
                .await;
            match done {
                Ok((id, how)) => tracing::info!(id, how = how.word(), "ended a session host from its window"),
                Err(why) => tracing::warn!("ending a session host from its window failed: {why}"),
            }
            this.update(cx, |this, cx| this.start(cx)).ok();
        })
        .detach();
    }

    fn forget(&mut self, id: String, cx: &mut Context<Self>) {
        let dir = hosts::hosts_dir(&wormhole_core::storage::paths::data_dir());
        cx.spawn(async move |this, cx| {
            let done = cx.background_executor().spawn(async move { hosts::forget(&dir, &id) }).await;
            if let Err(err) = done {
                tracing::warn!("forgetting a session host from its window failed: {err}");
            }
            this.update(cx, |this, cx| this.start(cx)).ok();
        })
        .detach();
    }

    /// Open a host's card, or fold it.
    fn toggle(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.open.remove(id) {
            self.open.insert(id.to_string());
        }
        cx.notify();
    }
}

impl Render for HostsWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = crate::settings::window_ui(cx, false);
        let surfaces = style::window_surfaces(cx, false);
        let body: AnyElement = match &self.report {
            None => style::empty_note(ui, text(HOSTS_LOADING)).into_any_element(),
            Some(report) if report.rows.is_empty() => {
                style::empty_note(ui, hosts_empty(&report.hosts_dir.display().to_string())).into_any_element()
            }
            Some(report) => {
                let cards: Vec<AnyElement> = report
                    .rows
                    .iter()
                    .enumerate()
                    .map(|(ix, row)| self.card(ix, row, report, ui, cx))
                    .collect();
                div().flex().flex_col().gap_2().children(cards).into_any_element()
            }
        };
        let header = self.report.as_ref().map(|report| self.header(report, ui, cx));
        div()
            .id("hosts-window")
            .track_focus(&self.focus_handle)
            .key_context("HostsWindow") // text: a key context
            .on_key_down(cx.listener(|_, event: &KeyDownEvent, window, _| {
                aux_window::close_on_esc_or_cmd_w(event, window);
            }))
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .pt(px(style::TITLE_ROW_H))
            .bg(surfaces.background)
            .text_color(ui.text)
            .child(style::drag_strip())
            .children(header)
            .child(
                div()
                    .id("hosts-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .px_6()
                    .pb_6()
                    .child(body),
            )
    }
}

/// Used by the devctl build to read the window typed.
#[cfg(feature = "devctl")]
impl HostsWindow {
    fn report(&self) -> Option<&Report> {
        self.report.as_ref()
    }
}
