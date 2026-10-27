//! The settings window, and settings themselves.

use std::path::PathBuf;

use anyhow::{Result, anyhow};

use wormhole_core::storage::files::{self, LinkPolicy};
use wormhole_core::theme::tokens::Appearance;
use wormhole_term::scrollbar::ScrollerStyle;

use super::Ctx;
use crate::platform::scroller::ScrollerPrefs;
use crate::settings::SettingsStore;

pub(super) fn open(ctx: &mut Ctx, _: &str) -> Result<String> {
    crate::settings_window::devctl::section("", ctx.cx);
    Ok(String::new())
}

pub(super) fn section(ctx: &mut Ctx, arg: &str) -> Result<String> {
    crate::settings_window::devctl::section(arg.trim(), ctx.cx);
    Ok(String::new())
}

/// `update-installed <version> [pending]`: Settings → Updates as it is once an update is installed
/// (its Restart), with "Restart When Idle" chosen when `pending` is given.
pub(super) fn update_installed(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut words = arg.split_whitespace();
    let version = words.next().ok_or_else(|| anyhow!("expected a version, e.g. 0.3.1"))?;
    crate::update::devctl_installed(version, words.next() == Some("pending"), ctx.cx);
    Ok(String::new())
}

pub(super) fn project(ctx: &mut Ctx, arg: &str) -> Result<String> {
    crate::settings_window::devctl::project(arg.trim(), ctx.cx);
    Ok(String::new())
}

pub(super) fn project_menu(ctx: &mut Ctx, arg: &str) -> Result<String> {
    crate::settings_window::devctl::project_menu(arg.trim(), ctx.cx);
    Ok(String::new())
}

pub(super) fn search(ctx: &mut Ctx, arg: &str) -> Result<String> {
    crate::settings_window::devctl::search(arg.trim(), ctx.cx);
    Ok(String::new())
}

pub(super) fn preview(ctx: &mut Ctx, arg: &str) -> Result<String> {
    crate::settings_window::devctl::preview(arg, ctx.cx);
    Ok(String::new())
}

pub(super) fn record(ctx: &mut Ctx, arg: &str) -> Result<String> {
    crate::settings_window::devctl::record(arg, ctx.cx);
    Ok(String::new())
}

/// `settings-reveal <id>`: the settings window at that row, as the palette opens it.
pub(super) fn reveal(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let id = arg.trim();
    if wormhole_core::config::settings_index::entry(id).is_none() {
        return Err(anyhow!("no setting {id:?}"));
    }
    crate::settings_window::devctl::reveal(id, ctx.cx);
    Ok(String::new())
}

/// `import-scan [<fixture home>]`: Settings → Import over a folder standing in for the home.
pub(super) fn import_scan(ctx: &mut Ctx, arg: &str) -> Result<String> {
    // `~/…` is the instance's own home, so a scenario can name its fixture without the scratch path.
    let home = (!arg.trim().is_empty()).then(|| match arg.trim().strip_prefix("~/") {
        Some(rest) => wormhole_core::platform::paths::home().unwrap_or_default().join(rest),
        None => PathBuf::from(arg.trim()),
    });
    if let Some(home) = &home
        && !home.is_dir()
    {
        return Err(anyhow!("no folder {}", home.display()));
    }
    crate::settings_window::devctl::import_scan(home, ctx.cx);
    Ok(String::new())
}

/// `import-open <source>`: open that terminal's card (`ghostty`, `iterm2`, …).
pub(super) fn import_open(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let source = wormhole_core::import::Source::from_key(arg.trim())
        .ok_or_else(|| anyhow!("no terminal {:?}", arg.trim()))?;
    crate::settings_window::devctl::import_open(source, ctx.cx);
    Ok(String::new())
}

/// `import-apply <source>`: press "Import selected" for that terminal (`ghostty`, `iterm2`, …).
pub(super) fn import_apply(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let source = wormhole_core::import::Source::from_key(arg.trim())
        .ok_or_else(|| anyhow!("no terminal {:?}", arg.trim()))?;
    crate::settings_window::devctl::import_apply(source, ctx.cx);
    Ok(String::new())
}

/// `profile-sheet`: the "New profile" sheet over the settings window.
pub(super) fn profile_sheet(ctx: &mut Ctx, _: &str) -> Result<String> {
    crate::settings_window::devctl::profile_sheet(ctx.cx);
    Ok(String::new())
}

pub(super) fn scroll(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let by: f32 = arg.trim().parse().map_err(|_| anyhow!("expected <px>"))?;
    crate::settings_window::devctl::scroll(by, ctx.cx);
    Ok(String::new())
}

pub(super) fn resize(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut dims = arg.split_whitespace().filter_map(|n| n.parse::<f32>().ok());
    let (Some(w), Some(h)) = (dims.next(), dims.next()) else {
        return Err(anyhow!("expected <w> <h>"));
    };
    crate::settings_window::devctl::resize(w, h, ctx.cx);
    Ok(String::new())
}

pub(super) fn select(ctx: &mut Ctx, arg: &str) -> Result<String> {
    crate::settings_window::devctl::select(arg.trim(), ctx.cx);
    Ok(String::new())
}

pub(super) fn reset_confirm(ctx: &mut Ctx, _: &str) -> Result<String> {
    crate::settings_window::devctl::reset_confirm(ctx.cx);
    Ok(String::new())
}

