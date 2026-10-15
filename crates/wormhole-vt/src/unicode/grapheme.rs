//! Extended grapheme cluster boundaries (UAX #29), one character at a time.
//!
//! The rules are a finite automaton: what a boundary depends on is the class of the last character
//! and a little context (GB9c, GB11, GB12 and GB13 count or wait for something), which together are
//! a handful of states. `MACHINE` is that automaton, built at compile time from the rules as they
//! are written in the standard, so a step is one table lookup and no branch.

use super::tables::{self, CLASS_SHIFT, WIDTH_MASK, class};

/// What a rule waits on beyond the previous character's class. GB9c (a linker, then only InCB=Extend
/// characters) and GB11 (a pictograph, then Extend characters, then a ZWJ) can both be going on at
/// once, because a linker is also an Extend character, so there is a state for each combination. A
/// regional-indicator run (GB12, GB13) excludes both: nothing but another indicator continues it.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Context {
    None,
    /// GB9c: a linker (a virama), then only InCB=Extend characters: a consonant now joins.
    Linked,
    /// GB11: a pictograph, then only Extend characters: a ZWJ continues the sequence.
    Pictographic,
    /// GB11 and GB9c both: a pictograph, then Extend characters including a linker.
    PictographicLinked,
    /// GB11: ... and the ZWJ: a pictograph now joins.
    PictographicZwj,
    /// GB11 and GB9c both, after the ZWJ.
    PictographicZwjLinked,
    /// GB12, GB13: an odd number of regional indicators so far: the next one pairs with the last.
    RegionalOdd,
}

impl Context {
    /// GB9c is waiting for a consonant.
    const fn linked(self) -> bool {
        matches!(self, Self::Linked | Self::PictographicLinked | Self::PictographicZwjLinked)
    }

    /// GB11 is waiting for a pictograph (the ZWJ has come).
    const fn zwj_seen(self) -> bool {
        matches!(self, Self::PictographicZwj | Self::PictographicZwjLinked)
    }
}

/// Where a run of text stands for [`breaks`]: the class of the last character and what a later rule
/// is waiting for, as one state of the automaton. One byte, `Copy`; make one with
/// [`GraphemeState::new`] at the start of the text and feed it each character in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphemeState(u8);

impl Default for GraphemeState {
    fn default() -> Self {
        Self::new()
    }
}

/// The answers for one character: see [`GraphemeState::step`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// Whether a cluster boundary falls before the character.
    pub breaks: bool,
    /// The character's own width, as [`width`](super::width).
    pub width: u8,
}

/// Whether a cluster boundary falls before `c`, and the state for the character after it. The first
/// character of a fresh [`GraphemeState`] always starts a cluster.
#[inline]
pub fn breaks(state: &mut GraphemeState, c: char) -> bool {
    state.step(c).breaks
}

impl GraphemeState {
    /// The state before any character. Use a new one whenever the next character is not printed right
    /// after the last one (the cursor moved, the screen was cleared): a cluster only grows while
    /// characters arrive one after another.
    pub const fn new() -> Self {
        Self(0)
    }

    /// The state after a printable ASCII character, whatever it is: they are all class Other, so a
    /// terminal that has just written a run of them can continue the cluster without stepping.
    pub(crate) const AFTER_ASCII: Self = Self(MACHINE.next[0][class::OTHER as usize] & !BREAK);

    /// Whether a boundary falls before `c` and `c`'s width, with one lookup. A boundary means `c`
    /// starts a new cluster; none means it belongs to the one before, and the cluster's width is
    /// [`extend_width`](super::extend_width) of what it was.
    #[inline(always)]
    pub fn step(&mut self, c: char) -> Step {
        let cp = c as u32;
        // Printable ASCII: width 1, class Other, no table. (Only a Prepend before it, GB9b, holds it
        // to the previous character, and the automaton knows.)
        if cp.wrapping_sub(0x20) < 0x5F {
            return Step { breaks: self.advance(class::OTHER), width: 1 };
        }
        let props = tables::props(cp);
        Step { breaks: self.advance(props >> CLASS_SHIFT), width: props & WIDTH_MASK }
    }

