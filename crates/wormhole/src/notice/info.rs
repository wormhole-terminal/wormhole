//! The explanation behind a warning's ⓘ: a native popover under the button, in plain words, with what
//! went wrong in the error's own words (to select, or copy, and report) and a way to the logs. It is a
//! `platform::popover` value, so what it says is tested as text. The words are the catalog's.

use std::path::PathBuf;
use std::time::Duration;

use wormhole_core::theme::tokens::Appearance;
use wormhole_core::view::notice::{Notice, Warning};
use wormhole_text::{keys, text};

use crate::platform::popover::{Behavior, Button, Glyph, Popover, Row};

/// How long "Copy reason" says "Copied".
const COPIED_FOR: Duration = Duration::from_millis(1200);

/// The explanation of `notice`, which has one (`Notice::has_info`), with `reason` (what the error said,
/// already one short line: core's `reason`) and the folder of the log files.
pub(super) fn popover(
    notice: Notice,
    reason: Option<&str>,
    logs: PathBuf,
    appearance: Appearance,
) -> Option<Popover> {
    match notice {
        Notice::Warned(Warning::HostFallback) => Some(host_fallback(reason, logs, appearance)),
        _ => None,
    }
}

fn host_fallback(reason: Option<&str>, logs: PathBuf, appearance: Appearance) -> Popover {
    let mut popover = Popover::new(Behavior::Transient, appearance)
        .row(Row::Heading(text(keys::NOTICE_INFO_HOST_HEADING).into()))
        .row(Row::Text(text(keys::NOTICE_INFO_HOST_WHAT).into()))
        .row(Row::Text(text(keys::NOTICE_INFO_HOST_OTHERS).into()))
        .row(Row::Text(text(keys::NOTICE_INFO_HOST_TRY).into()));
    let reason = reason.filter(|reason| !reason.is_empty());
    if let Some(reason) = reason {
        popover = popover
            .row(Row::Text(text(keys::NOTICE_INFO_HOST_REASON).into()))
            .row(Row::Code(reason.to_string().into()));
    }
    let mut buttons = vec![Button::new(text(keys::NOTICE_INFO_LOGS), Some(Glyph::Folder), move |_, cx| {
        cx.open_with_system(&logs);
    })];
    if let Some(reason) = reason.map(str::to_string) {
        buttons.push(
            Button::new(text(keys::NOTICE_INFO_COPY), Some(Glyph::Copy), move |_, cx| {
                crate::platform::clipboard::write(reason.clone(), cx);
            })
            .then_says(text(keys::NOTICE_INFO_COPIED), Some(Glyph::Check), COPIED_FOR),
        );
    }
    popover.row(Row::Buttons(buttons))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use wormhole_core::theme::tokens::Appearance;
    use wormhole_core::view::notice::{Copied, Notice, Warning};
    use wormhole_text::{Language, with_language};

    use super::popover;

    const HOST: Notice = Notice::Warned(Warning::HostFallback);

    fn lines(reason: Option<&str>) -> Vec<String> {
        with_language(Language::ENGLISH, || {
            popover(HOST, reason, PathBuf::from("/logs"), Appearance::Dark).expect("an explanation").outline()
        })
    }

    #[test]
    fn the_explanation_says_what_happened_what_to_do_and_what_the_error_said() {
        let outline = lines(Some("no such file: /x/wormhole-host"));
        assert_eq!(outline[0], "popover transient");
        assert_eq!(outline[1], "## This terminal can't outlive Wormhole");
        assert!(
            outline[2].starts_with("text: Wormhole normally keeps each terminal running in a small helper")
        );
        assert!(outline[2].contains("closes when Wormhole quits"));
        assert_eq!(outline[3], "text: Other terminals that already run in a helper are not affected.");
        assert!(outline[4].contains("Finish what is running here before you quit"));
        assert!(outline[4].contains("tries to start its helper again"));
        assert_eq!(outline[5], "text: What went wrong, if you want to report it:");
        assert_eq!(outline[6], "code: no such file: /x/wormhole-host", "the error's own words, as they are");
        assert_eq!(outline[7], "buttons: [Open Logs Folder] [Copy reason -> Copied]");
        assert_eq!(outline.len(), 8);
    }

    #[test]
    fn the_main_text_has_no_jargon() {
        let all = lines(Some("x")).join(" ");
        for word in ["session host", "PTY", "daemon", "socket"] {
            assert!(!all.contains(word), "{word}");
        }
    }

    #[test]
    fn without_a_reason_there_is_no_reason_line_and_no_copy_button() {
        for reason in [None, Some("")] {
            let outline = lines(reason);
            assert!(outline.iter().all(|line| !line.starts_with("code:")), "{outline:?}");
            assert_eq!(outline.last().map(String::as_str), Some("buttons: [Open Logs Folder]"));
        }
    }

    #[test]
    fn a_notice_with_nothing_more_to_say_has_no_explanation() {
        let none = |notice| popover(notice, None, PathBuf::new(), Appearance::Light).is_none();
        assert!(none(Notice::from(Copied::Path)));
        assert!(none(Notice::Warned(Warning::Test(1))));
    }
}
