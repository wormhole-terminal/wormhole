//! The importer's tests: each source over a fixture home in a temporary folder (never the real
//! one), the shell rule, the reader's limits, and applying a proposal.

mod alacritty;
mod apply;
mod ghostty;
mod iterm;
mod kitty;
mod reader;
mod scan;
mod shell;
mod terminal_app;
mod vscode;
mod warp;
mod wezterm;
mod zed;

use std::path::{Path, PathBuf};

use super::{Env, Found, Proposal, Reason, Setting, SettingValue, Source, scan_one};
use crate::config::OptionAsMeta;
use crate::theme::Theme;

/// A home folder in a temporary folder, and the [`Env`] over it. Nothing of the real machine is in
/// it: not the home, not the variables, not the installed apps.
pub(super) struct Fixture {
    root: tempfile::TempDir,
    pub(super) env: Env,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().expect("temp dir");
        let mut env = Env::new(root.path().join("home"));
        env.login_shell = "/bin/zsh".into();
        env.system_shells = vec!["/bin/zsh".into(), "/bin/bash".into()];
        env.applications = vec![root.path().join("Applications")];
        std::fs::create_dir_all(&env.home).expect("home");
        Self { root, env }
    }

    pub(super) fn root(&self) -> &Path {
        self.root.path()
    }

    /// Write a text file under the home folder.
    pub(super) fn home(&self, relative: &str, text: &str) -> PathBuf {
        self.bytes(&self.env.home.join(relative), text.as_bytes())
    }

    /// Write a file anywhere, making its folder.
    pub(super) fn bytes(&self, path: &Path, bytes: &[u8]) -> PathBuf {
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("folder");
        std::fs::write(path, bytes).expect("file");
        path.to_path_buf()
    }

    pub(super) fn found(&self, source: Source) -> Found {
        scan_one(&self.env, source).unwrap_or_else(|| panic!("{} was not found", source.key()))
    }
}

pub(super) fn number(proposal: &Proposal, setting: Setting) -> f32 {
    match &proposal.get(setting).unwrap_or_else(|| panic!("no {setting:?}")).value {
        SettingValue::Number(n) => *n,
        other => panic!("{setting:?} is {other:?}"),
    }
}

pub(super) fn count(proposal: &Proposal, setting: Setting) -> usize {
    match &proposal.get(setting).unwrap_or_else(|| panic!("no {setting:?}")).value {
        SettingValue::Count(n) => *n,
        other => panic!("{setting:?} is {other:?}"),
    }
}

pub(super) fn flag(proposal: &Proposal, setting: Setting) -> bool {
    match &proposal.get(setting).unwrap_or_else(|| panic!("no {setting:?}")).value {
        SettingValue::Flag(on) => *on,
        other => panic!("{setting:?} is {other:?}"),
    }
}

pub(super) fn meta(proposal: &Proposal) -> OptionAsMeta {
    match &proposal.get(Setting::OptionAsMeta).expect("option as meta").value {
        SettingValue::Meta(meta) => *meta,
        other => panic!("option as meta is {other:?}"),
    }
}

/// The font's name and whether it is a PostScript name.
pub(super) fn font(proposal: &Proposal) -> (String, bool) {
    match &proposal.get(Setting::FontFamily).expect("a font").value {
        SettingValue::Font(font) => (font.name.clone(), font.postscript),
        other => panic!("font is {other:?}"),
    }
}

pub(super) fn palette(proposal: &Proposal, setting: Setting) -> &Theme {
    match &proposal.get(setting).unwrap_or_else(|| panic!("no {setting:?}")).value {
        SettingValue::Palette(theme) => theme,
        other => panic!("{setting:?} is {other:?}"),
    }
}

/// Whether the proposal skipped a key that contains `needle`, for this reason.
pub(super) fn skipped(proposal: &Proposal, needle: &str, reason: &Reason) -> bool {
    proposal.skipped.iter().any(|skip| skip.key.contains(needle) && &skip.reason == reason)
}

/// The settings a proposal sets, in order.
pub(super) fn settings(proposal: &Proposal) -> Vec<Setting> {
    proposal.values.iter().map(|proposed| proposed.setting).collect()
}
