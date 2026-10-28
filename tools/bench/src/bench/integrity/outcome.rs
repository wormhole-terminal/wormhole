//! What the check found, as the results carry it: in the file, on the screen and in the comparison.

use std::cmp::Ordering;

use super::super::json::{self, Value};
use super::probe::BURST;
use super::{ENOUGH, SETTLE};

/// A test's input check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Check {
    /// Every checkpoint was answered, exactly once, before the cursor report that ended the run:
    /// `checkpoints` over all the measured runs.
    Verified { checkpoints: u64 },
    /// Fewer answers than questions, even a patient repeat later: the terminal lost input or withheld
    /// answers.
    Dropped { answered: u64, checkpoints: u64 },
    /// The check could not decide, and the number is shown without it.
    Unchecked(Why),
}

/// Why a check could not run or decide.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why {
    /// `--no-verify`.
    Off,
    /// The terminal did not answer DSR 5 when it was idle.
    Silent,
    /// Idle, it answered only `answered` of the [`BURST`] queries asked in a row: it cannot be counted
    /// on.
    Lossy { answered: u64 },
    /// More answers than questions: the count proves nothing.
    Surplus { answered: u64, checkpoints: u64 },
    /// All the checkpoints there were were answered, but there were fewer than [`ENOUGH`].
    TooFew { checkpoints: u64 },
    /// All the checkpoints were answered, but only `before_report` of them before the cursor report that
    /// ended the run: the time was taken before the terminal had got through what it was sent.
    OutOfOrder { before_report: u64, checkpoints: u64 },
    /// `answered` of `checkpoints` had been answered 3 s after the run (the first try's count); the run
    /// was repeated and then all of them were.
    Retried { answered: u64, checkpoints: u64 },
    /// A result file from a newer ttystorm says something this one does not know.
    Unknown,
}

impl Check {
    /// What the checkpoints of one run say: all answered, some missing, or too many. `in_time` is how
    /// many answers had arrived when the cursor report did: a terminal answers in order, so for one
    /// that does, the cursor report comes after all of them.
    pub fn judge(checkpoints: u64, answered: u64, in_time: u64) -> Check {
        match answered.cmp(&checkpoints) {
            Ordering::Less => Check::Dropped { answered, checkpoints },
            Ordering::Greater => Check::Unchecked(Why::Surplus { answered, checkpoints }),
            Ordering::Equal if in_time < checkpoints => {
                Check::Unchecked(Why::OutOfOrder { before_report: in_time, checkpoints })
            }
            Ordering::Equal if checkpoints < ENOUGH => Check::Unchecked(Why::TooFew { checkpoints }),
            Ordering::Equal => Check::Verified { checkpoints },
        }
    }

    /// The check of a test from its runs': dropped beats unchecked beats verified, and verified adds up.
    pub fn merge(runs: &[Check]) -> Option<Check> {
        if let Some(d) = runs.iter().find(|c| matches!(c, Check::Dropped { .. })) {
            return Some(d.clone());
        }
        if let Some(u) = runs.iter().find(|c| matches!(c, Check::Unchecked(_))) {
            return Some(u.clone());
        }
        let total: u64 = runs
            .iter()
            .map(|c| match c {
                Check::Verified { checkpoints } => *checkpoints,
                _ => 0,
            })
            .sum();
        (!runs.is_empty()).then_some(Check::Verified { checkpoints: total })
    }

    pub fn is_dropped(&self) -> bool {
        matches!(self, Check::Dropped { .. })
    }

    /// One short phrase for a result line, `None` when there is nothing to say.
    pub fn badge(&self) -> Option<String> {
        match self {
            Check::Verified { .. } => None,
            Check::Dropped { answered, checkpoints } => {
                Some(format!("dropped input ({answered} of {checkpoints} checkpoints answered)"))
            }
            Check::Unchecked(_) => Some("unverified".to_string()),
        }
    }

