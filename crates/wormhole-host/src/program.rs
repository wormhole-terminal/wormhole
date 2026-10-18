//! The seam between the session and the terminal it serves: [`Program`] is what the session needs
//! of a running terminal, [`Spawner`] starts one. The host runs `wormhole_term::hosted`'s
//! [`HostedTerminal`] behind them; the session's tests run a scripted one, so the protocol is
//! tested without a PTY and without timing.

use std::time::{Duration, Instant};

use wormhole_term::hosted::{Cursor, HostEvent, HostedTerminal, NotReady, Repainted, Wake};
use wormhole_wire as wire;

/// A running (or ended) terminal, as the session drives it.
pub trait Program {
    fn pid(&self) -> u32;
    fn size(&self) -> wire::Size;
    fn input(&mut self, bytes: &[u8]);
    /// Input bytes the PTY has taken (`InputAck`).
    fn input_acked(&self) -> u64;
    /// Apply a resize between two slices; it reaches every cursor in the stream.
    fn resize(&mut self, req: u32, size: wire::Size);
    /// A cursor from the stream's end: with the bytes, or (`bytes` false) resizes and events only.
    fn follow(&self, bytes: bool, gap: Duration, wake: Wake) -> Cursor;
    /// The attach repaint into `out` and a cursor from where it stands.
    fn repaint(&self, max_history: usize, out: &mut Vec<u8>, wake: Wake) -> Result<Repainted, NotReady>;
    /// The title, folder and OSC 7501 reports a client that attaches now did not hear, as events.
    fn remembered(&self) -> Vec<HostEvent>;
    fn counters(&self) -> wire::Counters;
    fn progress(&self) -> u64;
    fn exit(&self) -> Option<wire::ExitInfo>;
    /// The program is gone and its thread is over (exited, or hung up after `close`).
    fn finished(&self) -> bool;
    fn close(&mut self, grace: Duration);
    fn set_colors(&mut self, colors: &wire::Colors<'_>);
    fn set_options(&mut self, options: &wire::Options);
    fn set_controller(&self, attached: bool);
    fn clipboard_reply(&self, ticket: u32, data: &[u8]);
    /// Answer the clipboard reads that are out of time; when the next one is.
    fn clipboard_expire(&self, now: Instant) -> Option<Instant>;
    fn first_block_id(&self) -> u64;
}

/// Starts the program a `Spawn` asks for.
pub trait Spawner {
    type Program: Program;

    /// The program, and a cursor that reads its output from the first byte. The error's text is
    /// what the app shows as "could not start".
    fn spawn(&mut self, spawn: &wire::Spawn<'_>, wake: Wake) -> anyhow::Result<(Self::Program, Cursor)>;
}

/// The real thing: a PTY, the program, the engine.
pub struct PtySpawner {
    /// Run first on the PTY's reader thread (the QoS class).
    pub on_reader_start: fn(),
}

impl Spawner for PtySpawner {
    type Program = HostedTerminal;

    fn spawn(&mut self, spawn: &wire::Spawn<'_>, wake: Wake) -> anyhow::Result<(HostedTerminal, Cursor)> {
        let start = self.on_reader_start;
        HostedTerminal::start(spawn, wake, Some(Box::new(start)))
    }
}

impl Program for HostedTerminal {
    fn pid(&self) -> u32 {
        HostedTerminal::pid(self)
    }
    fn size(&self) -> wire::Size {
        HostedTerminal::size(self)
    }
    fn input(&mut self, bytes: &[u8]) {
        HostedTerminal::input(self, bytes);
    }
    fn input_acked(&self) -> u64 {
        HostedTerminal::input_acked(self)
    }
    fn resize(&mut self, req: u32, size: wire::Size) {
        HostedTerminal::resize(self, req, size);
    }
    fn follow(&self, bytes: bool, gap: Duration, wake: Wake) -> Cursor {
        HostedTerminal::follow(self, bytes, gap, wake)
    }
    fn repaint(&self, max_history: usize, out: &mut Vec<u8>, wake: Wake) -> Result<Repainted, NotReady> {
        HostedTerminal::repaint(self, max_history, out, wake)
    }
    fn remembered(&self) -> Vec<HostEvent> {
        HostedTerminal::remembered(self)
    }
    fn counters(&self) -> wire::Counters {
        HostedTerminal::counters(self)
    }
    fn progress(&self) -> u64 {
        HostedTerminal::progress(self)
    }
    fn exit(&self) -> Option<wire::ExitInfo> {
        HostedTerminal::exit(self)
    }
    fn finished(&self) -> bool {
        HostedTerminal::finished(self)
    }
    fn close(&mut self, grace: Duration) {
        HostedTerminal::close(self, grace);
    }
    fn set_colors(&mut self, colors: &wire::Colors<'_>) {
        HostedTerminal::set_colors(self, colors);
    }
    fn set_options(&mut self, options: &wire::Options) {
        HostedTerminal::set_options(self, options);
    }
    fn set_controller(&self, attached: bool) {
        HostedTerminal::set_controller(self, attached);
    }
    fn clipboard_reply(&self, ticket: u32, data: &[u8]) {
        HostedTerminal::clipboard_reply(self, ticket, data);
    }
    fn clipboard_expire(&self, now: Instant) -> Option<Instant> {
        HostedTerminal::clipboard_expire(self, now)
    }
    fn first_block_id(&self) -> u64 {
        HostedTerminal::first_block_id(self)
    }
}
