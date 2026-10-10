//! Caps and thresholds the panels share with the logic behind them, so the rule and its
//! tests live here and the app's `style` re-exports the numbers it draws with. The diff's
//! own limits are `git::DIFF_ASK_BYTES` and `git::DIFF_HIGHLIGHT_MAX_LINES`.

/// The largest number an in-app count badge spells out; more reads "999+". The menu bar
/// item has less room and caps at 9 (`menubar::model`).
pub const COUNT_BADGE_MAX: usize = 999;

/// A Changes section with more files than this starts folded, so a huge working tree
/// doesn't bury the rest of the panel.
pub const FOLD_ABOVE: usize = 200;

/// More done steps than this fold into one "N done" row under the open ones.
pub const DONE_FOLD_ABOVE: usize = 5;

/// An open note shows this many lines, then fades out above "Show all · N lines".
pub const NOTE_OPEN_MAX_LINES: usize = 12;

/// The least width (logical px) the diff needs to show old and new side by side, each at
/// least 280; narrower, it shows the unified view and hides the switch.
pub const DIFF_SPLIT_MIN: f32 = 560.0;

/// Window widths (logical px) where the status bar, when shown, sheds detail: below the
/// first its items drop their words (needs you keeps them), below the second the
/// terminal size goes.
pub const STATUS_BAR_WORDS_MIN: f32 = 900.0;
pub const STATUS_BAR_SIZE_MIN: f32 = 720.0;

/// Whether a diff body this wide has room for old and new side by side. Narrower, the
/// diff shows unified whatever the setting says, and hides the switch.
pub fn diff_split_fits(body_width: f32) -> bool {
    body_width >= DIFF_SPLIT_MIN
}

/// `n` as a count badge shows it: the number, or "999+" past [`COUNT_BADGE_MAX`].
pub fn count_label(n: usize) -> String {
    if n > COUNT_BADGE_MAX { format!("{COUNT_BADGE_MAX}+") } else { n.to_string() }
}

/// `n` with its thousands set apart ("1,102"), for a count that has room to say it in full:
/// a section band's badge, a sentence. The tab bar's badges cap instead ([`count_label`]).
pub fn count_grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (ix, digit) in digits.chars().enumerate() {
        if ix > 0 && (digits.len() - ix).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{COUNT_BADGE_MAX, count_grouped, count_label, diff_split_fits};

    #[test]
    fn counts_cap_at_999() {
        assert_eq!(count_label(0), "0");
        assert_eq!(count_label(12), "12");
        assert_eq!(count_label(COUNT_BADGE_MAX), "999");
        assert_eq!(count_label(COUNT_BADGE_MAX + 1), "999+");
        assert_eq!(count_label(48_210), "999+");
    }

    #[test]
    fn a_split_diff_needs_two_columns_of_280() {
        assert!(diff_split_fits(560.0));
        assert!(diff_split_fits(1000.0));
        assert!(!diff_split_fits(559.0));
        // A 320 terminal column holds a 288 card.
        assert!(!diff_split_fits(286.0));
    }

    #[test]
    fn grouped_counts_set_thousands_apart() {
        assert_eq!(count_grouped(0), "0");
        assert_eq!(count_grouped(999), "999");
        assert_eq!(count_grouped(1_102), "1,102");
        assert_eq!(count_grouped(48_210), "48,210");
        assert_eq!(count_grouped(1_234_567), "1,234,567");
    }
}