    /// The check as a results file has it. For `unchecked`, `checkpoints` and `answered` mean what the
    /// reason says (README, "Files"): the idle probe's burst for `lossy`, the answers in by the cursor
    /// report for `out_of_order`, the first try's counts for `retried`.
    pub fn to_json(&self) -> String {
        let q = json::quote;
        match self {
            Check::Verified { checkpoints } => {
                format!(
                    "{{\"status\": {}, \"checkpoints\": {checkpoints}, \"answered\": {checkpoints}}}",
                    q("verified")
                )
            }
            Check::Dropped { answered, checkpoints } => format!(
                "{{\"status\": {}, \"checkpoints\": {checkpoints}, \"answered\": {answered}}}",
                q("dropped")
            ),
            Check::Unchecked(why) => {
                let (reason, answered, checkpoints) = match why {
                    Why::Off => ("off", 0, 0),
                    Why::Silent => ("silent", 0, 0),
                    Why::Lossy { answered } => ("lossy", *answered, BURST),
                    Why::Surplus { answered, checkpoints } => ("surplus", *answered, *checkpoints),
                    Why::TooFew { checkpoints } => ("few", *checkpoints, *checkpoints),
                    Why::OutOfOrder { before_report, checkpoints } => {
                        ("out_of_order", *before_report, *checkpoints)
                    }
                    Why::Retried { answered, checkpoints } => ("retried", *answered, *checkpoints),
                    Why::Unknown => ("unknown", 0, 0),
                };
                format!(
                    "{{\"status\": {}, \"reason\": {}, \"checkpoints\": {checkpoints}, \"answered\": {answered}}}",
                    q("unchecked"),
                    q(reason)
                )
            }
        }
    }

    /// A test's check from a results file. `None` only when the test has none (a file from before
    /// the check, `roundtrip`); anything written there that this version cannot read is *unverified*,
    /// never a clean number.
    pub fn from_value(v: &Value) -> Option<Check> {
        if !matches!(v, Value::Obj(_)) {
            return None;
        }
        let n = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0) as u64;
        let (checkpoints, answered) = (n("checkpoints"), n("answered"));
        let unknown = Check::Unchecked(Why::Unknown);
        Some(match v.get("status").and_then(Value::as_str) {
            // Verified says what it counted; a file that claims it without enough to go on (a hand-written
            // one, or from a version with another idea of enough) is not taken at its word.
            Some("verified") if checkpoints >= ENOUGH && answered == checkpoints => {
                Check::Verified { checkpoints }
            }
            Some("dropped") => Check::Dropped { answered, checkpoints },
            Some("unchecked") => match v.get("reason").and_then(Value::as_str) {
                Some("off") => Check::Unchecked(Why::Off),
                Some("silent") => Check::Unchecked(Why::Silent),
                Some("lossy") => Check::Unchecked(Why::Lossy { answered }),
                Some("surplus") => Check::Unchecked(Why::Surplus { answered, checkpoints }),
                Some("few") => Check::Unchecked(Why::TooFew { checkpoints }),
                Some("out_of_order") => {
                    Check::Unchecked(Why::OutOfOrder { before_report: answered, checkpoints })
                }
                Some("retried") => Check::Unchecked(Why::Retried { answered, checkpoints }),
                _ => unknown,
            },
            _ => unknown,
        })
    }
}

