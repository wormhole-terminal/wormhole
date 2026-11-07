use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, SystemTime};

use wormhole_core::agents::AgentId;
use wormhole_core::ipc::Decision;
use wormhole_core::ipc::access::{Capability, Identity, Kind, Need, ProcessRef, Program, Resolved};
use wormhole_core::platform::codesign::{Signature, SignerKind};
use wormhole_core::storage::store::Store;

use crate::aux_backends::common::test_support::{Wakes, scratch};
use crate::aux_backends::{AccessAnswer, AccessCapability, AccessDesk, AccessKindRecord, AccessSignerKind};

const TYPE: Need = Need::Capability(Capability::Type);

fn agent(pin: &str) -> Resolved {
    Resolved {
        identity: Identity { kind: Kind::AdHoc, agent: Some(AgentId::Claude), pin: pin.into(), ..Identity::scripts() },
        label: "Claude Code".into(),
        process: Some(ProcessRef { pid: u32::MAX, started: SystemTime::UNIX_EPOCH }),
        program: Some(Program {
            pid: u32::MAX,
            path: "/opt/tools/claude".into(),
            signature: Signature::of(SignerKind::AdHoc),
            interpreter: None,
            started: SystemTime::UNIX_EPOCH,
        }),
        elevated: false,
        started_by: Some("zsh".into()),
        unusual_signer: true,
    }
}

fn scripts() -> Resolved {
    Resolved {
        identity: Identity::scripts(),
        label: "Scripts".into(),
        process: None,
        program: None,
        elevated: false,
        started_by: None,
        unusual_signer: false,
    }
}

fn desk() -> (tempfile::TempDir, std::path::PathBuf, std::sync::Arc<AccessDesk>, std::sync::Arc<Wakes>) {
    let (root, data) = scratch();
    let wakes = Wakes::new();
    let desk = AccessDesk::new(None, Some(data.display().to_string()), wakes.clone()).expect("a desk");
    (root, data, desk, wakes)
}

fn waiter() -> (std::sync::mpsc::SyncSender<Decision>, Receiver<Decision>) {
    mpsc::sync_channel(4)
}

fn settled(rx: &Receiver<Decision>) -> Decision {
    loop {
        match rx.recv_timeout(Duration::from_secs(5)).expect("an answer") {
            Decision::Shown => continue,
            other => return other,
        }
    }
}

#[test]
fn a_question_carries_the_programs_facts_and_one_is_on_screen_at_a_time() {
    let (_root, _data, desk, wakes) = desk();
    let first = desk.ask(agent("a"), TYPE, None, None).expect("room");
    let second = desk.ask(agent("b"), TYPE, None, None).expect("room");
    assert!(wakes.count() >= 1);
    assert_eq!(desk.waiting(), 2);

    let shown = desk.next_question().expect("the first");
    assert_eq!(shown.id, first);
    assert_eq!((shown.label.as_str(), shown.capability, shown.kind), ("Claude Code", AccessCapability::Type, AccessKindRecord::AdHoc));
    assert!(shown.unusual_signer && shown.can_be_remembered && shown.about_terminal.is_none());
    let program = shown.program.expect("a program");
    assert_eq!((program.path.as_str(), program.signer, program.started_by.as_deref()), ("/opt/tools/claude", AccessSignerKind::AdHoc, Some("zsh")));
    assert!(desk.next_question().is_none(), "the first is still up");

    desk.answer(first, AccessAnswer::Deny);
    assert_eq!(desk.next_question().expect("the second").id, second);
}

#[test]
fn a_program_that_asks_again_joins_its_open_question_and_all_are_answered() {
    let (_root, _data, desk, _) = desk();
    let (first_tx, first_rx) = waiter();
    let (second_tx, second_rx) = waiter();
    let id = desk.ask(agent("a"), TYPE, None, Some(first_tx)).expect("room");
    assert_eq!(desk.ask(agent("a"), TYPE, None, Some(second_tx)).expect("joins"), id);
    assert_eq!(desk.waiting(), 1);
    desk.answer(id, AccessAnswer::UntilQuit);
    assert!(matches!(settled(&first_rx), Decision::Retry));
    assert!(matches!(settled(&second_rx), Decision::Retry));
}