    /// Moves to the state after a character of `class`; true when a boundary fell before it.
    #[inline]
    fn advance(&mut self, class: u8) -> bool {
        let entry = MACHINE.next[usize::from(self.0)][usize::from(class & 31)];
        self.0 = entry & !BREAK;
        entry & BREAK != 0
    }
}

/// The high bit of a table entry: a boundary falls before the character.
const BREAK: u8 = 0x80;

/// The automaton: `next[state][class]` is the state after a character of that class, with `BREAK`
/// set when a boundary fell before it. State 0 is the start of text.
struct Machine {
    next: [[u8; 32]; MAX_STATES],
    /// How many states the rules reach (the rest of `next` is unused); only a test reads it.
    #[cfg_attr(not(test), allow(dead_code))]
    states: usize,
}

/// The automaton has 29 states; the build stops (as a compile error) if the rules ever need more.
const MAX_STATES: usize = 64;

const MACHINE: Machine = build();

/// Explores every state the rules can reach from the start, numbering them as they are found.
const fn build() -> Machine {
    let mut next = [[0u8; 32]; MAX_STATES];
    // A state is a (class of the previous character, context) pair.
    let mut prev_of = [class::START; MAX_STATES];
    let mut context_of = [Context::None; MAX_STATES];
    let mut count = 1;
    let mut state = 0;
    while state < count {
        let mut c = 0;
        while c < class::START {
            let (prev, context) = (prev_of[state], context_of[state]);
            let to_context = next_context(context, c);
            let mut to = 0;
            while to < count && !(prev_of[to] == c && context_of[to] as u8 == to_context as u8) {
                to += 1;
            }
            if to == count {
                prev_of[to] = c;
                context_of[to] = to_context;
                count += 1;
            }
            let breaks = !joins(prev, context, c);
            next[state][c as usize] = to as u8 | if breaks { BREAK } else { 0 };
            c += 1;
        }
        state += 1;
    }
    Machine { next, states: count }
}

/// Whether a character of class `next` stays in the cluster of the one before it (of class `prev`,
/// after the characters that gave `context`): the rules, in the standard's order, the first that
/// applies deciding.
const fn joins(prev: u8, context: Context, next: u8) -> bool {
    // GB1: a boundary at the start. GB2 (at the end) has nothing to decide before a character.
    if prev == class::START {
        return false;
    }
    // GB3.
    if prev == class::CR && next == class::LF {
        return true;
    }
    // GB4, GB5.
    if is_control(prev) || is_control(next) {
        return false;
    }
    // GB6, GB7, GB8: a Hangul syllable is L* V+ T*, with the precomposed LV and LVT.
    let hangul = match prev {
        class::L => matches!(next, class::L | class::V | class::LV | class::LVT),
        class::LV | class::V => matches!(next, class::V | class::T),
        class::LVT | class::T => next == class::T,
        _ => false,
    };
    if hangul {
        return true;
    }
    // GB9, GB9a: marks and ZWJ join the character before them. GB9b: a Prepend joins what follows it.
    if matches!(next, class::EXTEND | class::EXTEND_INCB | class::LINKER | class::ZWJ | class::SPACING_MARK)
        || prev == class::PREPEND
    {
        return true;
    }
    // GB9c (Unicode 18): virama, [InCB=Extend]* x consonant.
    if next == class::CONSONANT && context.linked() {
        return true;
    }
    // GB11: pictograph, Extend*, ZWJ x pictograph.
    if next == class::EXTENDED_PICTOGRAPHIC && context.zwj_seen() {
        return true;
    }
    // GB12, GB13: regional indicators pair up from the start of their run. GB999: otherwise a boundary.
    next == class::REGIONAL_INDICATOR && matches!(context, Context::RegionalOdd)
}

