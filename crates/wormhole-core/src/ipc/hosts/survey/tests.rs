use super::*;
use crate::ipc::hosts::Probed;
use crate::platform::socket::Listener;

/// A host whose process is this one (so it runs) and whose socket takes connections but never
/// answers: it is live, asked, and reported as not answering within the wait; the system's
/// readings of the process are there.
#[test]
fn a_running_host_that_stays_silent_is_live_unanswering_and_read_from_the_system() {
    let dir = tempfile::Builder::new().prefix("hs").tempdir_in("/tmp").expect("temp");
    let socket = dir.path().join("aaaaaaaaaaaa.sock");
    let _listener = Listener::bind(&socket).expect("binds");
    let host = crate::ipc::hosts::identity::spawn_fake_host(dir.path());
    let text = format!(
        "format=1\nid=aaaaaaaaaaaa\npid={}\nstarted_at={}\nhost_version=1.0.0\nproto_min=1\nproto_max=1\n\
         socket={}\ndata_dir=/d\n",
        host.pid(),
        host.started_at,
        socket.display()
    );
    std::fs::write(dir.path().join("aaaaaaaaaaaa.meta"), text).expect("meta");
    let survey = survey(dir.path(), Duration::from_millis(200)).expect("surveys");
    assert_eq!(survey.scan.live.len(), 1);
    let seen = &survey.observed["aaaaaaaaaaaa"];
    assert!(matches!(seen.probed, Some(Probed::Failed(_))), "{:?}", seen.probed);
    assert!(seen.rss.is_some_and(|bytes| bytes > 0), "this process has resident memory");
}

#[test]
fn a_folder_without_hosts_surveys_to_nothing() {
    let dir = tempfile::tempdir().expect("temp");
    let survey = survey(dir.path(), Duration::from_millis(50)).expect("surveys");
    assert!(survey.scan.live.is_empty() && survey.observed.is_empty());
}
