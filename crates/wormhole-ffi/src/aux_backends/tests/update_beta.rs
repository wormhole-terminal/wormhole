//! The beta build's updater: its own feed, its one channel, and a bundle that must name the version offered.

use std::time::{Duration, Instant};

use super::update_fixture::{Beta, Offer, Published, marker_of, publish};
use crate::aux_backends::updater::release_setup;
use crate::aux_backends::{UpdateChannelRecord, UpdatePhaseRecord, UpdateStateRecord};

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

fn beta(bundle_names: &'static str) -> Published {
    // The stable and preview feeds offer more than the beta feed does: a beta that read them would show it.
    publish(Offer {
        beta: Some(Beta { version: "2.0.0-beta.2", bundle_names }),
        preview: Some("9.0.0-dev.1"),
        channel: UpdateChannelRecord::Beta,
        current: "2.0.0-beta.1",
        ..Offer::default()
    })
}

fn settled(state: &UpdateStateRecord) -> bool {
    matches!(state.phase, UpdatePhaseRecord::Installed { .. } | UpdatePhaseRecord::Failed { .. })
}

#[test]
fn a_beta_is_offered_the_newer_beta_from_its_own_feed_only() {
    let published = beta("2.0.0-beta.2");
    published.engine.check(true);
    let state = wait_for(&published, |s| matches!(s.phase, UpdatePhaseRecord::Available { .. }));
    let UpdatePhaseRecord::Available { release } = state.phase else { unreachable!() };
    assert_eq!(release.version, "2.0.0-beta.2");
    assert_eq!(state.channel, UpdateChannelRecord::Beta);
}

#[test]
fn a_beta_installs_a_bundle_that_names_the_version_offered() {
    let published = beta("2.0.0-beta.2");
    published.engine.check(true);
    wait_for(&published, |s| matches!(s.phase, UpdatePhaseRecord::Available { .. }));
    published.engine.install("2.0.0-beta.2".into()).expect("started");
    let state = wait_for(&published, settled);
    assert_eq!(state.phase, UpdatePhaseRecord::Installed { version: "2.0.0-beta.2".into() }, "{state:?}");
    assert_eq!(marker_of(&published.bundle), "#!/bin/sh\necho new\n");
}

#[test]
fn a_signed_archive_whose_bundle_is_another_beta_is_refused_and_nothing_is_swapped() {
    // Signed for beta.2, as CI signs it, but the bundle inside says beta.1: the signature does not look inside.
    let published = beta("2.0.0-beta.1");
    published.engine.check(true);
    wait_for(&published, |s| matches!(s.phase, UpdatePhaseRecord::Available { .. }));
    published.engine.install("2.0.0-beta.2".into()).expect("started");
    let state = wait_for(&published, settled);
    let UpdatePhaseRecord::Failed { message } = state.phase else { panic!("{state:?}") };
    assert!(message.contains("WormholeVersion"), "{message}");
    assert_eq!(marker_of(&published.bundle), "#!/bin/sh\necho old\n");
}

#[test]
fn nothing_moves_an_engine_onto_the_beta_or_off_it() {
    let published = beta("2.0.0-beta.2");
    published.engine.set_channel(UpdateChannelRecord::Preview);
    published.engine.set_channel(UpdateChannelRecord::Stable);
    let state = published.engine.state();
    assert_eq!(state.channel, UpdateChannelRecord::Beta);
    assert_eq!(state.phase, UpdatePhaseRecord::Idle, "and no check was made for it");

    let stable = publish(Offer::default());
    stable.engine.set_channel(UpdateChannelRecord::Beta);
    assert_eq!(stable.engine.state().channel, UpdateChannelRecord::Stable);
}

#[test]
fn a_build_bakes_beta_and_nothing_else() {
    let url = Some("https://example.test/download");
    assert_eq!(
        release_setup(url, None, Some("beta")).expect("a setup").channel,
        Some(UpdateChannelRecord::Beta)
    );
    assert_eq!(release_setup(url, None, Some("")).expect("a setup").channel, None);
    assert_eq!(release_setup(url, None, None).expect("a setup").channel, None);
    // A typo is a build that does not update itself, not one on the wrong feed.
    assert_eq!(release_setup(url, None, Some("Beta")), None);
    assert_eq!(release_setup(url, None, Some("preview")), None);
}
