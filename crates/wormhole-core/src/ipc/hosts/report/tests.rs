use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use wormhole_term::client::{LinkMode, LinkStats};
use wormhole_wire::version::HostFit;

use super::*;
use crate::ipc::hosts::{ProbeFailure, StaleHost, StaleReason};

const NOW: u64 = 1_000_000;

fn app() -> AppFacts {
    AppFacts { version: "1.6.2".into(), proto_major: 1, proto_minor: 2 }
}

fn meta(id: &str, started_at: u64, version: &str, proto: (u16, u16)) -> HostMeta {
    HostMeta {
        id: id.into(),
        pid: 100,
        child_pid: Some(101),
        started_at,
        host_version: version.into(),
        proto_min: proto.0,
        proto_max: proto.1,
        socket: PathBuf::from(format!("/h/{id}.sock")),
        data_dir: PathBuf::from("/d"),
        proc_start: None,
    }
}

fn link(detached: bool) -> LinkStats {
    LinkStats {
        mode: LinkMode::Sync,
        bytes_in: 10,
        bytes_out: 5,
        frames_in: 3,
        frames_out: 2,
        detached,
        lost: false,
        ended: false,
        quiet_for: std::time::Duration::from_millis(40),
        not_responding_for: None,
    }
}

fn inputs(scan: Scan) -> Inputs {
    Inputs {
        app: app(),
        hosts_dir: PathBuf::from("/h"),
        flag_on: true,
        host_binary: Some(PathBuf::from("/Apps/wormhole-host")),
        scan,
        observed: HashMap::new(),
        attachments: Vec::new(),
        named: HashSet::new(),
        now: NOW,
    }
}

#[test]
fn a_held_host_is_attached_a_named_one_detached_and_a_nameless_one_an_orphan() {
    let scan = Scan {
        live: vec![
            meta("aaaaaaaaaaaa", 10, "1.6.2", (1, 1)),
            meta("bbbbbbbbbbbb", 20, "1.6.2", (1, 1)),
            meta("cccccccccccc", 30, "1.6.2", (1, 1)),
            meta("dddddddddddd", 40, "1.6.2", (1, 1)),
        ],
        stale: Vec::new(),
    };
    let mut i = inputs(scan);
    i.attachments = vec![
        Attachment {
            host_id: "aaaaaaaaaaaa".into(),
            unavailable: None,
            window: 2,
            title: "build".into(),
            link: Some(link(false)),
        },
        // The app let go of this one (devctl `detach`, or the window closing).
        Attachment {
            host_id: "bbbbbbbbbbbb".into(),
            unavailable: None,
            window: 2,
            title: "logs".into(),
            link: Some(link(true)),
        },
    ];
    i.named = HashSet::from(["cccccccccccc".to_string()]);
    let report = Report::build(i);
    let standing: Vec<_> = report.rows.iter().map(|r| r.standing.clone()).collect();
    assert_eq!(
        standing,
        [
            Standing::Attached { window: 2, title: "build".into() },
            Standing::Detached,
            Standing::Detached,
            Standing::Orphan
        ]
    );
    assert_eq!(report.counts(), Counts { attached: 1, detached: 2, orphan: 1, stale: 0 });
    assert_eq!(report.counts().live(), 4);
}

#[test]
fn stale_hosts_follow_the_live_ones_and_keep_their_reason() {
    let scan = Scan {
        live: vec![meta("aaaaaaaaaaaa", 10, "1.6.2", (1, 1))],
        stale: vec![
            StaleHost {
                id: "bbbbbbbbbbbb".into(),
                reason: StaleReason::PidGone,
                meta: Some(meta("bbbbbbbbbbbb", 5, "1.6.2", (1, 1))),
            },
            StaleHost { id: "cccccccccccc".into(), reason: StaleReason::NoMeta, meta: None },
        ],
    };
    let report = Report::build(inputs(scan));
    let ids: Vec<_> = report.rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["aaaaaaaaaaaa", "bbbbbbbbbbbb", "cccccccccccc"]);
    assert_eq!(report.rows[1].standing, Standing::Stale(StaleReason::PidGone));
    assert!(report.rows[1].can_forget() && !report.rows[1].can_end());
    assert!(report.rows[0].can_end() && !report.rows[0].can_forget());
    // A stale host has no uptime and nobody to ask, but keeps what its meta said.
    assert_eq!(report.rows[1].uptime_secs, None);
    assert_eq!(report.rows[1].answer, Answer::NotAsked);
    assert_eq!(report.rows[1].started_at, Some(5));
    // A bare one only has its name and where its files would be.
    assert_eq!(report.rows[2].host_pid, None);
    assert_eq!(report.rows[2].meta_path, PathBuf::from("/h/cccccccccccc.meta"));
    assert_eq!(report.counts(), Counts { attached: 0, detached: 0, orphan: 1, stale: 2 });
}

