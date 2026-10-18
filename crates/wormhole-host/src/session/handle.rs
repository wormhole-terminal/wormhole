//! What the session does with each message, by the connection's phase and role.
//!
//! Before `Hello` only `Hello` is read. After it: a controller may do everything, an observer may
//! attach and acknowledge, and an admin connection may only ask for the host's `Info` and `Kill`
//! it (that set is the same on every version). A message out of turn is answered with an
//! `Error` naming it; one that breaks the framing closes the connection.

use std::time::{Duration, Instant};

use wormhole_term::hosted::MAX_CELLS;
use wormhole_wire::{
    self as wire, Attach, DetachReason, Frame, Message, Role, StreamMode, error_code as code, ty,
    version::Range,
};

use super::conn::{Attachment, Phase, Waiting};
use super::{ConnId, Life, Notice, Session};
use crate::program::{Program, Spawner};

impl<S: Spawner> Session<S> {
    /// Read every complete frame connection `id` has sent.
    pub(super) fn handle_frames(&mut self, id: ConnId, now: Instant) {
        loop {
            let Some(at) = self.index(id) else { return };
            if self.conns[at].dead {
                return;
            }
            // Taken out for the frame's lifetime: the frame borrows the decoder's buffer, and the
            // handlers need the session.
            let mut dec = std::mem::take(&mut self.conns[at].dec);
            let done = match dec.next_frame() {
                Ok(Some(frame)) => {
                    self.handle(id, frame, now);
                    false
                }
                Ok(None) => true,
                Err(err) => {
                    tracing::info!(conn = id.0, fatal = err.is_fatal(), "bad frame, closing: {err}");
                    if let Some(at) = self.index(id) {
                        self.conns[at].dead = true;
                    }
                    true
                }
            };
            if let Some(at) = self.index(id) {
                self.conns[at].dec = dec;
            }
            if done {
                return;
            }
        }
    }

    fn handle(&mut self, id: ConnId, frame: Frame<'_>, now: Instant) {
        let Some(at) = self.index(id) else { return };
        let msg = frame.message;
        let role = match self.conns[at].phase {
            Phase::Hello { .. } => return self.hello(at, &msg),
            Phase::Ready { role } => role,
        };
        let t = msg.ty();
        if role == Role::Admin && !matches!(msg, Message::GetInfo | Message::Kill | Message::Ping(_)) {
            return self.conns[at].refuse(
                t,
                code::NOT_CONTROLLER,
                "an admin connection may only GetInfo and Kill",
            );
        }
        let controller = role == Role::Controller;
        let needs_controller = matches!(
            msg,
            Message::Spawn(_)
                | Message::Input(_)
                | Message::Resize(_)
                | Message::SetColors(_)
                | Message::SetOptions(_)
                | Message::Close { .. }
                | Message::ClipboardReply(_)
        );
        if needs_controller && !controller {
            return self.conns[at].refuse(t, code::NOT_CONTROLLER, "only the controller may do that");
        }
        let needs_program = matches!(
            msg,
            Message::Attach(_)
                | Message::Input(_)
                | Message::Resize(_)
                | Message::SetColors(_)
                | Message::SetOptions(_)
                | Message::ClipboardReply(_)
        );
        if needs_program && self.program.is_none() {
            return self.conns[at].refuse(t, code::BAD_STATE, "nothing was spawned yet");
        }
        match msg {
            Message::Hello(_) => self.conns[at].refuse(t, code::BAD_STATE, "Hello was said already"),
            Message::GetInfo => {
                let info = self.welcome(wire::version::PROTO_MAJOR);
                self.conns[at].send(&Message::Info(info));
            }
            Message::Kill => {
                tracing::info!(conn = id.0, "asked to kill");
                self.end(Duration::ZERO, now);
            }
            Message::Close { grace_ms } => {
                tracing::info!(conn = id.0, grace_ms, "asked to close");
                self.end(Duration::from_millis(u64::from(grace_ms)), now);
            }
            Message::Ping(n) => {
                let pty_progress = self.program.as_ref().map_or(0, |p| p.progress());
                self.conns[at].send(&Message::Pong { n, pty_progress });
            }
            Message::Detach => {
                self.detach(at);
                self.drop_spawn_cursor(id);
                // The place is free at once, for an app that reconnects before this one is gone.
                if self.controller == Some(id) {
                    self.controller = None;
                }
                self.conns[at].closing = true;
            }
            Message::Spawn(spawn) => self.spawn(at, &spawn),
            Message::Attach(attach) => self.attach(at, attach, now),
            Message::Ack(consumed) => {
                if let Some(attach) = &mut self.conns[at].attach {
                    // An ack can only be for what was sent: a larger one would open the window wide.
                    attach.acked = attach.acked.max(consumed.min(attach.sent));
                }
            }
            Message::Input(bytes) => {
                if let Some(program) = &mut self.program {
                    program.input(bytes);
                }
            }
            Message::Resize(resize) => {
                if !valid_size(resize.size) {
                    return self.conns[at].refuse(t, code::INVALID, "size out of range");
                }
                // Coalesced: the latest request is applied at the next turn that may resize.
                self.resize = Some(resize);
            }
            Message::SetColors(colors) => {
                if let Some(program) = &mut self.program {
                    program.set_colors(&colors);
                }
            }
            Message::SetOptions(options) => {
                if let Some(program) = &mut self.program {
                    program.set_options(&options);
                }
            }
            Message::ClipboardReply(reply) => {
                if let Some(program) = &self.program {
                    program.clipboard_reply(reply.ticket, reply.data);
                }
            }
            // The history budget is P2's, and hooks are P3's: accepted and not acted on yet.
            Message::SetLimits(_) | Message::Hook(_) => {}
            Message::PageGet(_) | Message::FindStart(_) | Message::FindCancel { .. } => {
                self.conns[at].refuse(t, code::UNSUPPORTED, "not supported by this host")
            }
            // What only a host sends.
            _ => self.conns[at].refuse(t, code::BAD_STATE, "not a message for the host"),
        }
    }