/// What is going on after a character of class `class`, given what was before it.
const fn next_context(context: Context, class: u8) -> Context {
    use Context::*;
    match class {
        class::EXTENDED_PICTOGRAPHIC => Pictographic,
        // A virama is an Extend character too, so it also carries a pictograph's sequence; after
        // a ZWJ it does not (GB11 wants the pictograph right after the ZWJ).
        class::LINKER => match context {
            Pictographic | PictographicLinked => PictographicLinked,
            _ => Linked,
        },
        // A linker that is no Extend character (Grapheme_Cluster_Break=Other) ends a pictograph's run.
        class::LINKER_OTHER => Linked,
        class::EXTEND_INCB => match context {
            Linked | Pictographic | PictographicLinked => context,
            PictographicZwjLinked => Linked,
            _ => None,
        },
        // Extend, but not InCB=Extend: it ends a conjunct and still continues a pictograph.
        class::EXTEND => match context {
            Pictographic | PictographicLinked => Pictographic,
            _ => None,
        },
        // ZWJ is InCB=Extend, and it is what GB11 waits for. It is no Extend, so a second one ends that.
        class::ZWJ => match context {
            Pictographic => PictographicZwj,
            PictographicLinked => PictographicZwjLinked,
            Linked | PictographicZwjLinked => Linked,
            _ => None,
        },
        class::REGIONAL_INDICATOR => match context {
            RegionalOdd => None,
            _ => RegionalOdd,
        },
        _ => None,
    }
}

const fn is_control(class: u8) -> bool {
    class == class::CR || class == class::LF || class == class::CONTROL
}

#[cfg(test)]
mod tests {
    use super::{GraphemeState, MACHINE, MAX_STATES};
    use crate::unicode::tables::{self, CLASS_SHIFT, class};

    fn class_of(c: char) -> u8 {
        tables::props(c as u32) >> CLASS_SHIFT
    }

    #[test]
    fn hangul_syllables_alternate_lv_and_lvt() {
        // The table has no formula for this; Unicode defines it: every 28th syllable has no final.
        for (index, cp) in (0xAC00..=0xD7A3u32).enumerate() {
            let c = char::from_u32(cp).expect("a Hangul syllable is a char");
            let expected = if index % 28 == 0 { class::LV } else { class::LVT };
            assert_eq!(class_of(c), expected, "U+{cp:04X}");
        }
    }

    #[test]
    fn every_class_is_below_the_start_state() {
        for c in (0..=0x10FFFFu32).filter_map(char::from_u32) {
            assert!(class_of(c) < class::START, "U+{:04X}", c as u32);
        }
    }

    #[test]
    fn ascii_shortcut_agrees_with_the_table() {
        for c in (0x20u8..0x7F).map(char::from) {
            assert_eq!(class_of(c), class::OTHER, "{c:?}");
            assert_eq!(tables::props(c as u32) & tables::WIDTH_MASK, 1, "{c:?}");
        }
    }

    #[test]
    fn the_automaton_is_small() {
        assert_eq!(size_of::<GraphemeState>(), 1);
        // Every state a transition leads to exists, and the table is not close to full.
        let mut reached = 1;
        for row in &MACHINE.next {
            for &entry in &row[..class::START as usize] {
                reached = reached.max((entry & 0x7F) as usize + 1);
            }
        }
        assert!(reached < MAX_STATES, "{reached} states");
    }

    /// The comment on `MAX_STATES` says how many states the rules need: it must stay true, as the
    /// number is what tells a reader how much room is left.
    #[test]
    fn the_automaton_has_the_states_the_comment_says() {
        assert_eq!(MACHINE.states, 29);
    }

