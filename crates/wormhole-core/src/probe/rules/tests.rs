use super::*;
use crate::probe::manifest::{Look, Manifest};

const HEAD: &str = "[probe]\nid = \"tiny\"\nname = \"Tiny\"\ndescription = \"d\"\nauthor = \"a\"\n\
                    version = \"0.1.0\"\nwormhole = \">=1.3\"\napi = 1\n";

fn rules(rest: &str) -> CompiledRules {
    let text = format!("{HEAD}{rest}");
    let parsed = Manifest::parse(&text).unwrap_or_else(|err| panic!("{err}\n{text}"));
    CompiledRules::compile(&parsed.manifest).unwrap_or_else(|err| panic!("{err}\n{text}"))
}

/// The text each match covers.
fn covered<'a>(found: &[Match], line: &'a str) -> Vec<&'a str> {
    found.iter().map(|m| &line[m.range()]).collect()
}

fn rule(id: &str, pattern: &str, extra: &str) -> String {
    format!("[[contributes.rules]]\nid = \"{id}\"\npattern = '{pattern}'\n{extra}\n")
}

#[test]
fn a_rule_finds_its_matches_in_order() {
    let r = rules(&rule("n", "[0-9]+", ""));
    let line = "a 12 b 345 c 6";
    let found = matches(&r, line);
    assert_eq!(covered(&found, line), ["12", "345", "6"]);
    assert!(found.iter().all(|m| m.rule == 0 && m.look == Look::None));
}

#[test]
fn a_line_nothing_matches_gives_nothing() {
    assert!(matches(&rules(&rule("n", "[0-9]+", "")), "no digits").is_empty());
    assert!(matches(&rules(""), "anything").is_empty());
}

#[test]
fn a_rule_finds_at_most_eight_matches_in_a_line() {
    let r = rules(&rule("n", "x", ""));
    assert_eq!(matches(&r, &"x ".repeat(20)).len(), MAX_MATCHES_PER_RULE);
    assert_eq!(matches(&r, &"x ".repeat(MAX_MATCHES_PER_RULE)).len(), MAX_MATCHES_PER_RULE);
    assert_eq!(matches(&r, &"x ".repeat(3)).len(), 3);
}

#[test]
fn the_cap_is_per_rule() {
    let r = rules(&format!("{}{}", rule("a", "x", ""), rule("b", "y", "")));
    let found = matches(&r, &"x y ".repeat(20));
    assert_eq!(found.iter().filter(|m| m.rule == 0).count(), MAX_MATCHES_PER_RULE);
    assert_eq!(found.iter().filter(|m| m.rule == 1).count(), MAX_MATCHES_PER_RULE);
}

#[test]
fn only_two_kib_of_a_line_are_scanned() {
    let r = rules(&rule("w", "needle", ""));
    let before = format!("{}needle", "a".repeat(MAX_SCAN_BYTES - 6));
    assert_eq!(matches(&r, &before).len(), 1, "ends exactly at the cap");
    let straddling = format!("{}needle", "a".repeat(MAX_SCAN_BYTES - 3));
    assert!(matches(&r, &straddling).is_empty(), "cut by the cap");
    let beyond = format!("{}needle", "a".repeat(MAX_SCAN_BYTES + 10));
    assert!(matches(&r, &beyond).is_empty(), "past the cap");
}

#[test]
fn the_scan_cuts_at_a_character_boundary() {
    let r = rules(&rule("e", "é+", ""));
    // Two-byte characters across the cap: a cut in the middle of one must not panic or match half.
    let line = "é".repeat(MAX_SCAN_BYTES);
    let found = matches(&r, &line);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].end, MAX_SCAN_BYTES);
    let odd = format!("a{}", "é".repeat(MAX_SCAN_BYTES));
    assert_eq!(matches(&r, &odd)[0].end, MAX_SCAN_BYTES - 1, "a character cut by the cap is left out");
}

