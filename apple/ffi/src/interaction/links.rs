//! Links under the pointer: a program's hyperlink (OSC 8) or a URL in the text, and what may be opened.

use wormhole_term::Openable;

use super::selection::RowSpan;
use crate::probes::{readout, rules};
use crate::terminal::Terminal;

/// A link at a cell, with where it shows (a long one wraps over rows).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LinkHit {
    pub uri: String,
    /// A program's own hyperlink rather than a URL found in the text.
    pub explicit: bool,
    /// The columns `first...=last` it covers on each viewport row it shows on.
    pub spans: Vec<RowSpan>,
    /// A Probe's rule made it and names a file (`uri` is `path[:line[:column]]`, as the rule wrote it): not an
    /// address; `Terminal::resolve_path` finds it, and the editor opens it.
    pub file: bool,
    /// What a rule's link says while ⌘ is held over it: its hover words, where it really goes and which Probe said so.
    pub readout: Option<String>,
}

/// What opening a link means; only the kinds the app allows.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum OpenTarget {
    /// http, https or mailto: the system opens it.
    Url { url: String },
    /// A `file:` link on this machine: shown in the Finder, never run.
    Reveal { path: String },
    /// Not a kind that is opened.
    Refused,
}

#[uniffi::export]
impl Terminal {
    /// The link at a viewport cell, if there is one: a program's, the text's, or a Probe's rule's.
    pub fn link_at(&self, row: u32, col: u32) -> Option<LinkHit> {
        let point = self.grid_point(row, col);
        let rules = rules().0;
        let link = self
            .with_pty(|pty| match &rules {
                Some(rules) => pty.link_at_with(point, &**rules),
                None => pty.link_at(point),
            })
            .flatten()?;
        // Where each row of the view shows it: a fold's marker row shows no text, and under folds a row's line
        // is not its place in the view.
        let spans = self.with_snapshot(|snapshot| {
            let cols = snapshot.columns();
            snapshot
                .rows()
                .iter()
                .enumerate()
                .filter(|(_, shown)| shown.fold().is_none())
                .filter_map(|(row, shown)| {
                    let columns = link.columns_on(shown.line(), cols)?;
                    Some(RowSpan {
                        row: row as u32,
                        first: columns.start as u32,
                        last: columns.end as u32 - 1,
                    })
                })
                .collect()
        });
        let rule = link.rule.as_ref();
        let readout = rule.map(|rule| readout(rule.hover.as_deref(), &link.uri, &rule.owner));
        let file = rule.is_some_and(|rule| rule.file);
        Some(LinkHit { uri: link.uri, explicit: link.explicit, spans, file, readout })
    }

    /// What `uri` opens as.
    pub fn open_target(&self, uri: String) -> OpenTarget {
        match Openable::from_uri(&uri) {
            Some(Openable::Url(url)) => OpenTarget::Url { url },
            Some(Openable::Reveal(path)) => OpenTarget::Reveal { path: path.to_string_lossy().into_owned() },
            None => OpenTarget::Refused,
        }
    }
}
