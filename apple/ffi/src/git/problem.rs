//! What a tab says when git cannot answer (`wormhole_core::git::problem`): the headline, a hint, what to read or run, and what
//! Copy error copies. The message arrives as the panels have it (`GitBackendError::Failed`); core reads it and words it.

use wormhole_core::git::problem::{GitTab, wording};
use wormhole_core::git::GitError;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitProblemWords {
    pub headline: String,
    pub hint: Option<String>,
    /// Shown in the code font: the command to run, or git's own first line.
    pub detail: String,
    /// The whole message for Copy error; nil when there is nothing more to copy.
    pub copy: Option<String>,
}

#[uniffi::export]
pub fn git_problem(message: String, history: bool) -> GitProblemWords {
    let error = if message == GitError::GitMissing.to_string() { GitError::GitMissing } else { GitError::Failed(message) };
    let tab = if history { GitTab::History } else { GitTab::Changes };
    match wording(tab, &error) {
        Some(words) => GitProblemWords { headline: words.headline.into(), hint: words.hint.map(Into::into), detail: words.detail, copy: words.copy },
        None => GitProblemWords { headline: String::new(), hint: None, detail: String::new(), copy: None },
    }
}

#[cfg(test)]
mod tests {
    use super::git_problem;

    #[test]
    fn core_words_the_three_problems() {
        assert_eq!(git_problem("git is not installed".into(), true).headline, "History needs git, and it isn't installed.");
        let owner = git_problem("fatal: detected dubious ownership in repository at '/srv/repo'\nmore".into(), false);
        assert_eq!(owner.detail, "git config --global --add safe.directory '/srv/repo'");
        let quoted = git_problem("fatal: detected dubious ownership in repository at '/srv/it's'".into(), false);
        assert_eq!(quoted.headline, "Couldn't read this repository.");
    }
}
