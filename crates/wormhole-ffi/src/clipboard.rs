//! A program's use of the clipboard (OSC 52) for a `Terminal`: core's rules, Swift's pasteboard.
//!
//! The engine hands over a store (set the clipboard) and a load (read it, with a reply slot the program's
//! later replies wait behind). What is allowed is core's (`config::clipboard`: the two settings, the
//! [`ReadGate`] that asks about a read once at a time and only where the person can see it); what Swift does is
//! the pasteboard and the question. Every path out of a load answers its slot, with the text or with nothing.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use wormhole_core::config::clipboard::{
    ASK_TIMEOUT, Choice, ClipboardRead, ClipboardWrite, ReadGate, Request,
};
use wormhole_core::view::sidebar::names::command_name;
use wormhole_term::{BODY_MAX_CHARS, Reply, TextFormatter, clean};

use crate::events::{EventQueue, TermEvent};
use crate::terminal::Terminal;

/// `clipboard_read` in `config.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ClipboardReadRule {
    Ask,
    Allow,
    Deny,
}

/// What the person chose on the question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ClipboardChoice {
    Once,
    Always,
    Deny,
}

struct Load {
    format: TextFormatter,
    reply: Reply,
}

impl Load {
    fn answer(self, text: &str) {
        self.reply.send((self.format)(text).into_bytes());
    }
}

struct Pending {
    gate: ReadGate,
    read: ClipboardRead,
    write: ClipboardWrite,
    /// Loads the gate let through, waiting for Swift to give the text, oldest first.
    ready: VecDeque<Load>,
    /// The load the question is about.
    ask: Option<(u64, Load)>,
}

/// One terminal's clipboard state, held by its event queue.
pub(crate) struct Clipboard {
    pending: Mutex<Pending>,
    /// The terminal is shown in a window the person can see: the only state in which a question can be asked.
    on_screen: AtomicBool,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            pending: Mutex::new(Pending {
                gate: ReadGate::default(),
                read: ClipboardRead::default(),
                write: ClipboardWrite::default(),
                ready: VecDeque::new(),
                ask: None,
            }),
            on_screen: AtomicBool::new(false),
        }
    }
}

impl Clipboard {
    fn lock(&self) -> std::sync::MutexGuard<'_, Pending> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// OSC 52 set the clipboard: the event for Swift, unless `clipboard_write` is deny.
    pub(crate) fn store(&self, text: String) -> Option<TermEvent> {
        if self.lock().write == ClipboardWrite::Deny {
            tracing::info!("a program's clipboard write was refused (clipboard_write is deny)");
            return None;
        }
        Some(TermEvent::ClipboardWrite { text })
    }

    /// OSC 52 asked for the clipboard: answered now (refused), or queued for Swift to give or to ask about.
    pub(crate) fn load(
        &self,
        queue: &Arc<EventQueue>,
        format: TextFormatter,
        reply: Reply,
    ) -> Option<TermEvent> {
        let mut pending = self.lock();
        let on_screen = self.on_screen.load(Ordering::Acquire);
        let load = Load { format, reply };
        let policy = pending.read;
        match pending.gate.request(policy, on_screen) {
            Request::Answer => {
                pending.ready.push_back(load);
                Some(TermEvent::ClipboardRead)
            }
            Request::Deny => {
                load.answer("");
                None
            }
            Request::Ask(id) => {
                pending.ask = Some((id, load));
                let queue = Arc::clone(queue);
                // The replies behind the question wait for it, so nobody answering counts as "Deny".
                let timeout =
                    std::thread::Builder::new().name("wormhole-ffi-clipboard-ask".into()).spawn(move || {
                        std::thread::sleep(ASK_TIMEOUT);
                        queue.settle_clipboard_ask(id, Choice::Deny, "");
                    });
                if timeout.is_err() {
                    if let Some((_, load)) = pending.ask.take() {
                        load.answer("");
                    }
                    return None;
                }
                Some(TermEvent::ClipboardAsk)
            }
        }
    }

