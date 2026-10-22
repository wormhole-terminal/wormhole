//! Shortening a file's name and its folder to fit a row. GPUI
//! cuts text only at its end, which loses what tells two files apart: `session_store_adapter.rs`
//! and `session_store_cache.rs` would both read `session_store_…`. So the panel shortens them
//! itself before drawing: a name loses its middle and keeps its extension and the end of its
//! stem, a folder loses its front and keeps the folders nearest the file.
//!
//! The shortening is by characters; [`fit`] finds how many to keep against a width the caller
//! measures (the app measures with GPUI's text system), so this stays testable without fonts.

use std::borrow::Cow;

/// What stands for the characters left out.
pub const ELLIPSIS: char = '…';

/// `name` in at most `keep` characters (the ellipsis included), cut in the middle: the
/// extension stays whole, and the rest of the room goes to the start and the end of the stem,
/// the start getting the odd one. A name whose extension leaves no room for both ends (or one
/// without an extension, or a dotfile) is cut in the middle as a whole.
pub fn middle(name: &str, keep: usize) -> Cow<'_, str> {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= keep {
        return Cow::Borrowed(name);
    }
    if keep <= 1 {
        return Cow::Owned(ELLIPSIS.to_string());
    }
    // The extension, from the last dot that isn't the first character (`.gitignore` has none).
    let ext = chars.iter().rposition(|&c| c == '.').filter(|&dot| dot > 0).map_or(0, |dot| chars.len() - dot);
    let (stem, ext) = chars.split_at(chars.len() - ext);
    // At least one character of the stem on each side of the ellipsis.
    let room = keep.saturating_sub(ext.len() + 1);
    let (stem, ext, room) = if room >= 2 { (stem, ext, room) } else { (&chars[..], &[][..], keep - 1) };
    let head = room.div_ceil(2);
    let tail = room - head;
    let mut out: String = stem[..head].iter().collect();
    out.push(ELLIPSIS);
    out.extend(&stem[stem.len() - tail..]);
    out.extend(ext);
    Cow::Owned(out)
}

/// `folder` (a path, `/`-separated) in at most `keep` characters (the ellipsis included), cut at
/// the front: `…/adapters/session`. Whole folder names are kept when the nearest one fits; when
/// not even that does, the end of it is kept (`…shots`).
pub fn front(folder: &str, keep: usize) -> Cow<'_, str> {
    let chars: Vec<char> = folder.chars().collect();
    if chars.len() <= keep {
        return Cow::Borrowed(folder);
    }
    if keep <= 1 {
        return Cow::Owned(ELLIPSIS.to_string());
    }
    let room = keep - 1;
    let tail = &chars[chars.len() - room..];
    // The longest end that starts at a separator, so it reads `…/name`, not `…me`.
    let cut = match tail.iter().position(|&c| c == '/') {
        Some(slash) => &tail[slash..],
        None => tail,
    };
    let mut out = String::with_capacity(keep * 4);
    out.push(ELLIPSIS);
    out.extend(cut);
    Cow::Owned(out)
}

