//! Cell widths: characters, and clusters where a variation selector or a mark joins a base.

use wormhole_vt::unicode::{UNICODE_VERSION, cluster_width, extend_width, has_emoji_variation, width};

fn widths(chars: &[(char, u8)]) {
    for &(c, expected) in chars {
        assert_eq!(width(c), expected, "U+{:04X} {c}", c as u32);
    }
}

#[test]
fn the_data_is_unicode_18() {
    assert_eq!(UNICODE_VERSION, (18, 0, 0));
}

#[test]
fn ascii_is_narrow() {
    for c in ' '..='~' {
        assert_eq!(width(c), 1, "{c:?}");
    }
}

#[test]
fn controls_take_no_cell() {
    for c in ('\0'..' ').chain(['\u{7F}', '\u{85}', '\u{9F}']) {
        assert_eq!(width(c), 0, "U+{:04X}", c as u32);
    }
}

#[test]
fn latin_and_ambiguous_characters_are_narrow() {
    widths(&[
        ('é', 1),
        ('ß', 1),
        ('\u{A0}', 1),
        // East Asian Ambiguous: narrow outside East Asian terminals.
        ('¡', 1),
        ('°', 1),
        ('Ω', 1),
        ('─', 1),
        ('█', 1),
        ('→', 1),
        ('\u{E000}', 1),
    ]);
    // Format and punctuation that terminals draw.
    widths(&[('…', 1), ('—', 1), ('•', 1), ('€', 1), ('\u{A0}', 1)]);
}

#[test]
fn the_soft_hyphen_takes_a_cell() {
    // A format character (Cf), but terminals show it as a hyphen when it falls at a line end.
    assert_eq!(width('\u{AD}'), 1);
}

#[test]
fn prepended_concatenation_marks_and_spacing_viramas_follow_their_glyphs() {
    // Format characters, but drawn: the sign that comes before a number (PropList.txt).
    widths(&[('\u{600}', 1), ('\u{6DD}', 1), ('\u{70F}', 1), ('\u{8E2}', 1), ('\u{110BD}', 1)]);
    // Viramas (Indic_Syllabic_Category) join letters and take no cell, spacing marks included:
    // Javanese pangkon, Balinese adeg adeg, Grantha virama.
    widths(&[('\u{A9C0}', 0), ('\u{1B44}', 0), ('\u{1134D}', 0), ('\u{94D}', 0)]);
    // Other spacing marks are cells of their own (the legacy width, one per code point).
    widths(&[('\u{93E}', 1), ('\u{9BF}', 1), ('\u{E33}', 1)]);
}

#[test]
fn combining_marks_and_format_characters_take_none() {
    widths(&[
        ('\u{301}', 0),   // COMBINING ACUTE ACCENT (Mn)
        ('\u{34F}', 0),   // COMBINING GRAPHEME JOINER
        ('\u{20DD}', 0),  // COMBINING ENCLOSING CIRCLE (Me)
        ('\u{20E3}', 0),  // COMBINING ENCLOSING KEYCAP (Me)
        ('\u{94D}', 0),   // DEVANAGARI SIGN VIRAMA
        ('\u{3099}', 0),  // COMBINING KATAKANA-HIRAGANA VOICED SOUND MARK (Mn, but East Asian Wide)
        ('\u{302A}', 0),  // IDEOGRAPHIC LEVEL TONE MARK
        ('\u{200B}', 0),  // ZERO WIDTH SPACE
        ('\u{200C}', 0),  // ZWNJ
        ('\u{200D}', 0),  // ZWJ
        ('\u{200E}', 0),  // LEFT-TO-RIGHT MARK
        ('\u{2060}', 0),  // WORD JOINER
        ('\u{FEFF}', 0),  // ZERO WIDTH NO-BREAK SPACE
        ('\u{E0001}', 0), // LANGUAGE TAG
        ('\u{E0041}', 0), // TAG LATIN CAPITAL LETTER A
        ('\u{E007F}', 0), // CANCEL TAG
    ]);
}

#[test]
fn variation_selectors_take_none() {
    widths(&[
        ('\u{FE00}', 0),
        ('\u{FE0E}', 0),
        ('\u{FE0F}', 0),
        ('\u{180B}', 0),
        ('\u{E0100}', 0),
        ('\u{E01EF}', 0),
    ]);
}

#[test]
fn spacing_marks_take_a_cell() {
    // Mc: a vowel sign that is drawn after its consonant, not on it.
    widths(&[('\u{93E}', 1), ('\u{9BE}', 1), ('\u{E33}', 1)]);
}

#[test]
fn cjk_is_wide() {
    widths(&[
        ('中', 2),
        ('日', 2),
        ('\u{3400}', 2), // extension A
        ('\u{4E00}', 2),
        ('\u{9FFF}', 2),
        ('\u{20000}', 2), // extension B
        ('\u{2A6DF}', 2),
        ('\u{30000}', 2), // extension G
        ('\u{F900}', 2),  // compatibility ideograph
        ('あ', 2),
        ('ア', 2),
        ('\u{3000}', 2), // IDEOGRAPHIC SPACE
        ('、', 2),
        ('「', 2),
        ('\u{FE30}', 2), // PRESENTATION FORM FOR VERTICAL TWO DOT LEADER
        // Unassigned code points in the CJK blocks are wide too.
        ('\u{2FFFD}', 2),
        ('\u{FA6E}', 2),
    ]);
}

