//! What the engine's events become: calls on [`EventProxy`] and [`Shared`], so everything above
//! this module hears what a program did (a title, a bell, a folder, a prompt mark, a
//! notification, its status, a clipboard request, a picture).
//!
//! The engine reads every sequence itself and hands the host what it must act on, raw (an OSC
//! 7 folder as written, a notification's text, a program-status body). Cleaning and policy are
//! core's (`osc`): these functions only put the engine's words into its terms.

use std::sync::Arc;

use base64::Engine as _;
use wormhole_vt::Event;
use wormhole_vt::event::{Progress as EngineProgressReport, Terminator};
use wormhole_vt::event::{ProgressState as EngineProgress, PromptKind as EnginePrompt, ShellMark};
use wormhole_vt::window::{Extent, WindowOp};

use super::images::Uploads;
use crate::events::{EventProxy, PtyEvent, TextFormatter};
use crate::osc::{KittyChunks, Mark, Osc, Progress, ProgressState, PromptKind, cwd, notification};
use crate::program_status::{self, Parsed};
use crate::screen::Screen;

/// The host side of the engine's events on the PTY thread: the proxy they go to, the count of
/// prompts (a block's id, counted as the engine numbers its blocks), kitty's chunked notifications being put
/// together, and the pictures being decoded.
pub(super) struct Host {
    proxy: EventProxy,
    prompts: u64,
    kitty: KittyChunks,
    pub(super) uploads: Uploads,
    /// Scratch for the events of one read, so a quiet read costs no allocation.
    events: Vec<Event>,
    /// How many bytes of the engine's replies this read has sent already (see `settle`).
    sent: usize,
}

impl Host {
    pub(super) fn new(proxy: EventProxy, uploads: Uploads) -> Self {
        Self { proxy, prompts: 0, kitty: KittyChunks::default(), uploads, events: Vec::new(), sent: 0 }
    }

    /// Act on everything the engine queued while it read: events in the order the program's
    /// sequences came, and the replies it wrote. A reply the UI still owes (an OSC 52 read) keeps
    /// the replies after it waiting, so the replies that came before the request go out first:
    /// the program reads its answers in the order it asked.
    pub(super) fn settle(&mut self, screen: &mut Screen) {
        self.events.extend(screen.term.events());
        let mut events = std::mem::take(&mut self.events);
        for event in events.drain(..) {
            if let Event::ClipboardLoad { replies_before, .. } = &event {
                self.send_replies(screen, *replies_before);
            }
            self.event(screen, event);
        }
        self.events = events;
        for folder in screen.term.cwd_reports() {
            if let Some(osc) = cwd(folder) {
                self.proxy.osc(osc);
            }
        }
        screen.term.clear_cwd_reports();
        self.send_replies(screen, usize::MAX);
        screen.term.clear_replies();
        self.sent = 0;
    }

    /// Send the engine's replies up to byte `until` that have not gone out yet.
    fn send_replies(&mut self, screen: &Screen, until: usize) {
        let replies = screen.term.replies();
        let until = until.min(replies.len());
        if until > self.sent {
            self.proxy.shared().reply(replies[self.sent..until].to_vec());
            self.sent = until;
        }
    }

