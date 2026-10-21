//! A hyperlink (OSC 8) as a host reads it: a target and the id that joins its pieces.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

/// A hyperlink (OSC 8). Cells printed under one share it; two with the same id and target are
/// the same link.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Hyperlink {
    /// What the program called it (cells with the same id belong together); a generated one when
    /// it gave none.
    pub id: LinkId,
    /// The target, as the program sent it (control characters are refused). Untrusted: show it
    /// before opening it.
    pub uri: Arc<str>,
}

/// The longest id kept inside the value; a longer one is shared as an `Arc<str>`.
const INLINE: usize = 22;

/// A hyperlink's id: text that reads like a `&str` (it derefs to one). Short ids, which are nearly all
/// of them (`link-123`, `l7`, a number) and every id the engine generates, are stored in the value,
/// so making one allocates nothing; a longer one is shared.
#[derive(Clone)]
pub struct LinkId(Repr);

#[derive(Clone)]
enum Repr {
    /// `len` bytes of valid UTF-8 in `bytes`.
    Inline {
        len: u8,
        bytes: [u8; INLINE],
    },
    Shared(Arc<str>),
}

impl LinkId {
    /// The id of a link that gave none: `link-{number}`, as the engine generates them.
    pub fn generated(number: u64) -> LinkId {
        use std::fmt::Write as _;
        let mut name = Inline::default();
        match write!(name, "link-{number}") {
            Ok(()) => LinkId(Repr::Inline { len: name.len, bytes: name.bytes }),
            // A number of 18 digits, which a terminal does not live to count to.
            Err(_) => LinkId(Repr::Shared(Arc::from(format!("link-{number}")))),
        }
    }

    /// An id made of `text`, kept inside the value when it is short.
    pub fn new(text: &str) -> LinkId {
        match Inline::of(text) {
            Some(Inline { len, bytes }) => LinkId(Repr::Inline { len, bytes }),
            None => LinkId(Repr::Shared(Arc::from(text))),
        }
    }

    /// Whether `text` is long enough that an id made of it is shared, not stored in the value.
    pub fn is_inline(text: &str) -> bool {
        text.len() <= INLINE
    }

    /// An id of a text a caller already shares (an address table's, say): kept as it is when it
    /// is long, copied into the value when it is short.
    pub fn from_shared(text: Arc<str>) -> LinkId {
        match Inline::of(&text) {
            Some(Inline { len, bytes }) => LinkId(Repr::Inline { len, bytes }),
            None => LinkId(Repr::Shared(text)),
        }
    }

    /// The id's bytes (valid UTF-8).
    pub fn as_bytes(&self) -> &[u8] {
        match &self.0 {
            Repr::Inline { len, bytes } => &bytes[..usize::from(*len)],
            Repr::Shared(text) => text.as_bytes(),
        }
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        match &self.0 {
            Repr::Inline { len, bytes } => std::str::from_utf8(&bytes[..usize::from(*len)]).unwrap_or(""),
            Repr::Shared(text) => text,
        }
    }

    /// Whether `self` and `other` are the same id, by the cheapest test there is: the same bytes
    /// when stored inline, the same allocation when shared. True means equal; false means nothing.
    pub(crate) fn is_same(&self, other: &LinkId) -> bool {
        match (&self.0, &other.0) {
            (Repr::Shared(a), Repr::Shared(b)) => Arc::ptr_eq(a, b),
            (Repr::Inline { .. }, Repr::Inline { .. }) => self.as_bytes() == other.as_bytes(),
            _ => false,
        }
    }
}

/// A short text being built, or the pieces of an inline id.
#[derive(Default)]
struct Inline {
    bytes: [u8; INLINE],
    len: u8,
}

impl Inline {
    fn of(text: &str) -> Option<Inline> {
        let mut inline = Inline::default();
        fmt::Write::write_str(&mut inline, text).ok()?;
        Some(inline)
    }
}

