use std::time::{Duration, SystemTime};

use super::*;
use crate::platform::process;

/// What `spawn_fake_host` runs: it does nothing but wait, so a copy of this program named like a
/// host stays up. Ignored, so a normal run of the tests never reaches it.
#[test]
#[ignore = "the body of the stand-in host (spawn_fake_host)"]
fn fake_host_sleeps() {
    std::fs::write(super::FAKE_HOST_READY, "").expect("says it runs");
    std::thread::sleep(Duration::from_secs(60));
}

fn at(secs: u64) -> Option<SystemTime> {
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
}

fn host_reading(start: Option<SystemTime>) -> Reading {
    Reading { exists: true, name: Some("wormhole-host".into()), ours: Some(true), start }
}

#[test]
fn a_host_has_the_name_the_user_and_a_start_no_later_than_the_meta_says() {
    assert_eq!(classify(&host_reading(at(1000)), 1000, None), Identity::Yes);
    assert_eq!(classify(&host_reading(at(1000)), 1000 - START_SLACK_SECS, None), Identity::Yes);
}

/// A slow first start (a scan after an update) puts the process's start well after the host
/// wrote its meta; that is the host. What a reused pid shows is a start after the meta, not before.
#[test]
fn a_process_that_started_before_the_meta_says_is_still_the_host() {
    assert_eq!(classify(&host_reading(at(970)), 1000, None), Identity::Yes);
    assert_eq!(classify(&host_reading(at(5)), 1000, None), Identity::Yes, "no lower bound");
}

#[test]
fn a_process_that_started_after_the_meta_says_is_another_one() {
    assert_eq!(classify(&host_reading(at(1000 + START_SLACK_SECS + 1)), 1000, None), Identity::No);
}

#[test]
fn what_cannot_be_read_is_unknown_never_no() {
    // No start of a missing `started_at`: the start cannot be compared.
    assert_eq!(classify(&host_reading(at(1000)), 0, None), Identity::Unknown);
    // The system will not say when the process started.
    assert_eq!(classify(&host_reading(None), 1000, None), Identity::Unknown);
    // Its user or its name cannot be read.
    let mut reading = host_reading(at(1000));
    reading.ours = None;
    assert_eq!(classify(&reading, 1000, None), Identity::Unknown);
    let mut reading = host_reading(at(1000));
    reading.name = None;
    assert_eq!(classify(&reading, 1000, None), Identity::Unknown);
}

#[test]
fn something_definite_is_no() {
    assert_eq!(classify(&Reading { exists: false, ..host_reading(at(1000)) }, 1000, None), Identity::No);
    let other = Reading { name: Some("zsh".into()), ..host_reading(at(1000)) };
    assert_eq!(classify(&other, 1000, None), Identity::No);
    let theirs = Reading { ours: Some(false), ..host_reading(at(1000)) };
    assert_eq!(classify(&theirs, 1000, None), Identity::No);
}

#[test]
fn a_start_the_host_wrote_is_compared_exactly() {
    let reading = host_reading(Some(SystemTime::UNIX_EPOCH + Duration::new(1000, 250_000_000)));
    assert_eq!(classify(&reading, 1000, Some((1000, 250_000))), Identity::Yes);
    // Another process with the same second but another moment is not the host...
    assert_eq!(classify(&reading, 1000, Some((1000, 900_000))), Identity::No);
    // ...and an exact start also catches a pid reused within the slack.
    assert_eq!(classify(&reading, 1000, Some((990, 0))), Identity::No);
    // It decides even when `started_at` is missing.
    assert_eq!(classify(&reading, 0, Some((1000, 250_000))), Identity::Yes);
}

#[test]
fn this_process_and_a_gone_one_are_not_hosts() {
    let this = HostMeta { pid: std::process::id(), ..meta(0) };
    assert_eq!(identify(&this), Identity::No, "named like a test, not like a host");
    assert_eq!(identify(&HostMeta { pid: 0, ..meta(0) }), Identity::No);
    assert_eq!(identify(&HostMeta { pid: u32::MAX, ..meta(0) }), Identity::No);
    assert!(!looks_like_host_process(std::process::id()));
}

fn meta(started_at: u64) -> HostMeta {
    HostMeta {
        id: "abcdefghij12".into(),
        pid: 1,
        child_pid: None,
        started_at,
        host_version: "1.0.0".into(),
        proto_min: 1,
        proto_max: 1,
        socket: "/s/x.sock".into(),
        data_dir: "/d".into(),
        proc_start: None,
    }
}

fn meta_of(host: &super::FakeHost) -> HostMeta {
    HostMeta { pid: host.pid(), started_at: host.started_at, ..meta(0) }
}

/// The real thing: a running process named `wormhole-host`, whose binary an update deletes.
#[test]
fn a_running_host_is_known_by_name_and_start_even_when_its_binary_is_gone() {
    let dir = tempfile::tempdir().expect("temp");
    let mut host = spawn_fake_host(dir.path());
    assert!(!host.has_exited());
    assert_eq!(identify(&meta_of(&host)), Identity::Yes, "running from its binary");
    std::fs::remove_file(dir.path().join(HOST_BINARY_NAME)).expect("deletes the binary");
    assert_eq!(identify(&meta_of(&host)), Identity::Yes, "its name and start are still there");
    assert!(looks_like_host_process(host.pid()));
    let exact = HostMeta { proc_start: Some(host.proc_start), ..meta_of(&host) };
    assert_eq!(identify(&exact), Identity::Yes);
    drop(host);
    assert_eq!(identify(&HostMeta { pid: 0, ..meta(0) }), Identity::No);
}

/// A live host whose process began 30 s before it wrote its meta (a slow first start: the system
/// scanned the new program) is not gone, and neither is one whose meta has no `started_at`.
#[test]
fn a_slow_start_live_host_is_not_gone() {
    let dir = tempfile::tempdir().expect("temp");
    let host = spawn_fake_host(dir.path());
    let slow = HostMeta { started_at: host.started_at + 30, ..meta_of(&host) };
    assert_eq!(identify(&slow), Identity::Yes);
    let missing = HostMeta { started_at: 0, ..meta_of(&host) };
    assert_eq!(identify(&missing), Identity::Unknown);
    // Another start than the one it has is gone: the pid was reused after the meta was written.
    let reused = HostMeta { started_at: host.started_at - 100, ..meta_of(&host) };
    assert_eq!(identify(&reused), Identity::No, "the process began 100 s after the host the meta describes");
    let exact = HostMeta { proc_start: Some((host.proc_start.0 - 100, 0)), ..meta_of(&host) };
    assert_eq!(identify(&exact), Identity::No);
    assert!(process::exists(host.pid()));
}
