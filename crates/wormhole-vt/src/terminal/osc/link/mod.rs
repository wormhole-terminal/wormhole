//! OSC 8: hyperlinks. The pen carries the link, so every cell printed while it is set is part of it.

use std::fmt::Write as _;
use std::sync::Arc;

use crate::types::Hyperlink;

use crate::terminal::{RECENT_LINKS, Terminal};

/// A short text built without allocating: `write!` into it, then read it back.
struct StackText {
    bytes: [u8; 40],
    len: usize,
}

impl StackText {
    fn new() -> StackText {
        StackText { bytes: [0; 40], len: 0 }
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

impl std::fmt::Write for StackText {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let end = self.len + text.len();
        self.bytes.get_mut(self.len..end).ok_or(std::fmt::Error)?.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl Terminal {
    /// OSC 8 `params ; uri`: the cells printed from here on are a link, until an empty `uri`.
    pub(in crate::terminal) fn osc_hyperlink(&mut self, rest: &[u8]) {
        let Some(at) = rest.iter().position(|&b| b == b';') else { return };
        let (params, uri) = (&rest[..at], &rest[at + 1..]);
        if uri.is_empty() {
            self.pen.link = None;
            return;
        }
        // The target is text a user may be shown and click: no control characters, and not longer
        // than the host's limit (the OSC string's own cap is the ceiling).
        let Ok(uri) = std::str::from_utf8(uri) else { return };
        if uri.len() > self.limits.link_uri || uri.chars().any(char::is_control) {
            return;
        }
        let id = params
            .split(|&b| b == b':')
            .find_map(|p| p.strip_prefix(b"id="))
            .and_then(|id| std::str::from_utf8(id).ok().filter(|id| !id.is_empty()));
        if let (Some(id), Some(open)) = (id, &self.pen.link)
            && &*open.id == id
            && &*open.uri == uri
        {
            return;
        }
        self.pen.link = Some(self.intern_link(id, uri));
    }

    /// The link `id` (or none) and `uri` name. The text of the last few is kept, so that a
    /// program that prints the same file name or address again and again (a build log, a file
    /// listing, an agent naming the files it touched) shares one allocation of it; a link with no
    /// id of its own gets a fresh id, since nothing ties two of them together.
    fn intern_link(&mut self, id: Option<&str>, uri: &str) -> Hyperlink {
        let hit = self
            .recent_links
            .iter()
            .position(|link| &*link.uri == uri && id.is_none_or(|id| &*link.id == id));
        let shared_uri = hit.map(|at| {
            // Most recent first.
            let link = self.recent_links.remove(at);
            self.recent_links.insert(0, link.clone());
            link
        });
        if let (Some(link), Some(_)) = (&shared_uri, id) {
            return link.clone();
        }
        let uri: Arc<str> = shared_uri.map_or_else(|| Arc::from(uri), |link| link.uri);
        let id: Arc<str> = match id {
            Some(id) => Arc::from(id),
            None => {
                self.link_seq += 1;
                let mut name = StackText::new();
                let _ = write!(name, "link-{}", self.link_seq);
                Arc::from(name.as_str())
            }
        };
        let link = Hyperlink { id, uri };
        self.recent_links.truncate(RECENT_LINKS - 1);
        self.recent_links.insert(0, link.clone());
        link
    }
}
