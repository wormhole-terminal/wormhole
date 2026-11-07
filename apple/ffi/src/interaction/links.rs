//! Links under the pointer: a program's hyperlink (OSC 8) or a URL in the text, and what may be opened.

use wormhole_term::Openable;
use wormhole_term::types::Line;

use super::selection::RowSpan;
use crate::terminal::Terminal;

/// A link at a cell, with where it shows (a long one wraps over rows).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LinkHit {
    pub uri: String,
    /// A program's own hyperlink rather than a URL found in the text.
    pub explicit: bool,
    /// The columns `first...=last` it covers on each viewport row it shows on.
    pub spans: Vec<RowSpan>,
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
    /// The link at a viewport cell, if there is one.
    pub fn link_at(&self, row: u32, col: u32) -> Option<LinkHit> {
        let point = self.grid_point(row, col);
        let link = self.with_pty(|pty| pty.link_at(point)).flatten()?;
        let (offset, cols, rows) = self.with_snapshot(|snapshot| {
            (snapshot.display_offset as i32, snapshot.columns(), snapshot.rows().len())
        });
        let spans = (0..rows)
            .filter_map(|row| {
                let columns = link.columns_on(Line(row as i32 - offset), cols)?;
                Some(RowSpan { row: row as u32, first: columns.start as u32, last: columns.end as u32 - 1 })
            })
            .collect();
        Some(LinkHit { uri: link.uri, explicit: link.explicit, spans })
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
