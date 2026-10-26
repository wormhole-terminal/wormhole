//! The settings of a line-based config file (Ghostty's, Kitty's) as a list of keys and values in
//! the order they take effect, so "the last one wins" and "repeatable keys add up" are both a
//! look at the list.

/// One line of a config file that sets something.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) key: String,
    pub(crate) value: String,
    /// The line as it was read, trimmed, for the review list.
    pub(crate) raw: String,
}

/// The entries of one source, in effect order.
#[derive(Clone, Debug, Default)]
pub(crate) struct Entries(Vec<Entry>);

impl Entries {
    pub(crate) fn push(&mut self, key: &str, value: &str, raw: &str) {
        self.0.push(Entry { key: key.to_string(), value: value.to_string(), raw: raw.to_string() });
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.0.iter()
    }

    /// The entry of `key` that takes effect: the last.
    pub(crate) fn last(&self, key: &str) -> Option<&Entry> {
        self.0.iter().rev().find(|entry| entry.key == key)
    }

    /// Every entry of `key`, in order.
    pub(crate) fn all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a Entry> {
        self.0.iter().filter(move |entry| entry.key == key)
    }
}