    fn hello(&mut self, at: usize, msg: &Message<'_>) {
        let Message::Hello(hello) = msg else {
            self.conns[at].refuse(msg.ty(), code::BAD_STATE, "Hello first");
            self.conns[at].closing = true;
            return;
        };
        let id = self.conns[at].id;
        let range = if hello.role == Role::Admin { Range::admin() } else { Range::current() };
        let proto = match range.answer(hello, self.config.host_version) {
            Ok(proto) => proto,
            Err(incompatible) => {
                tracing::info!(
                    conn = id.0,
                    min = hello.proto_min,
                    max = hello.proto_max,
                    "incompatible client"
                );
                self.conns[at].send(&Message::Incompatible(incompatible));
                self.conns[at].closing = true;
                return;
            }
        };
        match hello.role {
            Role::Controller => {
                if let Some(old) = self.controller {
                    if !hello.force {
                        self.conns[at].refuse(ty::HELLO, code::BUSY, "another controller is attached");
                        self.conns[at].closing = true;
                        return;
                    }
                    self.take_over(old);
                }
                self.controller = Some(id);
            }
            Role::Observer | Role::Admin => {}
            Role::Unknown(_) => {
                self.conns[at].refuse(ty::HELLO, code::INVALID, "unknown role");
                self.conns[at].closing = true;
                return;
            }
        }
        let Some(at) = self.index(id) else { return };
        let conn = &mut self.conns[at];
        conn.phase = Phase::Ready { role: hello.role };
        conn.dec.set_max_payload(wire::MAX_PAYLOAD);
        tracing::info!(conn = id.0, role = hello.role.to_wire(), proto, "hello");
        let welcome = self.welcome(proto);
        self.conns[at].send(&Message::Welcome(welcome));
    }

    /// A controller that says `Hello{force}` takes the place: the old one is told and let go.
    fn take_over(&mut self, old: ConnId) {
        let Some(at) = self.index(old) else { return };
        tracing::info!(conn = old.0, "controller taken over");
        self.detach(at);
        let conn = &mut self.conns[at];
        conn.send(&Message::Detached(DetachReason::TakenOver));
        conn.closing = true;
        self.controller = None;
    }

