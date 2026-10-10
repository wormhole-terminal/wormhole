//! Wormhole as the Mac's default terminal: Settings → Terminal makes it the app that runs a
//! double-clicked `.command` or program (Launch Services, by content type), and the files and
//! folders macOS asks it to open each get a new terminal in the window in front. Which types, and
//! what a terminal does with each, are core's `default_terminal`.

use gpui_kit::*;
use wormhole_core::default_terminal::{Opened, SCRIPT_TYPES, file_path};

use super::app::AppState;

/// Hear what macOS asks to open, from the launch on: the requests wait in the channel until
/// [`serve`] takes them, as a launch to open a file sends it before the windows are there.
pub(crate) fn listen(application: &Application) -> smol::channel::Receiver<Vec<String>> {
    let (opened, requests) = smol::channel::unbounded();
    application.on_open_urls(move |urls| {
        let _ = opened.try_send(urls);
    });
    requests
}

/// Open each request in a new terminal, once the windows are up.
pub(crate) fn serve(requests: smol::channel::Receiver<Vec<String>>, cx: &mut App) {
    cx.spawn(async move |cx| {
        while let Ok(urls) = requests.recv().await {
            cx.update(|cx| open_all(&urls, cx));
        }
    })
    .detach();
}

fn open_all(urls: &[String], cx: &mut App) {
    for url in urls {
        let Some(path) = file_path(url) else {
            tracing::warn!(url, "asked to open something that isn't a local file");
            continue;
        };
        let Some(opened) = path.metadata().ok().and_then(|meta| Opened::of(&path, meta.is_dir())) else {
            tracing::warn!(path = %path.display(), "asked to open a file that can't be read");
            continue;
        };
        open(&opened, cx);
    }
}

/// A new terminal for `opened` in the window in front, which comes forward.
fn open(opened: &Opened, cx: &mut App) {
    let Some(target) = AppState::front_window(cx) else {
        tracing::warn!("asked to open {} with no window open", opened.folder().display());
        return;
    };
    let folder = opened.folder().to_str().map(str::to_string);
    let reached = AppState::with_workspace(&target, cx, |ws, window, cx| {
        ws.control_new_terminal(folder, opened.line(), window, cx);
        crate::background::activate_window(window, cx);
    });
    if reached.is_none() {
        tracing::warn!("the window for {} could not be reached", opened.folder().display());
        return;
    }
    crate::background::activate(cx);
    tracing::info!(folder = %opened.folder().display(), script = opened.line().is_some(), "opened from the system");
}

/// Whether Wormhole runs scripts and programs when they are opened. `None` when this build can't
/// be the default: it isn't running from `Wormhole.app` (a `cargo run` build).
pub(crate) fn is_default() -> Option<bool> {
    let bundle = system::own_bundle()?;
    Some(SCRIPT_TYPES.iter().all(|kind| {
        system::default_shell_for(kind).is_some_and(|handler| handler.eq_ignore_ascii_case(&bundle))
    }))
}

/// Make Wormhole the app that runs scripts and programs when they are opened.
pub(crate) fn make_default() -> anyhow::Result<()> {
    let bundle = system::own_bundle().ok_or_else(|| anyhow::anyhow!("not running from Wormhole.app"))?;
    for kind in SCRIPT_TYPES {
        system::set_default_shell_for(kind, &bundle)?;
    }
    tracing::info!(bundle, "Wormhole is now the default terminal");
    Ok(())
}

/// The bundle identifier versions up to 0.2 ran under, before the rename.
const LEGACY_BUNDLE_ID: &str = "dev.hyperficial.andromeda";

/// When an earlier version was the default terminal, the renamed app takes the role over: macOS
/// keeps the choice by bundle identifier, so it would otherwise still name the app that is gone,
/// and a double-clicked `.command` would no longer open here.
pub(crate) fn upgrade_legacy_default() {
    let held = SCRIPT_TYPES.iter().any(|kind| {
        system::default_shell_for(kind).is_some_and(|handler| handler.eq_ignore_ascii_case(LEGACY_BUNDLE_ID))
    });
    if !held || system::own_bundle().is_none() {
        return;
    }
    if let Err(err) = make_default() {
        tracing::warn!("couldn't take over the default terminal from the earlier version: {err:#}");
    }
}

#[cfg(target_os = "macos")]
mod system {
    use std::ptr::NonNull;

    use objc2_core_foundation::{CFRetained, CFString};
    use objc2_foundation::NSBundle;

    /// `kLSRolesShell`: the app that runs a file, as Terminal runs a `.command`.
    const ROLES_SHELL: u32 = 0x0000_0008;

    pub(super) fn own_bundle() -> Option<String> {
        NSBundle::mainBundle().bundleIdentifier().map(|id| id.to_string())
    }

    /// The bundle identifier of the app that runs files of `kind`.
    #[allow(unsafe_code)] // Launch Services' C call; see the SAFETY note.
    pub(super) fn default_shell_for(kind: &str) -> Option<String> {
        #[link(name = "CoreServices", kind = "framework")]
        unsafe extern "C" {
            fn LSCopyDefaultRoleHandlerForContentType(
                content_type: &CFString,
                role: u32,
            ) -> Option<NonNull<CFString>>;
        }
        let kind = CFString::from_str(kind);
        // SAFETY: the content type is a valid CFString for the call; the handler it returns
        // follows the Copy rule (owned by us), so `from_raw` takes that one reference over.
        let handler =
            unsafe { CFRetained::from_raw(LSCopyDefaultRoleHandlerForContentType(&kind, ROLES_SHELL)?) };
        Some(handler.to_string())
    }

    /// Make the app with `bundle` the one that runs files of `kind`.
    #[allow(unsafe_code)] // Launch Services' C call; see the SAFETY note.
    pub(super) fn set_default_shell_for(kind: &str, bundle: &str) -> anyhow::Result<()> {
        #[link(name = "CoreServices", kind = "framework")]
        unsafe extern "C" {
            fn LSSetDefaultRoleHandlerForContentType(
                content_type: &CFString,
                role: u32,
                handler: &CFString,
            ) -> i32;
        }
        let (kind_cf, bundle_cf) = (CFString::from_str(kind), CFString::from_str(bundle));
        // SAFETY: both are valid CFStrings for the call, which only reads them.
        let status = unsafe { LSSetDefaultRoleHandlerForContentType(&kind_cf, ROLES_SHELL, &bundle_cf) };
        anyhow::ensure!(status == 0, "Launch Services refused {kind} (OSStatus {status})");
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
mod system {
    pub(super) fn own_bundle() -> Option<String> {
        None
    }

    pub(super) fn default_shell_for(_: &str) -> Option<String> {
        None
    }

    pub(super) fn set_default_shell_for(_: &str, _: &str) -> anyhow::Result<()> {
        anyhow::bail!("only macOS has a default terminal")
    }
}
