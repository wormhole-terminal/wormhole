//! Bringing settings over from other terminals: where Ghostty, iTerm2, Terminal.app, Alacritty,
//! Kitty, WezTerm, Warp, Zed's terminal and VS Code's terminal keep theirs (`locations`), reading
//! each into a [`Proposal`] (`ghostty`, `kitty`, `alacritty`, `iterm`, `terminal_app`, `warp`, `zed`,
//! `vscode`, `wezterm`), and taking what the person ticks (`apply`).
//!
//! The format of every source and what it can and cannot give is in `repo-docs/docs/importer-formats.md`.
//!
//! Nothing is executed (no Lua, no generated include, no bell command, no trigger), every file is read
//! capped and through [`Reader`](read), includes are depth-limited and cycle-checked, and nothing of the
//! source is written. The real home is reached only through [`Env::from_process`]; tests and devctl
//! build an [`Env`] over a folder of their own. The domain is data and enums: it writes no sentence, the
//! Settings window words each reason and setting. Tier 7: it uses `storage`, `platform::paths`, `theme`,
//! `config` and `terminal::shells`; only `onboarding` is above it.

mod alacritty;
mod apply;
mod archive;
mod entries;
mod env;
mod ghostty;
mod iterm;
mod json_value;
mod kitty;
mod locations;
mod plist_value;
mod proposal;
mod read;
mod scan;
mod shell;
mod source;
mod terminal_app;
#[cfg(test)]
mod tests;
mod toml_value;
mod units;
mod vscode;
mod warp;
mod warp_theme;
mod wezterm;
mod zed;

pub use self::apply::{SavedThemes, apply, save_themes};
pub use self::env::Env;
pub use self::proposal::{FontName, Proposal, Proposed, Reason, Setting, SettingValue, Skipped, SkippedKind};
pub use self::scan::{Found, scan, scan_one};
pub use self::source::Source;