#[test]
fn fullwidth_forms_are_wide_and_halfwidth_forms_narrow() {
    widths(&[
        ('Ａ', 2),
        ('！', 2),
        ('０', 2),
        ('\u{FF60}', 2),
        ('￦', 2),
        ('\u{FFE0}', 2),
        // Halfwidth: the half of a fullwidth form.
        ('ｱ', 1),
        ('ﾊ', 1),
        ('\u{FF9E}', 1), // HALFWIDTH KATAKANA VOICED SOUND MARK (Lm, not a mark)
        ('\u{FFA0}', 1),
        ('\u{FFE8}', 1),
        ('\u{FFEE}', 1),
    ]);
}

#[test]
fn hangul_syllables_are_wide_and_medial_and_final_jamo_take_none() {
    widths(&[
        ('한', 2),
        ('\u{AC00}', 2),
        ('\u{D7A3}', 2),
        // Initial consonants (choseong) are wide: they start a syllable.
        ('\u{1100}', 2),
        ('\u{115F}', 2),
        ('\u{A960}', 2),
        // Medial vowels (jungseong) and final consonants (jongseong) add to the syllable.
        ('\u{1160}', 0),
        ('\u{1161}', 0),
        ('\u{11A7}', 0),
        ('\u{11A8}', 0),
        ('\u{11FF}', 0),
        ('\u{D7B0}', 0),
        ('\u{D7C6}', 0),
        ('\u{D7CB}', 0),
        ('\u{D7FB}', 0),
        // Compatibility jamo are letters of their own.
        ('\u{3131}', 2),
        ('\u{3164}', 2),
    ]);
}

#[test]
fn emoji_with_presentation_are_wide() {
    widths(&[
        ('😀', 2),
        ('🚀', 2),
        ('👍', 2),
        ('🧑', 2),
        ('🫈', 2),        // new in Unicode 17
        ('\u{231A}', 2), // WATCH
        ('☕', 2),
        ('⚡', 2),
        ('\u{1F3FB}', 2), // EMOJI MODIFIER FITZPATRICK TYPE-1-2
        ('\u{1F1E9}', 2), // REGIONAL INDICATOR SYMBOL LETTER D
        ('\u{1F1FF}', 2),
    ]);
}

#[test]
fn text_default_emoji_are_narrow() {
    widths(&[
        ('\u{2764}', 1), // HEAVY BLACK HEART
        ('\u{2600}', 1), // BLACK SUN WITH RAYS
        ('\u{2601}', 1),
        ('\u{263A}', 1),
        ('\u{1F441}', 1), // EYE
        ('©', 1),
        ('®', 1),
        ('™', 1),
        ('#', 1),
        ('*', 1),
        ('7', 1),
    ]);
}

#[test]
fn which_characters_have_emoji_variation_sequences() {
    for c in ['\u{2764}', '\u{231A}', '\u{1F441}', '#', '*', '0', '9', '©', '\u{2600}', '\u{1F5E8}'] {
        assert!(has_emoji_variation(c), "U+{:04X}", c as u32);
    }
    for c in ['a', ' ', 'é', '中', '😀', '\u{FE0F}', '\u{200D}', '\u{1F1E9}', '\u{301}'] {
        assert!(!has_emoji_variation(c), "U+{:04X}", c as u32);
    }
}

fn cluster(text: &str) -> u8 {
    cluster_width(text.chars())
}

#[test]
fn a_cluster_is_as_wide_as_its_base_and_what_takes_a_cell_of_its_own() {
    assert_eq!(cluster(""), 0);
    assert_eq!(cluster("a"), 1);
    assert_eq!(cluster("e\u{301}"), 1);
    assert_eq!(cluster("e\u{301}\u{302}\u{323}"), 1);
    assert_eq!(cluster("中"), 2);
    assert_eq!(cluster("中\u{301}"), 2);
    // Hangul jamo L V T: the initial is wide, the others add nothing.
    assert_eq!(cluster("\u{1112}\u{1161}\u{11AB}"), 2);
    assert_eq!(cluster("한\u{11A8}"), 2);
}