    fn event(&mut self, screen: &mut Screen, event: Event) {
        let proxy = &self.proxy;
        match event {
            Event::Bell => proxy.send(PtyEvent::Bell),
            Event::Title => proxy.send(match screen.term.title() {
                Some(title) => PtyEvent::Title(title.to_owned()),
                None => PtyEvent::ResetTitle,
            }),
            // Its reports are read after the events (`settle`): one event may stand for several.
            Event::Cwd => {}
            Event::Notification { title, body } => {
                if let Some(notification) = notification(&title, &body) {
                    proxy.osc(Osc::Notify(notification));
                }
            }
            Event::KittyNotification { metadata, payload } => {
                if let Some(notification) = self.kitty.chunk(metadata.as_bytes(), &payload) {
                    proxy.osc(Osc::Notify(notification));
                }
            }
            Event::Progress(progress) => proxy.osc(Osc::Progress(progress.map(progress_from))),
            Event::ClipboardStore { text, .. } => proxy.send(PtyEvent::ClipboardStore(text)),
            Event::ClipboardLoad { selection, terminator, .. } => {
                let reply = proxy.shared().hold_reply();
                proxy.send(PtyEvent::ClipboardLoad(clipboard_reply(selection, terminator), reply));
            }
            Event::Mark(mark) => {
                if let Some((mark, prompt)) = numbered(mark, &mut self.prompts) {
                    proxy.osc(Osc::Mark(mark, prompt));
                }
            }
            Event::ProgramStatus { payload, terminator } => {
                let end = match terminator {
                    Terminator::Bel => 1,
                    Terminator::St => 2,
                };
                match program_status::parse(&payload, end) {
                    Some(Parsed::Query) => proxy.osc(Osc::ProgramStatusQuery),
                    Some(Parsed::Report(report)) => proxy.osc(Osc::ProgramStatus(report)),
                    None => {}
                }
            }
            Event::FullReset => proxy.osc(Osc::FullReset),
            // The pane's size is the window's: without window operations the engine clears and
            // homes the screen for DECCOLM and keeps the width the PTY has, and sends no event.
            // With them on, the engine has taken the width and the UI is asked for it, and brings
            // the engine back to the pane's size when it is done.
            Event::ColumnsRequested(cols) => {
                if screen.window_ops_allowed() {
                    proxy.send(PtyEvent::WindowOp(WindowOp::ResizeCells {
                        rows: Extent::Keep,
                        cols: Extent::Size(cols as u32),
                    }));
                }
            }
            Event::WindowOp(op) => proxy.send(PtyEvent::WindowOp(op)),
            Event::ImageData(upload) => self.uploads.begin(screen, upload),
            Event::ImageFree(image) => self.uploads.free(image),
            // The engine keeps the icon name to itself, and iTerm2's other keys (marks, the
            // current directory) are nothing Wormhole acts on today.
            Event::IconName(_) | Event::Iterm(_) => {}
            // An event a newer engine adds (`Event` is non-exhaustive) is nothing this host acts on.
            _ => {}
        }
    }
}

/// The shell's mark as core knows it, and the number of the prompt it belongs to (`prompts` counts
/// the initial ones, as the engine numbers its blocks).
fn numbered(mark: ShellMark, prompts: &mut u64) -> Option<(Mark, u64)> {
    let mark = mark_from(mark)?;
    if mark == Mark::PromptStart(PromptKind::Initial) {
        *prompts += 1;
    }
    Some((mark, *prompts))
}

/// `None` for a mark a newer engine adds.
fn mark_from(mark: ShellMark) -> Option<Mark> {
    Some(match mark {
        ShellMark::PromptStart(kind) => Mark::PromptStart(match kind {
            EnginePrompt::Initial => PromptKind::Initial,
            EnginePrompt::Secondary => PromptKind::Secondary,
            EnginePrompt::Right => PromptKind::Right,
            EnginePrompt::Continuation => PromptKind::Continuation,
            // A prompt kind we do not know belongs to the command it sits in, like the extra
            // prompts, and does not start a block of its own.
            _ => PromptKind::Secondary,
        }),
        ShellMark::CommandStart => Mark::CommandStart,
        ShellMark::OutputStart => Mark::OutputStart,
        ShellMark::Finished(code) => Mark::Finished(code),
        _ => return None,
    })
}

fn progress_from(progress: EngineProgressReport) -> Progress {
    Progress {
        state: match progress.state {
            EngineProgress::Normal => ProgressState::Normal,
            EngineProgress::Error => ProgressState::Error,
            EngineProgress::Indeterminate => ProgressState::Indeterminate,
            EngineProgress::Paused => ProgressState::Paused,
            // A state we do not know is shown as work without a percentage.
            _ => ProgressState::Indeterminate,
        },
        percent: progress.percent,
    }
}

/// The answer to an OSC 52 read: the clipboard's text for `selection`, ended as the request was.
fn clipboard_reply(selection: String, terminator: Terminator) -> TextFormatter {
    let end = match terminator {
        Terminator::Bel => "\x07",
        Terminator::St => "\x1b\\",
    };
    Arc::new(move |text: &str| {
        let data = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
        format!("\x1b]52;{selection};{data}{end}")
    })
}

/// The mark an engine event is, with its prompt number (see [`numbered`]); for the test terminal.
#[cfg(any(test, feature = "test-support"))]
pub(crate) fn mark_of(event: Event, prompts: &mut u64) -> Option<(Mark, u64)> {
    match event {
        Event::Mark(mark) => numbered(mark, prompts),
        _ => None,
    }
}
