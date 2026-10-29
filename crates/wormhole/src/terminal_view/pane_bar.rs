//! The bar along the bottom of a pane: a program's question about the clipboard, and the two notices that
//! stay here when the notice chip is switched off, because a person should still see them: that a program
//! wrote to the clipboard, and that Put at Prompt refused a command.
//!
//! A program may set the clipboard by default and the bar says so; reading it is asked about
//! (core's [`ReadGate`] decides, `clipboard_read` is the setting). The question holds the engine's
//! reply slot, so the replies the program asked for after the read wait behind it: every way out of
//! the question (a choice, the timeout, the terminal closing) lets the slot go, with the text or
//! with nothing. The bar itself is drawn in `render/pane_bar.rs`.

use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::config::clipboard::{
    ASK_TIMEOUT, Choice, ClipboardRead, ClipboardWrite, ReadGate, Request,
};
use wormhole_core::view::notice::Copied;
use wormhole_core::view::sidebar::names::command_name;
use wormhole_term::{BODY_MAX_CHARS, Reply, TextFormatter, clean};
use wormhole_text::{keys, text};

use super::TerminalView;
use crate::settings::settings_in;
use crate::style::BannerKind;

/// How long a notice stays.
const NOTICE_FOR: Duration = Duration::from_secs(4);

/// What the view keeps about the bar.
#[derive(Default)]
pub(super) struct State {
    gate: ReadGate,
    /// The question that is out.
    ask: Option<Ask>,
    /// What the chip would have said, until it is old.
    note: Option<Note>,
    /// Counts the notices, so an old one's timer leaves a newer one alone.
    notes: u64,
}

struct Ask {
    id: u64,
    format: TextFormatter,
    reply: Reply,
    /// Who asks, as the bar says it.
    program: String,
    /// Denies the read when nobody answers; dropped (cancelled) with the question.
    _timeout: Task<()>,
}

struct Note {
    serial: u64,
    kind: BannerKind,
    text: String,
    /// Removes the notice; dropped (cancelled) with it.
    _expiry: Task<()>,
}

/// What the bar over the pane says.
pub(super) enum Bar<'a> {
    /// A program wants to read the clipboard.
    Ask(&'a str),
    /// What the chip would have said, where it is off.
    Notice(BannerKind, &'a str),
}

impl TerminalView {
    /// OSC 52 set the clipboard.
    pub(super) fn clipboard_store(&mut self, text: String, cx: &mut Context<Self>) {
        if settings_in(&self.profile, cx).clipboard_write == ClipboardWrite::Deny {
            tracing::info!("a program's clipboard write was refused (clipboard_write is deny)");
            return;
        }
        let (copied, by_program) = (Copied::text(&text), Copied::by_program(&text));
        // A copy on select takes the clipboard; keep what it held for ⌘V over the selection.
        let replaced = self
            .app_selection
            .copy_from_selection(Instant::now())
            .then(|| crate::platform::clipboard::read(cx));
        crate::platform::clipboard::write(text.clone(), cx);
        if let Some(replaced) = replaced {
            self.app_selection.copied(text, replaced);
        }
        let program = self.foreground_name();
        if crate::notice::enabled(&self.profile, cx) {
            // The chip says it, about this terminal: never the bar as well. In the terminal the person is
            // typing in the copy is "Copied 12 characters", as any copy; anywhere else it says which program
            // did it, as the person did not. (A terminal that is not on screen gets no chip: see `notice`.)
            if self.focus_reported {
                self.say(copied, cx);
            } else {
                let words = program.and_then(|name| crate::notice::program_copy(&name, by_program));
                self.say_words(by_program, words, cx);
            }
            return;
        }
        // The chip is off in this window: a program silently writing to the clipboard is worth seeing.
        let named = program.and_then(|name| crate::notice::program_copy(&name, by_program));
        if let Some(words) = named.or_else(|| crate::notice::sentence(by_program.into())) {
            self.say_in_bar(BannerKind::Info, words, cx);
        }
    }

    /// Say `text` in the bar for a few seconds.
    pub(super) fn say_in_bar(&mut self, kind: BannerKind, text: String, cx: &mut Context<Self>) {
        let state = &mut self.bar;
        state.notes += 1;
        let serial = state.notes;
        let expiry = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(NOTICE_FOR).await;
            this.update(cx, |this, cx| {
                if this.bar.note.as_ref().is_some_and(|note| note.serial == serial) {
                    this.bar.note = None;
                    cx.notify();
                }
            })
            .ok();
        });
        self.bar.note = Some(Note { serial, kind, text, _expiry: expiry });
        cx.notify();
    }

    /// OSC 52 asked for the clipboard. `reply` is the program's place among the replies; every
    /// path below either answers it or keeps it in the question, which answers it later.
    pub(super) fn clipboard_load(&mut self, format: TextFormatter, reply: Reply, cx: &mut Context<Self>) {
        let policy = settings_in(&self.profile, cx).clipboard_read;
        match self.bar.gate.request(policy, self.in_front) {
            Request::Answer => self.give_clipboard(&format, &reply, cx),
            Request::Deny => {
                if policy == ClipboardRead::Ask {
                    tracing::info!(
                        "a clipboard read was refused (one is out, or nobody can see this terminal)"
                    );
                }
                reply.send(Vec::new());
            }
            Request::Ask(id) => {
                let timeout = cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(ASK_TIMEOUT).await;
                    this.update(cx, |this, cx| this.settle_ask(id, Choice::Deny, cx)).ok();
                });
                let program = self.program_label();
                crate::platform::a11y::announce(&ask_words(&program), true);
                self.bar.ask = Some(Ask { id, format, reply, program, _timeout: timeout });
                cx.notify();
            }
        }
    }

    /// The person chose on the bar.
    pub(super) fn answer_clipboard_ask(&mut self, choice: Choice, cx: &mut Context<Self>) {
        if let Some(id) = self.bar.ask.as_ref().map(|ask| ask.id) {
            self.settle_ask(id, choice, cx);
        }
    }

    /// Close question `id` with `choice`, answering the program.
    fn settle_ask(&mut self, id: u64, choice: Choice, cx: &mut Context<Self>) {
        let Some(allowed) = self.bar.gate.resolve(id, choice) else { return };
        let Some(ask) = self.bar.ask.take() else { return };
        if allowed {
            self.give_clipboard(&ask.format, &ask.reply, cx);
        } else {
            ask.reply.send(Vec::new());
        }
        cx.notify();
    }

    fn give_clipboard(&self, format: &TextFormatter, reply: &Reply, cx: &mut Context<Self>) {
        let text = crate::platform::clipboard::read(cx).unwrap_or_default();
        reply.send(format(&text).into_bytes());
    }

    /// What the bar over the pane says now: a question first, else the last notice.
    pub(super) fn pane_bar(&self) -> Option<Bar<'_>> {
        match (&self.bar.ask, &self.bar.note) {
            (Some(ask), _) => Some(Bar::Ask(&ask.program)),
            (None, Some(note)) => Some(Bar::Notice(note.kind, &note.text)),
            (None, None) => None,
        }
    }

    /// The program in front as the sidebar names it, cleaned and cut; `None` when there is none.
    fn foreground_name(&self) -> Option<String> {
        self.foreground
            .as_ref()
            .map(|foreground| clean(&command_name(&foreground.command), BODY_MAX_CHARS.min(40)))
            .filter(|name| !name.is_empty())
    }

    /// Who the question names: the program in front, else "A program".
    fn program_label(&self) -> String {
        self.foreground_name().unwrap_or_else(|| text(keys::TERMINAL_PROGRAM_UNNAMED).to_string())
    }

    /// The terminal is shown in a window in front (set by the workspace's tick): the only state
    /// in which a question can be asked.
    pub fn set_in_front(&mut self, in_front: bool) {
        self.in_front = in_front;
    }
}

