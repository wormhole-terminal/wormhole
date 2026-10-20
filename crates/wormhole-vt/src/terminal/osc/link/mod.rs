//! OSC 8: hyperlinks. The pen carries the link, so every cell printed while it is set is part of it.

mod text;

use crate::terminal::Terminal;
use crate::types::Hyperlink;

pub(in crate::terminal) use text::LinkText;

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

    /// The link `id` (or none) and `uri` name, made of the shared text of the links before it; a
    /// link with no id of its own gets a fresh one, since nothing ties two of them together.
    fn intern_link(&mut self, id: Option<&str>, uri: &str) -> Hyperlink {
        let uri = self.link_text.uri(uri);
        let id = match id {
            Some(id) => self.link_text.id(id),
            None => {
                self.link_seq += 1;
                LinkText::fresh_id(self.link_seq)
            }
        };
        Hyperlink { id, uri }
    }
}
