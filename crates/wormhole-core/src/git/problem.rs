//! What the Changes and History tabs say when git cannot answer: git is missing (and how to install it), or git failed (its first
//! line, and the whole message to copy). One rule and its sentences for the GPUI and the Swift app; each draws it.

use super::GitError;

/// The git tab a problem is drawn in, which its wording and its Retry are for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitTab {
    Changes,
    History,
}

/// What a problem says: a line for everyone, a hint, and something to read or run.
#[derive(Debug, PartialEq, Eq)]
pub struct Wording {
    pub headline: &'static str,
    pub hint: Option<&'static str>,
    /// Shown in the code font: the command to run, or git's own first line.
    pub detail: String,
    /// The whole message, for Copy error. `None` when there is nothing more to copy.
    pub copy: Option<String>,
}

/// How a git problem reads in `tab`. `None` for a folder that is not in a repository,
/// which is the plain empty note.
pub fn wording(tab: GitTab, error: &GitError) -> Option<Wording> {
    match error {
        GitError::NotARepo => None,
        GitError::GitMissing => Some(Wording {
            headline: match tab {
                GitTab::Changes => "Changes need git, and it isn't installed.",
                GitTab::History => "History needs git, and it isn't installed.",
            },
            hint: Some("Install the Xcode Command Line Tools, then retry."),
            detail: "xcode-select --install".into(),
            copy: None,
        }),
        failed => {
            let message = failed.to_string();
            if let Some(path) = dubious_path(&message) {
                // The command is shown, never run: marking a folder safe is the person's decision.
                return Some(Wording {
                    headline: "git doesn't trust who owns this folder.",
                    hint: Some("If you trust it, run this once, then retry."),
                    detail: format!("git config --global --add safe.directory '{path}'"),
                    copy: Some(message),
                });
            }
            Some(Wording {
                headline: "Couldn't read this repository.",
                hint: None,
                detail: message.lines().next().unwrap_or_default().to_string(),
                copy: Some(message),
            })
        }
    }
}

/// The folder git refuses for its owner's sake ("detected dubious ownership in repository at '<path>'"),
/// when it can be put in a shell command as it stands: a quote or a control character in it, from a
/// folder name someone else chose, leaves the generic wording.
fn dubious_path(message: &str) -> Option<&str> {
    let rest = message.lines().next()?.split_once("dubious ownership in repository at '")?.1;
    let path = rest.strip_suffix('\'')?;
    (!path.is_empty() && !path.contains('\'') && !path.chars().any(char::is_control)).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::{GitTab, Wording, wording};
    use crate::git::GitError;

    #[test]
    fn a_missing_git_says_how_to_install_it_and_names_the_tab() {
        let changes = wording(GitTab::Changes, &GitError::GitMissing).expect("a problem");
        assert_eq!(changes.headline, "Changes need git, and it isn't installed.");
        assert_eq!(changes.hint, Some("Install the Xcode Command Line Tools, then retry."));
        assert_eq!((changes.detail.as_str(), changes.copy), ("xcode-select --install", None));
        let history = wording(GitTab::History, &GitError::GitMissing).expect("a problem");
        assert_eq!(history.headline, "History needs git, and it isn't installed.");
    }

    #[test]
    fn a_failure_shows_gits_first_line_and_copies_all_of_it() {
        let message = "fatal: bad object HEAD\nerror: could not read the index";
        let failed = wording(GitTab::Changes, &GitError::Failed(message.into()));
        assert_eq!(
            failed,
            Some(Wording {
                headline: "Couldn't read this repository.",
                hint: None,
                detail: "fatal: bad object HEAD".into(),
                copy: Some(message.into()),
            })
        );
        let timed_out = wording(GitTab::History, &GitError::TimedOut(std::time::Duration::from_secs(30)));
        assert_eq!(timed_out.map(|w| w.detail), Some("git did not finish within 30s".into()));
    }

    #[test]
    fn a_folder_git_does_not_trust_gets_the_command_that_trusts_it() {
        let message = "fatal: detected dubious ownership in repository at '/Volumes/work/app'\n\
                       To add an exception for this directory, call:";
        let shown = wording(GitTab::Changes, &GitError::Failed(message.into())).expect("a problem");
        assert_eq!(shown.headline, "git doesn't trust who owns this folder.");
        assert_eq!(shown.detail, "git config --global --add safe.directory '/Volumes/work/app'");
        assert_eq!(shown.copy.as_deref(), Some(message));
        // A name that would break out of the quotes is not put in a command.
        let hostile = "fatal: detected dubious ownership in repository at '/x'; touch y; echo ''";
        let shown = wording(GitTab::Changes, &GitError::Failed(hostile.into())).expect("a problem");
        assert_eq!(shown.headline, "Couldn't read this repository.");
    }

    #[test]
    fn not_a_repository_is_the_plain_note() {
        assert_eq!(wording(GitTab::Changes, &GitError::NotARepo), None);
    }
}
