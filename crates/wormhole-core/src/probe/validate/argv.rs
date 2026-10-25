//! The argv of a command or a status item: what program it runs, with what, and where a capture
//! may go (sections 3.2 and 3.3 of the design).
//!
//! An argv is data. Nothing here is handed to a shell, so there is no quoting to get wrong; what
//! needs checking is *which* program (it must be listed, and a script must be inside the folder),
//! *what a capture can do* (only as a whole argument, so it is never spliced into a larger string),
//! and `git`, which runs through the app's hardened `GitCmd` and only for the read-only subcommands.

use super::Findings;
use super::program;
use crate::probe::read::ProbeFolder;

/// The `git` subcommands a Probe may run. `branch` is only for listing; see [`check_git`].
pub(crate) const READ_ONLY_GIT: &[&str] =
    &["log", "show", "status", "rev-parse", "diff", "describe", "branch", "ls-files"];

/// Flags no read-only use needs and some can write or run a program with.
const REFUSED_GIT_FLAGS: &[&str] = &[
    "--output",
    "--ext-diff",
    "--textconv",
    "--exec-path",
    "--upload-pack",
    // Verifying a signature runs the user's `gpg.program`.
    "--show-signature",
    "--verify-signatures",
    "--gpg-sign",
];
/// `git branch` flags that create, move, delete or configure a branch.
const BRANCH_WRITE_LONG: &[&str] = &[
    "--delete",
    "--move",
    "--copy",
    "--force",
    "--track",
    "--no-track",
    "--set-upstream-to",
    "--unset-upstream",
    "--edit-description",
    "--create-reflog",
];
/// The same, as the letters of short flags.
const BRANCH_WRITE_SHORT: &str = "dDmMcCfutT";
/// `git branch` flags that make it list rather than create.
const BRANCH_LIST_FLAGS: &[&str] = &[
    "--list",
    "-l",
    "--show-current",
    "--contains",
    "--no-contains",
    "--merged",
    "--no-merged",
    "--points-at",
];

/// Where an argv sits, for the findings and the rules that depend on it.
pub(super) struct Where<'a> {
    /// The manifest field of the argv, such as `contributes.commands[1].run`.
    pub path: &'a str,
    /// Run by the user's own shell in a new terminal, not spawned by Wormhole.
    pub new_terminal: bool,
    /// Captures from a rule can arrive (a command; not a status item).
    pub captures_allowed: bool,
}

