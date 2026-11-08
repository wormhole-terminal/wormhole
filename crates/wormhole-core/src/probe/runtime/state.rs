//! What the Probes' hooks have set, as the shells read it: the status items, the blocks a hook tinted and the
//! names of the Probes that are on. State only: the shell keeps one of these where it keeps its globals and
//! tells whatever draws it when `set_status` or `mark_block` says something changed.

use std::collections::VecDeque;

use crate::probe::hidden::reveal;
use crate::probe::tone::Tone;

/// Blocks a hook marked that are remembered: the oldest is forgotten first, as scrollback forgets
/// its rows. A hook is rate-limited, so this is a ceiling a program cannot raise.
const MARKED_BLOCKS: usize = 256;

/// A status item a hook set: the Probe's, labelled so (its name is in the tooltip).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusItem {
    pub probe: String,
    pub probe_name: String,
    pub item: String,
    pub text: String,
    pub tone: Option<Tone>,
    pub tooltip: Option<String>,
}

impl StatusItem {
    /// The tooltip the bar gives it: the Probe's name before whatever tooltip it gave, both made visible.
    pub fn shown_tooltip(&self) -> String {
        let name = reveal(&self.probe_name).into_owned();
        match &self.tooltip {
            Some(tooltip) => format!("{name}: {tooltip}"),
            None => name,
        }
    }
}

#[derive(Default)]
pub struct Effects {
    /// The Probes that are on: id and name.
    probes: Vec<(String, String)>,
    status: Vec<StatusItem>,
    /// `(terminal, block)` and the tone a hook gave it, oldest first.
    marked: VecDeque<((i64, u64), Tone)>,
}

impl Effects {
    /// The Probes of `active` (id and name) are the ones on: what the others set is gone, and so is every mark.
    pub fn install(&mut self, active: Vec<(String, String)>) {
        self.status.retain(|item| active.iter().any(|(id, _)| *id == item.probe));
        self.marked.clear();
        self.probes = active;
    }

    /// The name of the Probe `id`, when it is on.
    pub fn probe_name(&self, id: &str) -> Option<&str> {
        self.probes.iter().find(|(probe, _)| probe == id).map(|(_, name)| name.as_str())
    }

    pub fn status_items(&self) -> &[StatusItem] {
        &self.status
    }

    /// A hook set `item`: it replaces what that Probe's item said. Whether anything changed.
    pub fn set_status(&mut self, item: StatusItem) -> bool {
        match self.status.iter_mut().find(|old| old.probe == item.probe && old.item == item.item) {
            Some(old) if *old == item => false,
            Some(old) => {
                *old = item;
                true
            }
            None => {
                self.status.push(item);
                true
            }
        }
    }

    /// A hook gave the rows of `block` in `terminal` a tone.
    pub fn mark_block(&mut self, terminal: i64, block: u64, tone: Tone) {
        self.marked.retain(|(key, _)| *key != (terminal, block));
        self.marked.push_back(((terminal, block), tone));
        while self.marked.len() > MARKED_BLOCKS {
            self.marked.pop_front();
        }
    }

    pub fn block_tone(&self, terminal: i64, block: u64) -> Option<Tone> {
        self.marked.iter().rev().find(|(key, _)| *key == (terminal, block)).map(|(_, tone)| *tone)
    }
}

#[cfg(test)]
mod tests {
    use super::{Effects, StatusItem, Tone};

    fn item(text: &str) -> StatusItem {
        StatusItem {
            probe: "a".into(),
            probe_name: "A".into(),
            item: "build".into(),
            text: text.into(),
            tone: None,
            tooltip: None,
        }
    }

    #[test]
    fn an_item_is_replaced_not_added_and_a_probe_turned_off_takes_its_items_and_marks() {
        let mut effects = Effects::default();
        effects.install(vec![("a".into(), "A".into())]);
        assert!(effects.set_status(item("ok")));
        assert!(!effects.set_status(item("ok")), "the same word again changes nothing");
        assert!(effects.set_status(item("failed")));
        assert_eq!(effects.status_items().len(), 1);
        assert_eq!(effects.status_items()[0].shown_tooltip(), "A");
        effects.mark_block(1, 7, Tone::Danger);
        assert_eq!(effects.block_tone(1, 7), Some(Tone::Danger));
        effects.install(Vec::new());
        assert!(effects.status_items().is_empty() && effects.block_tone(1, 7).is_none());
    }
}