    fn spawn(&mut self, at: usize, spawn: &wire::Spawn<'_>) {
        if !matches!(self.life, Life::Unspawned { .. }) || self.program.is_some() {
            return self.conns[at].refuse(ty::SPAWN, code::BAD_STATE, "this host has a program already");
        }
        if spawn.program.is_empty() || !valid_size(spawn.size) {
            return self.conns[at].refuse(ty::SPAWN, code::INVALID, "no program, or a size out of range");
        }
        let id = self.conns[at].id;
        match self.spawner.spawn(spawn, self.wake.clone()) {
            Ok((program, cursor)) => {
                let pid = program.pid();
                tracing::info!(conn = id.0, pid, args = spawn.args.len(), env = spawn.env.len(), "spawned");
                let info = {
                    self.program = Some(program);
                    self.welcome(wire::version::PROTO_MAJOR)
                };
                self.spawn_cursor = Some((id, cursor));
                self.life = Life::Running;
                self.notices.push(Notice::Spawned { pid });
                self.conns[at].send(&Message::Info(info));
            }
            Err(err) => {
                // The text is the system's error about the program (not found, not executable).
                let text = format!("{err:#}");
                tracing::info!(conn = id.0, "spawn failed");
                let shown: String = text.chars().take(1024).collect();
                self.conns[at].send(&Message::Error(wire::ErrorMsg {
                    ref_type: ty::SPAWN,
                    code: code::SPAWN_FAILED,
                    text: &shown,
                }));
                self.notices.push(Notice::SpawnFailed(text));
                self.finish();
            }
        }
    }

    fn attach(&mut self, at: usize, attach: Attach, now: Instant) {
        let window = if attach.window == 0 { self.config.default_window } else { attach.window };
        let window = u64::from(window.clamp(self.config.min_window, self.config.max_window));
        if matches!(attach.stream, StreamMode::Unknown(_)) {
            return self.conns[at].refuse(ty::ATTACH, code::INVALID, "unknown stream mode");
        }
        let id = self.conns[at].id;
        // The spawning connection's first attach reads from the stream's first byte.
        let first = match self.spawn_cursor.take() {
            Some((owner, cursor)) if owner == id => Some(cursor),
            other => {
                self.spawn_cursor = other;
                None
            }
        };
        self.detach(at);
        let Some(program) = &self.program else { return };
        let exited = program.exit().is_some();
        let mut attachment = Attachment::new(attach.stream, window);
        match attach.stream {
            StreamMode::Raw => {
                // Anyone but the spawning connection reads from now on.
                let cursor = first.unwrap_or_else(|| program.follow(true, Duration::ZERO, self.wake.clone()));
                attachment.sent = cursor.sent();
                attachment.acked = attachment.sent;
                attachment.cursor = Some(cursor);
            }
            StreamMode::Sync => {
                attachment.waiting =
                    Some(Waiting { history: attach.history_lines as usize, since: now, retry: now });
                attachment.exited_after_repaint = exited;
            }
            _ => {
                attachment.cursor = Some(program.follow(false, self.config.counters_every, self.wake.clone()))
            }
        }
        tracing::info!(conn = id.0, mode = attach.stream.to_wire(), window, "attached");
        let controller = self.controller == Some(id);
        if controller {
            program.set_controller(true);
        }
        let conn = &mut self.conns[at];
        if exited && attach.stream != StreamMode::Sync {
            let exit = program.exit();
            if let Some(exit) = exit {
                let offset = attachment.sent;
                conn.send(&Message::Event(wire::Event { offset, kind: wire::EventKind::Exited(exit) }));
            }
        }
        conn.attach = Some(attachment);
        // A `Sync` attach makes its repaint at once if it can.
        if attach.stream == StreamMode::Sync {
            self.try_repaint(at, now);
        }
    }

    /// Stop the connection's stream (it stays connected).
    pub(super) fn detach(&mut self, at: usize) {
        let id = self.conns[at].id;
        if self.conns[at].attach.take().is_some()
            && self.controller == Some(id)
            && let Some(program) = &self.program
        {
            program.set_controller(false);
        }
    }

    pub(super) fn drop_spawn_cursor(&mut self, id: ConnId) {
        if self.spawn_cursor.as_ref().is_some_and(|(owner, _)| *owner == id) {
            self.spawn_cursor = None;
        }
    }
}

fn valid_size(size: wire::Size) -> bool {
    (1..=MAX_CELLS).contains(&size.cols) && (1..=MAX_CELLS).contains(&size.rows)
}