impl Why {
    /// The reason without its numbers: what two tests have in common when they are unverified for the
    /// same reason, though the counts differ.
    pub fn kind(&self) -> &'static str {
        match self {
            Why::Off => "off",
            Why::Silent => "silent",
            Why::Lossy { .. } => "lossy",
            Why::Surplus { .. } => "surplus",
            Why::TooFew { .. } => "few",
            Why::OutOfOrder { .. } => "out_of_order",
            Why::Retried { .. } => "retried",
            Why::Unknown => "unknown",
        }
    }

    /// The reason in a sentence, for the result screen and `--details`.
    pub fn sentence(&self) -> String {
        match self {
            Why::Lossy { answered } => format!(
                "the terminal answered only {answered} of {BURST} DSR 5 queries sent in a row, even when idle"
            ),
            Why::Surplus { answered, checkpoints } => {
                format!("the terminal sent {answered} answers to {checkpoints} questions")
            }
            Why::TooFew { checkpoints } => format!(
                "the workload has room for only {checkpoints} checkpoints (it takes {ENOUGH}) and all were answered"
            ),
            Why::OutOfOrder { before_report, checkpoints } => format!(
                "all {checkpoints} checkpoints were answered, but only {before_report} before the cursor report that ended the run: the terminal answers from another thread or out of order, and the time may have ended before it had parsed everything"
            ),
            // A file from a dev build wrote this without counts.
            Why::Retried { answered, checkpoints } if *checkpoints > 0 => format!(
                "{answered} of {checkpoints} checkpoints were answered after {} s; the run was repeated and all were, so the terminal answers slowly or unevenly",
                SETTLE.as_secs()
            ),
            other => other.general(),
        }
    }

    /// The same without the counts, for several tests at once.
    pub fn general(&self) -> String {
        match self {
            Why::Off => "the input check was turned off (--no-verify)".to_string(),
            Why::Silent => "the terminal does not answer DSR 5 (ESC [ 5 n)".to_string(),
            Why::Lossy { .. } => {
                format!("the terminal answered only some of {BURST} DSR 5 queries sent in a row, even when idle")
            }
            Why::Surplus { .. } => "the terminal sent more answers than questions".to_string(),
            Why::TooFew { .. } => {
                format!("the workload has room for fewer than {ENOUGH} checkpoints and all were answered")
            }
            Why::OutOfOrder { .. } => "all checkpoints were answered, but not all before the cursor report that ended the run: the terminal answers from another thread or out of order, and the time may have ended before it had parsed everything".to_string(),
            Why::Retried { .. } => format!(
                "answers were missing after {} s; the run was repeated and all came, so the terminal answers slowly or unevenly",
                SETTLE.as_secs()
            ),
            Why::Unknown => {
                "the result file says something this version of ttystorm does not know".to_string()
            }
        }
    }
}

/// The line under the result screen's header: what the checks found over all the tests, `None`
/// when no test had one (all saved by an older ttystorm, or only `roundtrip`).
pub fn summary(checks: &[(&str, &Check)]) -> Option<(Level, String)> {
    let dropped: Vec<String> = checks
        .iter()
        .filter_map(|(n, c)| match c {
            Check::Dropped { answered, checkpoints } => {
                Some(format!("{n} ({answered} of {checkpoints} answered)"))
            }
            _ => None,
        })
        .collect();
    let unverified = unverified_text(checks);
    if !dropped.is_empty() {
        let mut text = format!(
            "Dropped input in {}: checkpoints went unanswered in a patient repeat of the run too, so the terminal lost input or withheld answers, and no number is given.",
            dropped.join(", ")
        );
        if let Some(rest) = unverified {
            text.push(' ');
            text.push_str(&rest);
        }
        return Some((Level::Bad, text));
    }
    if let Some(text) = unverified {
        return Some((Level::Warn, text));
    }
    let total: u64 = checks
        .iter()
        .map(|(_, c)| match c {
            Check::Verified { checkpoints } => *checkpoints,
            _ => 0,
        })
        .sum();
    (total > 0).then(|| {
        (
            Level::Good,
            format!("Input verified: all {total} checkpoints answered. See the README for what that proves."),
        )
    })
}

