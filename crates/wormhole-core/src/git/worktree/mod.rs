//! A git worktree per terminal: a branch of its own in a folder of its own, so several agents
//! work on one repository without touching each other. This is the one exception to "git is
//! read-only", and it is kept narrow: `add` and `remove` are the only writes (through
//! `write::GitWrite`), they run only on the user's action, confirmed in the app, and never in a
//! project the user hasn't trusted (the app checks that before asking here).
//!
//! What this module decides: the name a branch may have ([`branch_name`], git's own rules), where
//! the folder goes ([`folder_for`]: outside the repository, in a sibling `<repo>-worktrees` folder
//! or under the configured base), when a worktree may be removed (never with changes or untracked
//! files in it; git refuses too, as the remove is never forced), and the set of worktrees Wormhole
//! made ([`Worktrees`], kept in the store across launches), so closing such a terminal can offer
//! to remove it and the sidebar can mark it. A worktree the user made by hand is never removed.
//!
//! Each worktree Wormhole makes also gets ports of its own ([`Worktrees::free_ports`]): a block of
//! [`PORT_BLOCK`] that no other of its worktrees has and nothing listened on when it was made, told
//! to its terminals as `WORMHOLE_PORT` and `WORMHOLE_PORTS` ([`Worktree::env`]), so the dev servers
//! of several worktrees of one repository don't fight over the same port.

mod build_folders;

pub use build_folders::clone_build_folders;

use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use super::write::GitWrite;
use super::{GitCmd, status_summary, worktree_of};

/// The store's meta key for [`Worktrees`].
pub const STORE_KEY: &str = "worktrees";

/// The folder next to a repository that holds its worktrees: `<repo>-worktrees`.
const SIBLING_SUFFIX: &str = "-worktrees";

/// How many ports a worktree gets, and the range their blocks are taken from: high enough to stay
/// clear of the usual dev-server defaults (3000, 5173, 8080, …), below the ephemeral ports macOS
/// hands out (49152 on), and room for a hundred worktrees.
pub const PORT_BLOCK: u16 = 10;
pub const PORTS_FROM: u16 = 41000;
pub const PORTS_TO: u16 = 42000;

/// The variables a worktree's terminals get: its first port, and the whole block ("41000-41009").
pub const PORT_VAR: &str = "WORMHOLE_PORT";
pub const PORTS_VAR: &str = "WORMHOLE_PORTS";

/// A worktree Wormhole made: of which repository (its main working tree), on which branch, where.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worktree {
    pub repo: PathBuf,
    pub branch: String,
    pub folder: PathBuf,
    /// The first of its [`PORT_BLOCK`] ports; `None` for one made before worktrees had ports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// The build folders cloned into it when it was made ([`clone_build_folders`]), relative to
    /// `folder`: [`remove`] deletes exactly these. Empty for one made without or before that.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cloned: Vec<PathBuf>,
}

impl Worktree {
    /// What its terminals get in their environment: its ports, when it has some.
    pub fn env(&self) -> Vec<(&'static str, String)> {
        let Some(first) = self.port else { return Vec::new() };
        let last = first.saturating_add(PORT_BLOCK - 1);
        vec![(PORT_VAR, first.to_string()), (PORTS_VAR, format!("{first}-{last}"))]
    }
}

/// `input` as a branch name git accepts (`git check-ref-format --branch`), trimmed, or why not,
/// in the dialog's words. Checked here so the dialog can say so as the name is typed, and so a
/// name never reads as an option (`-b` takes it as a value, but it still must not start with `-`).
pub fn branch_name(input: &str) -> Result<String, String> {
    let name = input.trim();
    if name.is_empty() {
        return Err("Name the branch".into());
    }
    if name.starts_with('-') {
        return Err("A branch name can't start with -".into());
    }
    if name == "@" {
        return Err("@ alone isn't a branch name".into());
    }
    if let Some(c) = name.chars().find(|&c| c.is_control() || " ~^:?*[\\".contains(c)) {
        return Err(match c {
            ' ' => "A branch name can't contain spaces".into(),
            c if c.is_control() => "A branch name can't contain control characters".into(),
            c => format!("A branch name can't contain {c}"),
        });
    }
    for pair in ["..", "@{", "//"] {
        if name.contains(pair) {
            return Err(format!("A branch name can't contain {pair}"));
        }
    }
    if name.starts_with('/') || name.ends_with('/') {
        return Err("A branch name can't start or end with /".into());
    }
    if name.ends_with('.') {
        return Err("A branch name can't end with .".into());
    }
    if name.split('/').any(|part| part.starts_with('.') || part.ends_with(".lock")) {
        return Err("A part of a branch name can't start with . or end with .lock".into());
    }
    Ok(name.to_string())
}

