//! What the title row's extras show and say, apart from how wide they are
//! (`layout::title_row` decides which of them fit): the branch's status, the ports a terminal
//! listens on, and the words of each for a tooltip and for VoiceOver. The agent's model and
//! context are `usage::Gauge`, the branch's pull request `git::pr`; the app draws all four.

use crate::git::{RepoStatus, Upstream};
use crate::sidebar::text::count;

/// What the branch chip's neighbours say: a dot while the tree has uncommitted changes, and how far
/// the branch is from its upstream. Only the departures are shown: a clean branch that is level
/// with its upstream has no status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchStatus {
    /// Uncommitted changes: a tracked file edited or staged, or an untracked one.
    pub dirty: bool,
    /// Commits the branch has that its upstream hasn't.
    pub ahead: u32,
    /// Commits its upstream has that the branch hasn't.
    pub behind: u32,
    /// `origin/main`, for the words.
    pub upstream: Option<String>,
}

impl BranchStatus {
    /// What `status` (the Changes tab's, so no git is run for this) says about the branch, if
    /// there is anything to say.
    pub fn of(status: &RepoStatus) -> Option<Self> {
        let (ahead, behind, upstream) = match &status.upstream {
            Some(Upstream { name, ahead, behind }) => (*ahead, *behind, Some(name.clone())),
            None => (0, 0, None),
        };
        let this = Self { dirty: status.dirty, ahead, behind, upstream };
        (this.dirty || ahead > 0 || behind > 0).then_some(this)
    }

    /// "Uncommitted changes, 2 commits ahead of origin/main, 1 commit behind origin/main".
    pub fn words(&self) -> String {
        let against = self.upstream.as_deref().map(|name| format!(" {name}")).unwrap_or_default();
        let mut parts = Vec::new();
        if self.dirty {
            parts.push("Uncommitted changes".to_string());
        }
        if self.ahead > 0 {
            parts.push(format!("{} ahead of{against}", count(self.ahead as usize, "commit", "commits")));
        }
        if self.behind > 0 {
            parts.push(format!("{} behind{against}", count(self.behind as usize, "commit", "commits")));
        }
        parts.join(", ")
    }
}

/// How many ports are drawn as chips of their own; the rest are in the "+n" menu.
pub const PORTS_SHOWN: usize = 2;

/// The ports a terminal listens on, split into the chips drawn and how many more there are.
pub fn ports_shown(ports: &[u16]) -> (&[u16], usize) {
    let shown = ports.len().min(PORTS_SHOWN);
    (&ports[..shown], ports.len() - shown)
}

/// What a port chip says.
pub fn port_label(port: u16) -> String {
    format!("localhost:{port}")
}

/// Where a click on the chip goes: the program on this Mac, spoken to plainly.
pub fn port_url(port: u16) -> String {
    format!("http://localhost:{port}")
}

/// A port chip's tooltip and its VoiceOver label.
pub fn port_words(port: u16) -> String {
    format!("Open localhost:{port} in the browser")
}

/// The "+n" chip's tooltip and label: the ports that didn't get a chip.
pub fn more_ports_words(more: &[u16]) -> String {
    let list = more.iter().map(u16::to_string).collect::<Vec<_>>().join(", ");
    format!("{} more listening: {list}", more.len())
}

#[cfg(test)]
mod tests {
    use super::{BranchStatus, PORTS_SHOWN, more_ports_words, port_label, port_url, port_words, ports_shown};
    use crate::git::{RepoStatus, Upstream};

    fn status(dirty: bool, upstream: Option<(u32, u32)>) -> RepoStatus {
        RepoStatus {
            root: "/repo".into(),
            branch: Some("main".into()),
            files: Vec::new(),
            base_rev: "HEAD".into(),
            compare: None,
            compare_missing: None,
            dirty,
            upstream: upstream.map(|(ahead, behind)| Upstream { name: "origin/main".into(), ahead, behind }),
            superproject: None,
        }
    }

    #[test]
    fn only_departures_have_a_status() {
        assert_eq!(BranchStatus::of(&status(false, None)), None);
        assert_eq!(BranchStatus::of(&status(false, Some((0, 0)))), None, "level with its upstream");
        let dirty = BranchStatus::of(&status(true, None)).expect("dirty");
        assert_eq!((dirty.dirty, dirty.ahead, dirty.behind), (true, 0, 0));
        let both = BranchStatus::of(&status(false, Some((2, 1)))).expect("ahead and behind");
        assert_eq!((both.dirty, both.ahead, both.behind), (false, 2, 1));
    }

    #[test]
    fn the_words_say_what_the_dot_and_the_arrows_mean() {
        let words = |dirty, upstream| BranchStatus::of(&status(dirty, upstream)).expect("status").words();
        assert_eq!(words(true, None), "Uncommitted changes");
        assert_eq!(words(false, Some((2, 0))), "2 commits ahead of origin/main");
        assert_eq!(words(false, Some((0, 1))), "1 commit behind origin/main");
        assert_eq!(
            words(true, Some((2, 1))),
            "Uncommitted changes, 2 commits ahead of origin/main, 1 commit behind origin/main"
        );
    }

    #[test]
    fn a_few_ports_get_chips_and_the_rest_a_plus_n() {
        assert_eq!(ports_shown(&[]), (&[][..], 0));
        assert_eq!(ports_shown(&[3000]), (&[3000][..], 0));
        assert_eq!(ports_shown(&[3000, 5173]), (&[3000, 5173][..], 0));
        assert_eq!(PORTS_SHOWN, 2);
        assert_eq!(ports_shown(&[3000, 5173, 8080, 9229]), (&[3000, 5173][..], 2));
    }

    #[test]
    fn a_port_opens_localhost_in_the_browser() {
        assert_eq!(port_label(3000), "localhost:3000");
        assert_eq!(port_url(3000), "http://localhost:3000");
        assert_eq!(port_words(3000), "Open localhost:3000 in the browser");
        assert_eq!(more_ports_words(&[8080, 9229]), "2 more listening: 8080, 9229");
    }
}
