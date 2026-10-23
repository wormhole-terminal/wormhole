//! Starting a terminal, whether its program still runs, its process id and what closing it does.

use crate::Pty;
use std::sync::Mutex;

use crate::events::EventProxy;
use crate::{Backend, GridSize, Launch, PtyEvents, StartOptions, engine, pty};

impl Pty {
    /// Start `launch` in a PTY of `size`, with the engine set up by `options`.
    pub fn start(
        size: GridSize,
        options: &StartOptions,
        launch: Launch,
    ) -> anyhow::Result<(Self, PtyEvents)> {
        let (proxy, events) = EventProxy::new();
        let shared = proxy.shared().clone();
        let engine::Spawned { screen, io, pid } = engine::spawn(size, options, &launch, proxy, &shared)?;
        Ok((
            Self {
                term: screen,
                shared,
                io: Backend::Local(io),
                size,
                pid,
                hangup_grace: pty::HANGUP_GRACE,
                reported_cwd: Mutex::new(None),
            },
            events,
        ))
    }

    /// `start` with an explicit program, in the environment a terminal's program has (the TERM it
    /// is told, its folder), without the app's launch around it.
    #[cfg(test)]
    pub(crate) fn spawn_program(
        size: GridSize,
        options: &StartOptions,
        program: &str,
        args: &[&str],
    ) -> anyhow::Result<(Self, PtyEvents)> {
        use crate::{Shell, sys};

        let shell = Shell::new(program.into(), args.iter().map(|arg| arg.to_string()).collect());
        let env = [("TERM", "xterm-256color"), ("COLORTERM", "truecolor"), ("TERM_PROGRAM", "Wormhole")]
            .into_iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        let cwd = sys::paths::home();
        Self::start(size, options, Launch { shell, cwd, env })
    }

    /// Whether the shell still runs. After it exits its screen stays, to read,
    /// select and save, but the process is gone: input goes nowhere and its pid
    /// is forgotten (it may already belong to another process).
    pub fn is_alive(&self) -> bool {
        !self.shared.exited()
    }

    pub(super) fn pid(&self) -> Option<u32> {
        self.is_alive().then_some(self.pid)
    }

    /// The shell's process id while it runs: what a query that is done on another thread
    /// (`listening_ports`) is given, since the terminal itself stays with the UI.
    pub fn shell_pid(&self) -> Option<u32> {
        self.pid()
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        match &self.io {
            // A shell that has exited is gone, and so is its thread (its pid may be someone else's).
            Backend::Local(io) => {
                if self.pid().is_some() {
                    io.close(self.hangup_grace);
                }
            }
            // The host keeps an exited program's screen for a week: closing the terminal ends it
            // too. Not after `detach`, which leaves it running for the next attach.
            Backend::Hosted(host) => host.close(self.hangup_grace),
        }
    }
}
