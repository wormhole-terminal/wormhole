//! The offer on the look page: the terminal the person used before, found on this machine, whose
//! settings the app can bring over (`WelcomeOp::Import`). The program finds it itself, in the shell's
//! own environment (`import::Env::from_process`), because only the app can apply it but only this
//! process knows which `HOME` and `XDG_CONFIG_HOME` the person's terminals were set up under. What
//! it reads is only counted; the values that come over are the app's own second reading of that one
//! terminal.

use crate::import::{self, Env, Source};

/// A terminal with settings to bring over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ImportOffer {
    pub(super) source: Source,
}

/// The offer for the terminal whose settings were changed last, among those with something ticked by
/// default (the same pick Settings → Import and the intro window's step make), or `None`.
pub(super) fn find(env: &Env) -> Option<ImportOffer> {
    import::scan(env).into_iter().find_map(|found| {
        // No count is promised: only the app knows which of iTerm2's fonts are installed.
        found.proposal.selected().next().is_some().then_some(ImportOffer { source: found.source })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch home, removed at the end of the test.
    struct Home(std::path::PathBuf);

    impl Home {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("wormhole-offer-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("scratch home");
            Self(dir)
        }

        fn file(&self, relative: &str, text: &str) {
            let path = self.0.join(relative);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("folder");
            std::fs::write(path, text).expect("file");
        }
    }

    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn nothing_is_offered_from_an_empty_home() {
        let home = Home::new("empty");
        assert_eq!(find(&Env::new(home.0.clone())), None);
    }

    #[test]
    fn a_terminal_with_settings_is_offered() {
        let home = Home::new("ghostty");
        home.file(".config/ghostty/config", "font-family = JetBrains Mono\nfont-size = 13\n");
        let offer = find(&Env::new(home.0.clone())).expect("an offer");
        assert_eq!(offer.source, Source::Ghostty);
    }

    #[test]
    fn a_terminal_with_nothing_to_bring_over_is_passed_by() {
        let home = Home::new("nothing");
        // Read, but no setting Wormhole has.
        home.file(".config/ghostty/config", "cursor-style = bar\n");
        assert_eq!(find(&Env::new(home.0.clone())), None);
    }
}
