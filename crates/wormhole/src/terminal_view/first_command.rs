//! A command a new terminal is asked to run (`wh new-terminal --command`, `new-window`, `split`): typed
//! once its shell shows a prompt, not when the terminal is made.
//!
//! A terminal in a session host has no PTY for a moment after it is made (the host starts off the UI
//! thread, a few hundred milliseconds), and input written before that is dropped without a word. Even
//! with a PTY the shell may not have started: its startup files can take a second, and a shell that
//! reads its input before drawing a prompt (zsh's first-run questions) would take the command for an
//! answer. So the command waits for what a person would: the first prompt, told by the shell's own
//! mark when it has integration, else by the output going quiet, and at last by a time limit.

use std::time::{Duration, Instant};

use gpui_kit::*;

use super::TerminalView;

/// How often the terminal is looked at while it waits.
const POLL: Duration = Duration::from_millis(50);
/// Output that has stopped for this long is a prompt waiting.
const QUIET: Duration = Duration::from_millis(300);
/// A shell that has printed nothing this long after its PTY came has no prompt to wait for (a program
/// that reads its input and says nothing).
const SILENT: Duration = Duration::from_secs(2);
/// A shell whose output never settles is typed at all the same.
const GIVE_UP: Duration = Duration::from_secs(5);
/// A PTY that never comes (the host could not start and the fallback failed): the wait ends.
const NO_PTY: Duration = Duration::from_secs(30);
/// The pause between the pasted command and the Enter that runs it: the program reads the paste first.
const SUBMIT_DELAY: Duration = Duration::from_millis(60);

/// What a look at the terminal found.
#[derive(Clone, Copy, Debug)]
pub(super) enum Look {
    /// No PTY yet.
    NoPty,
    Shell {
        /// The shell's own mark: at its prompt, or not; `None` for a shell without integration.
        at_prompt: Option<bool>,
        /// Changes whenever the terminal prints.
        revision: u64,
    },
}

/// When the first command may be typed, decided from looks at the terminal.
#[derive(Default)]
pub(super) struct Wait {
    /// When the PTY was first seen, the revision then, the latest revision and when it changed.
    seen: Option<(Instant, u64, u64, Instant)>,
}

impl Wait {
    pub(super) fn ready(&mut self, now: Instant, look: Look) -> bool {
        let Look::Shell { at_prompt, revision } = look else { return false };
        let (since, first, last, changed) = self.seen.get_or_insert((now, revision, revision, now));
        if *last != revision {
            *last = revision;
            *changed = now;
        }
        let heard = revision != *first;
        at_prompt == Some(true)
            || (heard && now.duration_since(*changed) >= QUIET)
            || (!heard && now.duration_since(*since) >= SILENT)
            || now.duration_since(*since) >= GIVE_UP
    }
}

impl TerminalView {
    fn look(&self) -> Look {
        match &self.pty {
            None => Look::NoPty,
            Some(pty) => {
                let reader = pty.reader();
                Look::Shell { at_prompt: reader.at_prompt(), revision: reader.revision() }
            }
        }
    }

    /// Type `command` into the shell and run it as soon as the shell is ready for it (see the module).
    /// Dropped with the terminal.
    pub fn run_when_ready(&mut self, command: &str, cx: &mut Context<Self>) {
        let command = command.to_string();
        self._tasks.push(cx.spawn(async move |this, cx| {
            let started = Instant::now();
            let mut wait = Wait::default();
            loop {
                cx.background_executor().timer(POLL).await;
                let Ok(look) = this.read_with(cx, |this, _| this.look()) else { return };
                if wait.ready(Instant::now(), look) {
                    break;
                }
                if matches!(look, Look::NoPty) && started.elapsed() >= NO_PTY {
                    tracing::warn!("a terminal never got a shell: its first command was not typed");
                    return;
                }
            }
            let _ = this.update(cx, |this, _| this.paste(&command));
            cx.background_executor().timer(SUBMIT_DELAY).await;
            let _ = this.update(cx, |this, _| this.write(b"\r".to_vec()));
        }));
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Look, Wait};

    fn shell(at_prompt: Option<bool>, revision: u64) -> Look {
        Look::Shell { at_prompt, revision }
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn nothing_is_typed_before_the_terminal_has_a_pty() {
        let t0 = Instant::now();
        let mut wait = Wait::default();
        // Not at 10 seconds either: a host that is slow to start is waited for, and the clock of the
        // wait starts when the PTY does.
        assert!(!wait.ready(t0, Look::NoPty));
        assert!(!wait.ready(t0 + Duration::from_secs(10), Look::NoPty));
        assert!(!wait.ready(t0 + Duration::from_secs(10), shell(None, 0)));
        assert!(wait.ready(t0 + Duration::from_secs(12), shell(None, 0)), "a silent shell, in the end");
    }

    #[test]
    fn a_shell_that_marks_its_prompt_is_typed_at_the_moment_it_does() {
        let t0 = Instant::now();
        let mut wait = Wait::default();
        assert!(!wait.ready(t0, shell(Some(false), 3)));
        assert!(!wait.ready(t0 + ms(100), shell(None, 4)));
        assert!(wait.ready(t0 + ms(150), shell(Some(true), 5)));
    }

    #[test]
    fn output_that_stops_is_a_prompt_and_output_that_goes_on_is_waited_out() {
        let t0 = Instant::now();
        let mut wait = Wait::default();
        assert!(!wait.ready(t0, shell(None, 1)));
        // It printed (the banner, the prompt), and is quiet for 300 ms.
        assert!(!wait.ready(t0 + ms(100), shell(None, 2)));
        assert!(!wait.ready(t0 + ms(300), shell(None, 2)));
        assert!(wait.ready(t0 + ms(400), shell(None, 2)));
        // A shell that never stops printing is typed at after five seconds.
        let mut busy = Wait::default();
        assert!(!busy.ready(t0, shell(None, 1)));
        for step in 1..49 {
            assert!(!busy.ready(t0 + ms(step * 100), shell(None, 1 + step)), "{step}");
        }
        assert!(busy.ready(t0 + Duration::from_secs(5), shell(None, 100)));
    }
}