/// The folder a worktree on `branch` gets: a `/` in the name becomes `-`, so every worktree of a
/// repository sits flat in one folder.
pub fn folder_name(branch: &str) -> String {
    branch.replace('/', "-")
}

/// Where a worktree of `repo` (its main working tree) on `branch` goes: under `base` when the
/// user set one (`worktree_base`), in a folder named after the repository; otherwise beside the
/// repository, in `<repo>-worktrees`. Always outside the repository.
pub fn folder_for(repo: &Path, branch: &str, base: Option<&Path>) -> PathBuf {
    let name = repo.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "repo".into());
    let holder = match base {
        Some(base) => base.join(&name),
        None => repo.parent().unwrap_or(repo).join(format!("{name}{SIBLING_SUFFIX}")),
    };
    holder.join(folder_name(branch))
}

/// What creating a worktree of `repo` on `branch` would do, or why it can't be done: the branch
/// name checked, the folder placed, and refused when it would sit inside the repository (a base
/// set to the repository itself) or is already there.
pub fn plan(repo: &Path, branch: &str, base: Option<&Path>) -> Result<Worktree, String> {
    let branch = branch_name(branch)?;
    let folder = folder_for(repo, &branch, base);
    if folder.starts_with(repo) {
        return Err(format!("{} is inside the repository", folder.display()));
    }
    if folder.exists() {
        return Err(format!("{} already exists", folder.display()));
    }
    Ok(Worktree { repo: repo.to_path_buf(), branch, folder, port: None, cloned: Vec::new() })
}

/// Whether `repo` has a local branch called `branch`.
fn branch_exists(repo: &Path, branch: &str) -> bool {
    GitCmd::new(repo)
        .args(["rev-parse", "--verify", "--quiet"])
        .revs([format!("refs/heads/{branch}")])
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Make `worktree` (from [`plan`]): its branch is created from HEAD, or checked out as it is
/// when the repository has it already. Blocking: run it off the UI thread.
pub fn add(worktree: &Worktree) -> Result<()> {
    if let Some(holder) = worktree.folder.parent() {
        std::fs::create_dir_all(holder).with_context(|| format!("creating {}", holder.display()))?;
    }
    let new_branch = !branch_exists(&worktree.repo, &worktree.branch);
    GitWrite::worktree_add(&worktree.repo, &worktree.folder, &worktree.branch, new_branch)
}

/// Whether the working tree at `folder` has changed or untracked files: what keeps it from being
/// removed. A folder that isn't a repository counts as clean (there is nothing to lose in git's
/// terms, and git itself decides at the remove).
pub fn is_dirty(folder: &Path) -> Result<bool> {
    Ok(status_summary(folder, None)?.is_some_and(|status| !status.files.is_empty()))
}

/// Remove `worktree`'s folder from disk and from the repository, keeping its branch. Refused when
/// the folder has changes, and when it isn't a worktree of that repository any more (something
/// else is there now). The build folders cloned into it are deleted first (a failure there leaves the
/// worktree in place). A folder that is already gone leaves nothing to do.
pub fn remove(worktree: &Worktree) -> Result<()> {
    if !worktree.folder.exists() {
        return Ok(());
    }
    let same = |a: &Path, b: &Path| a.canonicalize().ok() == b.canonicalize().ok();
    match worktree_of(&worktree.folder) {
        Some(main) if same(&main, &worktree.repo) => {}
        _ => anyhow::bail!("{} is not a worktree of {}", worktree.folder.display(), worktree.repo.display()),
    }
    if is_dirty(&worktree.folder)? {
        anyhow::bail!("{} has uncommitted changes", worktree.folder.display());
    }
    build_folders::remove_clones(worktree)?;
    GitWrite::worktree_remove(&worktree.repo, &worktree.folder)
}

/// The worktrees Wormhole made, across launches. Only these are ever offered for removal.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worktrees {
    made: Vec<Worktree>,
}