/// The tests that are not verified, a sentence for each kind of reason with the tests it applies to;
/// `None` when there are none. Tests with the same reason and other counts share a sentence, without
/// the counts.
fn unverified_text(checks: &[(&str, &Check)]) -> Option<String> {
    let mut groups: Vec<(&str, Vec<(&str, &Why)>)> = Vec::new();
    for (name, check) in checks {
        let Check::Unchecked(why) = check else { continue };
        match groups.iter_mut().find(|(kind, _)| *kind == why.kind()) {
            Some((_, members)) => members.push((name, why)),
            None => groups.push((why.kind(), vec![(name, why)])),
        }
    }
    if groups.is_empty() {
        return None;
    }
    let parts: Vec<String> = groups
        .iter()
        .map(|(_, members)| {
            let names: Vec<&str> = members.iter().map(|(n, _)| *n).collect();
            let which = if names.len() == checks.len() { "all tests".to_string() } else { names.join(", ") };
            let first = members[0].1;
            let same = members.iter().all(|(_, why)| *why == first);
            let reason = if same { first.sentence() } else { first.general() };
            format!("Not verified ({which}): {reason}.")
        })
        .collect();
    Some(format!("{} Unverified numbers rank only among themselves in `compare`.", parts.join(" ")))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Good,
    Warn,
    Bad,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_are_judged_against_the_questions() {
        assert_eq!(Check::judge(16, 16, 16), Check::Verified { checkpoints: 16 });
        assert_eq!(Check::judge(16, 3, 3), Check::Dropped { answered: 3, checkpoints: 16 });
        assert_eq!(
            Check::judge(16, 17, 17),
            Check::Unchecked(Why::Surplus { answered: 17, checkpoints: 16 })
        );
        assert_eq!(Check::judge(0, 0, 0), Check::Unchecked(Why::TooFew { checkpoints: 0 }));
        // Few, all answered: not verified. Few, one missing: still dropped.
        assert_eq!(Check::judge(15, 15, 15), Check::Unchecked(Why::TooFew { checkpoints: 15 }));
        assert_eq!(Check::judge(15, 14, 14), Check::Dropped { answered: 14, checkpoints: 15 });
    }

    #[test]
    fn answers_that_were_not_all_in_before_the_cursor_report_are_never_verified() {
        // All there, one behind the cursor report: unverified, and not dropped.
        assert_eq!(
            Check::judge(16, 16, 15),
            Check::Unchecked(Why::OutOfOrder { before_report: 15, checkpoints: 16 })
        );
        assert_eq!(
            Check::judge(48, 48, 0),
            Check::Unchecked(Why::OutOfOrder { before_report: 0, checkpoints: 48 })
        );
        // Missing ones are still dropped, whatever the order; too many is still no witness.
        assert_eq!(Check::judge(16, 10, 2), Check::Dropped { answered: 10, checkpoints: 16 });
        assert!(matches!(Check::judge(16, 20, 3), Check::Unchecked(Why::Surplus { .. })));
    }

    #[test]
    fn runs_merge_with_the_worst_first() {
        let ok = Check::Verified { checkpoints: 16 };
        let drop = Check::Dropped { answered: 1, checkpoints: 16 };
        let unk = Check::Unchecked(Why::Off);
        assert_eq!(Check::merge(&[ok.clone(), ok.clone()]), Some(Check::Verified { checkpoints: 32 }));
        assert_eq!(Check::merge(&[ok.clone(), unk.clone(), drop.clone()]), Some(drop));
        assert_eq!(Check::merge(&[ok, unk.clone()]), Some(unk));
        assert_eq!(Check::merge(&[]), None);
    }

    #[test]
    fn every_check_survives_the_file() {
        for c in [
            Check::Verified { checkpoints: 48 },
            Check::Dropped { answered: 2, checkpoints: 48 },
            Check::Unchecked(Why::Off),
            Check::Unchecked(Why::Silent),
            Check::Unchecked(Why::Lossy { answered: 5 }),
            Check::Unchecked(Why::Surplus { answered: 50, checkpoints: 48 }),
            Check::Unchecked(Why::TooFew { checkpoints: 9 }),
            Check::Unchecked(Why::OutOfOrder { before_report: 40, checkpoints: 48 }),
            Check::Unchecked(Why::Retried { answered: 31, checkpoints: 48 }),
            Check::Unchecked(Why::Unknown),
        ] {
            let v = json::parse(&c.to_json()).expect("json");
            assert_eq!(Check::from_value(&v), Some(c.clone()), "{c:?}");
        }
    }

    #[test]
    fn the_summary_says_the_worst_thing() {
        let ok = Check::Verified { checkpoints: 16 };
        let drop = Check::Dropped { answered: 1, checkpoints: 16 };
        let unk = Check::Unchecked(Why::Silent);
        let (level, text) = summary(&[("a", &ok), ("b", &drop), ("c", &unk)]).expect("summary");
        assert_eq!(level, Level::Bad);
        assert!(text.contains(" b (1 of 16 answered):"), "{text}");
        assert_eq!(summary(&[("a", &ok), ("c", &unk)]).map(|s| s.0), Some(Level::Warn));
        let (level, text) = summary(&[("a", &ok), ("b", &ok)]).expect("summary");
        assert_eq!(level, Level::Good);
        assert!(text.contains("32"), "{text}");
        assert!(summary(&[]).is_none());
    }

    #[test]
    fn what_this_version_cannot_read_is_unverified_and_never_a_clean_number() {
        let read = |text: &str| Check::from_value(&json::parse(text).expect("json"));
        assert_eq!(read("{\"status\": \"checksummed\"}"), Some(Check::Unchecked(Why::Unknown)));
        assert_eq!(
            read("{\"status\": \"unchecked\", \"reason\": \"cosmic rays\"}"),
            Some(Check::Unchecked(Why::Unknown))
        );
        assert_eq!(read("{}"), Some(Check::Unchecked(Why::Unknown)));
        // Nothing there is a file from before the check.
        assert_eq!(read("null"), None);
        // A "verified" that doesn't say what it counted, or counted too little, is not taken at its word.
        let unknown = Some(Check::Unchecked(Why::Unknown));
        assert_eq!(read("{\"status\": \"verified\"}"), unknown);
        assert_eq!(read("{\"status\": \"verified\", \"checkpoints\": 15, \"answered\": 15}"), unknown);
        assert_eq!(read("{\"status\": \"verified\", \"checkpoints\": 48, \"answered\": 40}"), unknown);
        assert_eq!(
            read("{\"status\": \"verified\", \"checkpoints\": 16, \"answered\": 16}"),
            Some(Check::Verified { checkpoints: 16 })
        );
    }

    #[test]
    fn the_summary_names_the_tests_that_are_not_verified() {
        let ok = Check::Verified { checkpoints: 16 };
        let few = Check::Unchecked(Why::TooFew { checkpoints: 9 });
        let (_, text) = summary(&[("logflood", &ok), ("glyphstorm", &few)]).expect("summary");
        assert!(text.starts_with("Not verified (glyphstorm)"), "{text}");
        let (_, text) = summary(&[("glyphstorm", &few)]).expect("summary");
        assert!(text.starts_with("Not verified (all tests)"), "{text}");
    }

    #[test]
    fn each_reason_is_given_with_its_own_tests() {
        let ok = Check::Verified { checkpoints: 16 };
        let few = Check::Unchecked(Why::TooFew { checkpoints: 9 });
        let late = Check::Unchecked(Why::OutOfOrder { before_report: 3, checkpoints: 40 });
        let (level, text) =
            summary(&[("logflood", &ok), ("glyphstorm", &few), ("buildlog", &late), ("editstorm", &few)])
                .expect("summary");
        assert_eq!(level, Level::Warn);
        assert!(
            text.contains("Not verified (glyphstorm, editstorm): the workload has room for only 9"),
            "{text}"
        );
        assert!(
            text.contains("Not verified (buildlog): all 40 checkpoints were answered, but only 3"),
            "{text}"
        );
        assert_eq!(text.matches("rank only among themselves").count(), 1, "{text}");
    }

    #[test]
    fn the_same_reason_with_other_counts_is_one_group_without_the_counts() {
        let a = Check::Unchecked(Why::TooFew { checkpoints: 9 });
        let b = Check::Unchecked(Why::TooFew { checkpoints: 12 });
        let (_, text) = summary(&[("glyphstorm", &a), ("editstorm", &b)]).expect("summary");
        assert_eq!(text.matches("Not verified").count(), 1, "{text}");
        assert!(
            text.contains("Not verified (all tests): the workload has room for fewer than 16 checkpoints"),
            "{text}"
        );
        // Equal counts keep them.
        let (_, text) = summary(&[("glyphstorm", &a), ("editstorm", &a)]).expect("summary");
        assert!(text.contains("room for only 9 checkpoints"), "{text}");
    }

    #[test]
    fn a_retry_from_a_dev_file_without_counts_does_not_say_0_of_0() {
        let text = Why::Retried { answered: 0, checkpoints: 0 }.sentence();
        assert!(!text.contains("0 of 0") && text.contains("after 3 s"), "{text}");
        let text = Why::Retried { answered: 31, checkpoints: 48 }.sentence();
        assert!(text.starts_with("31 of 48 checkpoints were answered after 3 s"), "{text}");
    }

    #[test]
    fn a_dropped_test_does_not_hide_the_unverified_ones() {
        let drop = Check::Dropped { answered: 20, checkpoints: 60 };
        let off = Check::Unchecked(Why::Silent);
        let (level, text) = summary(&[("logflood", &drop), ("buildlog", &off)]).expect("summary");
        assert_eq!(level, Level::Bad);
        assert!(text.starts_with("Dropped input in logflood (20 of 60 answered):"), "{text}");
        assert!(
            text.contains("lost input or withheld answers") && text.contains("Not verified (buildlog)"),
            "{text}"
        );
    }
}
