//! Which app the system runs a kind of file with (as Terminal runs a `.command`), and making
//! Wormhole that app for the kinds core's `default_terminal` names. What a terminal does with a
//! file it is asked to open is the app's (`workspace::default_terminal`).

use wormhole_core::terminal::default_terminal::SCRIPT_TYPES;

use super::os::default_app as os;

/// The bundle identifier this build runs under: `None` outside an app bundle (a `cargo run`
/// build), which can't be anyone's default.
pub fn own_bundle() -> Option<String> {
    os::own_bundle()
}

/// The bundle identifier of the app that runs files of `kind` (a content type), if any.
pub fn runner_for(kind: &str) -> Option<String> {
    os::runner_for(kind)
}

/// Make the app with `bundle` the one that runs files of `kind`.
pub fn set_runner_for(kind: &str, bundle: &str) -> anyhow::Result<()> {
    os::set_runner_for(kind, bundle)
}

/// Whether Wormhole runs scripts and programs when they are opened. `None` when this build can't
/// be the default: it isn't running from `Wormhole.app` (a `cargo run` build).
pub fn is_default() -> Option<bool> {
    let bundle = own_bundle()?;
    Some(
        SCRIPT_TYPES
            .iter()
            .all(|kind| runner_for(kind).is_some_and(|handler| handler.eq_ignore_ascii_case(&bundle))),
    )
}

/// Make Wormhole the app that runs scripts and programs when they are opened.
pub fn make_default() -> anyhow::Result<()> {
    let bundle = own_bundle().ok_or_else(|| anyhow::anyhow!("not running from Wormhole.app"))?;
    for kind in SCRIPT_TYPES {
        set_runner_for(kind, &bundle)?;
    }
    tracing::info!(bundle, "Wormhole is now the default terminal");
    Ok(())
}

/// The bundle identifier versions up to 0.2 ran under, before the rename.
const LEGACY_BUNDLE_ID: &str = "dev.hyperficial.andromeda";

/// When an earlier version was the default terminal, the renamed app takes the role over: macOS
/// keeps the choice by bundle identifier, so it would otherwise still name the app that is gone,
/// and a double-clicked `.command` would no longer open here.
pub fn upgrade_legacy_default() {
    let held = SCRIPT_TYPES
        .iter()
        .any(|kind| runner_for(kind).is_some_and(|handler| handler.eq_ignore_ascii_case(LEGACY_BUNDLE_ID)));
    if !held || own_bundle().is_none() {
        return;
    }
    if let Err(err) = make_default() {
        tracing::warn!("couldn't take over the default terminal from the earlier version: {err:#}");
    }
}
