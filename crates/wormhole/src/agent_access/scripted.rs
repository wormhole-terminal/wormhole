//! Who answers the question: the person, on a sheet, or a script, in a test or a test instance. Nothing
//! that runs under a test, and no test instance (`WORMHOLE_BACKGROUND=1`), ever puts a sheet on a screen.

use super::sheet::Button;

/// What answers a question in a test or a test instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Script {
    /// Nobody, until something answers it (`devctl`'s `agent-ask`, a test).
    Hold,
    /// This button, at once.
    Answer(Button),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Asker {
    /// A sheet on the front window.
    Sheet,
    Scripted(Script),
}

impl Asker {
    /// A sheet, except under a test and in a test instance, where `WORMHOLE_AGENT_ASK` says what answers
    /// (`deny` when it is unset or not understood: nothing gets in by accident).
    pub(super) fn detect() -> Self {
        if cfg!(test) || crate::platform::background_mode::on() {
            let word = if cfg!(feature = "devctl") { std::env::var("WORMHOLE_AGENT_ASK").ok() } else { None };
            Asker::Scripted(Self::script_of(word.as_deref()))
        } else {
            Asker::Sheet
        }
    }

    fn script_of(word: Option<&str>) -> Script {
        match word.map(|word| word.trim().to_ascii_lowercase()).as_deref() {
            Some("hold") => Script::Hold,
            Some("once" | "session") => Script::Answer(Button::Session),
            Some("quit") => Script::Answer(Button::Quit),
            Some("always") => Script::Answer(Button::Always),
            Some("deny") | None => Script::Answer(Button::Deny),
            Some(other) => {
                tracing::warn!(
                    "WORMHOLE_AGENT_ASK={other:?} is not deny, once, session, quit, always or hold"
                );
                Script::Answer(Button::Deny)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Asker, Script};
    use crate::agent_access::sheet::Button;

    #[test]
    fn under_a_test_the_asker_is_a_script_that_says_no() {
        assert_eq!(Asker::detect(), Asker::Scripted(Script::Answer(Button::Deny)));
    }

    #[test]
    fn the_variables_words_and_anything_else_is_no() {
        let said = |word| Asker::script_of(word);
        assert_eq!(said(Some("hold")), Script::Hold);
        assert_eq!(said(Some("Once")), Script::Answer(Button::Session));
        assert_eq!(said(Some("session")), Script::Answer(Button::Session));
        assert_eq!(said(Some("quit")), Script::Answer(Button::Quit));
        assert_eq!(said(Some("always")), Script::Answer(Button::Always));
        for no in [None, Some("deny"), Some("yes please"), Some("")] {
            assert_eq!(said(no), Script::Answer(Button::Deny), "{no:?}");
        }
    }
}
