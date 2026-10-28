//! The order `ls` lists names in on a Mac with a UTF-8 locale (`en_US.UTF-8`): not by byte, so `Bravo`
//! comes after `alpha`, `é` beside `e`, and punctuation and digits ahead of letters. The system's own
//! collation isn't reachable from here without a library, so this is its shape for the names a folder has:
//! compare the letters alone first (case and accents ignored), then the accents, then the case (lower
//! before upper), then the bytes so the order is total.

use std::cmp::Ordering;

/// ASCII punctuation in the order of the Unicode root collation, which macOS follows: a space, then these.
const PUNCTUATION: &str = "_-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";

/// How `a` and `b` are ordered, as `ls` would list them.
pub fn collate(a: &str, b: &str) -> Ordering {
    primary(a)
        .cmp(primary(b))
        .then_with(|| accents(a).cmp(accents(b)))
        .then_with(|| cases(a).cmp(cases(b)))
        .then_with(|| a.cmp(b))
}

fn primary(text: &str) -> impl Iterator<Item = u32> + '_ {
    text.chars().map(|c| rank(base_letter(c)))
}

/// Where a character sorts at the first level: space and punctuation, then other symbols, then digits, then
/// letters (a letter by its lowercase form).
fn rank(c: char) -> u32 {
    if c.is_whitespace() {
        0
    } else if let Some(at) = PUNCTUATION.find(c) {
        1 + at as u32
    } else if c.is_ascii_digit() {
        0x1000 + (c as u32 - '0' as u32)
    } else if c.is_alphabetic() {
        0x2000 + c.to_lowercase().next().map_or(c as u32, |l| l as u32)
    } else {
        // Other symbols and other numerals, after the ASCII punctuation and before the digits.
        0x100 + (c as u32).min(0xeff)
    }
}

/// Whether each character carries an accent: a name without sorts before the same name with.
fn accents(text: &str) -> impl Iterator<Item = bool> + '_ {
    text.chars().map(|c| base_letter(c) != c)
}

/// Lowercase before uppercase, as `en_US` has it.
fn cases(text: &str) -> impl Iterator<Item = bool> + '_ {
    text.chars().map(char::is_uppercase)
}

/// The letter without its accent, for the Latin letters with one; any other character as it is.
fn base_letter(c: char) -> char {
    const FOLDED: &[(&str, char)] = &[
        ("àáâãäåāăą", 'a'),
        ("ÀÁÂÃÄÅĀĂĄ", 'A'),
        ("çćĉċč", 'c'),
        ("ÇĆĈĊČ", 'C'),
        ("ďđ", 'd'),
        ("ĎĐ", 'D'),
        ("èéêëēĕėęě", 'e'),
        ("ÈÉÊËĒĔĖĘĚ", 'E'),
        ("ĝğġģ", 'g'),
        ("ĜĞĠĢ", 'G'),
        ("ìíîïĩīĭįı", 'i'),
        ("ÌÍÎÏĨĪĬĮİ", 'I'),
        ("ñńņňŉ", 'n'),
        ("ÑŃŅŇ", 'N'),
        ("òóôõöøōŏő", 'o'),
        ("ÒÓÔÕÖØŌŎŐ", 'O'),
        ("řŕŗ", 'r'),
        ("ŘŔŖ", 'R'),
        ("śŝşš", 's'),
        ("ŚŜŞŠ", 'S'),
        ("ţťŧ", 't'),
        ("ŢŤŦ", 'T'),
        ("ùúûüũūŭůűų", 'u'),
        ("ÙÚÛÜŨŪŬŮŰŲ", 'U'),
        ("ýÿŷ", 'y'),
        ("ÝŸŶ", 'Y'),
        ("źżž", 'z'),
        ("ŹŻŽ", 'Z'),
    ];
    if c.is_ascii() {
        return c;
    }
    FOLDED.iter().find(|(from, _)| from.contains(c)).map_or(c, |&(_, base)| base)
}

#[cfg(test)]
mod tests {
    use super::collate;

    fn sorted(names: &[&str]) -> Vec<String> {
        let mut names: Vec<String> = names.iter().map(ToString::to_string).collect();
        names.sort_by(|a, b| collate(a, b));
        names
    }

    /// What `ls` printed for these names on macOS in `en_US.UTF-8`.
    #[test]
    fn it_orders_as_ls_does_on_a_mac() {
        assert_eq!(
            sorted(&[
                "zebra", "Zed", "é", "e", "f", "a1", "ab", "a.b", "a-b", "a_b", "a b", "a10", "a2", "10",
                "2", "1", "~t", "#h", "@at", "-x", "a", "b"
            ]),
            [
                "-x", "@at", "#h", "~t", "1", "10", "2", "a", "a b", "a_b", "a-b", "a.b", "a1", "a10", "a2",
                "ab", "b", "e", "é", "f", "zebra", "Zed"
            ]
        );
    }

    #[test]
    fn letters_ignore_case_and_lower_comes_before_upper_on_a_tie() {
        assert_eq!(sorted(&["Bravo", "alpha", "Charlie", "delta"]), ["alpha", "Bravo", "Charlie", "delta"]);
        assert_eq!(sorted(&["Ab", "ab", "aB", "AB"]), ["ab", "aB", "Ab", "AB"]);
    }

    #[test]
    fn an_accented_letter_sits_with_its_letter_and_after_it() {
        assert_eq!(
            sorted(&["Éclair", "delta", "ünï", "umlaut", "unit"]),
            ["delta", "Éclair", "umlaut", "ünï", "unit"]
        );
    }

    #[test]
    fn the_order_is_total() {
        assert_eq!(collate("a", "a"), std::cmp::Ordering::Equal);
        assert_ne!(collate("a\u{301}", "á"), std::cmp::Ordering::Equal);
    }
}
