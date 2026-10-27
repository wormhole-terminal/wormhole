//! The MCP revisions this server speaks, and what each one changes for a tools-only server.
//!
//! Read from https://modelcontextprotocol.io/specification (the pages of each revision, 2026-10-27):
//! `2026-07-28` is the latest. It has no `initialize` handshake: every request carries its protocol
//! version and the client's capabilities in `_meta`, `server/discover` tells a client what the server
//! speaks, and results say `resultType`. The four before it (`2025-11-25`, `2025-06-18`, `2025-03-26`,
//! `2024-11-05`) open with `initialize`. A server may serve both kinds ("dual-era"), which this one does:
//! a request with the `_meta` fields is served as `2026-07-28`, an `initialize` starts the older flow.

use std::fmt;

use serde_json::{Value, json};

/// `_meta` keys of `2026-07-28`.
pub(super) const META_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
pub(super) const META_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";
pub(super) const META_SERVER_INFO: &str = "io.modelcontextprotocol/serverInfo";

/// A protocol revision, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Version {
    V2024_11_05,
    V2025_03_26,
    V2025_06_18,
    V2025_11_25,
    V2026_07_28,
}

impl Version {
    /// Every revision this server speaks, oldest first.
    pub(super) const ALL: [Version; 5] = [
        Version::V2024_11_05,
        Version::V2025_03_26,
        Version::V2025_06_18,
        Version::V2025_11_25,
        Version::V2026_07_28,
    ];

    /// What an `initialize` for a revision we do not know is answered with: the newest handshake-based one.
    pub(super) const LATEST_HANDSHAKE: Version = Version::V2025_11_25;

    pub(super) fn parse(text: &str) -> Option<Version> {
        Self::ALL.into_iter().find(|version| version.as_str() == text)
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Version::V2024_11_05 => "2024-11-05",
            Version::V2025_03_26 => "2025-03-26",
            Version::V2025_06_18 => "2025-06-18",
            Version::V2025_11_25 => "2025-11-25",
            Version::V2026_07_28 => "2026-07-28",
        }
    }

    /// No handshake: version and capabilities travel with every request.
    pub(super) fn is_stateless(self) -> bool {
        self >= Version::V2026_07_28
    }

    /// JSON-RPC batches: added by `2025-03-26`, removed again by `2025-06-18`.
    pub(super) fn allows_batches(self) -> bool {
        self == Version::V2025_03_26
    }

    /// Tool annotations (`readOnlyHint` and kin) and the progress notification's `message`.
    pub(super) fn has_annotations(self) -> bool {
        self >= Version::V2025_03_26
    }

    /// Titles, `outputSchema` and `structuredContent`.
    pub(super) fn has_structured_output(self) -> bool {
        self >= Version::V2025_06_18
    }

    /// The revisions this server speaks, newest first, as an error and `server/discover` list them.
    pub(super) fn listed() -> Vec<Value> {
        Self::ALL.into_iter().rev().map(|v| json!(v.as_str())).collect()
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::Version;

    #[test]
    fn versions_are_read_by_their_dates_and_ordered() {
        for version in Version::ALL {
            assert_eq!(Version::parse(version.as_str()), Some(version));
        }
        assert_eq!(Version::parse("2025-13-01"), None);
        assert_eq!(Version::parse(""), None);
        assert!(Version::ALL.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(Version::V2026_07_28.is_stateless() && !Version::V2025_11_25.is_stateless());
    }

    /// What each revision allows, as the changelogs say: batches only in the one between their addition and
    /// removal, annotations from `2025-03-26`, structured output from `2025-06-18`.
    #[test]
    fn what_each_revision_allows() {
        let batches: Vec<_> = Version::ALL.into_iter().filter(|v| v.allows_batches()).collect();
        assert_eq!(batches, [Version::V2025_03_26]);
        assert!(!Version::V2024_11_05.has_annotations() && Version::V2025_03_26.has_annotations());
        assert!(
            !Version::V2025_03_26.has_structured_output() && Version::V2025_06_18.has_structured_output()
        );
        assert_eq!(Version::listed()[0], serde_json::json!("2026-07-28"));
        assert_eq!(Version::listed().len(), 5);
    }
}
