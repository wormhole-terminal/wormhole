use std::time::{Duration, Instant};

use super::update_fixture::{Offer, Published, marker_of, publish};
use crate::aux_backends::{UpdateChannelRecord, UpdatePhaseRecord, UpdateStateRecord};

/// Waits (up to 30 s) for the state to satisfy `done`; the engine's threads do the work.
fn wait_for(published: &Published, done: impl Fn(&UpdateStateRecord) -> bool) -> UpdateStateRecord {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let state = published.engine.state();
        if done(&state) {
            return state;
        }
        assert!(Instant::now() < deadline, "the updater did not get there: {state:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn offered(state: &UpdateStateRecord) -> Option<String> {
    match &state.phase {
        UpdatePhaseRecord::Available { release } => Some(release.version.clone()),
        _ => None,
    }
}

#[test]
fn a_check_finds_the_release_with_its_notes_and_remembers_when() {
    let published = publish(Offer::default());
    assert_eq!(published.engine.state().phase, UpdatePhaseRecord::Idle);
    assert!(published.engine.state().last_checked.is_none());
    published.engine.check(true);
    let state = wait_for(&published, |s| offered(s).is_some());
    let UpdatePhaseRecord::Available { release } = state.phase else { unreachable!() };
    assert_eq!(release.version, "0.2.0");
    assert_eq!(release.notes, "New stuff");
    assert_eq!(release.published.as_deref(), Some("2026-10-01T12:00:00Z"));
    assert!(!release.is_security);
    assert!(state.last_checked.is_some());
    assert!(published.wakes.count() >= 1, "the window was woken");
    assert_eq!(state.channel, UpdateChannelRecord::Stable);
}

#[test]
fn install_downloads_verifies_and_swaps_the_scratch_bundle_only() {
    let published = publish(Offer::default());
    published.engine.check(true);
    wait_for(&published, |s| offered(s).is_some());
    assert!(published.engine.install("9.9.9".into()).is_err(), "only what was offered");
    assert_eq!(marker_of(&published.bundle), "#!/bin/sh\necho old\n");

    published.engine.install("0.2.0".into()).expect("started");
    let state = wait_for(&published, |s| matches!(s.phase, UpdatePhaseRecord::Installed { .. } | UpdatePhaseRecord::Failed { .. }));
    assert_eq!(state.phase, UpdatePhaseRecord::Installed { version: "0.2.0".into() }, "{state:?}");
    assert_eq!(marker_of(&published.bundle), "#!/bin/sh\necho new\n", "the scratch bundle was replaced");
    // Nothing else was left beside it.
    let others: Vec<_> = std::fs::read_dir(published.apps.path())
        .expect("folder")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n != "Wormhole.app" && n != "update-schedule.json")
        .collect();
    assert!(others.is_empty(), "{others:?}");
}

#[test]
fn an_archive_signed_by_another_key_is_refused_and_nothing_is_swapped() {
    let published = publish(Offer { signer_is_trusted: false, preview: None });
    published.engine.check(true);
    wait_for(&published, |s| offered(s).is_some());
    published.engine.install("0.2.0".into()).expect("started");
    let state = wait_for(&published, |s| matches!(s.phase, UpdatePhaseRecord::Installed { .. } | UpdatePhaseRecord::Failed { .. }));
    let UpdatePhaseRecord::Failed { message } = state.phase else { panic!("{state:?}") };
    assert!(message.contains("signature"), "{message}");
    assert_eq!(marker_of(&published.bundle), "#!/bin/sh\necho old\n");
}

#[test]
fn skipping_quiets_automatic_checks_but_a_manual_one_still_shows_it() {
    let published = publish(Offer::default());
    published.engine.check(true);
    wait_for(&published, |s| offered(s).is_some());
    published.engine.skip("0.2.0".into()).expect("skipped");
    assert_eq!(published.engine.state().phase, UpdatePhaseRecord::Idle);

    published.engine.check(false);
    wait_for(&published, |s| s.phase == UpdatePhaseRecord::Idle && s.last_checked.is_some());
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(published.engine.state().phase, UpdatePhaseRecord::Idle, "an automatic check says nothing");

    published.engine.check(true);
    wait_for(&published, |s| offered(s).is_some());
}

#[test]
fn the_preview_channel_reads_its_own_feed_and_takes_the_newer() {
    let published = publish(Offer { signer_is_trusted: true, preview: Some("0.3.0-dev.1") });
    published.engine.check(true);
    wait_for(&published, |s| offered(s).as_deref() == Some("0.2.0"));
    // Switching asks the new channel at once: the person just made a choice about updates.
    published.engine.set_channel(UpdateChannelRecord::Preview);
    let state = wait_for(&published, |s| offered(s).as_deref() == Some("0.3.0-dev.1"));
    assert_eq!(state.channel, UpdateChannelRecord::Preview);
    assert!(state.channel_note.is_none());
}

#[test]
fn restart_needs_a_bundle_and_nothing_here_relaunches() {
    let published = publish(Offer::default());
    // A build that does not run from a bundle cannot restart; this is the only restart path the tests take.
    let engine = crate::aux_backends::UpdateEngine::new(
        crate::aux_backends::UpdateSetup {
            base_url: "http://127.0.0.1:1".into(),
            public_key: String::new(),
            current_version: "0.1.0".into(),
            bundle_id: super::update_fixture::BUNDLE_ID.into(),
            team_id: None,
            app_bundle: None,
            verify_codesign: false,
            fallback_url: None,
            schedule_file: Some(published.apps.path().join("other-schedule.json").display().to_string()),
            channel: UpdateChannelRecord::Stable,
        },
        published.wakes.clone(),
    )
    .expect("an engine");
    assert!(engine.restart().is_err());
    assert!(engine.install("0.2.0".into()).is_err());
    assert!(engine.next_check_in_seconds() == 0 && engine.due_at_launch(), "never checked: due now");
}
