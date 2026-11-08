//! The words of a fold's marker row ("124 lines hidden · build output"), shell-neutral: both the GPUI
//! terminal and the Swift app's frame carry what this says, so VoiceOver reads the same sentence in either.

use wormhole_text::{keys, whole};

/// What a marker says, what VoiceOver says and its tooltip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerWords {
    pub label: String,
    pub spoken: String,
    /// Only a Probe's fold has one: it names the author, which the person's own fold has none of.
    pub tooltip: Option<String>,
}

/// The line ("124 lines hidden", or "shown" while it is open), then a Probe's own fixed label (never text of the
/// output) with the Probe's name in what is spoken and in the tooltip, then, while a search is open, how many
/// matches are inside. `owner` is the Probe's `(name, label)`.
pub fn marker_words(
    hidden: usize,
    expanded: bool,
    owner: Option<(&str, &str)>,
    matches: usize,
) -> MarkerWords {
    let count = whole(hidden);
    let line = if expanded { keys::terminal_fold_shown(count) } else { keys::terminal_fold_hidden(count) };
    let (mut label, mut spoken) = (line.clone(), line);
    if let Some((name, fixed)) = owner {
        spoken = keys::terminal_fold_spoken_by(&spoken, name, fixed);
        label = keys::terminal_fold_labelled(&label, fixed);
    }
    if matches > 0 {
        let inside = keys::terminal_fold_matches(whole(matches));
        label = keys::terminal_fold_with_matches(&label, &inside);
        spoken = keys::terminal_fold_with_matches(&spoken, &inside);
    }
    MarkerWords { label, spoken, tooltip: owner.map(|(name, _)| keys::terminal_fold_tooltip_by(name)) }
}

#[cfg(test)]
mod tests {
    use super::marker_words;

    #[test]
    fn a_probes_marker_carries_its_label_and_only_the_spoken_words_and_tooltip_its_name() {
        let words = marker_words(124, false, Some(("Acme tools", "build output")), 0);
        assert_eq!(words.label, "124 lines hidden · build output");
        assert_eq!(words.spoken, "124 lines hidden by Acme tools, build output");
        assert_eq!(words.tooltip.as_deref(), Some("Folded by Acme tools"));
        let own = marker_words(124, false, None, 2);
        assert_eq!(own.label, "124 lines hidden · 2 matches inside");
        assert_eq!(own.spoken, own.label);
        assert_eq!(own.tooltip, None, "the person's own fold has no author to name");
        assert_eq!(marker_words(1, true, None, 0).label, "1 line shown");
    }
}