/// What the question says (the bar, VoiceOver, devctl): `program` wants to read the clipboard.
pub(super) fn ask_words(program: &str) -> String {
    keys::terminal_clipboard_ask(program)
}

/// What the tests of the window do to the bar.
#[cfg(test)]
impl TerminalView {
    /// A program asks to read the clipboard (OSC 52), in a terminal that is shown: the bar asks.
    pub(crate) fn ask_clipboard_for_test(&mut self, cx: &mut Context<Self>) {
        use wormhole_term::{PtyEvent, Reply};

        self.set_in_front(true);
        let format: TextFormatter = std::sync::Arc::new(|text: &str| text.to_string());
        self.event_for_test(PtyEvent::ClipboardLoad(format, Reply::detached()), cx);
    }

    /// Which program is in front, as the sidebar would show it.
    pub(crate) fn set_foreground_for_test(&mut self, command: &str) {
        self.foreground = Some(wormhole_core::agents::Foreground {
            command: command.to_string(),
            started: std::time::SystemTime::now(),
        });
    }

    /// What the bar's notice says, if it has one.
    pub(crate) fn bar_notice_for_test(&self) -> Option<String> {
        match self.pane_bar() {
            Some(Bar::Notice(_, text)) => Some(text.to_string()),
            _ => None,
        }
    }

    /// A program writes `text` to the clipboard (OSC 52), in a terminal that is `focused` (its window active,
    /// the terminal focused) or not.
    pub(crate) fn store_clipboard_for_test(&mut self, text: &str, focused: bool, cx: &mut Context<Self>) {
        use wormhole_term::PtyEvent;

        self.focus_reported = focused;
        self.event_for_test(PtyEvent::ClipboardStore(text.to_string()), cx);
    }

    /// Select everything and copy it, as ⌘A and ⌘C do.
    pub(crate) fn copy_all_for_test(&mut self, cx: &mut Context<Self>) {
        if let Some(pty) = &self.pty {
            pty.select_all();
        }
        self.run_command(super::input::TerminalCommand::Copy, cx);
    }

    /// The bar asks the person about the clipboard.
    pub(crate) fn asking_for_test(&self) -> bool {
        matches!(self.pane_bar(), Some(Bar::Ask(_)))
    }
}

/// devctl's view of the bar.
#[cfg(feature = "devctl")]
impl TerminalView {
    /// `pane-bar`: what the bar over the pane says, or "none".
    pub fn devctl_pane_bar(&self) -> String {
        match self.pane_bar() {
            Some(Bar::Ask(program)) => format!("ask: {}", ask_words(program)),
            Some(Bar::Notice(_, text)) => format!("notice: {text}"),
            None => "none".to_string(),
        }
    }

    /// `clipboard-answer once|always|deny`.
    pub fn devctl_clipboard_answer(&mut self, choice: &str, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let choice = match choice {
            "once" => Choice::Once,
            "always" => Choice::Always,
            "deny" => Choice::Deny,
            other => anyhow::bail!("expected once, always or deny, got {other:?}"),
        };
        anyhow::ensure!(self.bar.ask.is_some(), "no question is out");
        self.answer_clipboard_ask(choice, cx);
        Ok(())
    }
}