#[test]
fn the_answers_are_remembered_as_long_as_the_button_says() {
    let (_root, data, desk, _) = desk();

    let (tx, rx) = waiter();
    let id = desk.ask(agent("session"), TYPE, None, Some(tx)).expect("room");
    desk.answer(id, AccessAnswer::Session);
    assert!(matches!(settled(&rx), Decision::Retry));
    let held = desk.held();
    assert_eq!(held.len(), 1);
    assert!(!held[0].until_quit, "while its process runs");
    assert!(desk.revoke(held[0].id));
    assert!(desk.held().is_empty());

    let (tx, rx) = waiter();
    let id = desk.ask(agent("quit"), TYPE, None, Some(tx)).expect("room");
    desk.answer(id, AccessAnswer::UntilQuit);
    assert!(matches!(settled(&rx), Decision::Retry));
    assert!(desk.held()[0].until_quit);

    let (tx, rx) = waiter();
    let id = desk.ask(agent("always"), TYPE, None, Some(tx)).expect("room");
    desk.answer(id, AccessAnswer::Always);
    assert!(matches!(settled(&rx), Decision::Retry));
    let stored = Store::open_at(&data).expect("the database").agent_grants().expect("rows");
    assert_eq!(stored.len(), 1, "for good: in the profile's database");
    assert_eq!(stored[0].label, "Claude Code");
    assert_eq!(stored[0].location, "/opt/tools/claude");
}

#[test]
fn a_no_tells_every_request_not_to_retry_and_a_withdrawn_question_is_not_remembered() {
    let (_root, _data, desk, _) = desk();
    let (tx, rx) = waiter();
    let id = desk.ask(agent("no"), TYPE, None, Some(tx)).expect("room");
    desk.answer(id, AccessAnswer::Deny);
    let Decision::Refuse(reply) = settled(&rx) else { panic!("a refusal") };
    let message = reply.error.expect("an error").message;
    assert!(message.contains("did not allow Claude Code") && message.contains("do not retry"), "{message}");
    assert!(desk.held().is_empty());

    let (tx, rx) = waiter();
    let id = desk.ask(agent("gone"), TYPE, None, Some(tx)).expect("room");
    desk.withdraw(id);
    let Decision::Refuse(reply) = settled(&rx) else { panic!("a refusal") };
    assert!(reply.error.expect("an error").message.contains("went away without an answer"));
    assert!(desk.held().is_empty());
}

#[test]
fn scripts_and_terminals_never_get_always() {
    let (_root, _data, desk, _) = desk();
    desk.ask(scripts(), TYPE, None, None).expect("room");
    let question = desk.next_question().expect("scripts");
    assert_eq!(question.kind, AccessKindRecord::Scripts);
    assert!(question.program.is_none() && !question.can_be_remembered);
    desk.answer(question.id, AccessAnswer::Deny);

    let (tx, rx) = waiter();
    desk.ask(agent("t"), TYPE, Some(7), Some(tx)).expect("room");
    let question = desk.next_question().expect("about a terminal");
    assert_eq!(question.about_terminal, Some(7));
    assert!(!question.can_be_remembered, "a terminal is not a stable thing to approve for good");
    desk.answer(question.id, AccessAnswer::Session);
    assert!(matches!(settled(&rx), Decision::Retry));
    assert!(desk.held().is_empty(), "it opened one terminal up, it did not let the program in");
}

#[test]
fn a_folder_outside_the_experiment_is_refused() {
    assert!(AccessDesk::new(None, Some("/Users/x/Wormhole".into()), Wakes::new()).is_err());
}