pub(super) fn cli_tool(ctx: &mut Ctx, arg: &str) -> Result<String> {
    crate::settings_window::devctl::cli_tool(arg, ctx.cx);
    Ok(String::new())
}

pub(super) fn font_picker(ctx: &mut Ctx, arg: &str) -> Result<String> {
    crate::settings_window::devctl::font_picker(arg, ctx.cx);
    Ok(String::new())
}

/// `set key=value` (a JSON value): edits config.json the way a user would.
pub(super) fn set(_: &mut Ctx, arg: &str) -> Result<String> {
    let Some((key, value)) = arg.split_once('=') else { return Ok(String::new()) };
    let path: PathBuf = SettingsStore::config_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut config: serde_json::Map<String, serde_json::Value> =
        wormhole_core::storage::jsonc::parse(&text).unwrap_or_default();
    let value = serde_json::from_str(value.trim())
        .unwrap_or_else(|_| serde_json::Value::String(value.trim().to_string()));
    config.insert(key.trim().to_string(), value);
    let json = serde_json::to_string_pretty(&config).unwrap_or_default();
    let _ = files::write_atomic(&path, (json + "\n").as_bytes(), LinkPolicy::Follow);
    Ok(String::new())
}

/// `reduce-motion on|off|system`: stand in for the system's Reduce Motion (animations hold
/// still: the spinner's three dots, the steady halo, cuts instead of slides), or follow the
/// system again. Without an argument, what the app goes by now.
pub(super) fn reduce_motion(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let value = match arg.trim() {
        "" => return Ok(format!("reduce-motion {}\n", on_off(ctx.cx.reduce_motion()))),
        other => switch(other)?,
    };
    crate::app_state::look::override_look(ctx.cx, |look| look.reduce_motion = value);
    Ok(String::new())
}

/// `increase-contrast on|off|system`: stand in for the system's Increase Contrast (the theme's
/// high-contrast variant, every window opaque, a 3 pt focus ring), or follow the system
/// again. Without an argument, what the app goes by now.
pub(super) fn increase_contrast(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let value = match arg.trim() {
        "" => {
            let on = ctx.cx.global::<SettingsStore>().system().increase_contrast;
            return Ok(format!("increase-contrast {}\n", on_off(on)));
        }
        other => switch(other)?,
    };
    crate::app_state::look::override_look(ctx.cx, |look| look.increase_contrast = value);
    Ok(String::new())
}

/// `on`, `off` or `system` (follow the system: `None`).
fn switch(arg: &str) -> Result<Option<bool>> {
    match arg {
        "on" => Ok(Some(true)),
        "off" => Ok(Some(false)),
        "system" => Ok(None),
        other => Err(anyhow!("expected on|off|system, got {other:?}")),
    }
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

/// `scroller overlay|legacy|system`: stand in for the system's "Show scroll bars" (overlay: thin
/// thumbs that fade; legacy: a gutter that is always there), or follow the system again.
pub(super) fn scroller(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let style = match arg.trim() {
        "overlay" => Some(ScrollerStyle::Overlay),
        "legacy" => Some(ScrollerStyle::Legacy),
        "system" => None,
        other => return Err(anyhow!("expected overlay|legacy|system, got {other:?}")),
    };
    ScrollerPrefs::stand_in(ctx.cx, style);
    Ok(String::new())
}

/// `appearance light|dark|system`: stand in for the system's Light or Dark Mode (what Match
/// system follows), or follow the system again. Without an argument, what the app goes by now.
pub(super) fn appearance(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let value = match arg.trim() {
        "light" => Some(Appearance::Light),
        "dark" => Some(Appearance::Dark),
        "system" => None,
        "" => {
            let now = ctx.cx.global::<SettingsStore>().system().appearance;
            return Ok(format!("appearance {}\n", if now == Appearance::Light { "light" } else { "dark" }));
        }
        other => return Err(anyhow!("expected light|dark|system, got {other:?}")),
    };
    crate::app_state::look::override_look(ctx.cx, |look| look.appearance = value);
    Ok(String::new())
}

/// `zen [on|off|toggle]`: Zen mode, as View → Zen Mode turns it on and off (nothing is saved). Without
/// an argument, whether it is on.
pub(super) fn zen(ctx: &mut Ctx, arg: &str) -> Result<String> {
    match arg.trim() {
        "" => {}
        "on" => crate::settings::set_zen_mode(ctx.cx, true),
        "off" => crate::settings::set_zen_mode(ctx.cx, false),
        "toggle" => crate::settings::toggle_zen_mode(ctx.cx),
        other => return Err(anyhow!("expected on|off|toggle, got {other:?}")),
    }
    Ok(format!("zen {}\n", on_off(crate::settings::zen(ctx.cx))))
}

/// `reduce-transparency on|off|system`: stand in for the system's Reduce Transparency, or follow
/// the system again. Without an argument, what the app goes by now.
pub(super) fn reduce_transparency(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let value = match arg.trim() {
        "" => {
            let on = ctx.cx.global::<SettingsStore>().system().reduce_transparency;
            return Ok(format!("reduce-transparency {}\n", on_off(on)));
        }
        other => switch(other)?,
    };
    crate::app_state::look::override_look(ctx.cx, |look| look.reduce_transparency = value);
    Ok(String::new())
}

pub(super) fn theme(ctx: &mut Ctx, arg: &str) -> Result<String> {
    SettingsStore::update(ctx.cx, |s| s.theme = arg.to_string());
    ctx.cx.refresh_windows();
    Ok(String::new())
}
