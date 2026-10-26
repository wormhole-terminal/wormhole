//! A notice as a sentence, from the catalog (`locales/en/notices.ftl`), and so what VoiceOver hears.

use wormhole_core::view::notice::{Copied, Notice};
use wormhole_text::{keys, text, whole};

/// The sentence for `notice`; `None` for a test notice, whose words come with the post.
pub(super) fn of(notice: Notice) -> Option<String> {
    let Notice::Copied(copied) = notice else { return None };
    Some(match copied {
        Copied::Lines(lines) => keys::notice_copied_lines(whole(lines)),
        Copied::Characters(chars) => keys::notice_copied_characters(whole(chars)),
        Copied::Output { lines } => keys::notice_copied_output(whole(lines)),
        Copied::Command => text(keys::NOTICE_COPIED_COMMAND).to_string(),
        Copied::Path => text(keys::NOTICE_COPIED_PATH).to_string(),
        Copied::SettingKey => text(keys::NOTICE_COPIED_SETTING_KEY).to_string(),
        Copied::Error => text(keys::NOTICE_COPIED_ERROR).to_string(),
        Copied::ObjectId => text(keys::NOTICE_COPIED_OBJECT_ID).to_string(),
    })
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::notice::{Copied, Notice};
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
            assert_eq!(said(Copied::ObjectId), "Copied object id");
        });
    }

    #[test]
    fn a_test_notice_has_no_words_of_its_own() {
        assert_eq!(of(Notice::Test(7)), None);
    }
}
