//! The words under the text of a restored terminal, in the person's language and date order, and the seed made
//! of a terminal's saved text. The engine's seed is VT that ends with its own divider (`wormhole_term::checkpoint`);
//! what the divider says is the app's, since only the app knows the language.

use wormhole_core::platform::clock::local_date_time;
use wormhole_term::checkpoint::{divider, text_seed};

/// "restored 2026-11-02 14:03" (in the catalog's order for the language), for a screen kept at `unix_seconds`.
pub(crate) fn restored_words(unix_seconds: u64) -> String {
    match local_date_time(unix_seconds) {
        Some(at) => wormhole_text::keys::terminal_restored_at(
            &format!("{:04}", at.year),
            &format!("{:02}", at.month),
            &format!("{:02}", at.day),
            &format!("{:02}", at.hour),
            &format!("{:02}", at.minute),
        ),
        // The system can't say what zone it is in: the parts stay unknown.
        None => wormhole_text::keys::terminal_restored_at("????", "??", "??", "??", "??"),
    }
}

/// The saved text of a terminal as the seed a new shell is started above: each line dim, then the divider that says
/// the text was drawn back now.
pub(crate) fn seed_from_text(text: &str, now_unix_seconds: u64) -> String {
    text_seed(text, &divider(&restored_words(now_unix_seconds)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_carry_the_date_in_the_catalogs_order() {
        let words = restored_words(1_793_023_327);
        assert!(words.starts_with("restored 2026-10-"), "{words}");
    }

    #[test]
    fn saved_text_ends_with_the_divider_and_empty_text_is_no_seed() {
        let seed = seed_from_text("one\ntwo", 1_793_023_327);
        assert!(
            seed.starts_with("\x1b[2mone\x1b[0m\r\n\x1b[2mtwo\x1b[0m\r\n\x1b[2;3m──── restored "),
            "{seed:?}"
        );
        assert!(seed.ends_with("\x1b[0m\r\n"));
        assert_eq!(seed_from_text("  \n", 1), "");
    }
}