impl Worktrees {
    /// What the store holds under [`STORE_KEY`]; nothing readable is none.
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }

    pub fn record(&mut self, worktree: Worktree) {
        self.forget(&worktree.folder);
        self.made.push(worktree);
    }

    /// Drop the worktree at `folder`; whether there was one.
    pub fn forget(&mut self, folder: &Path) -> bool {
        let before = self.made.len();
        self.made.retain(|w| w.folder != folder);
        self.made.len() != before
    }

    /// The worktree `path` is in (its folder, or anything under it).
    pub fn find(&self, path: &Path) -> Option<&Worktree> {
        self.made.iter().find(|w| path.starts_with(&w.folder))
    }

    /// Drop the worktrees whose folder is gone (removed by hand); whether any was.
    pub fn prune(&mut self) -> bool {
        let before = self.made.len();
        self.made.retain(|w| w.folder.exists());
        self.made.len() != before
    }

    pub fn iter(&self) -> impl Iterator<Item = &Worktree> {
        self.made.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.made.is_empty()
    }

    /// The first port of a block for a new worktree: the lowest in [`PORTS_FROM`]..[`PORTS_TO`]
    /// that none of these worktrees has and of which `free` says every port is free. `None` when
    /// every block is taken.
    pub fn free_ports(&self, free: impl Fn(u16) -> bool) -> Option<u16> {
        (PORTS_FROM..PORTS_TO).step_by(PORT_BLOCK as usize).find(|&first| {
            let block = first..first + PORT_BLOCK;
            !self.made.iter().any(|w| w.port.is_some_and(|port| block.contains(&port)))
                && block.clone().all(&free)
        })
    }
}

