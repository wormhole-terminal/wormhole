//! Where each terminal keeps its settings on a Mac, in the order it looks itself. Only files that
//! exist are named; what is in them is for the parsers.

use std::path::{Path, PathBuf};

use super::env::Env;
use super::source::Source;

/// The settings files of `source` that exist, in the order they take effect or are looked for
/// (see each function for what that order means).
pub(crate) fn find(source: Source, env: &Env) -> Vec<PathBuf> {
    match source {
        Source::Ghostty => ghostty(env),
        Source::ITerm2 => existing(&[iterm(env)]),
        Source::TerminalApp => existing(&[terminal_app(env)]),
        Source::Alacritty => first(alacritty_candidates(env)),
        Source::Kitty => first(kitty_candidates(env)),
        Source::WezTerm => first(wezterm_candidates(env)),
        Source::Warp => first(vec![warp(env, ".warp"), warp(env, ".warp-preview")]),
        Source::Zed => existing(&[zed(env)]),
        Source::VsCode => first(vsc_candidates(env)),
    }
}

fn existing(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths.iter().filter(|path| path.is_file()).cloned().collect()
}

/// The first of `paths` that exists, as a list of one or none.
fn first(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.into_iter().find(|path| path.is_file()).into_iter().collect()
}

/// Ghostty loads all four, the XDG ones first and each name's older spelling first; a later one
/// overrides an earlier.
fn ghostty(env: &Env) -> Vec<PathBuf> {
    let xdg = env.config_home().join("ghostty");
    let support = env.app_support().join("com.mitchellh.ghostty");
    existing(&[
        xdg.join("config"),
        xdg.join("config.ghostty"),
        support.join("config"),
        support.join("config.ghostty"),
    ])
}

/// Ghostty's folder of the person's own themes: only the XDG one.
pub(crate) fn ghostty_theme_dirs(env: &Env) -> Vec<PathBuf> {
    let mut dirs = vec![env.config_home().join("ghostty").join("themes")];
    dirs.extend(
        env.applications.iter().map(|apps| apps.join("Ghostty.app/Contents/Resources/ghostty/themes")),
    );
    dirs
}

pub(crate) fn iterm(env: &Env) -> PathBuf {
    env.home.join("Library/Preferences/com.googlecode.iterm2.plist")
}

/// iTerm2's Dynamic Profiles folder.
pub(crate) fn iterm_dynamic_profiles(env: &Env) -> PathBuf {
    env.app_support().join("iTerm2/DynamicProfiles")
}

fn terminal_app(env: &Env) -> PathBuf {
    env.home.join("Library/Preferences/com.apple.Terminal.plist")
}

/// Alacritty uses the first file that exists; there is no Mac-specific place. The TOML names come
/// first, then the YAML ones it replaced in 0.13.
fn alacritty_candidates(env: &Env) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for name in ["alacritty.toml", "alacritty.yml"] {
        if let Some(xdg) = &env.xdg_config_home {
            out.push(xdg.join("alacritty").join(name));
            out.push(xdg.join(name));
        }
        out.push(env.home.join(".config/alacritty").join(name));
        out.push(env.home.join(format!(".{name}")));
        if let Some(etc) = &env.etc {
            out.push(etc.join("alacritty").join(name));
        }
    }
    out
}

/// Kitty's folder is the first of these that holds a `kitty.conf`.
pub(crate) fn kitty_candidates(env: &Env) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    dirs.extend(env.kitty_config_directory.clone());
    dirs.extend(env.xdg_config_home.as_ref().map(|xdg| xdg.join("kitty")));
    dirs.extend(env.xdg_config_dirs.iter().map(|dir| dir.join("kitty")));
    dirs.push(env.home.join(".config/kitty"));
    dirs.push(env.home.join("Library/Preferences/kitty"));
    dirs.into_iter().map(|dir| dir.join("kitty.conf")).collect()
}

/// WezTerm uses the first file that exists; one that exists but fails to load is not skipped for the
/// next, so neither is it here.
fn wezterm_candidates(env: &Env) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    out.extend(env.wezterm_config_file.clone());
    out.extend(env.xdg_config_home.as_ref().map(|xdg| xdg.join("wezterm/wezterm.lua")));
    out.extend(env.xdg_config_dirs.iter().map(|dir| dir.join("wezterm/wezterm.lua")));
    out.push(env.home.join(".config/wezterm/wezterm.lua"));
    out.push(env.home.join(".wezterm.lua"));
    out
}

/// WezTerm's folder of colour scheme files, beside its config.
pub(crate) fn wezterm_colors_dir(config_file: &Path, env: &Env) -> PathBuf {
    let beside = config_file.parent().map(|dir| dir.join("colors"));
    beside.filter(|dir| dir.is_dir()).unwrap_or_else(|| env.home.join(".config/wezterm/colors"))
}

/// Warp's settings file in its stable (`.warp`) or preview folder.
fn warp(env: &Env, folder: &str) -> PathBuf {
    env.home.join(folder).join("settings.toml")
}

fn zed(env: &Env) -> PathBuf {
    // Zed does not read `$XDG_CONFIG_HOME` on a Mac.
    env.home.join(".config/zed/settings.json")
}

/// Zed's folders of theme files: the person's own, and the extensions'.
pub(crate) fn zed_theme_dirs(env: &Env) -> Vec<PathBuf> {
    vec![env.home.join(".config/zed/themes"), env.app_support().join("Zed/extensions/installed")]
}

/// VS Code's user settings, then its Insiders and VSCodium builds, which keep theirs the same way.
fn vsc_candidates(env: &Env) -> Vec<PathBuf> {
    ["Code", "Code - Insiders", "VSCodium"]
        .into_iter()
        .map(|folder| env.app_support().join(folder).join("User/settings.json"))
        .collect()
}