impl fmt::Write for Inline {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let start = usize::from(self.len);
        let end = start + text.len();
        self.bytes.get_mut(start..end).ok_or(fmt::Error)?.copy_from_slice(text.as_bytes());
        self.len = end as u8;
        Ok(())
    }
}

impl Deref for LinkId {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq for LinkId {
    fn eq(&self, other: &LinkId) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for LinkId {}

impl PartialEq<str> for LinkId {
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<&str> for LinkId {
    fn eq(&self, other: &&str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Hash for LinkId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl fmt::Debug for LinkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for LinkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for LinkId {
    fn from(text: &str) -> LinkId {
        LinkId::new(text)
    }
}

impl From<String> for LinkId {
    fn from(text: String) -> LinkId {
        LinkId::new(&text)
    }
}

impl From<Arc<str>> for LinkId {
    fn from(text: Arc<str>) -> LinkId {
        LinkId::from_shared(text)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;

    use super::{Hyperlink, LinkId};

    #[test]
    fn short_and_long_ids_read_the_same_and_compare_by_text() {
        let short = LinkId::new("l7");
        let long = LinkId::new(&"x".repeat(40));
        assert_eq!((&*short, short.len()), ("l7", 2));
        assert_eq!(long.len(), 40);
        assert_eq!(short, LinkId::from(Arc::<str>::from("l7")), "a shared short text is the same id");
        assert_eq!(long, LinkId::from(Arc::<str>::from("x".repeat(40))));
        assert_ne!(short, long);
        assert_eq!(short, *"l7");
        assert_eq!(format!("{short} {short:?}"), "l7 \"l7\"");
        let set: HashSet<LinkId> = [short.clone(), LinkId::from("l7".to_string())].into_iter().collect();
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn an_id_is_not_equal_to_other_text() {
        let short = LinkId::new("l7");
        let long = LinkId::new(&"x".repeat(40));
        assert_ne!(short, *"l8");
        assert_ne!(short, *"l");
        assert_ne!(long, *"x");
        assert_eq!(short, "l7");
        assert_ne!(short, "l8");
        assert_ne!(short, "");
        assert_eq!(long, "x".repeat(40).as_str());
        assert_ne!(long, "x".repeat(39).as_str());
    }

    #[test]
    fn an_id_hashes_as_the_text_it_is() {
        use std::hash::{DefaultHasher, Hash, Hasher};
        fn hash_of(value: &impl Hash) -> u64 {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }
        let long = "x".repeat(40);
        assert_eq!(hash_of(&LinkId::new("l7")), hash_of(&"l7"));
        assert_eq!(hash_of(&LinkId::new(&long)), hash_of(&long.as_str()));
        assert_ne!(hash_of(&LinkId::new("l7")), hash_of(&LinkId::new("l8")));
    }

    #[test]
    fn the_longest_inline_id_fits_and_the_next_is_shared() {
        for len in [0, 1, 21, 22, 23, 60] {
            let text = "a".repeat(len);
            let id = LinkId::new(&text);
            assert_eq!(id.as_str(), text);
            assert_eq!(LinkId::is_inline(&text), len <= 22);
        }
        let a = LinkId::new(&"a".repeat(23));
        assert!(a.is_same(&a.clone()) && !a.is_same(&LinkId::new(&"a".repeat(23))));
        assert!(LinkId::new("b").is_same(&LinkId::new("b")));
    }

    #[test]
    fn generated_ids_are_named_by_their_number() {
        assert_eq!(LinkId::generated(7), *"link-7");
        assert_eq!(LinkId::generated(u64::MAX).as_str(), format!("link-{}", u64::MAX));
        let link = Hyperlink { id: LinkId::generated(1), uri: "https://a".into() };
        assert_eq!(format!("{link:?}"), "Hyperlink { id: \"link-1\", uri: \"https://a\" }");
    }
}