/// `text` as it fits in `max` (whatever unit `measure` returns): itself when it does, else the
/// longest `shorten(text, keep)` that does, or the ellipsis alone when nothing else fits. Each
/// try costs one `measure`, about log₂ of the length of `text` in all.
pub fn fit<'a>(
    text: &'a str,
    max: f32,
    measure: impl Fn(&str) -> f32,
    shorten: impl Fn(&'a str, usize) -> Cow<'a, str>,
) -> Cow<'a, str> {
    if measure(text) <= max {
        return Cow::Borrowed(text);
    }
    // The longest keep whose result fits; shortening keeps the result's width growing with keep.
    let (mut lo, mut hi) = (1, text.chars().count().saturating_sub(1));
    let mut best = Cow::Owned(ELLIPSIS.to_string());
    while lo <= hi {
        let keep = lo + (hi - lo) / 2;
        let candidate = shorten(text, keep);
        if measure(&candidate) <= max {
            best = candidate;
            lo = keep + 1;
        } else {
            hi = keep - 1;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::{ELLIPSIS, fit, front, middle};

    /// One unit per character, as a monospace font would measure.
    fn chars(text: &str) -> f32 {
        text.chars().count() as f32
    }

    #[test]
    fn a_name_keeps_its_extension_and_both_ends_of_its_stem() {
        assert_eq!(middle("session_store_adapter.rs", 20), "session_…_adapter.rs");
        assert_eq!(middle("integration__nightly_suite_times_out_on_arm64.snap", 19), "integra…_arm64.snap");
        // The start gets the odd character.
        assert_eq!(middle("abcdefgh.rs", 9), "abc…gh.rs");
    }

    #[test]
    fn a_name_that_fits_is_left_alone() {
        assert!(matches!(middle("lib.rs", 6), std::borrow::Cow::Borrowed("lib.rs")));
        assert_eq!(middle("lib.rs", 40), "lib.rs");
    }

    #[test]
    fn without_room_for_the_extension_the_whole_name_is_cut() {
        // `.snapshot` and the ellipsis leave less than a character for each end of the stem.
        assert_eq!(middle("a_long_name.snapshot", 10), "a_lon…shot");
        // No extension, and a dotfile, whose dot starts its name.
        assert_eq!(middle("Makefile.generated-twice", 6), "Mak…ce");
        assert_eq!(middle(".gitignore_global_list", 7), ".gi…ist");
        assert_eq!(middle("README", 4), "RE…E");
    }

    #[test]
    fn nothing_but_the_ellipsis_when_there_is_no_room() {
        assert_eq!(middle("main.rs", 1), ELLIPSIS.to_string());
        assert_eq!(middle("main.rs", 0), ELLIPSIS.to_string());
        assert_eq!(front("crates/app", 1), ELLIPSIS.to_string());
    }

    #[test]
    fn counts_characters_not_bytes() {
        assert_eq!(middle("übersicht_änderung.md", 12), "über…rung.md");
        assert_eq!(front("données/été/fichiers", 10), "…/fichiers");
    }

    #[test]
    fn a_folder_keeps_the_folders_nearest_the_file() {
        assert_eq!(front("crates/wormhole/src/adapters/session", 19), "…/adapters/session");
        assert_eq!(front("crates/wormhole/src/workspace", 12), "…/workspace");
        // Room to spare after the cut, but the next folder doesn't fit whole.
        assert_eq!(front("crates/wormhole/src/workspace", 14), "…/workspace");
    }

    #[test]
    fn a_folder_too_long_for_its_last_name_keeps_the_end_of_it() {
        assert_eq!(front("tests/snapshots", 6), "…shots");
        assert_eq!(front("crates/wormhole/src/session", 2), "…n");
    }

    #[test]
    fn fit_keeps_what_fits_and_shortens_the_rest() {
        assert_eq!(fit("lib.rs", 10.0, chars, middle), "lib.rs");
        assert_eq!(fit("session_store_adapter.rs", 20.0, chars, middle), "session_…_adapter.rs");
        assert_eq!(fit("crates/wormhole/src/workspace", 12.0, chars, front), "…/workspace");
        assert_eq!(fit("tests/snapshots", 6.0, chars, front), "…shots");
        // Nothing fits: the ellipsis alone, never an empty string.
        assert_eq!(fit("main.rs", 0.5, chars, middle), ELLIPSIS.to_string());
    }

    #[test]
    fn fit_measures_widths_not_characters() {
        // Wide characters count double: `W` is two units, the rest one.
        let measure = |text: &str| text.chars().map(|c| if c == 'W' { 2.0 } else { 1.0 }).sum::<f32>();
        let fitted = fit("WWWWWWWWWW.rs", 9.0, measure, middle);
        assert!(measure(&fitted) <= 9.0, "{fitted} is too wide");
        assert_eq!(fitted, "W…W.rs");
    }
}
