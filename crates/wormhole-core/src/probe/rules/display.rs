//! What `look` leaves of a matched text on screen: the display-only half of a rule.
//!
//! The text itself is never changed (copy, find, the saved scrollback and `wh read` see the real
//! characters); this says what each cell of the span *shows*. The result has one character for each
//! character of the match, so nothing on the row moves: a shortened text is left in place and the cells it
//! gave up are blank (`repo-docs/docs/probes.md`, section 3.1).

use super::compile::CompiledRule;
use crate::probe::manifest::{KeepSpec, Look, Reveal};

/// The ellipsis a shortened text shows where it dropped some.
const GAP: char = '…';
/// A redacted text: a solid block per character.
pub const BLOCK: char = '█';

impl CompiledRule {
    /// What each character of `text`, a match of this rule, shows; `None` for a rule that only tones
    /// or acts (`look = "none"`).
    pub fn display(&self, text: &str) -> Option<Vec<char>> {
        let chars: Vec<char> = text.chars().collect();
        match self.rule.look {
            Look::None => None,
            Look::Redact => Some(vec![BLOCK; chars.len()]),
            Look::Shorten => {
                let keep = self.rule.keep.as_deref().and_then(|keep| KeepSpec::parse(keep).ok());
                Some(shorten(&chars, keep.unwrap_or_default()))
            }
        }
    }

    /// When the real text shows again: the rule's `reveal`, which is `hover` without one.
    pub fn reveal(&self) -> Reveal {
        self.rule.reveal.unwrap_or(Reveal::Hover)
    }
}

/// `chars` with its middle replaced by `…`: the first `start` characters, the last `end` ones, and from
/// the last `@` on for `domain`. The rest of the span is blank. Text that keeps everything is shown whole.
fn shorten(chars: &[char], keep: KeepSpec) -> Vec<char> {
    let n = chars.len();
    let head = usize::from(keep.start.unwrap_or(0)).min(n);
    let domain = match keep.domain {
        true => chars.iter().rposition(|c| *c == '@').map_or(0, |at| n - at),
        false => 0,
    };
    let tail = usize::from(keep.end.unwrap_or(0)).max(domain).min(n - head);
    if head + tail + 1 > n {
        return chars.to_vec();
    }
    let mut shown: Vec<char> = chars[..head].to_vec();
    shown.push(GAP);
    shown.extend_from_slice(&chars[n - tail..]);
    shown.resize(n, ' ');
    shown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::manifest::Manifest;
    use crate::probe::rules::CompiledRules;

    fn rule(extra: &str) -> CompiledRule {
        let text = format!(
            "[probe]\nid = \"t\"\nname = \"T\"\ndescription = \"d\"\nauthor = \"a\"\nversion = \"0.1.0\"\n\
             wormhole = \">=1.3\"\napi = 1\n[[contributes.rules]]\nid = \"r\"\npattern = 'x'\n{extra}\n"
        );
        let parsed = Manifest::parse(&text).unwrap_or_else(|err| panic!("{err}"));
        CompiledRules::compile(&parsed.manifest).expect("compiles").iter().next().expect("one").clone()
    }

    fn shown(extra: &str, text: &str) -> Option<String> {
        rule(extra).display(text).map(|chars| chars.into_iter().collect())
    }

    #[test]
    fn a_shortened_email_keeps_its_start_and_domain_and_does_not_move() {
        let text = "bob@mailbox.org";
        let out = shown("look = 'shorten'\nkeep = 'start:1,domain'", text).expect("shortened");
        assert_eq!(out.trim_end(), "b…@mailbox.org");
        assert_eq!(out.chars().count(), text.chars().count(), "one character per character");
    }

    #[test]
    fn a_key_keeps_its_ends() {
        let out =
            shown("look = 'shorten'\nkeep = 'start:3,end:4'", "sk-abcdefghijklmnop3f9a").expect("shortened");
        assert_eq!(out.trim_end(), "sk-…3f9a");
    }

    #[test]
    fn text_that_keeps_all_of_itself_is_shown_whole_and_nothing_kept_is_a_gap() {
        assert_eq!(shown("look = 'shorten'\nkeep = 'start:8'", "abc").as_deref(), Some("abc"));
        assert_eq!(shown("look = 'shorten'", "secret").as_deref(), Some("…     "));
    }

    #[test]
    fn a_redacted_text_is_blocks_and_no_look_shows_nothing_of_its_own() {
        assert_eq!(shown("look = 'redact'", "hunter2").as_deref(), Some("███████"));
        assert_eq!(shown("tone = 'link'", "x"), None);
    }
}
