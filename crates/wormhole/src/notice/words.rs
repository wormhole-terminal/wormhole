//! A notice as a sentence, from the catalog (`locales/en/notices.ftl`), and so what VoiceOver hears.

use wormhole_core::view::notice::{Copied, Driven, Driver, Notice};
use wormhole_text::{keys, text, whole};

/// The sentence for `notice`; `None` for a test notice, whose words come with the post.
pub(super) fn of(notice: Notice) -> Option<String> {
    let copied = match notice {
        Notice::Copied(copied) => copied,
        Notice::Driven(driven) => return Some(driven_words(driven)),
        Notice::Test(_) => return None,
    };
    Some(match copied {
        Copied::Lines(lines) => keys::notice_copied_lines(whole(lines)),
        Copied::Characters(chars) => keys::notice_copied_characters(whole(chars)),
        Copied::Output { lines } => keys::notice_copied_output(whole(lines)),
        Copied::Command => text(keys::NOTICE_COPIED_COMMAND).to_string(),
        Copied::Path => text(keys::NOTICE_COPIED_PATH).to_string(),
        Copied::SettingKey => text(keys::NOTICE_COPIED_SETTING_KEY).to_string(),
        Copied::Error => text(keys::NOTICE_COPIED_ERROR).to_string(),
        Copied::ObjectId => text(keys::NOTICE_COPIED_OBJECT_ID).to_string(),
        Copied::CommitHash => text(keys::NOTICE_COPIED_COMMIT_HASH).to_string(),
        Copied::CommitShortHash => text(keys::NOTICE_COPIED_COMMIT_SHORT_HASH).to_string(),
        Copied::CommitMessage => text(keys::NOTICE_COPIED_COMMIT_MESSAGE).to_string(),
        Copied::CommitSubject => text(keys::NOTICE_COPIED_COMMIT_SUBJECT).to_string(),
        Copied::CommitAuthor => text(keys::NOTICE_COPIED_COMMIT_AUTHOR).to_string(),
    })
}

/// What a program that controls Wormhole did, as the chip says it.
fn driven_words(driven: Driven) -> String {
    let (typing, driver) = match driven {
        Driven::Reading(driver) => (false, driver),
        Driven::Typing(driver) => (true, driver),
    };
    match (driver, typing) {
        (Driver::Agent(agent), true) => keys::notice_driven_typing(wormhole_core::agents::agent(agent).name),
        (Driver::Agent(agent), false) => {
            keys::notice_driven_reading(wormhole_core::agents::agent(agent).name)
        }
        (Driver::Scripts, true) => text(keys::NOTICE_DRIVEN_SCRIPTS_TYPING).to_string(),
        (Driver::Scripts, false) => text(keys::NOTICE_DRIVEN_SCRIPTS_READING).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::notice::{Copied, Driven, Driver, Notice};
    use wormhole_text::{Language, with_language};

    use super::of;

    fn said(copied: Copied) -> String {
        of(Notice::Copied(copied)).unwrap_or_default()
    }

    #[test]
    fn what_was_copied_is_told_with_how_much() {
        with_language(Language::ENGLISH, || {
            assert_eq!(said(Copied::Lines(3)), "Copied 3 lines");
            assert_eq!(said(Copied::Characters(12)), "Copied 12 characters");
            assert_eq!(said(Copied::Characters(1)), "Copied 1 character");
            assert_eq!(said(Copied::Output { lines: 1 }), "Copied output, 1 line");
            assert_eq!(said(Copied::Output { lines: 8 }), "Copied output, 8 lines");
            assert_eq!(said(Copied::Command), "Copied command");
            assert_eq!(said(Copied::Path), "Copied path");
            assert_eq!(said(Copied::SettingKey), "Copied setting key");
            assert_eq!(said(Copied::Error), "Copied error");
            assert_eq!(said(Copied::CommitHash), "Copied commit hash");
            assert_eq!(said(Copied::CommitMessage), "Copied commit message");
            assert_eq!(said(Copied::ObjectId), "Copied object id");
        });
    }

    #[test]
    fn a_program_that_drives_a_terminal_is_named_and_a_script_is_not() {
        use wormhole_core::agents::AgentId;
        with_language(Language::ENGLISH, || {
            let said = |driven| of(Notice::Driven(driven)).unwrap_or_default();
            assert_eq!(said(Driven::Typing(Driver::Agent(AgentId::Claude))), "Claude Code is typing");
            assert_eq!(said(Driven::Reading(Driver::Agent(AgentId::Codex))), "Codex CLI is reading");
            assert_eq!(said(Driven::Typing(Driver::Scripts)), "A script is typing");
            assert_eq!(said(Driven::Reading(Driver::Scripts)), "A script is reading");
        });
    }

    #[test]
    fn a_test_notice_has_no_words_of_its_own() {
        assert_eq!(of(Notice::Test(7)), None);
    }
}
