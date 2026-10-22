//! `en-XA`: English made readable but unmistakable, to show what the interface still hard-codes and
//! where a longer language would not fit.
//!
//! Fluent's transform runs on the message's own text only, so `{ $count }` and the values handed in
//! stay as they are; letters get accents. What follows the whole formatted message is the rest:
//! padding to a third longer (German is often that, or more) and the `⟦ ⟧` brackets, whose closing
//! half is the first thing a truncating label loses.

use std::borrow::Cow;

/// Fluent's per-text-element transform: `Open` becomes `Öpéñ`.
pub(crate) fn accent(text: &str) -> Cow<'_, str> {
    if !text.chars().any(|c| accented(c) != c) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(text.chars().map(accented).collect())
}

fn accented(c: char) -> char {
    match c {
        'a' => 'á',
        'c' => 'ç',
        'e' => 'é',
        'g' => 'ĝ',
        'i' => 'í',
        'n' => 'ñ',
        'o' => 'ö',
        's' => 'š',
        'u' => 'ü',
        'y' => 'ý',
        'A' => 'Å',
        'C' => 'Ç',
        'E' => 'É',
        'I' => 'Î',
        'N' => 'Ñ',
        'O' => 'Ö',
        'S' => 'Š',
        'U' => 'Ü',
        'Y' => 'Ý',
        other => other,
    }
}

/// A formatted message made longer by 40% (rounded up) with `·`, inside `⟦ ⟧`.
pub(crate) fn decorate(text: &str) -> String {
    let chars = text.chars().count();
    let padding = (2 * chars).div_ceil(5);
    let mut out = String::with_capacity(text.len() + padding * '·'.len_utf8() + 2 * '⟦'.len_utf8());
    out.push('⟦');
    out.push_str(text);
    out.extend(std::iter::repeat_n('·', padding));
    out.push('⟧');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_get_accents_and_the_rest_stays() {
        assert_eq!(accent("Open 3 files"), "Öpéñ 3 fíléš");
        assert!(matches!(accent("123 - ·"), Cow::Borrowed(_)));
    }

    #[test]
    fn a_message_grows_by_two_fifths_inside_brackets() {
        assert_eq!(decorate(""), "⟦⟧");
        assert_eq!(decorate("abcde"), "⟦abcde··⟧");
        assert_eq!(decorate("abcdef"), "⟦abcdef···⟧");
        // Counted in characters, not bytes: an accent does not make the padding longer.
        assert_eq!(decorate("ééééé"), "⟦ééééé··⟧");
    }
}