#[test]
fn a_version_or_protocol_mismatch_is_flagged() {
    let scan = Scan {
        live: vec![
            meta("aaaaaaaaaaaa", 1, "1.6.2", (1, 1)),
            meta("bbbbbbbbbbbb", 1, "1.5.0", (1, 1)),
            meta("cccccccccccc", 1, "1.6.2", (3, 4)),
        ],
        stale: Vec::new(),
    };
    let report = Report::build(inputs(scan));
    let [fine, older, newer] = [&report.rows[0], &report.rows[1], &report.rows[2]];
    assert!(!fine.needs_attention(&report.app));
    assert_eq!(fine.fit, Some(HostFit::Compatible(1)));
    assert!(older.version_differs(&report.app) && older.needs_attention(&report.app));
    assert!(!older.fit_problem());
    assert_eq!(newer.fit, Some(HostFit::TooNew));
    assert!(newer.needs_attention(&report.app));
}

#[test]
fn what_a_host_says_to_get_info_becomes_its_answer() {
    use wormhole_wire::{AppVersion, Counters, Features, HostState, Size, Welcome};
    let info = |state| Welcome {
        proto: 1,
        proto_minor: 2,
        state,
        host_version: AppVersion { major: 1, minor: 6, patch: 2 },
        pid: 101,
        session_id: *b"aaaaaaaaaaaa",
        features: Features::NONE,
        started_at_ms: 0,
        first_block_id: 0,
        size: Size { cols: 100, rows: 30, cell_w: 8, cell_h: 16 },
        counters: Counters {
            epoch: 0,
            history_lines: 12,
            trimmed_total: 0,
            screen_rows: 30,
            alt_screen: true,
            revision: 99,
        },
        exit: None,
        token: None,
    };
    let scan = Scan {
        live: vec![meta("aaaaaaaaaaaa", 1, "1.6.2", (1, 1)), meta("bbbbbbbbbbbb", 1, "1.6.2", (1, 1))],
        stale: Vec::new(),
    };
    let mut i = inputs(scan);
    i.observed.insert(
        "aaaaaaaaaaaa".into(),
        Observed {
            probed: Some(Probed::Answered(info(HostState::Running))),
            rss: Some(40 << 20),
            ..Default::default()
        },
    );
    i.observed.insert(
        "bbbbbbbbbbbb".into(),
        Observed {
            probed: Some(Probed::Failed(ProbeFailure::Connect("refused".into()))),
            ..Default::default()
        },
    );
    let report = Report::build(i);
    let Answer::Yes(host) = &report.rows[0].answer else { panic!("answered: {:?}", report.rows[0].answer) };
    assert_eq!((host.cols, host.rows, host.history_lines, host.revision), (100, 30, 12, 99));
    assert_eq!(host.version, "1.6.2");
    assert!(host.alt_screen && !host.exited);
    assert_eq!(report.rows[0].rss, Some(40 << 20));
    assert!(!report.rows[0].needs_attention(&report.app));
    // A live host that does not answer is worth a look; a stale one would not be asked at all.
    assert_eq!(report.rows[1].answer, Answer::No(ProbeFailure::Connect("refused".into())));
    assert!(report.rows[1].needs_attention(&report.app));
}

#[test]
fn a_host_that_does_not_answer_needs_attention_and_the_text_says_so() {
    let scan = Scan { live: vec![meta("aaaaaaaaaaaa", 1, "1.6.2", (1, 1))], stale: Vec::new() };
    let mut i = inputs(scan);
    let mut stuck = link(false);
    stuck.not_responding_for = Some(std::time::Duration::from_secs(12));
    i.attachments = vec![Attachment {
        host_id: "aaaaaaaaaaaa".into(),
        unavailable: None,
        window: 1,
        title: "t".into(),
        link: Some(stuck),
    }];
    let report = Report::build(i);
    assert!(report.rows[0].needs_attention(&report.app));
    assert!(report.to_text().contains("not responding for 12s"), "{}", report.to_text());
}

#[test]
fn a_lost_link_needs_attention() {
    let scan = Scan { live: vec![meta("aaaaaaaaaaaa", 1, "1.6.2", (1, 1))], stale: Vec::new() };
    let mut i = inputs(scan);
    let mut lost = link(false);
    lost.lost = true;
    i.attachments = vec![Attachment {
        host_id: "aaaaaaaaaaaa".into(),
        unavailable: None,
        window: 1,
        title: "t".into(),
        link: Some(lost),
    }];
    let report = Report::build(i);
    assert!(report.rows[0].needs_attention(&report.app));
}

#[test]
fn uptime_is_counted_from_the_meta_start_and_never_goes_negative() {
    let scan = Scan {
        live: vec![
            meta("aaaaaaaaaaaa", NOW - 3700, "1.6.2", (1, 1)),
            meta("bbbbbbbbbbbb", NOW + 50, "1.6.2", (1, 1)),
        ],
        stale: Vec::new(),
    };
    let report = Report::build(inputs(scan));
    assert_eq!(report.rows[0].uptime_secs, Some(3700));
    assert_eq!(report.rows[1].uptime_secs, Some(0), "a clock set back shows 0, not a huge number");
}

