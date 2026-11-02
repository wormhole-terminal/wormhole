//! What a failed attach to a session host means for the terminal that wanted it. A host that is
//! **gone** left nothing to attach to, and the terminal comes back as a new shell with its saved
//! text. A host that runs but did not take the app (it refused it, speaks another protocol, is
//! busy, did not answer) still holds the person's program: its terminal keeps the host's id and
//! says why it cannot show it, so the next launch finds the same row instead of adopting the host
//! as a stranger once more.
//!
//! Only a definite "gone" drops a host's id: the connection found nothing ([`AttachError::Unreachable`])
//! *and* the process the meta names is not the host ([`Identity::No`]).

use wormhole_term::client::AttachError;
use wormhole_wire::version::{HostFit, Range};

use super::identity::Identity;

/// What a failed attach comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachFailure {
    /// The host is not there: its id may go.
    Gone,
    /// The host runs and cannot be shown; the terminal keeps its id.
    Unavailable(Unavailable),
}

/// Why a running host cannot be shown, kept with the terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unavailable {
    pub kind: UnavailableKind,
    /// The English of the underlying error, for the log and the report.
    pub detail: String,
}

/// The reasons, which the interface words each in its own way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableKind {
    /// It will not serve this app (a signature that is not the app's, another user, or it closed
    /// the connection without a word).
    Refused,
    /// It speaks a protocol this app does not, older or newer.
    TooOld,
    TooNew,
    /// It has all the connections it allows.
    Busy,
    /// It took the connection and did not answer.
    TimedOut,
    /// Its process runs but nothing answers on its socket.
    Unreachable,
    /// Anything else.
    Failed,
}

/// Decide what `error` means, given what the system says of the host's process (`identity`).
pub fn classify(error: &AttachError, identity: Identity) -> AttachFailure {
    let detail = error.to_string();
    let kind = match error {
        AttachError::Unreachable(_) if identity == Identity::No => return AttachFailure::Gone,
        AttachError::Unreachable(_) => UnavailableKind::Unreachable,
        AttachError::Refused(_) => UnavailableKind::Refused,
        AttachError::Incompatible { host_max, ours, .. } if *host_max < ours.min => UnavailableKind::TooOld,
        AttachError::Incompatible { .. } => UnavailableKind::TooNew,
        AttachError::Busy => UnavailableKind::Busy,
        AttachError::TimedOut => UnavailableKind::TimedOut,
        AttachError::Failed(_) => UnavailableKind::Failed,
    };
    AttachFailure::Unavailable(Unavailable { kind, detail })
}

/// Whether the protocol a host's meta says it speaks fits this build, before any connection: a
/// host this app cannot speak to is not tried. `None` when it fits.
pub fn unfit(proto_min: u16, proto_max: u16) -> Option<Unavailable> {
    let ours = Range::current();
    let kind = match ours.fit(Range::new(proto_min, proto_max)) {
        HostFit::Compatible(_) => return None,
        HostFit::TooOld => UnavailableKind::TooOld,
        HostFit::TooNew => UnavailableKind::TooNew,
    };
    Some(Unavailable {
        kind,
        detail: format!(
            "the session host speaks protocol {proto_min}..{proto_max}, this app {}..{}",
            ours.min, ours.max
        ),
    })
}

