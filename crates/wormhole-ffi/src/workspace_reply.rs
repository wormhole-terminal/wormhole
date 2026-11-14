//! Reply, from a notification or the quick panel (the GPUI app's `notifications::reply`,
//! `Workspace::type_reply` in `window/control/terminals.rs` and `paste_text` in
//! `window/feature_terminals.rs`): the answer is pasted into the terminal the way its program takes a paste,
//! and Return follows a moment later. The terminal is found in the workspace, not through a view, so one that
//! runs without having been shown yet is answered too.

use std::time::Duration;

use crate::workspace::Workspace;

/// The pause between a pasted text and the Return that submits it: the program reads the paste first.
const SUBMIT_DELAY: Duration = Duration::from_millis(60);

#[uniffi::export]
impl Workspace {
    /// Paste `text`, trimmed, into terminal `tab` and press Return a moment later. Whether there was anything
    /// to send and a running terminal of this workspace to send it to: a reply of nothing presses no Return.
    pub fn type_reply(&self, tab: i64, text: String) -> bool {
        let text = text.trim();
        let Some(terminal) = self.terminal(tab) else { return false };
        if text.is_empty() {
            return false;
        }
        // Bracketed where the program asked, ESC and the C1 controls removed (`wormhole_term::input::paste`);
        // typing, as far as the terminal and its view go.
        terminal.paste(text.to_string());
        terminal.control_typed();
        // A thread for the pause: the caller is the app's main thread.
        let spawned = std::thread::Builder::new().name("wormhole-reply-submit".into()).spawn(move || {
            std::thread::sleep(SUBMIT_DELAY);
            terminal.write(b"\r".to_vec());
        });
        if let Err(err) = spawned {
            tracing::warn!("couldn't press Return after a reply: {err}");
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use wormhole_core::workspace::Action;

    use crate::{Terminal, TerminalListener, Workspace, WorkspaceConfig, WorkspaceEvent, WorkspaceListener};

    struct Quiet;

    impl WorkspaceListener for Quiet {
        fn wake(&self) {}
    }

    impl TerminalListener for Quiet {
        fn wake(&self) {}
    }

    /// A workspace with one terminal running `/bin/cat`, registered as Swift registers it: no view is
    /// involved.
    fn workspace_with_cat() -> (Arc<Workspace>, i64, Arc<Terminal>) {
        crate::isolate_for_tests();
        let config = WorkspaceConfig {
            home: None,
            numbers_follow_list: true,
            start_folder: None,
            pane_header_height: 0.0,
        };
        let workspace = Workspace::new(config, Arc::new(Quiet));
        let id = match workspace.apply(Action::NewTerminal).first() {
            Some(WorkspaceEvent::Opened { id, .. }) => *id,
            other => panic!("a new terminal opens: {other:?}"),
        };
        let terminal = Terminal::start("/bin/cat".into(), vec![], 80, 24, 8.0, 16.0, Arc::new(Quiet))
            .expect("cat starts in an isolated process");
        workspace.register_terminal(id, terminal.clone());
        (workspace, id, terminal)
    }

    /// Wait (bounded) until `done` holds: a terminal's output arrives on a thread of its own.
    fn eventually(what: &str, mut done: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !done() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn a_reply_is_trimmed_pasted_and_submitted() {
        let (workspace, id, terminal) = workspace_with_cat();
        assert!(workspace.type_reply(id, "  yes please \n".into()));
        // Typed once (the echo) and, after the Return, said again by `cat`: the line was submitted.
        eventually("the reply said twice", || terminal.screen_text().matches("yes please").count() >= 2);
        let screen = terminal.screen_text();
        assert!(!screen.contains(" yes please"), "the reply is trimmed: {screen:?}");
    }

    #[test]
    fn nothing_is_sent_for_an_empty_reply_or_an_unknown_terminal() {
        let (workspace, id, terminal) = workspace_with_cat();
        assert!(!workspace.type_reply(id, " \t\n".into()), "spaces alone press no Return");
        assert!(!workspace.type_reply(id + 99, "yes".into()), "no such terminal");
        // What is typed next is the first thing on the screen: nothing came before it, not an empty line.
        assert!(workspace.type_reply(id, "after".into()));
        eventually("the next reply", || terminal.screen_text().matches("after").count() >= 2);
        let screen = terminal.screen_text();
        assert!(screen.starts_with("after"), "{screen:?}");
    }
}
