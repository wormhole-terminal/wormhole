//! Forgiving matching of what is typed against a name, with the characters that matched, for highlighting.
//!
//! A name fits in one of these ways, best first, and a better way always outranks a worse one whatever the
//! details: it begins with the text; the text begins a word of it (`term` in `my-terminal`, `Term` in
//! `myTerminal`); the text is the first letters of its words (`pt` for `project-terminal`); the text is
//! somewhere inside it; the text's letters are in it in order (`prjct` for `project`); and, for a text of four
//! letters or more, one slip away from the name's start or the whole name (a letter wrong, missing, extra, or
//! two swapped: `terminla`). Case never matters.

use std::ops::Range;

/// How a name fits a text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fit {
    /// Higher is better; see the tiers above (1000 down to 100).
    pub score: i32,
    /// The matched characters, as byte ranges of the name, in order and not touching.
    pub marks: Vec<Range<usize>>,
}

/// The shortest text the one-slip match is tried for: a shorter one is within a slip of too many names.
const TYPO_MIN: usize = 4;

const PREFIX: i32 = 1000;
const WORD_START: i32 = 800;
const INITIALS: i32 = 750;
const INSIDE: i32 = 600;
const LETTERS: i32 = 300;
const SLIP: i32 = 100;

/// One character of the name: where it is (bytes), its lowercase, and whether it starts a word.
struct Glyph {
    at: usize,
    end: usize,
    folded: char,
    word_start: bool,
}

fn glyphs(name: &str) -> Vec<Glyph> {
    let mut out: Vec<Glyph> = Vec::new();
    let mut prev: Option<char> = None;
    for (at, c) in name.char_indices() {
        let word_start = c.is_alphanumeric()
            && match prev {
                None => true,
                Some(p) => !p.is_alphanumeric() || (p.is_lowercase() && c.is_uppercase()),
            };
        out.push(Glyph { at, end: at + c.len_utf8(), folded: fold(c), word_start });
        prev = Some(c);
    }
    out
}

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// How `name` fits `query`, if it does. An empty query fits everything with no marks and a score of zero.
pub fn fit(name: &str, query: &str) -> Option<Fit> {
    let wanted: Vec<char> = query.chars().map(fold).collect();
    if wanted.is_empty() {
        return Some(Fit { score: 0, marks: Vec::new() });
    }
    let g = glyphs(name);
    if wanted.len() > g.len() + 1 {
        return None;
    }
    let tidy = |positions: &[usize]| marks(&g, positions);
    let run = |from: usize| (from..from + wanted.len()).collect::<Vec<_>>();

    if g.len() >= wanted.len() && starts_at(&g, 0, &wanted) {
        let spare = (g.len() - wanted.len()).min(100) as i32;
        return Some(Fit { score: PREFIX - spare, marks: tidy(&run(0)) });
    }
    if let Some(from) = (1..g.len()).find(|&i| g[i].word_start && starts_at(&g, i, &wanted)) {
        return Some(Fit { score: WORD_START - from.min(60) as i32, marks: tidy(&run(from)) });
    }
    if let Some(positions) = initials(&g, &wanted) {
        return Some(Fit { score: INITIALS - positions[0].min(40) as i32, marks: tidy(&positions) });
    }
    if let Some(from) = (1..g.len()).find(|&i| starts_at(&g, i, &wanted)) {
        return Some(Fit { score: INSIDE - from.min(100) as i32, marks: tidy(&run(from)) });
    }
    if let Some(positions) = letters(&g, &wanted) {
        return Some(Fit { score: letters_score(&g, &positions), marks: tidy(&positions) });
    }
    if wanted.len() >= TYPO_MIN && one_slip(&g, &wanted) {
        let common = g.iter().zip(&wanted).take_while(|(a, b)| a.folded == **b).count();
        return Some(Fit { score: SLIP, marks: tidy(&(0..common).collect::<Vec<_>>()) });
    }
    None
}

/// Whether the glyphs from `from` begin with `wanted`.
fn starts_at(g: &[Glyph], from: usize, wanted: &[char]) -> bool {
    g.len() >= from + wanted.len() && g[from..].iter().zip(wanted).all(|(a, b)| a.folded == *b)
}

/// Each wanted character matching the start of a successive word, if they do (the words' first letters, not
/// necessarily every word's).
fn initials(g: &[Glyph], wanted: &[char]) -> Option<Vec<usize>> {
    if wanted.len() < 2 {
        return None;
    }
    let mut positions = Vec::with_capacity(wanted.len());
    let mut from = 0;
    for &c in wanted {
        let at = (from..g.len()).find(|&i| g[i].word_start && g[i].folded == c)?;
        positions.push(at);
        from = at + 1;
    }
    Some(positions)
}

/// The wanted characters in order: found going forward, then pulled back as tight as they can be (the latest
/// start that still reaches the same end), so `prjct` marks the letters of `Project` and not a spread over a
/// longer name.
fn letters(g: &[Glyph], wanted: &[char]) -> Option<Vec<usize>> {
    let mut at = 0;
    let mut end = 0;
    for &c in wanted {
        let found = (at..g.len()).find(|&i| g[i].folded == c)?;
        end = found;
        at = found + 1;
    }
    let mut positions = vec![0; wanted.len()];
    let mut limit = end + 1;
    for (slot, &c) in positions.iter_mut().zip(wanted).rev() {
        let found = (0..limit).rev().find(|&i| g[i].folded == c)?;
        *slot = found;
        limit = found;
    }
    Some(positions)
}

