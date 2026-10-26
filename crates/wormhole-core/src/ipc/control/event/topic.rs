//! The kinds of event a stream can ask for.

use serde::{Deserialize, Serialize};

/// A kind of event, which a subscriber asks for by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    /// Windows opening and closing.
    Windows,
    /// Terminals opening, closing, moving to another window and being renamed.
    Terminals,
    /// A terminal's state changing.
    Status,
    /// A terminal's title changing.
    Title,
    /// A terminal's folder changing.
    Folder,
    /// A terminal's bell.
    Bell,
    /// Commands starting and finishing, in shells with integration.
    Blocks,
    /// Splits being made, changed and dissolved.
    Splits,
    /// What a terminal prints: terminal text, so it needs the grant to read.
    Output,
}

impl Topic {
    pub const ALL: [Topic; 9] = [
        Topic::Windows,
        Topic::Terminals,
        Topic::Status,
        Topic::Title,
        Topic::Folder,
        Topic::Bell,
        Topic::Blocks,
        Topic::Splits,
        Topic::Output,
    ];

    /// The bit this topic has in a [`Topics`] set.
    pub const fn bit(self) -> u32 {
        1 << self as u32
    }

    /// The name it has on the wire and on the command line.
    pub const fn name(self) -> &'static str {
        match self {
            Topic::Windows => "windows",
            Topic::Terminals => "terminals",
            Topic::Status => "status",
            Topic::Title => "title",
            Topic::Folder => "folder",
            Topic::Bell => "bell",
            Topic::Blocks => "blocks",
            Topic::Splits => "splits",
            Topic::Output => "output",
        }
    }

    pub fn from_name(name: &str) -> Option<Topic> {
        Topic::ALL.into_iter().find(|topic| topic.name() == name)
    }
}

/// A set of topics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Topics(u32);

impl Topics {
    /// What a subscriber gets without naming topics: everything but what terminals print.
    pub fn default_set() -> Self {
        Topic::ALL.into_iter().filter(|topic| *topic != Topic::Output).fold(Self::default(), Self::with)
    }

    /// Every topic, `output` too.
    pub fn all() -> Self {
        Topic::ALL.into_iter().fold(Self::default(), Self::with)
    }

    #[must_use]
    pub fn with(self, topic: Topic) -> Self {
        Self(self.0 | topic.bit())
    }

    pub fn contains(self, topic: Topic) -> bool {
        self.0 & topic.bit() != 0
    }

    /// The set as the bits the bus keeps its interest in.
    pub fn bits(self) -> u32 {
        self.0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The topics' names, in the order of [`Topic::ALL`].
    pub fn names(self) -> Vec<String> {
        Topic::ALL.into_iter().filter(|topic| self.contains(*topic)).map(|t| t.name().to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Topic, Topics};

    #[test]
    fn every_topic_has_its_own_bit_and_a_name_that_reads_back() {
        let mut seen = 0u32;
        for topic in Topic::ALL {
            assert_eq!(seen & topic.bit(), 0, "{topic:?} shares a bit");
            seen |= topic.bit();
            assert_eq!(Topic::from_name(topic.name()), Some(topic));
            assert_eq!(serde_json::to_string(&topic).expect("serializes"), format!("{:?}", topic.name()));
        }
        assert_eq!(Topic::from_name("nope"), None);
    }

    #[test]
    fn the_default_set_is_everything_but_output() {
        let set = Topics::default_set();
        assert!(Topic::ALL.into_iter().filter(|t| *t != Topic::Output).all(|t| set.contains(t)));
        assert!(!set.contains(Topic::Output));
        assert!(Topics::default().is_empty());
        assert!(set.with(Topic::Output).contains(Topic::Output));
        assert_eq!(Topics::default().with(Topic::Bell).with(Topic::Status).names(), ["status", "bell"]);
    }
}