#[test]
fn words_for_durations_sizes_and_times() {
    assert_eq!(duration_words(0), "0s");
    assert_eq!(duration_words(59), "59s");
    assert_eq!(duration_words(75), "1m 15s");
    assert_eq!(duration_words(3700), "1h 1m");
    assert_eq!(duration_words(90_000), "1d 1h");
    assert_eq!(bytes_words(512), "512 B");
    assert_eq!(bytes_words(300 * 1024), "300 KiB");
    assert_eq!(bytes_words(40 * 1024 * 1024 + 512 * 1024), "40.5 MiB");
    assert_eq!(time_words(0), "1970-01-01 00:00:00 UTC");
}

#[test]
fn the_copied_text_has_the_header_and_a_block_a_host() {
    let scan = Scan {
        live: vec![meta("aaaaaaaaaaaa", NOW - 60, "1.5.0", (1, 1))],
        stale: vec![StaleHost { id: "bbbbbbbbbbbb".into(), reason: StaleReason::PidGone, meta: None }],
    };
    let mut i = inputs(scan);
    i.attachments = vec![Attachment {
        host_id: "aaaaaaaaaaaa".into(),
        unavailable: None,
        window: 1,
        title: "server".into(),
        link: Some(link(false)),
    }];
    let text = Report::build(i).to_text();
    for want in [
        "Wormhole 1.6.2 (protocol 1 (minor 2))",
        "hosts folder: /h",
        "persistent terminals: on",
        "host binary: /Apps/wormhole-host",
        "hosts: 1 live (1 attached, 0 detached, 0 orphan), 1 stale",
        "[aaaaaaaaaaaa]",
        "state: attached (window 1, \"server\")",
        "host version: 1.5.0 (this app is 1.6.2)",
        "protocol: 1, compatible (major 1)",
        "link: sync, 10 B in / 5 B out, 3 frames in / 2 out",
        "[bbbbbbbbbbbb]",
        "state: stale (its process is gone)",
    ] {
        assert!(text.contains(want), "missing {want:?} in:\n{text}");
    }
}

/// A file that looks stale but whose host this app is connected to is not offered for forgetting.
#[test]
fn a_stale_looking_host_with_a_live_link_cannot_be_forgotten() {
    let scan = Scan {
        live: Vec::new(),
        stale: vec![StaleHost {
            id: "aaaaaaaaaaaa".into(),
            reason: StaleReason::PidGone,
            meta: Some(meta("aaaaaaaaaaaa", 5, "1.6.2", (1, 1))),
        }],
    };
    let mut i = inputs(scan);
    i.attachments = vec![Attachment {
        host_id: "aaaaaaaaaaaa".into(),
        unavailable: None,
        window: 1,
        title: "t".into(),
        link: Some(link(false)),
    }];
    assert!(!Report::build(i).rows[0].can_forget(), "the link is up");
    let scan = Scan {
        live: Vec::new(),
        stale: vec![StaleHost {
            id: "aaaaaaaaaaaa".into(),
            reason: StaleReason::PidGone,
            meta: Some(meta("aaaaaaaaaaaa", 5, "1.6.2", (1, 1))),
        }],
    };
    let mut i = inputs(scan);
    let mut lost = link(false);
    lost.lost = true;
    i.attachments = vec![Attachment {
        host_id: "aaaaaaaaaaaa".into(),
        unavailable: None,
        window: 1,
        title: "t".into(),
        link: Some(lost),
    }];
    assert!(Report::build(i).rows[0].can_forget(), "a lost link is the host gone");
}

#[test]
fn a_report_that_only_ticked_shows_the_same() {
    let scan = || Scan { live: vec![meta("aaaaaaaaaaaa", NOW - 100, "1.6.2", (1, 1))], stale: Vec::new() };
    let first = Report::build(inputs(scan()));
    let mut later = inputs(scan());
    later.now = NOW + 5; // 105 s: "1m 45s", not "1m 40s"
    assert!(!first.same_as(&Report::build(later)), "the uptime reads differently");
    // A clock tick that rounds to the same words (days show hours only).
    let old = || Scan { live: vec![meta("aaaaaaaaaaaa", 1, "1.6.2", (1, 1))], stale: Vec::new() };
    let mut a = inputs(old());
    a.now = 1 + 3 * 86_400 + 100;
    let mut b = inputs(old());
    b.now = 1 + 3 * 86_400 + 200;
    assert!(Report::build(a).same_as(&Report::build(b)));
    // A fact that changed shows.
    let mut other = inputs(old());
    other.flag_on = false;
    let mut same = inputs(old());
    same.flag_on = true;
    assert!(!Report::build(other).same_as(&Report::build(same)));
}
