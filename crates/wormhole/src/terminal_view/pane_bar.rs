//! The bar along the bottom of a pane: a program's question about the clipboard, and the short
//! notices a terminal gives about what a program (or a block's action) did or was refused.
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
use wormhole_core::view::sidebar::names::command_name;
use wormhole_term::{BODY_MAX_CHARS, Reply, TextFormatter, clean};

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
    /// The last notice, until it is old.
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
    Notice(BannerKind, &'a str),
}

impl TerminalView {
    /// OSC 52 set the clipboard.
    pub(super) fn clipboard_store(&mut self, text: String, cx: &mut Context<Self>) {
        if settings_in(&self.profile, cx).clipboard_write == ClipboardWrite::Deny {
            tracing::info!("a program's clipboard write was refused (clipboard_write is deny)");
            return;
        }
        let chars = text.chars().count();
        // A copy on select takes the clipboard; keep what it held for ⌘V over the selection.
        let replaced = self
            .app_selection
            .copy_from_selection(Instant::now())
            .then(|| cx.read_from_clipboard().and_then(|item| item.text()));
        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        if let Some(replaced) = replaced {
            self.app_selection.copied(text, replaced);
        }
        let what = if chars == 1 { "1 character".to_string() } else { format!("{chars} characters") };
        let text = format!("{} copied {what} to the clipboard", self.program_label());
        self.say_in_bar(BannerKind::Info, text, cx);
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
                crate::platform::a11y::announce(&format!("{program} wants to read the clipboard"), true);
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
        let text = cx.read_from_clipboard().and_then(|item| item.text()).unwrap_or_default();
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

    /// Who the bar names: the program in front, as the sidebar names it, else "A program".
    fn program_label(&self) -> String {
        self.foreground
            .as_ref()
            .map(|foreground| clean(&command_name(&foreground.command), BODY_MAX_CHARS.min(40)))
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "A program".to_string())
    }

    /// The terminal is shown in a window in front (set by the workspace's tick): the only state
    /// in which a question can be asked.
    pub fn set_in_front(&mut self, in_front: bool) {
        self.in_front = in_front;
    }
}

/// devctl's view of the bar.
#[cfg(feature = "devctl")]
impl TerminalView {
    /// `pane-bar`: what the bar over the pane says, or "none".
    pub fn devctl_pane_bar(&self) -> String {
        match self.pane_bar() {
            Some(Bar::Ask(program)) => format!("ask: {program} wants to read the clipboard"),
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