#[test]
fn the_leftmost_of_two_overlapping_matches_wins() {
    let r = rules(&format!("{}{}", rule("a", "abc", ""), rule("b", "bcd", "")));
    let found = matches(&r, "abcd");
    assert_eq!(covered(&found, "abcd"), ["abc"]);
    assert_eq!(found[0].rule, 0);
    // Listed the other way round, the left one still wins.
    let r = rules(&format!("{}{}", rule("b", "bcd", ""), rule("a", "abc", "")));
    assert_eq!(covered(&matches(&r, "abcd"), "abcd"), ["abc"]);
}

#[test]
fn of_two_matches_from_the_same_place_the_longer_wins() {
    let r = rules(&format!("{}{}", rule("short", "ab", ""), rule("long", "abcd", "")));
    let found = matches(&r, "abcd");
    assert_eq!(covered(&found, "abcd"), ["abcd"]);
    assert_eq!(found[0].rule, 1);
}

#[test]
fn of_two_equal_spans_the_earlier_rule_wins() {
    let r =
        rules(&format!("{}{}", rule("one", "ab", "tone = \"link\""), rule("two", "ab", "tone = \"muted\"")));
    let found = matches(&r, "ab");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rule, 0);
}

#[test]
fn matches_that_do_not_touch_all_stay() {
    let r = rules(&format!("{}{}", rule("a", "ab", ""), rule("b", "cd", "")));
    assert_eq!(covered(&matches(&r, "ab cd"), "ab cd"), ["ab", "cd"]);
    assert_eq!(covered(&matches(&r, "abcd"), "abcd"), ["ab", "cd"], "touching is not overlapping");
}

#[test]
fn a_redact_span_beats_a_longer_span_that_starts_earlier() {
    let r = rules(&format!(
        "{}{}",
        rule("wide", "key: sk-[a-z0-9]+", "tone = \"warning\""),
        rule("secret", "sk-[a-z0-9]+", "look = \"redact\"")
    ));
    let line = "key: sk-abc123";
    let found = matches(&r, line);
    assert_eq!(covered(&found, line), ["sk-abc123"]);
    assert_eq!(found[0].look, Look::Redact);
}

#[test]
fn a_redact_span_beats_an_equal_span_of_an_earlier_rule() {
    let r = rules(&format!(
        "{}{}",
        rule("plain", "sk-[a-z0-9]+", "look = \"shorten\"\nkeep = \"start:2\""),
        rule("secret", "sk-[a-z0-9]+", "look = \"redact\"")
    ));
    let found = matches(&r, "sk-abc123");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rule, 1);
}

#[test]
fn redact_spans_settle_among_themselves_leftmost_longest() {
    let r = rules(&format!(
        "{}{}",
        rule("a", "abc", "look = \"redact\""),
        rule("b", "bcde", "look = \"redact\"")
    ));
    assert_eq!(covered(&matches(&r, "abcde"), "abcde"), ["abc"]);
}

#[test]
fn a_redacted_span_hides_the_matches_inside_it() {
    let r = rules(&format!(
        "{}{}",
        rule("num", "[0-9]+", ""),
        rule("secret", "sk-[0-9]+-end", "look = \"redact\"")
    ));
    let line = "sk-123-end 456";
    assert_eq!(covered(&matches(&r, line), line), ["sk-123-end", "456"]);
}

#[test]
fn a_rule_whose_prefilter_could_not_be_built_still_matches() {
    let mut r = rules(&rule("n", "[0-9]+", ""));
    r.prefilter = None;
    assert_eq!(matches(&r, "a 1").len(), 1);
}

#[test]
fn compiling_names_the_rule_that_fails() {
    let text = format!("{HEAD}{}{}", rule("ok", "a", ""), rule("bad", "(a", ""));
    let parsed = Manifest::parse(&text).expect("parses");
    let err = CompiledRules::compile(&parsed.manifest).expect_err("fails");
    assert_eq!(err.index, 1);
    assert!(err.to_string().starts_with("contributes.rules[1].pattern"), "{err}");
}

