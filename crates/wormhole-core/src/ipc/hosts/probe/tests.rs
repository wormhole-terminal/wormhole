use std::path::PathBuf;

use wormhole_wire::{Counters, Features, HostState, Role, Size};

use super::*;
use crate::platform::socket::Listener;

fn welcome(id: &str) -> Welcome {
    let mut session_id = [0u8; 12];
    session_id.copy_from_slice(id.as_bytes());
    Welcome {
        proto: 1,
        proto_minor: 2,
        state: HostState::Running,
        host_version: AppVersion { major: 1, minor: 6, patch: 2 },
        pid: 7,
        session_id,
        features: Features::NONE,
        started_at_ms: 0,
        first_block_id: 0,
        size: Size { cols: 80, rows: 24, cell_w: 8, cell_h: 16 },
        counters: Counters {
            epoch: 0,
            history_lines: 5,
            trimmed_total: 0,
            screen_rows: 24,
            alt_screen: false,
            revision: 3,
        },
        exit: None,
        token: None,
    }
}

fn meta(id: &str, socket: &std::path::Path) -> HostMeta {
    HostMeta {
        id: id.into(),
        pid: 1,
        child_pid: None,
        started_at: 1,
        host_version: "1.6.2".into(),
        proto_min: 1,
        proto_max: 1,
        socket: socket.to_path_buf(),
        data_dir: PathBuf::from("/d"),
    }
}

/// A stand-in host: `Welcome` for an admin `Hello` (or `Incompatible`), `Info` for `GetInfo`.
fn serve(listener: Listener, id: &'static str, incompatible: bool) -> std::thread::JoinHandle<bool> {
    std::thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else { return false };
        let mut reader = FrameReader::new(&stream);
        let mut writer = FrameWriter::new(&stream);
        let admin = matches!(
            reader.read_frame(),
            Ok(Some(f)) if matches!(&f.message, Message::Hello(h) if h.role == Role::Admin)
        );
        if !admin {
            return false;
        }
        if incompatible {
            let inc = Incompatible {
                host_min: 7,
                host_max: 8,
                host_version: AppVersion { major: 9, minor: 0, patch: 0 },
            };
            return writer.send(CHANNEL_LOCAL, 0, &Message::Incompatible(inc)).is_ok();
        }
        if writer.send(CHANNEL_LOCAL, 0, &Message::Welcome(welcome(id))).is_err() {
            return false;
        }
        let asked = matches!(reader.read_frame(), Ok(Some(f)) if matches!(f.message, Message::GetInfo));
        asked && writer.send(CHANNEL_LOCAL, 0, &Message::Info(welcome(id))).is_ok()
    })
}

fn short_dir() -> tempfile::TempDir {
    // Short paths: a temp dir can be too long for sun_path.
    tempfile::Builder::new().prefix("hp").tempdir_in("/tmp").expect("temp")
}

#[test]
fn a_host_that_names_itself_answers_get_info() {
    let dir = short_dir();
    let sock = dir.path().join("aaaaaaaaaaaa.sock");
    let server = serve(Listener::bind(&sock).expect("binds"), "aaaaaaaaaaaa", false);
    let probed = probe(&meta("aaaaaaaaaaaa", &sock), Duration::from_secs(5));
    let Probed::Answered(info) = probed else { panic!("answered: {probed:?}") };
    assert_eq!(info.counters.history_lines, 5);
    assert!(server.join().expect("joins"), "the host got Hello and GetInfo");
}

#[test]
fn a_socket_that_names_another_host_is_not_believed() {
    let dir = short_dir();
    let sock = dir.path().join("bbbbbbbbbbbb.sock");
    let server = serve(Listener::bind(&sock).expect("binds"), "zzzzzzzzzzzz", false);
    let probed = probe(&meta("bbbbbbbbbbbb", &sock), Duration::from_secs(5));
    assert!(matches!(&probed, Probed::Failed(why) if why.contains("another host")), "{probed:?}");
    let _ = server.join();
}

#[test]
fn an_incompatible_host_says_which_protocols_it_speaks() {
    let dir = short_dir();
    let sock = dir.path().join("cccccccccccc.sock");
    let server = serve(Listener::bind(&sock).expect("binds"), "cccccccccccc", true);
    let probed = probe(&meta("cccccccccccc", &sock), Duration::from_secs(5));
    assert!(matches!(probed, Probed::Incompatible { host_min: 7, host_max: 8, .. }), "{probed:?}");
    let _ = server.join();
}

#[test]
fn a_missing_socket_is_a_failure_not_a_panic() {
    let dir = short_dir();
    let probed = probe(&meta("dddddddddddd", &dir.path().join("none.sock")), Duration::from_millis(200));
    assert!(matches!(probed, Probed::Failed(_)), "{probed:?}");
}
