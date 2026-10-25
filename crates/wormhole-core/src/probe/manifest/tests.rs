use super::*;

/// The example of `probes.md`, section 2.
pub(crate) const REFERENCE: &str = include_str!("../../../tests/probes/ok/acme-tools/probe.toml");

fn parse(text: &str) -> Parsed {
    Manifest::parse(text).unwrap_or_else(|err| panic!("{err}"))
}

const MINIMAL: &str = r#"
[probe]
id = "tiny"
name = "Tiny"
description = "d"
author = "a"
version = "0.1.0"
wormhole = ">=1.3"
api = 1
"#;

#[test]
fn the_reference_manifest_parses_without_a_warning() {
    let parsed = parse(REFERENCE);
    assert_eq!(parsed.warnings, []);
    let m = parsed.manifest;
    assert_eq!(m.probe.id, "acme-tools");
    assert_eq!(m.probe.api, API);
    assert_eq!(m.probe.main, None);
    assert_eq!(m.permissions.process, ["git", "./scripts/deploy.sh"]);
    assert!(m.permissions.notify);
    let c = m.contributes;
    assert_eq!(c.rules.len(), 4);
    assert_eq!(c.rules[0].look, Look::Shorten);
    assert_eq!(c.rules[0].reveal, Some(Reveal::Hover));
    assert_eq!(c.rules[1].look, Look::None, "look defaults to none");
    let action = c.rules[1].action.as_ref().expect("action");
    assert_eq!(action.kind, ActionKind::OpenUrl);
    assert_eq!(c.rules[3].action.as_ref().map(|a| a.kind), Some(ActionKind::Command));
    assert_eq!(c.commands[0].run_in, RunIn::NewTerminal);
    assert_eq!(c.commands[1].run_in, RunIn::Background);
    assert_eq!(c.commands[1].cwd, Cwd::Terminal);
    assert_eq!(c.status[0].interval, Some(30));
    assert_eq!(c.status[0].side, Side::Right);
    assert_eq!(c.agents[0].status, AgentStatus::Osc7501);
    assert_eq!(c.agents[0].executables, ["acme", "acme-agent"]);
}

#[test]
fn a_manifest_may_be_only_its_identity() {
    let m = parse(MINIMAL).manifest;
    assert_eq!(m.permissions, Permissions::default());
    assert_eq!(m.contributes, Contributes::default());
}

#[test]
fn unknown_keys_are_collected_with_their_place() {
    let text = format!(
        "{MINIMAL}\nflavour = 1\n[permissions]\nnotifiy = true\n\
         [[contributes.rules]]\nid = \"a\"\npattern = \"a\"\ncolour = \"red\"\naction = {{ type = \"none\", urll = \"x\" }}\n\
         [[contributes.rules]]\nid = \"b\"\npattern = \"b\"\nhoverr = \"x\"\n"
    );
    // `flavour` after `[probe]`'s keys belongs to `[probe]`.
    let paths: Vec<_> = parse(&text).warnings.into_iter().map(|w| w.path).collect();
    assert_eq!(
        paths,
        [
            "probe.flavour",
            "permissions.notifiy",
            "contributes.rules[0].colour",
            "contributes.rules[0].action.urll",
            "contributes.rules[1].hoverr",
        ]
    );
}

#[test]
fn an_unknown_table_is_a_warning_too() {
    let paths: Vec<_> =
        parse(&format!("{MINIMAL}\n[extras]\nx = 1\n")).warnings.into_iter().map(|w| w.path).collect();
    assert_eq!(paths, ["extras"]);
}

#[test]
fn a_value_outside_a_closed_set_is_an_error_with_its_line() {
    let text = format!("{MINIMAL}\n[[contributes.rules]]\nid = \"a\"\npattern = \"a\"\nlook = \"blur\"\n");
    let err = Manifest::parse(&text).expect_err("blur is not a look");
    assert!(err.message.contains("blur"), "{err}");
    assert_eq!(err.line, Some(14), "{err}");
}

#[test]
fn a_missing_required_key_is_an_error() {
    let err = Manifest::parse("[probe]\nid = \"tiny\"\n").expect_err("incomplete");
    assert!(err.message.contains("missing field"), "{err}");
    let err = Manifest::parse("not toml").expect_err("not toml");
    assert!(err.line.is_some());
}

#[test]
fn a_wrong_type_is_an_error() {
    let text = MINIMAL.replace("api = 1", "api = \"1\"");
    assert!(Manifest::parse(&text).is_err());
    let text = format!("{MINIMAL}\n[[contributes.status]]\nid = \"s\"\ninterval = -5\n");
    assert!(Manifest::parse(&text).is_err(), "a negative interval is not a number of seconds");
}

#[test]
fn the_version_range_decides_which_app_a_probe_supports() {
    let info = parse(MINIMAL).manifest.probe;
    assert!(info.supports(&Version::new(1, 3, 0)));
    assert!(info.supports(&Version::new(2, 0, 0)));
    assert!(!info.supports(&Version::new(1, 2, 9)));
    assert_eq!(info.parsed_version().ok(), Some(Version::new(0, 1, 0)));
}

#[test]
fn positions_count_lines_and_characters() {
    assert_eq!(position("ab\ncd", 0), (1, 1));
    assert_eq!(position("ab\ncd", 4), (2, 2));
    assert_eq!(position("é\nxé", 4), (2, 2));
}