#[test]
fn a_pattern_over_the_size_limit_does_not_compile() {
    let text = format!("{HEAD}{}", rule("big", r"(\w{40}){40}", ""));
    let parsed = Manifest::parse(&text).expect("parses");
    assert!(CompiledRules::compile(&parsed.manifest).is_err());
}

#[test]
fn too_many_rules_do_not_compile() {
    let many: String = (0..65).map(|i| rule(&format!("r{i}"), "a", "")).collect();
    let parsed = Manifest::parse(&format!("{HEAD}{many}")).expect("parses");
    assert!(CompiledRules::compile(&parsed.manifest).is_err());
}

// ---- targets -------------------------------------------------------------------------------

const JIRA: &str = "[permissions]\nopen = [\"acme.atlassian.net\"]\n";

fn jira(url: &str) -> CompiledRules {
    rules(&format!(
        "{JIRA}{}",
        rule("t", r"ACME-([^ ]+)", &format!("action = {{ type = \"open-url\", url = \"{url}\" }}"))
    ))
}

fn target_of(r: &CompiledRules, line: &str) -> Option<Target> {
    let found = matches(r, line);
    target(r, found.first().expect("a match"), line)
}

#[test]
fn an_open_url_target_has_its_captures_in_place() {
    let r = jira("https://acme.atlassian.net/browse/ACME-{1}");
    assert_eq!(
        target_of(&r, "see ACME-123 now"),
        Some(Target::OpenUrl("https://acme.atlassian.net/browse/ACME-123".into()))
    );
}

#[test]
fn every_capture_is_percent_encoded_so_it_cannot_change_the_target() {
    let r = jira("https://acme.atlassian.net/browse/{1}");
    for (capture, encoded) in [
        ("1?x=1#f", "1%3Fx%3D1%23f"),
        ("../../admin", "..%2F..%2Fadmin"),
        ("a/b", "a%2Fb"),
        ("\u{e9}\u{20ac}", "%C3%A9%E2%82%AC"),
        ("%2e", "%252e"),
        ("a@evil.example", "a%40evil.example"),
        ("A-z_0.9~", "A-z_0.9~"),
    ] {
        assert_eq!(
            target_of(&r, &format!("ACME-{capture}")),
            Some(Target::OpenUrl(format!("https://acme.atlassian.net/browse/{encoded}"))),
            "{capture}"
        );
    }
}

#[test]
fn a_capture_that_is_a_dot_segment_leaves_no_target() {
    let r = jira("https://acme.atlassian.net/browse/{1}");
    for capture in ["..", ".", "%2e", "%2E%2e"] {
        // `%2e` is encoded to `%252e`, which stays text: only the bare dots walk a path.
        let target = target_of(&r, &format!("ACME-{capture}"));
        let walks = capture == ".." || capture == ".";
        assert_eq!(target.is_none(), walks, "{capture}: {target:?}");
    }
    // Neither is there one when two captures meet in one segment.
    let r = rules(&format!(
        "{JIRA}{}",
        rule(
            "t",
            r"(\.)(\.)",
            "action = { type = \"open-url\", url = \"https://acme.atlassian.net/{1}{2}/issues\" }"
        )
    ));
    assert_eq!(target_of(&r, "a..b"), None);
    // One dot is a name, not a segment of the path walked up.
    let r = rules(&format!(
        "{JIRA}{}",
        rule(
            "t",
            r"(\.)(\.)",
            "action = { type = \"open-url\", url = \"https://acme.atlassian.net/{1}x{2}\" }"
        )
    ));
    assert!(target_of(&r, "a..b").is_some());
    // The query and the fragment are not the path.
    let r = jira("https://acme.atlassian.net/search?q={1}#{1}");
    assert!(target_of(&r, "ACME-..").is_some());
}

