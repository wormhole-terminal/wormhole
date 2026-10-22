//! What the app does with the session hosts it finds when it starts (`repo-docs/docs/session-host.md`,
//! section 10, "Restore at launch"), as plain data the app only carries out.
//!
//! The hosts folder is scanned once. A saved terminal whose row names a live host attaches to it; a
//! row whose host is gone takes today's restore path (a new shell, the saved text replayed, the
//! marker); a live host that no row of any profile names is an orphan, shown in the window's
//! Recovered group and never ended by the app on its own. Of the terminals that attach, the panes of
//! the selected split are fed a repaint at once and the rest attach headless.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::HostMeta;

/// The live hosts found at launch, by id, and which of them nobody's row names.
#[derive(Debug, Default)]
pub struct LaunchHosts {
    /// Every live host's socket, until a terminal attaches to it (each host is attached once: a
    /// second controller would take it over from the first).
    live: HashMap<String, PathBuf>,
    /// The live hosts no row names, oldest first, until the app adopts them.
    orphans: Vec<HostMeta>,
}

/// How a saved terminal comes back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Restore {
    /// Its host runs: attach to the socket.
    Attach(PathBuf),
    /// No host (none was named, or it is gone): start a shell and replay the saved text, as before
    /// hosts.
    Replay,
}

impl LaunchHosts {
    /// Match the live hosts against `named`, the host ids the saved rows of every profile name (a
    /// host another profile's terminal runs in is that profile's, not an orphan).
    pub fn plan<'a>(named: impl IntoIterator<Item = &'a str>, live: Vec<HostMeta>) -> Self {
        let named: HashSet<&str> = named.into_iter().collect();
        let orphans = live.iter().filter(|meta| !named.contains(meta.id.as_str())).cloned().collect();
        let live = live.into_iter().map(|meta| (meta.id, meta.socket)).collect();
        Self { live, orphans }
    }

    /// How the terminal whose row names `host_id` comes back. An attach takes the host: asked again
    /// for the same id, it is [`Restore::Replay`].
    pub fn restore(&mut self, host_id: Option<&str>) -> Restore {
        match host_id.and_then(|id| self.live.remove(id)) {
            Some(socket) => Restore::Attach(socket),
            None => Restore::Replay,
        }
    }

    /// The orphans, once: the window that adopts them attaches to each through [`Self::restore`].
    pub fn take_orphans(&mut self) -> Vec<HostMeta> {
        std::mem::take(&mut self.orphans)
    }

    /// No live host is left to attach to.
    pub fn is_empty(&self) -> bool {
        self.live.is_empty()
    }
}

/// The terminals of a restored window that are on screen as it opens: the selected one (`active`,
/// an index into `sessions`, clamped as the window clamps it) and the other members of its split.
/// `sessions` is each saved terminal's id and split, in the window's order.
pub fn shown_at_launch(active: usize, sessions: &[(i64, Option<i64>)]) -> Vec<i64> {
    let Some(&(id, split)) = sessions.get(active.min(sessions.len().saturating_sub(1))) else {
        return Vec::new();
    };
    match split {
        None => vec![id],
        Some(split) => sessions.iter().filter(|(_, s)| *s == Some(split)).map(|(id, _)| *id).collect(),
    }
}

/// The host a terminal's row is saved with: its own while it has one, none once the host was lost
/// (a row naming a dead host would only be skipped next time) and none while Quit and End All
/// Terminals ends every host (`ending`), so the saves on the way out don't name hosts that are
/// going away and the terminals come back as text.
pub fn persisted_host(host: Option<&str>, lost: bool, ending: bool) -> Option<String> {
    host.filter(|_| !lost && !ending).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{LaunchHosts, Restore, persisted_host, shown_at_launch};
    use crate::ipc::hosts::HostMeta;

    fn meta(id: &str, started_at: u64) -> HostMeta {
        HostMeta {
            id: id.into(),
            pid: 100,
            child_pid: Some(101),
            started_at,
            host_version: "0.3.0".into(),
            proto_min: 1,
            proto_max: 1,
            socket: PathBuf::from(format!("/h/{id}.sock")),
            data_dir: PathBuf::from("/d"),
        }
    }

    #[test]
    fn a_row_whose_host_lives_attaches_and_the_rest_replay() {
        let live = vec![meta("aaaaaaaaaaaa", 1), meta("bbbbbbbbbbbb", 2)];
        let mut hosts = LaunchHosts::plan(["aaaaaaaaaaaa", "cccccccccccc"], live);
        assert_eq!(
            hosts.restore(Some("aaaaaaaaaaaa")),
            Restore::Attach(Path::new("/h/aaaaaaaaaaaa.sock").into())
        );
        assert_eq!(hosts.restore(Some("cccccccccccc")), Restore::Replay, "its host is gone");
        assert_eq!(hosts.restore(None), Restore::Replay, "a terminal that ran in the app");
        assert_eq!(hosts.restore(Some("aaaaaaaaaaaa")), Restore::Replay, "a host is attached once");
    }

    #[test]
    fn a_live_host_nobody_names_is_an_orphan_and_can_be_adopted_once() {
        let live = vec![meta("aaaaaaaaaaaa", 1), meta("bbbbbbbbbbbb", 2), meta("dddddddddddd", 3)];
        let mut hosts = LaunchHosts::plan(["bbbbbbbbbbbb"], live);
        let orphans: Vec<String> = hosts.take_orphans().into_iter().map(|m| m.id).collect();
        assert_eq!(orphans, ["aaaaaaaaaaaa", "dddddddddddd"], "oldest first, as the scan lists them");
        assert!(hosts.take_orphans().is_empty(), "adopted once");
        // The window that adopts one attaches to it like to a saved terminal's.
        assert!(matches!(hosts.restore(Some("dddddddddddd")), Restore::Attach(_)));
        assert!(matches!(hosts.restore(Some("bbbbbbbbbbbb")), Restore::Attach(_)));
        assert!(!hosts.is_empty());
        assert!(matches!(hosts.restore(Some("aaaaaaaaaaaa")), Restore::Attach(_)));
        assert!(hosts.is_empty());
    }

    #[test]
    fn a_host_another_profile_names_is_not_an_orphan() {
        let mut hosts = LaunchHosts::plan(["aaaaaaaaaaaa", "bbbbbbbbbbbb"], vec![meta("bbbbbbbbbbbb", 1)]);
        assert!(hosts.take_orphans().is_empty());
    }

    #[test]
    fn the_selected_terminal_and_its_split_are_on_screen_at_launch() {
        let sessions = [(1, None), (2, Some(10)), (3, Some(10)), (4, None)];
        assert_eq!(shown_at_launch(0, &sessions), [1]);
        assert_eq!(shown_at_launch(2, &sessions), [2, 3], "the whole split");
        assert_eq!(shown_at_launch(9, &sessions), [4], "clamped to the last, as the window does");
        assert!(shown_at_launch(0, &[]).is_empty());
    }

    #[test]
    fn quit_keeps_a_running_host_and_drops_a_lost_or_ending_one() {
        assert_eq!(persisted_host(Some("aaaaaaaaaaaa"), false, false).as_deref(), Some("aaaaaaaaaaaa"));
        assert_eq!(persisted_host(Some("aaaaaaaaaaaa"), true, false), None, "the host crashed");
        assert_eq!(persisted_host(Some("aaaaaaaaaaaa"), false, true), None, "Quit and End All");
        assert_eq!(persisted_host(None, false, false), None, "in the app's own process");
    }
}
