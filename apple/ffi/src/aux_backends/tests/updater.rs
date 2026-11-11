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
    let state = wait_for(&published, |s| {
        matches!(s.phase, UpdatePhaseRecord::Installed { .. } | UpdatePhaseRecord::Failed { .. })
    });
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
    let published = publish(Offer { signer_is_trusted: false, ..Offer::default() });
    published.engine.check(true);
    wait_for(&published, |s| offered(s).is_some());
    published.engine.install("0.2.0".into()).expect("started");
    let state = wait_for(&published, |s| {
        matches!(s.phase, UpdatePhaseRecord::Installed { .. } | UpdatePhaseRecord::Failed { .. })
    });
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
    let published = publish(Offer { signer_is_trusted: true, preview: Some("0.3.0-dev.1"), security: None });
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
    assert_eq!(
        engine.state().phase,
        UpdatePhaseRecord::Failed { message: "Wormhole isn't running from an app bundle.".into() },
        "the button is not silent"
    );
    assert!(engine.install("0.2.0".into()).is_err());
    assert!(engine.next_check_in_seconds() == 0 && engine.due_at_launch(), "never checked: due now");
}

/// The schedule's own check leaves an update that is offered or installed as it is (GPUI's `schedule_check`): it would
/// withdraw the chips, drop a pending Restart When Idle and fetch the same release again. A person's check still runs.
#[test]
fn an_automatic_check_leaves_an_offered_or_installed_update_alone() {
    let published = publish(Offer::default());
    published.engine.check(true);
    wait_for(&published, |s| offered(s).is_some());
    let before = published.engine.state();
    published.engine.check(false);
    assert_eq!(published.engine.state(), before, "still on offer, no check started");

    published.engine.install("0.2.0".into()).expect("started");
    wait_for(&published, |s| matches!(s.phase, UpdatePhaseRecord::Installed { .. }));
    let before = published.engine.state();
    published.engine.check(false);
    assert_eq!(published.engine.state(), before, "installed stays installed");
    published.engine.check(true);
    assert_eq!(published.engine.state().phase, UpdatePhaseRecord::Checking, "a person's check runs");
}

/// With auto_update on, an automatic check installs what it finds, once, in the core alone (Swift does not decide).
#[test]
fn with_auto_update_on_a_check_goes_straight_to_installed() {
    let published = publish(Offer::default());
    published.engine.set_auto_update(true);
    published.engine.check(false);
    let state = wait_for(&published, |s| {
        matches!(s.phase, UpdatePhaseRecord::Installed { .. } | UpdatePhaseRecord::Failed { .. })
    });
    assert_eq!(state.phase, UpdatePhaseRecord::Installed { version: "0.2.0".into() }, "{state:?}");
    assert_eq!(marker_of(&published.bundle), "#!/bin/sh\necho new\n");
}

#[test]
fn a_security_update_says_what_it_fixes_and_where_the_advisories_are() {
    let published = publish(Offer {
        security: Some(
            serde_json::json!({"severity": "high", "advisories": ["ghsa-2222-3333-4444", "../../x"]}),
        ),
        ..Offer::default()
    });
    published.engine.check(true);
    let state = wait_for(&published, |s| offered(s).is_some());
    let UpdatePhaseRecord::Available { release } = state.phase else { panic!("offered") };
    assert!(release.is_security);
    let security = release.security.expect("a security line");
    assert!(security.danger, "high reads as danger");
    assert_eq!(security.line, "Fixes a high-severity security issue.");
    // A malformed id never reaches a link.
    assert_eq!(security.advisories.len(), 1);
    assert_eq!(security.advisories[0].id, "GHSA-2222-3333-4444");
    assert_eq!(
        security.advisories[0].url,
        "https://github.com/wormhole-terminal/wormhole/security/advisories/ghsa-2222-3333-4444"
    );
}

#[test]
fn a_channel_switch_says_what_happened_to_the_build_it_leaves() {
    use crate::aux_backends::updater::note_words;
    let stays = note_words("0.3.0-dev.1", true);
    assert!(stays.contains("0.3.0-dev.1") && stays.contains("installed"), "{stays}");
    let removed = note_words("0.3.0-dev.1", false);
    assert!(removed.contains("0.3.0-dev.1") && removed.contains("removed"), "{removed}");
    assert_ne!(stays, removed);
}

#[test]
fn a_bundle_replaced_while_running_is_installed_and_waits_for_a_restart() {
    let published = publish(Offer::default());
    published.engine.note_replaced_for_test("0.2.0");
    assert_eq!(published.engine.state().phase, UpdatePhaseRecord::Installed { version: "0.2.0".into() });
}

#[test]
fn a_release_build_has_its_feed_key_and_fallback_baked_in_and_a_dev_build_has_none() {
    assert_eq!(crate::aux_backends::updater::release_setup(None, None), None);
    assert_eq!(crate::aux_backends::updater::release_setup(Some(""), None), None);
    let setup =
        crate::aux_backends::updater::release_setup(Some("https://downloads.example"), Some("TEAM123"))
            .expect("a feed");
    assert_eq!(setup.base_url, "https://downloads.example");
    assert_eq!(setup.team_id.as_deref(), Some("TEAM123"));
    assert!(setup.public_key.contains("untrusted comment"), "the minisign key of packaging/updater.pub");
    assert!(setup.fallback_url.ends_with("/releases"));
}