    /// Close question `id`; the program gets `text` when the choice allows it. Whether it was the open one.
    pub(crate) fn settle(&self, id: u64, choice: Choice, text: &str) -> bool {
        let mut pending = self.lock();
        let Some(allowed) = pending.gate.resolve(id, choice) else { return false };
        if let Some((_, load)) = pending.ask.take() {
            load.answer(if allowed { text } else { "" });
        }
        true
    }
}

impl EventQueue {
    /// A choice or the timeout settled the open question: Swift's bar goes.
    pub(crate) fn settle_clipboard_ask(&self, id: u64, choice: Choice, text: &str) {
        if self.clipboard.settle(id, choice, text) {
            self.push_event(TermEvent::ClipboardAskClosed);
        }
    }
}

#[uniffi::export]
impl Terminal {
    /// The settings `clipboard_read` and `clipboard_write` (false: deny), for this terminal's programs.
    pub fn set_clipboard_rules(&self, read: ClipboardReadRule, write_allowed: bool) {
        let mut pending = self.events.clipboard.lock();
        pending.read = match read {
            ClipboardReadRule::Ask => ClipboardRead::Ask,
            ClipboardReadRule::Allow => ClipboardRead::Allow,
            ClipboardReadRule::Deny => ClipboardRead::Deny,
        };
        pending.write = if write_allowed { ClipboardWrite::Allow } else { ClipboardWrite::Deny };
    }

    /// Whether the terminal is shown where the person can see it (a read is asked about only then).
    pub fn set_on_screen(&self, on_screen: bool) {
        self.events.clipboard.on_screen.store(on_screen, Ordering::Release);
    }

    /// The program in front as the sidebar names it, cleaned and cut: who a question names. `None` when there is none.
    pub fn foreground_program(&self) -> Option<String> {
        let name = clean(&command_name(&self.probe().command?.command), BODY_MAX_CHARS.min(40));
        (!name.is_empty()).then_some(name)
    }

    /// The clipboard's text for the oldest read that was let through (`ClipboardRead`).
    pub fn clipboard_give(&self, text: String) {
        let load = self.events.clipboard.lock().ready.pop_front();
        if let Some(load) = load {
            load.answer(&text);
        }
    }

    /// The person's answer to the question (`ClipboardAsk`); `text` is the clipboard's, read only for a yes.
    pub fn clipboard_answer(&self, choice: ClipboardChoice, text: String) {
        let Some(id) = self.events.clipboard.lock().ask.as_ref().map(|(id, _)| *id) else { return };
        let choice = match choice {
            ClipboardChoice::Once => Choice::Once,
            ClipboardChoice::Always => Choice::Always,
            ClipboardChoice::Deny => Choice::Deny,
        };
        self.events.settle_clipboard_ask(id, choice, &text);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::events::TerminalListener;

    struct Silent;
    impl TerminalListener for Silent {
        fn wake(&self) {}
    }

    fn load(queue: &Arc<EventQueue>) -> Option<TermEvent> {
        queue.clipboard.load(queue, Arc::new(|text: &str| text.to_string()), Reply::detached())
    }

    #[test]
    fn a_read_is_asked_once_at_a_time_and_only_where_it_can_be_seen() {
        let queue = EventQueue::new(Arc::new(Silent));
        assert_eq!(load(&queue), None, "nobody can see this terminal");
        queue.clipboard.on_screen.store(true, Ordering::Release);
        assert_eq!(load(&queue), Some(TermEvent::ClipboardAsk));
        assert_eq!(load(&queue), None, "one question at a time");
        let id = queue.clipboard.lock().ask.as_ref().map(|(id, _)| *id).expect("a question is out");
        queue.settle_clipboard_ask(id, Choice::Always, "text");
        assert_eq!(load(&queue), Some(TermEvent::ClipboardRead), "always lets the later reads through");
    }

    #[test]
    fn a_write_obeys_its_setting() {
        let queue = EventQueue::new(Arc::new(Silent));
        assert_eq!(queue.clipboard.store("x".into()), Some(TermEvent::ClipboardWrite { text: "x".into() }));
        queue.clipboard.lock().write = ClipboardWrite::Deny;
        assert_eq!(queue.clipboard.store("x".into()), None);
    }
}
