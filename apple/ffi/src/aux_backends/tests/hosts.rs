use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use wormhole_core::platform::process;
use wormhole_term::hosted::meta::Meta;

use crate::aux_backends::common::test_support::{Wakes, scratch};
use crate::aux_backends::{
    AuxError, HostAttachmentRecord, HostFieldRecord, HostLinkMode, HostLinkRecord, HostStandingRecord,
    HostToneRecord, HostsDesk,
};
use crate::hosting::HostUnavailableKind;

const ID_LIVE: &str = "livehost0001";
const ID_GONE: &str = "gonehost0002";

/// What the stand-in host runs: it only waits, so a copy of this test program named like a host stays up
/// (the same device core's own host tests use). Ignored, so a normal run never reaches it.
#[test]
#[ignore = "the body of the stand-in host (spawn_host)"]
fn fake_host_sleeps() {
    std::fs::write("fake-host-ready", "").expect("says it runs");
    std::thread::sleep(Duration::from_secs(60));
}

struct StandIn(Child);

impl Drop for StandIn {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_host(dir: &Path) -> StandIn {
    let binary = dir.join("wormhole-host");
    std::fs::copy(std::env::current_exe().expect("this program"), &binary).expect("copies it");
    let child = Command::new(&binary)
        .args(["--ignored", "--exact", "aux_backends::tests::hosts::fake_host_sleeps", "--nocapture"])
        .current_dir(dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("starts");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !dir.join("fake-host-ready").exists() {
        assert!(Instant::now() < deadline, "the stand-in host did not start");
        std::thread::sleep(Duration::from_millis(10));
    }
    StandIn(child)
}

fn write_meta(hosts: &Path, id: &str, pid: u32, listening: bool) -> PathBuf {
    let socket = hosts.join(format!("{id}.sock"));
    if listening {
        // A socket file nobody listens on: the host's end of a connection is refused at once.
        drop(std::os::unix::net::UnixListener::bind(&socket).expect("binds"));
    }
    let start = process::start_time(pid).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
    let meta = Meta {
        id: id.to_string(),
        pid,
        child_pid: None,
        started_at: start.map_or(0, |d| d.as_secs()),
        host_version: "1.6.2".into(),
        proto_min: 1,
        proto_max: 1,
        socket,
        data_dir: hosts.parent().expect("data dir").to_path_buf(),
        proc_start: start.map(|d| (d.as_secs(), d.subsec_micros())),
    };
    meta.write(hosts).expect("writes the meta")
}

/// A pid that was a process a moment ago and is gone now.
fn dead_pid() -> u32 {
    let mut child = Command::new("/usr/bin/true").spawn().expect("starts");
    let pid = child.id();
    child.wait().expect("ends");
    pid
}

fn hosts_dir(data: &Path) -> PathBuf {
    let dir = data.join("hosts");
    std::fs::create_dir_all(&dir).expect("hosts folder");
    dir
}

#[test]
fn a_folder_outside_the_experiment_is_refused() {
    let err = HostsDesk::new(Some("/tmp/Wormhole-dev".into()), Wakes::new()).err().expect("refused");
    assert!(matches!(err, AuxError::NotIsolated { .. }));
}

#[test]
fn live_and_stale_hosts_are_classified_and_attachments_name_their_terminal() {
    let (_root, data) = scratch();
    let hosts = hosts_dir(&data);
    let bin = tempfile::tempdir().expect("temp");
    let host = spawn_host(bin.path());
    write_meta(&hosts, ID_LIVE, host.0.id(), true);
    write_meta(&hosts, ID_GONE, dead_pid(), true);

    let desk = HostsDesk::new(Some(data.display().to_string()), Wakes::new()).expect("desk");
    desk.set_persistent_terminals(true);
    let report = desk.report();
    assert!(report.flag_on);
    assert_eq!(report.folder, hosts.display().to_string());
    let live = report.cards.iter().find(|c| c.id == ID_LIVE).expect("the live host");
    assert_eq!(live.standing, HostStandingRecord::Orphan, "nothing names it");
    let pid = live.facts.iter().find(|f| f.field == HostFieldRecord::HostPid).expect("pid");
    assert_eq!(pid.value, host.0.id().to_string());
    let answer = live.facts.iter().find(|f| f.field == HostFieldRecord::Answer).expect("answer");
    assert_eq!(answer.tone, HostToneRecord::Danger, "nothing listens on its socket");
    assert!(live.needs_attention);
    let stale = report.cards.iter().find(|c| c.id == ID_GONE).expect("the stale host");
    assert_eq!(stale.standing, HostStandingRecord::Stale);

    let attachment = |link, unavailable| HostAttachmentRecord {
        host_id: ID_LIVE.into(),
        window: 2,
        title: "api".into(),
        link,
        unavailable,
        unavailable_detail: None,
    };
    desk.set_attachments(vec![attachment(None, None)]);
    let report = desk.report();
    let live = report.cards.iter().find(|c| c.id == ID_LIVE).expect("the live host");
    assert_eq!(live.standing, HostStandingRecord::Attached);
    assert_eq!(live.attached_title.as_deref(), Some("api"));
    assert!(live.note.contains("api"), "{}", live.note);
    let fact = |card: &crate::aux_backends::HostCardRecord, field| {
        card.facts.iter().find(|f| f.field == field).cloned()
    };
    assert_eq!(fact(live, HostFieldRecord::Link).map(|f| f.value).as_deref(), Some("Connecting…"));
    assert!(fact(live, HostFieldRecord::Heard).is_none() && fact(live, HostFieldRecord::Quiet).is_none());

    // The connection's own facts: counters and the last word from the host while it is attached.
    let link = |mode, lost| HostLinkRecord {
        mode,
        bytes_in: 4096,
        bytes_out: 128,
        frames_in: 7,
        frames_out: 3,
        detached: false,
        lost,
        ended: false,
        quiet_secs: 12.0,
        not_responding_secs: None,
    };
    desk.set_attachments(vec![attachment(Some(link(HostLinkMode::Sync, false)), None)]);
    let report = desk.report();
    let live = report.cards.iter().find(|c| c.id == ID_LIVE).expect("the live host");
    let traffic = fact(live, HostFieldRecord::Link).expect("link");
    assert!(traffic.value.contains("7") && traffic.value.contains("3"), "{}", traffic.value);
    assert_eq!(traffic.tone, HostToneRecord::Normal);
    assert!(fact(live, HostFieldRecord::Heard).is_some(), "a link on screen is pinged");
    desk.set_attachments(vec![attachment(Some(link(HostLinkMode::Headless, false)), None)]);
    let report = desk.report();
    let live = report.cards.iter().find(|c| c.id == ID_LIVE).expect("the live host");
    assert!(
        fact(live, HostFieldRecord::Quiet).is_some(),
        "a link that is not on screen is not pinged: only quiet"
    );
    // A lost link, or a host that runs and cannot be shown, is a problem.
    desk.set_attachments(vec![attachment(None, Some(HostUnavailableKind::TooNew))]);
    let report = desk.report();
    let live = report.cards.iter().find(|c| c.id == ID_LIVE).expect("the live host");
    let why = fact(live, HostFieldRecord::Link).expect("link");
    assert_eq!(why.tone, HostToneRecord::Danger);
    assert_ne!(why.value, "Connecting…");
    desk.set_attachments(vec![attachment(Some(link(HostLinkMode::Sync, true)), None)]);
    let report = desk.report();
    let live = report.cards.iter().find(|c| c.id == ID_LIVE).expect("the live host");
    assert_eq!(fact(live, HostFieldRecord::Link).map(|f| f.tone), Some(HostToneRecord::Danger));
}

#[test]
fn forget_removes_only_what_is_not_alive_and_end_signals_the_stand_in() {
    let (_root, data) = scratch();
    let hosts = hosts_dir(&data);
    let bin = tempfile::tempdir().expect("temp");
    let mut host = spawn_host(bin.path());
    write_meta(&hosts, ID_LIVE, host.0.id(), true);
    let gone_meta = write_meta(&hosts, ID_GONE, dead_pid(), true);
    let wakes = Wakes::new();
    let desk = HostsDesk::new(Some(data.display().to_string()), wakes.clone()).expect("desk");

    assert!(desk.forget(ID_LIVE.into()).is_err(), "a running host keeps its files");
    desk.forget(ID_GONE.into()).expect("the stale one goes");
    assert!(!gone_meta.exists());
    assert!(desk.forget("not-an-id".into()).is_err());
    assert!(wakes.count() >= 1, "a change wakes the window");

    let how = desk.end(ID_LIVE.into()).expect("ended");
    assert_eq!(how, "ended (signalled)", "its socket has nobody behind it");
    let deadline = Instant::now() + Duration::from_secs(10);
    while host.0.try_wait().expect("waits").is_none() {
        assert!(Instant::now() < deadline, "the stand-in host did not end");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn the_folder_watch_wakes_once_for_a_burst_until_the_report_is_read() {
    let (_root, data) = scratch();
    let _ = hosts_dir(&data);
    let wakes = Wakes::new();
    let desk = HostsDesk::new(Some(data.display().to_string()), wakes.clone()).expect("desk");
    if !desk.follow() {
        return; // no event stream here (a volume that sends none): the window's slow look stands in
    }
    for n in 0..5 {
        std::fs::write(data.join("hosts").join(format!("burst{n}.txt")), b"x").expect("a file");
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while wakes.count() == 0 {
        assert!(Instant::now() < deadline, "no event came");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(wakes.count(), 1, "a burst is one wake until the state is read");
    let _ = desk.report();
    std::fs::write(data.join("hosts").join("again.txt"), b"x").expect("a file");
    let deadline = Instant::now() + Duration::from_secs(10);
    while wakes.count() < 2 {
        assert!(Instant::now() < deadline, "no wake after the read");
        std::thread::sleep(Duration::from_millis(20));
    }
    desk.unfollow();
}
