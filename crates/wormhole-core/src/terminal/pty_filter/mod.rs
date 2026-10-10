//! The PTY's output on its way to alacritty. `clear_fix` reorders `clear`'s erase so the
//! scrollback goes too; `images` lifts the inline-image sequences (kitty, iTerm2, Sixel) out and
//! prints placeholder cells in their place; `osc_tap` reads the shell-integration and
//! notification sequences alacritty drops (`osc` says what they mean) and reports them through
//! the `EventProxy`. All run on alacritty's reader thread, inside its `read`, in that order, so
//! they see every byte once, in order, before alacritty's parser does.

mod clear_fix;
mod images;
mod osc;
mod osc_tap;

use std::io::{self, Read};
use std::sync::Arc;

use alacritty_terminal::event::{OnResize, WindowSize};
use alacritty_terminal::tty::{ChildEvent, EventedPty, EventedReadWrite};
use polling::{Event, PollMode, Poller};

use super::events::EventProxy;
use clear_fix::ClearRewriter;
use images::ImageFilter;
pub(super) use osc::Osc;
pub(crate) use osc::percent_decode;
pub use osc::{
    BODY_MAX_CHARS, Mark, Notification, Progress, ProgressState, PromptKind, TITLE_MAX_CHARS, clean,
};
use osc_tap::OscTap;

/// A PTY whose output goes through [`ClearRewriter`], [`ImageFilter`] and then [`OscTap`];
/// everything else is passed straight to the wrapped PTY.
pub(crate) struct FilteredPty<P> {
    pty: P,
    clear: ClearRewriter,
    images: ImageFilter,
    tap: OscTap,
    proxy: EventProxy,
}

impl<P> FilteredPty<P> {
    pub(crate) fn new(pty: P, proxy: EventProxy) -> Self {
        let images = ImageFilter::new(proxy.clone());
        Self { pty, clear: ClearRewriter::default(), images, tap: OscTap::new(), proxy }
    }
}

impl<P: EventedReadWrite> Read for FilteredPty<P> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Self { pty, clear, images, tap, proxy } = self;
        let read = tap.read_from(|buf| images.read_from(|buf| clear.read_from(pty.reader(), buf), buf), buf);
        for osc in tap.drain_events() {
            proxy.osc(osc);
        }
        read
    }
}

impl<P: EventedReadWrite> EventedReadWrite for FilteredPty<P> {
    type Reader = Self;
    type Writer = P::Writer;

    #[allow(unsafe_code)] // Forwards to the wrapped PTY's (unsafe) registration.
    unsafe fn register(&mut self, poller: &Arc<Poller>, event: Event, mode: PollMode) -> io::Result<()> {
        // SAFETY: the caller's guarantee (the sources outlive their
        // registration) carries over, as `self` owns the wrapped PTY.
        unsafe { self.pty.register(poller, event, mode) }
    }

    fn reregister(&mut self, poller: &Arc<Poller>, event: Event, mode: PollMode) -> io::Result<()> {
        self.pty.reregister(poller, event, mode)
    }

    fn deregister(&mut self, poller: &Arc<Poller>) -> io::Result<()> {
        self.pty.deregister(poller)
    }

    fn reader(&mut self) -> &mut Self {
        self
    }

    fn writer(&mut self) -> &mut P::Writer {
        self.pty.writer()
    }
}

impl<P: EventedPty> EventedPty for FilteredPty<P> {
    fn next_child_event(&mut self) -> Option<ChildEvent> {
        self.pty.next_child_event()
    }
}

impl<P: OnResize> OnResize for FilteredPty<P> {
    fn on_resize(&mut self, window_size: WindowSize) {
        self.pty.on_resize(window_size);
    }
}

/// `bytes` as the tap hands them to alacritty (its marks injected) and what it reported, for the
/// tests of what reads the marks back (`marks`, `blocks`, `snapshot`).
#[cfg(test)]
pub(super) fn tapped(bytes: &[u8]) -> (Vec<u8>, Vec<Osc>) {
    let mut tap = OscTap::new();
    let mut rest = bytes;
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = tap
            .read_from(
                |buf| {
                    let n = rest.len().min(buf.len());
                    buf[..n].copy_from_slice(&rest[..n]);
                    rest = &rest[n..];
                    Ok(n)
                },
                &mut buf,
            )
            .unwrap_or(0);
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
    }
    let events = tap.drain_events().collect();
    (out, events)
}
