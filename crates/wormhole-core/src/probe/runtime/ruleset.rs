//! The rules of every Probe that is on, as one set the terminal asks about a line of text: what paints or
//! masks a span (`decorate`) and what a pointer over a span finds (`LineRules`). Shell-neutral: both shells
//! draw what it says.

use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::probe::manifest::{Look, Reveal};
use crate::probe::rules::{CompiledRules, Match, Target, hover, matches, target};
use crate::probe::tone::Tone;
use wormhole_term::{LineRules, RevealWhen, RuleHit, RuleTarget};

/// What one rule makes of a span of a line on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoration {
    /// Bytes of the line.
    pub range: Range<usize>,
    pub tone: Option<Tone>,
    /// What each character shows (shortened or redacted); `None` leaves the text as it is.
    pub mask: Option<Vec<char>>,
    pub reveal: RevealWhen,
}

/// The compiled rules of the Probes that are on.
pub struct RuleSet {
    rules: CompiledRules,
    /// How many times a line was run through the rules: the cost of rules, counted, for the tests.
    scans: AtomicUsize,
}

impl RuleSet {
    pub fn new(rules: CompiledRules) -> Self {
        Self { rules, scans: AtomicUsize::new(0) }
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Lines run through the rules since the set was made.
    #[cfg(test)]
    pub fn scans(&self) -> usize {
        self.scans.load(Ordering::Relaxed)
    }

    fn found(&self, line: &str) -> Vec<Match> {
        self.scans.fetch_add(1, Ordering::Relaxed);
        matches(&self.rules, line)
    }

    /// What the rules paint or mask in `line`, a logical line, in the order the spans lie in it. A rule that
    /// only has a link and a hover says nothing here: it costs nothing until a ⌘-hover (`LineRules`).
    pub fn decorate(&self, line: &str) -> Vec<Decoration> {
        self.found(line)
            .into_iter()
            .filter_map(|found| {
                let rule = self.rules.rule(found.rule)?;
                let tone = rule.rule.tone.as_deref().and_then(Tone::from_name);
                let mask = (found.look != Look::None).then(|| rule.display(&line[found.range()])).flatten();
                (tone.is_some() || mask.is_some()).then(|| Decoration {
                    range: found.range(),
                    tone,
                    mask,
                    reveal: reveal_when(rule.reveal()),
                })
            })
            .collect()
    }
}

fn reveal_when(reveal: Reveal) -> RevealWhen {
    match reveal {
        Reveal::Hover => RevealWhen::Hover,
        Reveal::Deliberate => RevealWhen::Deliberate,
        Reveal::Never => RevealWhen::Never,
    }
}

impl LineRules for RuleSet {
    fn hit_at(&self, line: &str, at: usize) -> Option<RuleHit> {
        let found = self.found(line).into_iter().find(|found| found.range().contains(&at))?;
        let rule = self.rules.rule(found.rule)?;
        let link = target(&self.rules, &found, line).and_then(|target| match target {
            Target::OpenUrl(url) => Some(RuleTarget::Url(url)),
            Target::OpenPath(path) => Some(RuleTarget::Path(path.text())),
            // Not run by this build: the loader takes these actions off, and one that is left is no link.
            Target::Copy(_) | Target::Command { .. } => None,
        });
        Some(RuleHit {
            range: found.range(),
            owner: rule.owner.clone(),
            masked: found.look != Look::None,
            reveal: reveal_when(rule.reveal()),
            link,
            hover: hover(&self.rules, &found, line),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::probe::manifest::Manifest;

    use super::*;

    fn set(rules: &str) -> RuleSet {
        let text = format!(
            "[probe]\nid = \"tiny\"\nname = \"Tiny\"\ndescription = \"d\"\nauthor = \"a\"\n\
             version = \"0.1.0\"\nwormhole = \">=1.3\"\napi = 1\n{rules}"
        );
        let parsed = Manifest::parse(&text).unwrap_or_else(|err| panic!("{err}"));
        RuleSet::new(CompiledRules::compile(&parsed.manifest).expect("compiles"))
    }

    const RULES: &str = r#"
[[contributes.rules]]
id = "failed"
pattern = 'FAILED [a-z_]+'
tone = "danger"

[[contributes.rules]]
id = "mail"
pattern = '[a-z]+@[a-z.]+'
look = "shorten"
keep = "start:1,domain"

[[contributes.rules]]
id = "path"
pattern = '[a-z/]+\.rs:[0-9]+'
hover = "Open {0}"
action = { type = "open-path" }
"#;

    #[test]
    fn what_paints_is_decorated_and_a_link_alone_is_not() {
        let rules = set(RULES);
        let line = "FAILED test_a by bob@mail.org in src/a.rs:3";
        let decorations = rules.decorate(line);
        assert_eq!(decorations.len(), 2, "{decorations:?}");
        assert_eq!(
            (decorations[0].tone, &line[decorations[0].range.clone()]),
            (Some(Tone::Danger), "FAILED test_a")
        );
        let mail = &decorations[1];
        assert_eq!(&line[mail.range.clone()], "bob@mail.org");
        assert_eq!(mail.mask.as_ref().map(|m| m.iter().collect::<String>()).as_deref(), Some("b…@mail.org "));
    }
}
