//! The feed's optional `security` key: which updates fix a security problem.
//!
//! The feed is not signed (see the trust model in [`super`]), so this only changes how an update
//! is *presented and offered*: its wording, and that "Skip this version" does not apply. It never
//! reaches the download, the signature check or the install. For the same reason nothing in it may
//! make a feed fail to parse: an update must stay installable whatever this key holds.

use serde_json::Value;

/// More advisories than this are not a list anyone reads; the rest are dropped.
pub const MAX_ADVISORIES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    /// The key is there but names no severity we know (a newer feed, a typo). Still a security
    /// update: dropping the flag would let a skipped version stay skipped.
    Unknown,
}

impl Severity {
    /// Case-insensitive; anything else is [`Severity::Unknown`].
    pub fn parse(text: &str) -> Self {
        match text.trim().to_ascii_lowercase().as_str() {
            "critical" => Self::Critical,
            "high" => Self::High,
            "medium" | "moderate" => Self::Medium,
            "low" => Self::Low,
            _ => Self::Unknown,
        }
    }
}

/// What the feed says about a release's security fixes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Security {
    pub severity: Severity,
    /// GitHub advisory IDs (`ghsa-xxxx-xxxx-xxxx`, lowercase), valid ones only, at most
    /// [`MAX_ADVISORIES`], without duplicates. Each becomes part of a link, so the format is strict.
    pub advisories: Vec<String>,
}

impl Security {
    /// Reads the key leniently. `null` and `false` mean "not a security update"; any other value
    /// that is not an object is still a (severity unknown) security update, and an object's bad
    /// parts are dropped one by one.
    pub(super) fn from_feed(value: Option<&Value>) -> Option<Self> {
        match value? {
            Value::Null | Value::Bool(false) => None,
            Value::Object(map) => Some(Self {
                severity: map
                    .get("severity")
                    .and_then(Value::as_str)
                    .map_or(Severity::Unknown, Severity::parse),
                advisories: map.get("advisories").and_then(Value::as_array).map_or_else(Vec::new, |list| {
                    let mut ids: Vec<String> = Vec::new();
                    for id in list.iter().filter_map(Value::as_str).filter_map(normalize_advisory) {
                        if !ids.contains(&id) {
                            ids.push(id);
                        }
                        if ids.len() == MAX_ADVISORIES {
                            break;
                        }
                    }
                    ids
                }),
            }),
            _ => Some(Self { severity: Severity::Unknown, advisories: Vec::new() }),
        }
    }
}

/// `GHSA-` and three groups of four characters from GitHub's alphabet (digits 2-9 and
/// `cfghjmpqrvwx`), lowercased; `None` for anything else.
fn normalize_advisory(id: &str) -> Option<String> {
    let id = id.trim().to_ascii_lowercase();
    let groups = id.strip_prefix("ghsa-")?.split('-').collect::<Vec<_>>();
    let valid = groups.len() == 3
        && groups.iter().all(|g| g.len() == 4 && g.bytes().all(|b| b"23456789cfghjmpqrvwx".contains(&b)));
    valid.then_some(id)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{MAX_ADVISORIES, Security, Severity};

    fn parse(value: serde_json::Value) -> Option<Security> {
        Security::from_feed(Some(&value))
    }

    #[test]
    fn absent_null_and_false_are_ordinary_releases() {
        assert_eq!(Security::from_feed(None), None);
        assert_eq!(parse(json!(null)), None);
        assert_eq!(parse(json!(false)), None);
    }

    #[test]
    fn severities_parse_case_insensitively_and_unknown_ones_still_count() {
        for (text, want) in [
            ("critical", Severity::Critical),
            (" High ", Severity::High),
            ("MEDIUM", Severity::Medium),
            ("low", Severity::Low),
            ("catastrophic", Severity::Unknown),
            ("", Severity::Unknown),
        ] {
            let security = parse(json!({ "severity": text })).unwrap();
            assert_eq!(security.severity, want, "{text:?}");
        }
        assert_eq!(parse(json!({ "severity": 7 })).unwrap().severity, Severity::Unknown);
        assert_eq!(parse(json!({})).unwrap().severity, Severity::Unknown);
        assert_eq!(parse(json!(true)).unwrap().severity, Severity::Unknown);
        assert_eq!(parse(json!("high")).unwrap().severity, Severity::Unknown);
    }

    #[test]
    fn junk_advisories_are_dropped_and_good_ones_normalized() {
        let security = parse(json!({
            "severity": "high",
            "advisories": [
                "GHSA-cfgh-2345-rvwx",
                "ghsa-cfgh-2345-rvwx",
                "ghsa-2222-3333-4444",
                "GHSA-aaaa-bbbb-cccc",
                "ghsa-abcd-2345",
                "ghsa-cfgh-2345-rvwx-2222",
                "CVE-2026-12345",
                "ghsa-cfgh-2345-wxr/",
                "ghsa-cfgh-2345-rvwx/../x",
                "https://evil.example/ghsa-cfgh-2345-rvwx",
                17,
                null
            ]
        }))
        .unwrap();
        assert_eq!(security.advisories, ["ghsa-cfgh-2345-rvwx", "ghsa-2222-3333-4444"]);
        assert_eq!(parse(json!({ "advisories": "ghsa-cfgh-2345-rvwx" })).unwrap().advisories.len(), 0);
    }

    #[test]
    fn the_advisory_list_is_capped() {
        let many: Vec<String> =
            (0..40).map(|i| format!("ghsa-2222-3333-{}{}22", 2 + i / 8, 2 + i % 8)).collect();
        let security = parse(json!({ "severity": "low", "advisories": many })).unwrap();
        assert_eq!(security.advisories.len(), MAX_ADVISORIES);
    }
}