#[test]
fn spacing_marks_and_conjuncts_make_a_cluster_two_cells() {
    // Spacing marks (Mc) are cells of their own, however many follow, up to the cluster's 2.
    assert_eq!(cluster("\u{0995}\u{09BF}"), 2); // Bengali KA, vowel sign I
    assert_eq!(cluster("\u{0995}\u{09BF}\u{0982}"), 2);
    assert_eq!(cluster("\u{0995}\u{09BE}\u{09BE}"), 2);
    assert_eq!(cluster("\u{1000}\u{103C}\u{1031}"), 2); // Burmese KA, medial RA, vowel sign E
    assert_eq!(cluster("\u{0E01}\u{0E33}"), 2); // Thai KO KAI, SARA AM
    assert_eq!(cluster("\u{0E81}\u{0EB3}"), 2); // Lao KO, AM
    assert_eq!(cluster("\u{0995}\u{0981}"), 1); // a Mn candrabindu adds nothing
    assert_eq!(cluster("\u{0E01}\u{0E48}"), 1); // Thai mai ek
    // A consonant joined by a virama is a cell of its own; further ones do not widen it again.
    assert_eq!(cluster("\u{0915}\u{094D}"), 1);
    assert_eq!(cluster("\u{0915}\u{094D}\u{0937}"), 2);
    assert_eq!(cluster("\u{0915}\u{094D}\u{0937}\u{094D}\u{0924}"), 2);
    assert_eq!(cluster("\u{0915}\u{094D}\u{0937}\u{093E}"), 2);
    assert_eq!(cluster("\u{0915}\u{094D}\u{200D}"), 1); // virama and joiner: no consonant yet
    assert_eq!(cluster("\u{1780}\u{17D2}\u{1780}"), 2); // Khmer coeng
    assert_eq!(cluster("\u{11107}\u{11133}\u{11120}\u{1112C}"), 2); // Chakma
    // A virama that Unicode calls a spacing mark joins and takes no cell.
    assert_eq!(cluster("\u{A98F}\u{A9C0}"), 1);
    assert_eq!(cluster("\u{A98F}\u{A9C0}\u{A98F}"), 2);
    assert_eq!(cluster("\u{11315}\u{1134D}"), 1);
    // Wide stays wide.
    assert_eq!(cluster("中\u{093E}"), 2);
}

#[test]
fn emoji_variation_selectors_set_the_width() {
    // VS16 makes a text-default emoji wide, VS15 makes a default-emoji narrow.
    assert_eq!(cluster("\u{2764}"), 1);
    assert_eq!(cluster("\u{2764}\u{FE0F}"), 2);
    assert_eq!(cluster("\u{2764}\u{FE0E}"), 1);
    assert_eq!(cluster("\u{231A}"), 2);
    assert_eq!(cluster("\u{231A}\u{FE0E}"), 1);
    assert_eq!(cluster("\u{231A}\u{FE0F}"), 2);
    assert_eq!(cluster("\u{1F441}\u{FE0F}"), 2);
    // A selector after a character that has no such sequence changes nothing.
    assert_eq!(cluster("a\u{FE0F}"), 1);
    assert_eq!(cluster("中\u{FE0E}"), 2);
    assert_eq!(cluster("😀\u{FE0E}"), 2);
    // Keycaps: a digit, VS16, the enclosing keycap.
    assert_eq!(cluster("1\u{FE0F}\u{20E3}"), 2);
    assert_eq!(cluster("#\u{FE0F}\u{20E3}"), 2);
    assert_eq!(cluster("1\u{20E3}"), 1);
    // A VS16 in the middle of a ZWJ sequence widens a narrow start: the eye in a speech bubble.
    assert_eq!(cluster("\u{1F441}\u{FE0F}\u{200D}\u{1F5E8}\u{FE0F}"), 2);
}

#[test]
fn emoji_sequences_are_two_cells() {
    // Regional indicators: a pair is a flag, each is wide already.
    assert_eq!(cluster("\u{1F1E9}"), 2);
    assert_eq!(cluster("🇩🇪"), 2);
    // ZWJ family, skin tone, flag of a subdivision (tag sequence), pride flag.
    assert_eq!(cluster("👨\u{200D}👩\u{200D}👧\u{200D}👦"), 2);
    assert_eq!(cluster("👍🏽"), 2);
    assert_eq!(cluster("🏴\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}"), 2);
    assert_eq!(cluster("🏳\u{FE0F}\u{200D}🌈"), 2);
}

#[test]
fn a_cluster_that_starts_without_width_takes_the_width_of_what_joins_it() {
    // A Prepend format character without a glyph is drawn with what follows.
    assert_eq!(cluster("\u{110BD}\u{0661}"), 2);
    assert_eq!(cluster("\u{0600}\u{0661}"), 2); // the number sign has a cell, then the digit
    assert_eq!(cluster("\u{0600}"), 1);
    // A mark that starts a cluster has nothing to sit on.
    assert_eq!(cluster("\u{301}"), 0);
    assert_eq!(cluster("\u{200D}"), 0);
}

#[test]
fn extend_width_folds_one_character_at_a_time() {
    // The incremental form the grid uses as characters arrive.
    assert_eq!(extend_width(1, '\u{2764}', '\u{FE0F}'), 2);
    assert_eq!(extend_width(2, '\u{231A}', '\u{FE0E}'), 1);
    assert_eq!(extend_width(1, 'e', '\u{301}'), 1);
    assert_eq!(extend_width(2, '🇩', '🇪'), 2);
    assert_eq!(extend_width(0, '\u{200D}', '1'), 1);
    assert_eq!(extend_width(1, 'k', '\u{93E}'), 2);
    assert_eq!(extend_width(2, 'k', '\u{93E}'), 2);
    assert_eq!(extend_width(1, '\u{94D}', '\u{937}'), 2);
}
