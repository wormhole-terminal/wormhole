//! The desk's side of a mode widened outside Wormhole (`modes` holds the rule): the looks at what the profile's file
//! says, the question about a widening and the person's answer to it.

use wormhole_core::config::AgentAccess;
use wormhole_text::keys::{
    SETTINGS_AGENTS_ACCESS_ASK, SETTINGS_AGENTS_ACCESS_OFF, SETTINGS_AGENTS_ACCESS_OPEN,
};
use wormhole_text::text;

use super::{AccessDesk, locked, modes};
use crate::aux_backends::access_records::AccessModeRecord;

#[uniffi::export]
impl AccessDesk {
    /// The question about a mode that was widened outside Wormhole, when one waits to be put on screen (it comes
    /// before the questions of programs). Re-arms the wake.
    pub fn next_mode_question(&self) -> Option<AccessModeRecord> {
        self.waker.rearm();
        let change = locked(&self.modes).next_to_ask()?;
        let token = change.token;
        // Not answered is not kept.
        self.after_wait(move |desk| desk.answer_mode(token, false));
        let word = |mode: AgentAccess| {
            text(match mode {
                AgentAccess::Off => SETTINGS_AGENTS_ACCESS_OFF,
                AgentAccess::Ask => SETTINGS_AGENTS_ACCESS_ASK,
                AgentAccess::Open => SETTINGS_AGENTS_ACCESS_OPEN,
            })
            .to_string()
        };
        Some(AccessModeRecord { token, previous: word(change.previous), requested: word(change.requested) })
    }

    /// Whether the mode question `token` is still waiting for the person.
    pub fn is_waiting_mode(&self, token: u64) -> bool {
        // A read after a wake, like taking the next question: while a question is up this is the only read the window
        // makes, and without re-arming here the wake that says it was settled would never be sent.
        self.waker.rearm();
        locked(&self.modes).is_waiting(token)
    }

    /// The person's answer to the mode question `token`: keep the widened mode, or put the earlier one back in the
    /// file (that write is the person's own edit, with nothing to ask). An answer to another question does nothing.
    pub fn answer_mode(&self, token: u64, keep: bool) {
        let Some(change) = locked(&self.modes).pending(token) else { return };
        if !keep {
            // Written while the question still waits: a look in between would find the wider mode in the file with nothing
            // pending, and ask again. Through the settings the app holds of the profile, as the GPUI app's goes through
            // its settings store (`agent_access/change.rs:80`), never around them from a load of the file.
            let previous = change.previous;
            crate::settings::live::update(&self.profile, |settings| settings.agent_access = previous);
        }
        if locked(&self.modes).answered(token, keep).is_none() {
            return;
        }
        // Kept, and it is Allowed while open: there is nothing left to ask, so what waits is let in, as the GPUI app's
        // next tick does (`agent_access/ask.rs:434-443`).
        if keep && change.requested == AgentAccess::Open {
            self.settle_questions(Some(AgentAccess::Open));
        }
        self.waker.notify();
    }
}

impl AccessDesk {
    /// What a look at the profile's file is stamped with, taken before the file is read ([`Self::mode_in_force`]).
    pub(crate) fn stamp(&self) -> u64 {
        locked(&self.modes).stamp()
    }

    /// The mode in force with no look at the file; none before the first look.
    pub(crate) fn mode_held(&self) -> Option<AgentAccess> {
        locked(&self.modes).held()
    }

    /// The mode in force for this profile when its file, read after `stamp` was taken, says `want`: that, unless it
    /// is a widening the person did not make in Settings, which keeps the earlier mode and, where `asks`, asks them
    /// (`modes`). `None` when the read is too old to say: read the file and look again.
    pub(crate) fn mode_in_force(&self, want: AgentAccess, stamp: u64, asks: bool) -> Option<AgentAccess> {
        // Read after the file was: an edit Settings made is noted before its write, so a file that holds it is never
        // looked at without it.
        let edit = crate::settings::edits::last(self.profile.dir());
        let (mode, asked) = locked(&self.modes).reconcile(modes::Look { want, edit, stamp, asks })?;
        if asked {
            tracing::warn!(
                "agent access was widened outside Wormhole to {}: asking whether to keep it",
                want.word()
            );
            self.waker.notify();
        }
        Some(mode)
    }
}
