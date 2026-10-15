//! Grapheme cluster breaks against Unicode's own test file, and the rules it can't say in one line.

use wormhole_vt::unicode::{GraphemeState, breaks, width};

#[path = "common/rng.rs"]
mod rng;
use rng::Rng;

/// `GraphemeBreakTest.txt`, copied unchanged by `scripts/gen-unicode.py`.
const TEST_DATA: &str = include_str!("data/GraphemeBreakTest.txt");

struct Case {
    chars: Vec<char>,
    /// `breaks[i]`: whether a boundary falls before `chars[i]`.
    breaks: Vec<bool>,
    line: usize,
}

/// `÷ 000D × 000A ÷ # comment`: a sign, a code point, a sign, ... where ÷ is a boundary.
fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for (index, line) in TEST_DATA.lines().enumerate() {
        let data = line.split('#').next().unwrap_or("").trim();
        if data.is_empty() {
            continue;
        }
        let mut chars = Vec::new();
        let mut breaks = Vec::new();
        let mut boundary = None;
        for token in data.split_whitespace() {
            match token {
                "÷" => boundary = Some(true),
                "×" => boundary = Some(false),
                hex => {
                    let cp = u32::from_str_radix(hex, 16).expect("a code point in hex");
                    chars.push(char::from_u32(cp).expect("a scalar value"));
                    breaks.push(boundary.take().expect("a sign before each code point"));
                }
            }
        }
        cases.push(Case { chars, breaks, line: index + 1 });
    }
    cases
}

#[test]
fn unicodes_grapheme_break_test() {
    let cases = cases();
    // The file has 853 cases in 18.0.0 (766 in 17.0.0); a truncated or empty copy must not pass.
    assert!(cases.len() > 700, "only {} cases", cases.len());
    for case in &cases {
        let mut state = GraphemeState::new();
        let got: Vec<bool> = case.chars.iter().map(|&c| breaks(&mut state, c)).collect();
        assert_eq!(got, case.breaks, "GraphemeBreakTest.txt line {}: {:04X?}", case.line, case.chars);
    }
}

#[test]
fn step_answers_both_with_the_same_state() {
    for case in cases() {
        let mut state = GraphemeState::new();
        for (i, &c) in case.chars.iter().enumerate() {
            let step = state.step(c);
            assert_eq!(step.breaks, case.breaks[i], "line {}", case.line);
            assert_eq!(step.width, width(c), "line {}: U+{:04X}", case.line, c as u32);
        }
    }
}

#[test]
fn the_first_character_always_starts_a_cluster() {
    for c in (0..=0x10FFFFu32).filter_map(char::from_u32) {
        assert!(breaks(&mut GraphemeState::new(), c), "U+{:04X}", c as u32);
    }
}

fn clusters(text: &str) -> Vec<String> {
    let mut state = GraphemeState::new();
    let mut clusters: Vec<String> = Vec::new();
    for c in text.chars() {
        // Every character goes through the state, the first one included.
        let starts = breaks(&mut state, c);
        match clusters.last_mut() {
            Some(last) if !starts => last.push(c),
            _ => clusters.push(c.to_string()),
        }
    }
    clusters
}

#[test]
fn clusters_of_real_text() {
    // Latin with a combining accent, a CRLF, and ASCII.
    assert_eq!(clusters("e\u{301}x\r\ny"), ["e\u{301}", "x", "\r\n", "y"]);
    // Emoji: a ZWJ family, a skin tone, a variation selector, a keycap, a flag, two flags.
    assert_eq!(
        clusters("👨\u{200D}👩\u{200D}👧👍🏽❤\u{FE0F}1\u{FE0F}\u{20E3}🇩🇪🇫🇷"),
        ["👨\u{200D}👩\u{200D}👧", "👍🏽", "❤\u{FE0F}", "1\u{FE0F}\u{20E3}", "🇩🇪", "🇫🇷"]
    );
    // Three regional indicators: a pair and one alone.
    assert_eq!(clusters("🇩🇪🇫"), ["🇩🇪", "🇫"]);
    // Hangul: jamo L V T are one syllable, a precomposed syllable and a jamo T join, the next L does not.
    assert_eq!(clusters("\u{1112}\u{1161}\u{11AB}\u{1100}"), ["\u{1112}\u{1161}\u{11AB}", "\u{1100}"]);
    assert_eq!(clusters("한\u{11A8}글"), ["한\u{11A8}", "글"]);
    // Devanagari: KA + virama + SSA is one conjunct, KA + SSA are two letters.
    assert_eq!(clusters("\u{0915}\u{094D}\u{0937}"), ["\u{0915}\u{094D}\u{0937}"]);
    assert_eq!(clusters("\u{0915}\u{0937}"), ["\u{0915}", "\u{0937}"]);
    // A ZWNJ between virama and the next consonant ends the conjunct (it is not InCB=Extend).
    assert_eq!(clusters("\u{0915}\u{094D}\u{200C}\u{0937}"), ["\u{0915}\u{094D}\u{200C}", "\u{0937}"]);
    // A Prepend character (ARABIC NUMBER SIGN) joins the digit after it.
    assert_eq!(clusters("\u{0600}\u{0661}"), ["\u{0600}\u{0661}"]);
    // CJK and ASCII never join.
    assert_eq!(clusters("日本語abc"), ["日", "本", "語", "a", "b", "c"]);
}

