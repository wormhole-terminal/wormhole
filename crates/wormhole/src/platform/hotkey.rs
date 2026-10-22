//! System-wide show/hide hotkey.
//!
//! Built on `global-hotkey`, which on macOS calls Carbon `RegisterEventHotKey` with a handler
//! on the application event target. Carbon events are dispatched by whatever runs
//! `NSApplication`'s event loop, so this works with GPUI owning the run loop; no extra thread
//! or event tap is involved. Carbon hotkeys need no Accessibility / Input Monitoring
//! permission. (Only media keys would use a CGEventTap, and we don't accept those.)
//!
//! Presses go to the callback from [`Hotkey::on_press`], invoked on the main thread inside the
//! Carbon callback.

use std::sync::{Mutex, Once};

use global_hotkey::hotkey::HotKey;
pub use global_hotkey::hotkey::{Code, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use wormhole_core::keyspec::{KeySpec, KeySpecError};

type Callback = Box<dyn Fn() + Send + Sync + 'static>;

struct Dispatch {
    ids: Vec<u32>,
    callback: Option<Callback>,
}

static DISPATCH: Mutex<Dispatch> = Mutex::new(Dispatch { ids: Vec::new(), callback: None });
static HANDLER: Once = Once::new();

fn with_dispatch<R>(f: impl FnOnce(&mut Dispatch) -> R) -> R {
    // A panic inside a callback must not disable the hotkey for the rest of the session.
    let mut guard = DISPATCH.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut guard)
}

fn install_handler() {
    // `set_event_handler` can only be set once per process; it replaces the crate's global
    // crossbeam channel, so all events flow through `DISPATCH`.
    HANDLER.call_once(|| {
        GlobalHotKeyEvent::set_event_handler(Some(|event: GlobalHotKeyEvent| {
            if event.state != HotKeyState::Pressed {
                return;
            }
            with_dispatch(|d| {
                if !d.ids.contains(&event.id) {
                    return;
                }
                tracing::debug!(id = event.id, "global hotkey pressed");
                if let Some(callback) = &d.callback {
                    callback();
                }
            });
        }));
    });
}

/// A registered global hotkey. Dropping it (or [`Hotkey::unregister`]) releases the key.
pub struct Hotkey {
    /// `None` in background mode, where nothing is registered.
    held: Option<Held>,
}

struct Held {
    hotkey: HotKey,
    // Dropping the manager unregisters the key and removes its Carbon handler.
    _manager: GlobalHotKeyManager,
}

impl Hotkey {
    /// Registers `spec` (e.g. `"alt-space"`, `"cmd-`"`, `"ctrl-alt-t"`) system-wide.
    /// Must be called on the main thread. Fails if another app already owns the combination.
    /// A test instance in background mode leaves the keys of the person at the Mac alone: it gets
    /// a hotkey that holds nothing, so the app shows the key as it would be.
    pub fn register(spec: &str) -> Result<Self, String> {
        if super::background_mode::on() {
            tracing::info!(spec, "background mode: the global hotkey is not registered");
            return Ok(Self { held: None });
        }
        if !super::os::hotkey::on_registering_thread() {
            return Err("global hotkeys must be registered on the main thread".into());
        }
        let (mods, code) = parse_spec(spec)?;
        let hotkey = HotKey::new(Some(mods), code);
        install_handler();
        let manager = GlobalHotKeyManager::new().map_err(|e| format!("hotkey manager: {e}"))?;
        manager.register(hotkey).map_err(|e| format!("could not register {spec:?}: {e}"))?;
        with_dispatch(|d| d.ids.push(hotkey.id()));
        tracing::info!(spec, "registered global hotkey");
        Ok(Self { held: Some(Held { hotkey, _manager: manager }) })
    }

    /// Calls `f` on the main thread for every press (replacing any previous callback), e.g. to
    /// push into an async channel that a GPUI task awaits. Keep it short and non-blocking.
    pub fn on_press(f: impl Fn() + Send + Sync + 'static) {
        with_dispatch(|d| d.callback = Some(Box::new(f)));
    }
}

impl Drop for Hotkey {
    fn drop(&mut self) {
        let Some(held) = &self.held else { return };
        with_dispatch(|d| {
            if let Some(i) = d.ids.iter().position(|id| *id == held.hotkey.id()) {
                d.ids.swap_remove(i);
            }
        });
        tracing::info!(hotkey = %held.hotkey, "unregistered global hotkey");
    }
}

fn parse_key(name: &str) -> Result<Code, String> {
    let code = name.parse::<HotKey>().map(|h| h.key).map_err(|_| format!("unknown key {name:?}"))?;
    if matches!(code, Code::MediaPlayPause | Code::MediaTrackNext | Code::MediaTrackPrevious)
        || matches!(code, Code::MediaFastForward | Code::MediaRewind | Code::MediaStop)
    {
        // Media keys need a CGEventTap (Accessibility permission); not worth it for show/hide.
        return Err(format!("media key {name:?} is not supported"));
    }
    Ok(code)
}

