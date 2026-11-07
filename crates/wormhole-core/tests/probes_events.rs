//! The hook vocabulary (`probe::events`) against the bus it is mapped onto (`ipc::control::event`).
//!
//! `probe` sits below `ipc` and cannot import it, so the names are text there and this test is the
//! one place that holds them to the real ones: a topic or event that is renamed on the bus, or a state
//! that is added, fails here instead of quietly never firing.

use wormhole_core::ipc::control::TerminalState;
use wormhole_core::ipc::control::event::{BlockPhase, Body, Topic};
use wormhole_core::probe::events::{self, STATES, Source};

/// One of each body a hook reads, with every optional field present.
fn samples() -> Vec<Body> {
    vec![
        Body::Status {
            status: TerminalState::NeedsYou,
            was: TerminalState::Working,
            agent: Some("claude".into()),
            message: Some("m".into()),
        },
        Body::Title { title: Some("t".into()), name: "n".into() },
        Body::Folder { folder: Some("/f".into()) },
        Body::Bell,
        Body::Block {
            block: 1,
            phase: BlockPhase::Finished,
            exit_code: Some(0),
            took_ms: Some(1),
            command: Some("c".into()),
        },
        Body::TerminalOpened { folder: Some("/f".into()) },
        Body::TerminalClosed,
        Body::Output { screen: "main".into(), lines: vec![], partial: None, skipped: 0, lost: false },
    ]
}

#[test]
fn every_bus_event_of_the_table_is_a_real_topic_and_body() {
    let samples = samples();
    for spec in events::all() {
        let Source::Bus { topic, body, phase } = spec.source else { continue };
        let real =
            Topic::from_name(topic).unwrap_or_else(|| panic!("{}: no topic `{topic}`", spec.event.name()));
        let sample = samples
            .iter()
            .find(|b| b.name() == body)
            .unwrap_or_else(|| panic!("{}: the bus has no event `{body}`", spec.event.name()));
        assert_eq!(sample.topic(), Some(real), "{}", spec.event.name());
        if let Some(phase) = phase {
            let written: Vec<_> = [BlockPhase::Started, BlockPhase::Finished]
                .into_iter()
                .map(|p| serde_json::to_value(p).expect("serializes"))
                .collect();
            assert!(
                written.iter().any(|w| w == phase),
                "{}: the bus has no phase `{phase}`",
                spec.event.name()
            );
        }
    }
}

#[test]
fn the_fields_a_hook_reads_are_the_bus_s_or_worked_out_by_the_app() {
    let samples = samples();
    // What the app works out when it tells the event, so the bus has no field of the name.
    let worked_out = [
        "terminal",
        "block",
        "output_lines",
        "line_no",
        "line",
        "branch",
        "ahead",
        "behind",
        "dirty",
        "staged",
        "untracked",
    ];
    for spec in events::all() {
        let Source::Bus { body, .. } = spec.source else { continue };
        let sample = samples.iter().find(|b| b.name() == body).expect("sample");
        let json = serde_json::to_value(sample).expect("serializes");
        for field in spec.fields {
            // The bus says `exit_code`; a hook says `exit`.
            let wire = if field.name == "exit" { "exit_code" } else { field.name };
            assert!(
                json.get(wire).is_some() || worked_out.contains(&field.name),
                "{}: the bus has no `{wire}`: {json}",
                spec.event.name()
            );
        }
    }
}

#[test]
fn the_states_are_the_bus_s_states() {
    let wire: Vec<String> = [
        TerminalState::Idle,
        TerminalState::Working,
        TerminalState::NeedsYou,
        TerminalState::Finished,
        TerminalState::Failed,
        TerminalState::Exited,
    ]
    .into_iter()
    .map(|s| serde_json::to_value(s).expect("serializes").as_str().expect("a string").to_string())
    .collect();
    assert_eq!(wire, STATES);
}

#[test]
fn the_only_event_without_a_topic_is_the_git_watch() {
    let without: Vec<_> =
        events::all().iter().filter(|s| s.source == Source::GitWatch).map(|s| s.event.name()).collect();
    assert_eq!(without, ["git.changed"]);
}