/// Whether nothing on this Mac listens on `port`: it can be bound on the loopback addresses and on
/// every address, IPv4 and IPv6 alike. One bind is not enough: std's listeners reuse addresses, so
/// on macOS `127.0.0.1` binds fine while a dev server listens on `0.0.0.0`, `::` (Node's default)
/// or `::1` (`localhost` on a recent Node). A Mac without IPv6 has no IPv6 address to take.
pub fn port_free(port: u16) -> bool {
    let hosts: [IpAddr; 4] = [
        Ipv4Addr::LOCALHOST.into(),
        Ipv4Addr::UNSPECIFIED.into(),
        Ipv6Addr::LOCALHOST.into(),
        Ipv6Addr::UNSPECIFIED.into(),
    ];
    hosts.into_iter().all(|host| match TcpListener::bind((host, port)) {
        Ok(_) => true,
        Err(err) => host.is_ipv6() && err.kind() == ErrorKind::AddrNotAvailable,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        PORT_BLOCK, PORT_VAR, PORTS_FROM, PORTS_VAR, Worktree, Worktrees, add, branch_name, folder_for,
        is_dirty, plan, port_free, remove,
    };
    use crate::git::testing::{git, repo_with, write};
    use crate::git::worktree_of;

    #[test]
    fn branch_names_follow_gits_rules() {
        assert_eq!(branch_name(" fix/login ").as_deref(), Ok("fix/login"));
        assert_eq!(branch_name("feature-1.2").as_deref(), Ok("feature-1.2"));
        assert_eq!(branch_name("agent/a@b").as_deref(), Ok("agent/a@b"));
        for bad in [
            "",
            "  ",
            "-x",
            "--output=/x",
            "a b",
            "a..b",
            "a@{b",
            "a//b",
            "/a",
            "a/",
            "a.",
            ".a",
            "x/.a",
            "a.lock",
            "a/b.lock",
            "a~b",
            "a^b",
            "a:b",
            "a?b",
            "a*b",
            "a[b",
            "a\\b",
            "a\x07b",
            "@",
        ] {
            assert!(branch_name(bad).is_err(), "{bad:?} must be refused");
        }
    }

    #[test]
    fn each_worktree_gets_a_block_of_ports_no_other_has() {
        let made = |port: u16| Worktree {
            repo: PathBuf::from("/r"),
            branch: format!("b{port}"),
            folder: PathBuf::from(format!("/w/{port}")),
            port: Some(port),
            cloned: Vec::new(),
        };
        let mut set = Worktrees::default();
        assert_eq!(set.free_ports(|_| true), Some(PORTS_FROM));
        set.record(made(PORTS_FROM));
        assert_eq!(set.free_ports(|_| true), Some(PORTS_FROM + PORT_BLOCK));
        // A port something else listens on puts its whole block out of reach.
        let busy = PORTS_FROM + PORT_BLOCK + 3;
        assert_eq!(set.free_ports(|port| port != busy), Some(PORTS_FROM + 2 * PORT_BLOCK));
        // A worktree that is gone gives its block back.
        set.forget(Path::new(&format!("/w/{PORTS_FROM}")));
        assert_eq!(set.free_ports(|_| true), Some(PORTS_FROM));
        assert_eq!(set.free_ports(|_| false), None, "nothing free");

        assert_eq!(
            made(41020).env(),
            [(PORT_VAR, "41020".to_string()), (PORTS_VAR, "41020-41029".to_string())]
        );
        assert!(Worktree { port: None, ..made(1) }.env().is_empty());
        // Worktrees stored before ports existed still load, without one.
        let old = r#"{"made":[{"repo":"/r","branch":"a","folder":"/w/a"}]}"#;
        let loaded = Worktrees::from_json(old);
        assert_eq!(loaded.iter().next().map(|w| w.port), Some(None));
        assert_eq!(Worktrees::from_json(&set.to_json()), set);
    }

    #[test]
    fn a_port_listened_on_at_any_address_is_not_free() {
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener};
        let hosts: [IpAddr; 4] = [
            Ipv4Addr::LOCALHOST.into(),
            Ipv4Addr::UNSPECIFIED.into(),
            Ipv6Addr::LOCALHOST.into(),
            Ipv6Addr::UNSPECIFIED.into(),
        ];
        for host in hosts {
            // A machine without IPv6 (some CI containers) can't listen there to begin with.
            let Ok(listener) = TcpListener::bind((host, 0)) else { continue };
            let port = listener.local_addr().expect("addr").port();
            assert!(!port_free(port), "a listener on {host} leaves port {port} taken");
            drop(listener);
        }
    }

    #[test]
    fn the_folder_sits_beside_the_repository_or_under_the_base() {
        let repo = Path::new("/Users/me/src/api");
        assert_eq!(
            folder_for(repo, "fix/login", None),
            PathBuf::from("/Users/me/src/api-worktrees/fix-login")
        );
        assert_eq!(
            folder_for(repo, "main", Some(Path::new("/Users/me/worktrees"))),
            PathBuf::from("/Users/me/worktrees/api/main")
        );
        // Never inside the repository, whatever the base says.
        let inside = plan(repo, "fix", Some(repo)).expect_err("refused");
        assert!(inside.contains("inside the repository"), "{inside}");
        assert!(plan(repo, "bad name", None).is_err());
    }

    /// The whole life of a worktree against a throwaway repository: made on a new branch from
    /// HEAD, with the files checked out and no hook run; refused for removal while it has
    /// changes; removed once clean, its branch kept.
    #[test]
    fn a_worktree_is_added_without_hooks_and_removed_only_when_clean() {
        let repo = repo_with(&[("a.txt", "hello\n")]);
        let base = tempfile::tempdir().expect("tempdir");
        // A post-checkout hook that leaves a trace: it must never run.
        write(repo.path(), ".git/hooks/post-checkout", "#!/bin/sh\ntouch \"$PWD/hooked\"\n");
        {
            let hook = repo.path().join(".git/hooks/post-checkout");
            std::fs::set_permissions(&hook, crate::platform::fs::permissions(0o755)).expect("chmod");
        }

        let worktree = plan(repo.path(), "fix/login", Some(base.path())).expect("plans");
        assert_eq!(
            worktree.folder,
            base.path().join(repo.path().file_name().expect("name")).join("fix-login")
        );
        add(&worktree).expect("adds");
        assert_eq!(std::fs::read_to_string(worktree.folder.join("a.txt")).expect("checked out"), "hello\n");
        assert!(
            !worktree.folder.join("hooked").exists() && !repo.path().join("hooked").exists(),
            "no hook ran"
        );
        let main = worktree_of(&worktree.folder).expect("a worktree");
        assert_eq!(main.canonicalize().ok(), repo.path().canonicalize().ok());
        // The same plan again: the folder is there now.
        assert!(plan(repo.path(), "fix/login", Some(base.path())).is_err());

        // Changes keep it: untracked files count as much as edits.
        write(&worktree.folder, "b.txt", "new\n");
        assert!(is_dirty(&worktree.folder).expect("status"));
        let refused = remove(&worktree).expect_err("refused while dirty");
        assert!(refused.to_string().contains("uncommitted changes"), "{refused:#}");
        assert!(worktree.folder.exists());
        std::fs::remove_file(worktree.folder.join("b.txt")).expect("clean again");
        assert!(!is_dirty(&worktree.folder).expect("status"));

        remove(&worktree).expect("removes");
        assert!(!worktree.folder.exists());
        // The branch stays; and a folder that is gone is nothing to remove.
        git(repo.path(), &["rev-parse", "--verify", "--quiet", "refs/heads/fix/login"]);
        remove(&worktree).expect("nothing to do");
    }

    #[test]
    fn an_existing_branch_is_checked_out_as_it_is() {
        let repo = repo_with(&[("a.txt", "one\n")]);
        let base = tempfile::tempdir().expect("tempdir");
        git(repo.path(), &["branch", "feature"]);
        let worktree = plan(repo.path(), "feature", Some(base.path())).expect("plans");
        add(&worktree).expect("adds");
        let head = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&worktree.folder)
            .output()
            .expect("git runs");
        assert_eq!(String::from_utf8_lossy(&head.stdout).trim(), "feature");
        // Something else at the folder is never removed as a worktree.
        let stranger = Worktree { repo: base.path().to_path_buf(), ..worktree.clone() };
        assert!(remove(&stranger).is_err());
        remove(&worktree).expect("removes");
    }

    #[test]
    fn the_set_of_made_worktrees_round_trips_and_prunes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let kept = dir.path().join("kept");
        std::fs::create_dir_all(&kept).expect("mkdir");
        let one = Worktree {
            repo: PathBuf::from("/r"),
            branch: "a".into(),
            folder: kept.clone(),
            port: None,
            cloned: Vec::new(),
        };
        let gone = Worktree {
            repo: PathBuf::from("/r"),
            branch: "b".into(),
            folder: dir.path().join("gone"),
            port: None,
            cloned: Vec::new(),
        };
        let mut set = Worktrees::default();
        set.record(one.clone());
        set.record(gone.clone());
        set.record(one.clone());
        assert_eq!(set.iter().count(), 2, "recorded once per folder");
        assert_eq!(set.find(&kept.join("src/main.rs")), Some(&one), "anything under the folder");
        assert_eq!(set.find(Path::new("/elsewhere")), None);
        let back = Worktrees::from_json(&set.to_json());
        assert_eq!(back, set);
        assert_eq!(Worktrees::from_json("nonsense"), Worktrees::default());
        assert!(set.prune(), "the gone one");
        assert_eq!(set.iter().collect::<Vec<_>>(), [&one]);
        assert!(!set.prune());
        assert!(set.forget(&kept) && !set.forget(&kept));
        assert!(set.is_empty());
    }
}