fn is_function_key(code: Code) -> bool {
    use Code::*;
    matches!(
        code,
        F1 | F2
            | F3
            | F4
            | F5
            | F6
            | F7
            | F8
            | F9
            | F10
            | F11
            | F12
            | F13
            | F14
            | F15
            | F16
            | F17
            | F18
            | F19
            | F20
    )
}

/// Parses `"alt-space"`, `"cmd-`"`, `"ctrl-alt-t"`, `"cmd+shift+space"`, `"cmd--"` (minus) with
/// the shared grammar of [`KeySpec`]. A key without modifiers is only accepted for F-keys, so a
/// stray setting can't swallow normal typing.
pub fn parse_spec(spec: &str) -> Result<(Modifiers, Code), String> {
    let parsed: KeySpec = spec.parse().map_err(|err| match err {
        KeySpecError::Empty => "empty hotkey".to_string(),
        other => other.to_string(),
    })?;
    let mut mods = Modifiers::empty();
    for (on, flag) in [
        (parsed.mods.cmd, Modifiers::SUPER),
        (parsed.mods.alt, Modifiers::ALT),
        (parsed.mods.ctrl, Modifiers::CONTROL),
        (parsed.mods.shift, Modifiers::SHIFT),
    ] {
        if on {
            mods |= flag;
        }
    }
    let code = parse_key(&parsed.key)?;
    if mods.is_empty() && !is_function_key(code) {
        return Err(format!("{:?} needs at least one modifier", spec.trim()));
    }
    Ok((mods, code))
}

/// What already has `spec` ("macOS uses ⌘Space for Spotlight", "⌘T is New Tab", "The shell uses
/// ⌥Space"), if anything: a global hotkey would take it from them (the recorder says so). The
/// key table answers. A spec the hotkey couldn't register anyway (a bare key, nonsense in
/// config.json) has its registration error to show instead.
pub fn system_conflict(spec: &str) -> Option<String> {
    parse_spec(spec).ok()?;
    let spec: KeySpec = spec.parse().ok()?;
    wormhole_core::shortcut_record::conflict(&spec, None)
}

#[cfg(test)]
mod tests {
    use super::{Code, Modifiers, parse_spec, system_conflict};

    #[test]
    fn system_shortcuts_are_named() {
        assert_eq!(system_conflict("cmd-space").as_deref(), Some("macOS uses ⌘Space for Spotlight"));
        assert_eq!(system_conflict("cmd+Space").as_deref(), Some("macOS uses ⌘Space for Spotlight"));
        assert!(system_conflict("ctrl-space").is_some());
        assert_eq!(system_conflict("shift-cmd-space"), None, "the suggested hotkey is free");
        assert_eq!(system_conflict("alt-space").as_deref(), Some("The shell uses ⌥Space"));
        assert_eq!(system_conflict("not a key"), None);
    }

    #[test]
    fn parses_common_specs() {
        assert_eq!(parse_spec("alt-space"), Ok((Modifiers::ALT, Code::Space)));
        assert_eq!(parse_spec("cmd-`"), Ok((Modifiers::SUPER, Code::Backquote)));
        assert_eq!(parse_spec("ctrl-alt-t"), Ok((Modifiers::CONTROL | Modifiers::ALT, Code::KeyT)));
        assert_eq!(parse_spec(" Cmd+Shift+Space "), Ok((Modifiers::SUPER | Modifiers::SHIFT, Code::Space)));
        assert_eq!(parse_spec("option-1"), Ok((Modifiers::ALT, Code::Digit1)));
        assert_eq!(parse_spec("⌃⌥-t"), Err("unknown modifier \"⌃⌥\"".into()));
        assert_eq!(parse_spec("⌃-⌥-t"), Ok((Modifiers::CONTROL | Modifiers::ALT, Code::KeyT)));
        assert_eq!(parse_spec("f12"), Ok((Modifiers::empty(), Code::F12)));
        assert_eq!(parse_spec("ctrl-backtick"), Ok((Modifiers::CONTROL, Code::Backquote)));
    }

    #[test]
    fn separator_as_key() {
        assert_eq!(parse_spec("cmd--"), Ok((Modifiers::SUPER, Code::Minus)));
        assert_eq!(parse_spec("ctrl+-"), Ok((Modifiers::CONTROL, Code::Minus)));
        assert_eq!(parse_spec("ctrl-shift-="), Ok((Modifiers::CONTROL | Modifiers::SHIFT, Code::Equal)));
    }

    #[test]
    fn rejects_bad_specs() {
        assert!(parse_spec("").is_err());
        assert!(parse_spec("space").is_err(), "bare key would eat typing");
        assert!(parse_spec("t").is_err());
        assert!(parse_spec("-").is_err());
        assert!(parse_spec("cmd-").is_err());
        assert!(parse_spec("hyper-t").is_err());
        assert!(parse_spec("cmd-cmd-t").is_err());
        assert!(parse_spec("cmd-nosuchkey").is_err());
        assert!(parse_spec("cmd-t-alt").is_err());
        assert!(parse_spec("alt-mediaplaypause").is_err());
    }
}