#[test]
fn the_template_around_a_capture_is_not_encoded() {
    let r = jira("https://acme.atlassian.net/search?q={0}&n={1}");
    assert_eq!(
        target_of(&r, "ACME-1 2"),
        Some(Target::OpenUrl("https://acme.atlassian.net/search?q=ACME-1&n=1".into()))
    );
}

#[test]
fn a_group_that_did_not_take_part_is_empty_in_the_link() {
    let r = rules(&format!(
        "{JIRA}{}",
        rule("t", "A(B)?C", "action = { type = \"open-url\", url = \"https://acme.atlassian.net/{1}x\" }")
    ));
    assert_eq!(target_of(&r, "AC"), Some(Target::OpenUrl("https://acme.atlassian.net/x".into())));
}

#[test]
fn a_copy_target_is_raw_text_or_the_whole_match() {
    let r = rules(&format!(
        "{}{}",
        rule("t", "k=([a-z]+)", "action = { type = \"copy\", text = \"<{1}>\" }"),
        rule("u", "id[0-9]+", "action = { type = \"copy\" }")
    ));
    assert_eq!(target_of(&r, "k=a/b"), Some(Target::Copy("<a>".into())));
    assert_eq!(target_of(&r, "id42"), Some(Target::Copy("id42".into())));
}

#[test]
fn a_rule_without_an_action_has_no_target() {
    let r = rules(&rule("n", "x", "tone = \"link\""));
    assert_eq!(target_of(&r, "x"), None);
    let r = rules(&rule("n", "x", "action = { type = \"none\" }"));
    assert_eq!(target_of(&r, "x"), None);
}

const SHOW: &str = "[permissions]\nprocess = [\"git\"]\n\
    [[contributes.commands]]\nid = \"show\"\ntitle = \"Show\"\nrun = [\"git\", \"show\", \"--stat\", \"$1\"]\n\
    in = \"background\"\n";

#[test]
fn a_command_target_names_the_command_and_the_values_of_its_arguments() {
    let r = rules(&format!(
        "{SHOW}{}",
        rule("sha", "([0-9a-f][0-9a-f]{6,39})", "action = { type = \"command\", command = \"show\" }")
    ));
    assert_eq!(
        target_of(&r, "at abc1234 ok"),
        Some(Target::Command { id: "show".into(), args: vec!["abc1234".into()] })
    );
}

#[test]
fn a_command_is_refused_a_capture_that_starts_with_a_dash() {
    // Not a pattern `validate` accepts; the match must still not become an option for the program.
    let r = rules(&format!(
        "{SHOW}{}",
        rule("sha", "x(.+)", "action = { type = \"command\", command = \"show\" }")
    ));
    assert_eq!(target_of(&r, "x--upload-pack=evil"), None);
    assert!(target_of(&r, "xabc").is_some());
}

#[test]
fn the_target_uses_the_context_the_match_was_found_in() {
    let r = rules(&format!(
        "{JIRA}{}",
        rule(
            "w",
            r"(?-u:\b)(ACME-[0-9]+)",
            "action = { type = \"open-url\", url = \"https://acme.atlassian.net/{1}\" }"
        )
    ));
    // `XACME-1` has no boundary before ACME, so only the second one is a match.
    let line = "XACME-1 ACME-2";
    let found = matches(&r, line);
    assert_eq!(found.len(), 1);
    assert_eq!(
        target(&r, &found[0], line),
        Some(Target::OpenUrl("https://acme.atlassian.net/ACME-2".into()))
    );
}

#[test]
fn percent_encoding_keeps_unreserved_characters() {
    assert_eq!(target::percent_encode("AZaz09-._~"), "AZaz09-._~");
    assert_eq!(target::percent_encode(" /?#&=+%"), "%20%2F%3F%23%26%3D%2B%25");
    assert_eq!(target::percent_encode(""), "");
}