#[test]
fn zwj_joins_a_pictograph_only_after_a_pictograph() {
    // GB11 needs the pictograph before the ZWJ (and only Extend characters between).
    assert_eq!(clusters("a\u{200D}👩"), ["a\u{200D}", "👩"]);
    assert_eq!(clusters("👩\u{1F3FD}\u{200D}👩"), ["👩\u{1F3FD}\u{200D}👩"]);
    // Two ZWJ in a row: the second one is no Extend, so the pictograph after is a new cluster.
    assert_eq!(clusters("👩\u{200D}\u{200D}👩"), ["👩\u{200D}\u{200D}", "👩"]);
}

/// A deterministic mix of characters that exercise every rule, in sequences no test file lists.
fn mixed_text(seed: u64, len: usize) -> Vec<char> {
    const POOL: &[char] = &[
        'a',
        'z',
        ' ',
        '\r',
        '\n',
        '\u{0}',
        'é',
        '\u{301}',
        '\u{200D}',
        '\u{200C}',
        '\u{FE0F}',
        '\u{FE0E}',
        '\u{0600}',
        '\u{0915}',
        '\u{0937}',
        '\u{094D}',
        '\u{093C}',
        '\u{093E}',
        '\u{1100}',
        '\u{1161}',
        '\u{11A8}',
        '한',
        '글',
        '中',
        '👩',
        '👨',
        '🇩',
        '🇪',
        '🇫',
        '\u{1F3FD}',
        '❤',
        '⌚',
        '1',
        '\u{20E3}',
        '\u{E0020}',
        '\u{E007F}',
    ];
    let mut rng = Rng::new(seed);
    (0..len).map(|_| rng.pick(POOL)).collect()
}

#[test]
fn a_cluster_alone_is_one_cluster() {
    // Streaming must not depend on what came before the cluster: feeding each cluster found in a
    // long text to a fresh state gives one cluster, a break only at its start.
    for seed in 0..200 {
        let text: String = mixed_text(seed, 40).into_iter().collect();
        for cluster in clusters(&text) {
            let mut state = GraphemeState::new();
            let got: Vec<bool> = cluster.chars().map(|c| breaks(&mut state, c)).collect();
            let mut expected = vec![false; got.len()];
            expected[0] = true;
            assert_eq!(got, expected, "{cluster:?} in {text:?}");
        }
    }
}

#[test]
fn a_prefix_is_segmented_as_the_whole() {
    // Boundaries before a character don't depend on the characters after it.
    for seed in 0..100 {
        let text = mixed_text(seed, 30);
        let whole: Vec<bool> = {
            let mut state = GraphemeState::new();
            text.iter().map(|&c| breaks(&mut state, c)).collect()
        };
        for cut in 1..text.len() {
            let mut state = GraphemeState::new();
            let part: Vec<bool> = text[..cut].iter().map(|&c| breaks(&mut state, c)).collect();
            assert_eq!(part[..], whole[..cut]);
        }
    }
}

#[test]
fn the_state_is_copy_and_forkable() {
    // A copy taken mid-cluster continues as the original would: the engine can save and restore it.
    let mut state = GraphemeState::new();
    breaks(&mut state, '🇩');
    let saved = state;
    assert!(!breaks(&mut state, '🇪'));
    let mut again = saved;
    assert!(!breaks(&mut again, '🇪'));
    assert_eq!(state, again);
    // The pair is complete: a third indicator starts a new flag, in the original and the copy.
    assert!(breaks(&mut state, '🇫'));
    assert!(breaks(&mut again, '🇫'));
}