/// `$n` as a whole argument: capture `n` (0 is the whole match).
fn capture_index(arg: &str) -> Option<usize> {
    let digits = arg.strip_prefix('$')?;
    (!digits.is_empty() && digits.len() <= 2 && digits.bytes().all(|b| b.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}

/// The captures an argv takes, sorted, each once. Only whole arguments after the program count.
pub(crate) fn captures_of(run: &[String]) -> Vec<usize> {
    let mut found: Vec<usize> = run.iter().skip(1).filter_map(|arg| capture_index(arg)).collect();
    found.sort_unstable();
    found.dedup();
    found
}

/// The program of an argv, when it has one.
pub(crate) fn program_of(run: &[String]) -> Option<&str> {
    run.first().map(String::as_str)
}

pub(super) fn check(folder: &ProbeFolder, run: &[String], at: &Where<'_>, out: &mut Findings) {
    let Some(program) = program_of(run) else {
        out.error("run-empty", at.path, "must name a program");
        return;
    };
    if run.iter().any(|arg| arg.contains('\0')) {
        out.error("run-nul", at.path, "an argument holds a NUL character, which no program can receive");
    }
    check_program(folder, program, at, out);
    for (ix, arg) in run.iter().enumerate().skip(1) {
        if capture_index(arg).is_some() {
            if !at.captures_allowed {
                out.error(
                    "run-capture-not-allowed",
                    format!("{}[{ix}]", at.path),
                    "`$n` is for commands a rule runs; nothing is captured here",
                );
            }
        } else if has_capture_inside(arg) {
            out.error(
                "run-capture-embedded",
                format!("{}[{ix}]", at.path),
                format!("`{arg}` splices a capture into a larger argument; a capture is only ever a whole argument, `\"$1\"`"),
            );
        }
    }
    // Whatever the spelling: the name check above refuses anything but `git`, but this must not rest on it.
    if program.eq_ignore_ascii_case("git") {
        check_git(run, at, out);
    }
}

/// Whether `arg` holds `$` followed by a digit (and is not just that capture).
fn has_capture_inside(arg: &str) -> bool {
    arg.as_bytes().windows(2).any(|pair| pair[0] == b'$' && pair[1].is_ascii_digit())
}

fn check_program(folder: &ProbeFolder, program: &str, at: &Where<'_>, out: &mut Findings) {
    let path = format!("{}[0]", at.path);
    if program.is_empty() {
        out.error("run-program", path, "must name a program");
        return;
    }
    if capture_index(program).is_some() {
        out.error("run-program-capture", path, "the program cannot come from a capture");
        return;
    }
    // A `./script` is checked once, where it is listed (`permissions.process`): it must be plain and
    // exist there, and here it must be listed.
    if program.starts_with("./") {
        // nothing more to say about its path
    } else if program.contains('/') {
        out.error(
            "run-program-path",
            &path,
            format!("`{program}`: a program is a bare name that Wormhole finds, or `./path` inside the Probe's folder"),
        );
        return;
    } else if let Some(why) = program::bare_name_problem(program) {
        out.error("run-program-name", &path, why);
        return;
    }
    if !folder.manifest.permissions.process.iter().any(|allowed| allowed == program) {
        out.error(
            "run-program-not-permitted",
            path,
            format!(
                "`{program}` is not in `permissions.process`, which lists every program the Probe may run"
            ),
        );
    }
}

/// `git` in a Probe's argv is run by the app through `GitCmd` and is read-only: the subcommand is the
/// first argument (a `-c` or `-C` before it would be an option, not a subcommand), from a short
/// list, and the flags that write a file or run a program are refused.
fn check_git(run: &[String], at: &Where<'_>, out: &mut Findings) {
    if at.new_terminal {
        out.error(
            "git-in-new-terminal",
            at.path,
            "a command in a new terminal is run by the user's shell, not through the app's hardened git; use `in = \"background\"`",
        );
    }
    let sub_path = format!("{}[1]", at.path);
    let Some(sub) = run.get(1) else {
        out.error(
            "git-subcommand",
            at.path,
            "`git` needs a subcommand: one of ".to_string() + &READ_ONLY_GIT.join(", "),
        );
        return;
    };
    if !READ_ONLY_GIT.contains(&sub.as_str()) {
        out.error(
            "git-subcommand",
            sub_path,
            format!(
                "`{sub}` is not a read-only subcommand the Probe may run ({}); an option before the subcommand is refused too",
                READ_ONLY_GIT.join(", ")
            ),
        );
        return;
    }
    let rest = &run[2..];
    for (ix, arg) in rest.iter().enumerate() {
        let flag = arg.split('=').next().unwrap_or(arg);
        if REFUSED_GIT_FLAGS.contains(&flag) {
            out.error(
                "git-flag",
                format!("{}[{}]", at.path, ix + 2),
                format!("`{flag}` can write a file or run a program, which a read-only git does not"),
            );
        }
    }
    if sub == "branch" {
        check_branch(rest, at, out);
    }
}

/// `git branch` creates a branch when it is given a name and nothing says "list".
fn check_branch(rest: &[String], at: &Where<'_>, out: &mut Findings) {
    let writes = rest.iter().any(|arg| {
        if let Some(long) = arg.strip_prefix("--") {
            BRANCH_WRITE_LONG.contains(&format!("--{}", long.split('=').next().unwrap_or(long)).as_str())
        } else if let Some(short) = arg.strip_prefix('-') {
            short.chars().any(|c| BRANCH_WRITE_SHORT.contains(c))
        } else {
            false
        }
    });
    let lists = rest.iter().any(|arg| BRANCH_LIST_FLAGS.contains(&arg.split('=').next().unwrap_or(arg)));
    let names_something = rest.iter().any(|arg| !arg.starts_with('-'));
    if writes || (names_something && !lists) {
        out.error(
            "git-branch-writes",
            at.path,
            "`git branch` here may only list: it creates, moves or deletes a branch with a name or `-d`, `-m`, `-f`; add `--list` to list",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn a_capture_is_a_whole_argument_after_the_program() {
        assert_eq!(captures_of(&args("git show --stat $1")), [1]);
        assert_eq!(captures_of(&args("./x.sh $2 $1 $2 $0")), [0, 1, 2]);
        assert_eq!(captures_of(&args("$1 a")), Vec::<usize>::new(), "the program is not an argument");
        assert_eq!(captures_of(&args("x --rev=$1 $a $ $123 \\$1")), Vec::<usize>::new());
    }

    #[test]
    fn a_dollar_and_a_digit_inside_a_larger_argument_is_a_splice() {
        assert!(has_capture_inside("--rev=$1"));
        assert!(has_capture_inside("a$1b"));
        assert!(!has_capture_inside("$name"));
        assert!(!has_capture_inside("cost: 5$"));
    }

    fn git_errors(line: &str, new_terminal: bool) -> Vec<&'static str> {
        let mut out = Findings::default();
        check_git(&args(line), &Where { path: "run", new_terminal, captures_allowed: true }, &mut out);
        out.0.iter().map(|f| f.code).collect()
    }

    #[test]
    fn git_runs_read_only_subcommands_and_nothing_before_them() {
        for ok in [
            "git log -1 --format=%h",
            "git show --stat $1",
            "git status --porcelain",
            "git rev-parse HEAD",
            "git diff --stat",
            "git describe --tags",
            "git branch",
            "git branch -a",
            "git branch --list main*",
            "git branch --contains $1",
            "git branch --show-current",
            "git ls-files",
        ] {
            assert_eq!(git_errors(ok, false), Vec::<&str>::new(), "{ok}");
        }
        for bad in [
            "git",
            "git push",
            "git -c core.pager=x log",
            "git -C /tmp log",
            "git config --global x y",
            "git fetch",
            "git checkout main",
            "git diff --output=/tmp/x",
            "git log --ext-diff",
            "git show --textconv",
            "git log --show-signature",
            "git log --verify-signatures",
            "git show --gpg-sign=x",
        ] {
            assert!(!git_errors(bad, false).is_empty(), "{bad}");
        }
    }

    #[test]
    fn git_branch_only_lists() {
        for bad in [
            "git branch newbranch",
            "git branch -d main",
            "git branch -D main",
            "git branch -m a b",
            "git branch --delete x",
            "git branch -f x",
            "git branch --list -d x",
            "git branch -u origin/x",
            "git branch --set-upstream-to=o/x",
            "git branch $1",
        ] {
            assert_eq!(git_errors(bad, false), ["git-branch-writes"], "{bad}");
        }
    }

    #[test]
    fn git_never_runs_in_a_new_terminal() {
        assert_eq!(git_errors("git log", true), ["git-in-new-terminal"]);
    }
}