    /// The rules of UAX #29 as the standard words them, over the classes before a position, with none
    /// of the automaton's bookkeeping: what a reader would write from the text.
    fn reference_breaks(classes: &[u8], at: usize) -> bool {
        use class::*;
        let is_extend = |c: u8| matches!(c, EXTEND | EXTEND_INCB | LINKER);
        let is_control = |c: u8| matches!(c, CR | LF | CONTROL);
        if at == 0 {
            return true; // GB1
        }
        let (prev, next) = (classes[at - 1], classes[at]);
        if prev == CR && next == LF {
            return false; // GB3
        }
        if is_control(prev) || is_control(next) {
            return true; // GB4, GB5
        }
        match (prev, next) {
            (L, L | V | LV | LVT) | (LV | V, V | T) | (LVT | T, T) => return false, // GB6 to GB8
            _ => {}
        }
        if is_extend(next) || next == ZWJ || next == SPACING_MARK || prev == PREPEND {
            return false; // GB9, GB9a, GB9b
        }
        if next == CONSONANT {
            // GB9c: back over InCB=Extend* to a Linker.
            let mut start = at;
            while start > 0 && matches!(classes[start - 1], EXTEND_INCB | ZWJ) {
                start -= 1;
            }
            if start > 0 && matches!(classes[start - 1], LINKER | LINKER_OTHER) {
                return false;
            }
        }
        if next == EXTENDED_PICTOGRAPHIC && prev == ZWJ {
            // GB11: back over Extend* from the ZWJ to a pictograph.
            let mut start = at - 1;
            while start > 0 && is_extend(classes[start - 1]) {
                start -= 1;
            }
            if start > 0 && classes[start - 1] == EXTENDED_PICTOGRAPHIC {
                return false;
            }
        }
        if next == REGIONAL_INDICATOR && prev == REGIONAL_INDICATOR {
            // GB12, GB13: an odd number of indicators before this one, in a row.
            let run = classes[..at].iter().rev().take_while(|&&c| c == REGIONAL_INDICATOR).count();
            if run % 2 == 1 {
                return false;
            }
        }
        true // GB999
    }

    fn automaton_breaks(classes: &[u8]) -> Vec<bool> {
        let mut state = GraphemeState::new();
        classes.iter().map(|&c| state.advance(c)).collect()
    }

    fn check(classes: &[u8]) {
        let expected: Vec<bool> = (0..classes.len()).map(|at| reference_breaks(classes, at)).collect();
        assert_eq!(automaton_breaks(classes), expected, "{classes:?}");
    }

    #[test]
    fn the_automaton_follows_the_rules_for_every_short_sequence() {
        // All 19^4 sequences of four classes: every pair, and every rule's context up to a few characters.
        let n = class::START;
        for a in 0..n {
            for b in 0..n {
                for c in 0..n {
                    for d in 0..n {
                        check(&[a, b, c, d]);
                    }
                }
            }
        }
    }

    #[test]
    fn the_automaton_follows_the_rules_for_long_sequences() {
        // Runs of the classes the contexts are made of, much longer than the exhaustive test reaches.
        use class::*;
        let pool = [
            OTHER,
            CONSONANT,
            LINKER,
            LINKER_OTHER,
            EXTEND_INCB,
            EXTEND,
            ZWJ,
            EXTENDED_PICTOGRAPHIC,
            REGIONAL_INDICATOR,
            PREPEND,
            SPACING_MARK,
            L,
            V,
            T,
            LV,
            LVT,
            CR,
            LF,
            CONTROL,
        ];
        let mut x = 0x2545_F491_4F6C_DD1Du64;
        for _ in 0..20_000 {
            let mut classes = Vec::new();
            for _ in 0..14 {
                x ^= x >> 12;
                x ^= x << 25;
                x ^= x >> 27;
                // Mostly the context classes, so long conjuncts, ZWJ chains and flag runs happen.
                let pick =
                    if (x >> 60) < 12 { (x >> 20) as usize % 10 } else { (x >> 20) as usize % pool.len() };
                classes.push(pool[pick]);
            }
            check(&classes);
        }
    }
}