/// [`classify`] for the host `id` of `hosts_dir`, looking at its meta for the process's identity: no
/// meta file at all is a host that is gone; one that cannot be read, or says nothing certain, is not.
pub fn failure_of(hosts_dir: &std::path::Path, id: &str, error: &AttachError) -> AttachFailure {
    let identity = match super::find(hosts_dir, id) {
        Ok(meta) => super::identity::identify(&meta),
        Err(_) if !super::meta_path(hosts_dir, id).exists() => Identity::No,
        Err(_) => Identity::Unknown,
    };
    classify(error, identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind_of(error: AttachError, identity: Identity) -> Option<UnavailableKind> {
        match classify(&error, identity) {
            AttachFailure::Gone => None,
            AttachFailure::Unavailable(u) => Some(u.kind),
        }
    }

    #[test]
    fn only_nothing_to_connect_to_and_no_such_process_is_gone() {
        let unreachable = || AttachError::Unreachable("no such file".into());
        assert_eq!(classify(&unreachable(), Identity::No), AttachFailure::Gone);
        // The process still runs, or cannot be told apart: the host is not gone.
        assert_eq!(kind_of(unreachable(), Identity::Yes), Some(UnavailableKind::Unreachable));
        assert_eq!(kind_of(unreachable(), Identity::Unknown), Some(UnavailableKind::Unreachable));
    }

    #[test]
    fn every_other_failure_keeps_the_host_whatever_the_process_says() {
        let ours = Range::current();
        let cases = [
            (AttachError::Refused("no".into()), UnavailableKind::Refused),
            (AttachError::Busy, UnavailableKind::Busy),
            (AttachError::TimedOut, UnavailableKind::TimedOut),
            (AttachError::Failed("x".into()), UnavailableKind::Failed),
            (
                AttachError::Incompatible { host_min: 0, host_max: 0, host_version: "0.1.0".into(), ours },
                UnavailableKind::TooOld,
            ),
            (
                AttachError::Incompatible { host_min: 9, host_max: 9, host_version: "9.0.0".into(), ours },
                UnavailableKind::TooNew,
            ),
        ];
        for (error, want) in cases {
            for identity in [Identity::Yes, Identity::No, Identity::Unknown] {
                assert_eq!(kind_of(error.clone(), identity), Some(want), "{error:?} / {identity:?}");
            }
        }
    }

    #[test]
    fn the_protocol_a_meta_names_is_judged_before_connecting() {
        let ours = Range::current();
        assert_eq!(unfit(ours.min, ours.max), None);
        assert_eq!(unfit(ours.max, ours.max + 3).map(|u| u.kind), None, "a range that reaches ours fits");
        assert_eq!(unfit(ours.max + 1, ours.max + 2).map(|u| u.kind), Some(UnavailableKind::TooNew));
    }

    #[test]
    fn a_host_is_gone_when_its_meta_is_gone_or_its_process_is_not_it() {
        let dir = tempfile::Builder::new().prefix("ha").tempdir_in("/tmp").expect("temp");
        let d = dir.path();
        let unreachable = AttachError::Unreachable("no such file".into());
        // No meta file: gone.
        assert_eq!(failure_of(d, "aaaaaaaaaaaa", &unreachable), AttachFailure::Gone);
        // A meta whose pid is this test program (not a host): gone.
        let text = format!(
            "format=1\nid=bbbbbbbbbbbb\npid={}\nstarted_at=1\nhost_version=1.0.0\nproto_min=1\nproto_max=1\n\
             socket=/s/x.sock\ndata_dir=/d\n",
            std::process::id()
        );
        std::fs::write(d.join("bbbbbbbbbbbb.meta"), text).expect("meta");
        assert_eq!(failure_of(d, "bbbbbbbbbbbb", &unreachable), AttachFailure::Gone);
        // A meta whose process is a running host: it is there, only the connection failed.
        let host = crate::ipc::hosts::spawn_fake_host(d);
        let text = format!(
            "format=1\nid=cccccccccccc\npid={}\nstarted_at={}\nhost_version=1.0.0\nproto_min=1\nproto_max=1\n\
             socket=/s/x.sock\ndata_dir=/d\n",
            host.pid(),
            host.started_at
        );
        std::fs::write(d.join("cccccccccccc.meta"), text).expect("meta");
        assert!(matches!(
            failure_of(d, "cccccccccccc", &unreachable),
            AttachFailure::Unavailable(Unavailable { kind: UnavailableKind::Unreachable, .. })
        ));
    }
}