fn letters_score(g: &[Glyph], positions: &[usize]) -> i32 {
    let touching = positions.windows(2).filter(|w| w[1] == w[0] + 1).count() as i32;
    let at_words = positions.iter().filter(|&&p| g[p].word_start).count() as i32;
    let span = (positions[positions.len() - 1] - positions[0] + 1 - positions.len()) as i32;
    let late = positions[0].min(30) as i32;
    (LETTERS + 12 * touching + 8 * at_words - 3 * span.min(40) - late).clamp(LETTERS - 150, LETTERS + 240)
}

/// Whether the name, or its start as long as the text (give or take a letter), is one slip from the text.
fn one_slip(g: &[Glyph], wanted: &[char]) -> bool {
    let folded: Vec<char> = g.iter().map(|x| x.folded).collect();
    let n = wanted.len();
    let whole = within_one(&folded, wanted);
    let starts = (n.saturating_sub(1)..=n + 1)
        .filter(|&len| len <= folded.len())
        .any(|len| within_one(&folded[..len], wanted));
    whole || starts
}

/// Whether `a` and `b` are equal or one letter wrong, missing, extra or two next to each other swapped.
fn within_one(a: &[char], b: &[char]) -> bool {
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let common = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    if common == a.len() && common == b.len() {
        return true;
    }
    let (ra, rb) = (&a[common..], &b[common..]);
    if a.len() == b.len() {
        let swapped = ra.len() >= 2 && ra[0] == rb[1] && ra[1] == rb[0] && ra[2..] == rb[2..];
        ra[1..] == rb[1..] || swapped
    } else if a.len() > b.len() {
        ra[1..] == *rb
    } else {
        *ra == rb[1..]
    }
}

/// The byte ranges of the glyphs at `positions` (ascending), touching ones joined.
fn marks(g: &[Glyph], positions: &[usize]) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    for &p in positions {
        let Glyph { at, end, .. } = g[p];
        match out.last_mut() {
            Some(last) if last.end == at => last.end = end,
            _ => out.push(at..end),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::fit;

    fn score(name: &str, query: &str) -> i32 {
        fit(name, query).unwrap_or_else(|| panic!("{query:?} should fit {name:?}")).score
    }

    fn marked<'a>(name: &'a str, query: &str) -> Vec<&'a str> {
        fit(name, query).expect("fits").marks.into_iter().map(|r| &name[r]).collect()
    }

    #[test]
    fn the_ways_to_fit_rank_prefix_then_word_start_then_initials_then_inside_then_letters_then_a_slip() {
        let ranked = [
            score("terminal", "term"),
            score("my-terminal", "term"),
            score("project-terminal", "pt"),
            score("determine", "term"),
            score("xtxmxnxl", "tmnl"),
            score("terminal", "terminla"),
        ];
        assert!(ranked.windows(2).all(|w| w[0] > w[1]), "{ranked:?}");
    }

    #[test]
    fn a_shorter_name_beats_a_longer_one_with_the_same_prefix() {
        assert!(score("src", "sr") > score("src-old", "sr"));
    }

    #[test]
    fn case_never_matters_and_marks_are_in_the_names_own_case() {
        assert_eq!(marked("Projects", "PRO"), ["Pro"]);
        assert_eq!(
            marked("myTerminal", "term"),
            ["Term"],
            "a capital after a lower case letter starts a word"
        );
        assert_eq!(marked("my-terminal", "term"), ["term"]);
    }

    #[test]
    fn initials_mark_each_letter() {
        assert_eq!(marked("project-terminal", "pt"), ["p", "t"]);
    }

    #[test]
    fn letters_in_order_are_marked_tight() {
        assert_eq!(marked("Projects", "prjcts"), ["Pr", "j", "cts"]);
        assert_eq!(marked("xtxmxnxl", "tmnl"), ["t", "m", "n", "l"]);
    }

    #[test]
    fn a_text_whose_letters_are_not_all_there_does_not_fit() {
        assert!(fit("src", "srcxy").is_none());
        assert!(fit("src", "zz").is_none());
        assert!(fit("abc", "cba").is_none());
    }

    #[test]
    fn one_slip_is_forgiven_from_four_letters_up() {
        for typed in ["terminla", "termnal", "termminal", "terminel", "erminal"] {
            assert!(fit("terminal", typed).is_some(), "{typed}");
        }
        assert!(fit("terminal", "tarmanal").is_none(), "two slips");
        assert!(fit("cat", "cta").is_none(), "too short for a slip");
    }

    #[test]
    fn an_empty_text_fits_everything_without_marks() {
        let f = fit("anything", "").expect("fits");
        assert_eq!((f.score, f.marks.len()), (0, 0));
    }

    #[test]
    fn marks_are_byte_ranges_past_wide_characters() {
        let m = fit("éa-docs", "doc").expect("fits");
        assert_eq!(&"éa-docs"[m.marks[0].clone()], "doc");
    }

    #[test]
    fn a_text_longer_than_the_name_by_more_than_a_slip_is_dropped_early() {
        assert!(fit("a", "abcdefgh").is_none());
    }
}
