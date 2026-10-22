//! The ambient surfaces: the status item's menu and the Dock as text, the
//! notification rules run on demand, and the quick panel.
//!
//! They read every window, and this one is being updated while a command runs: so they run
//! right after this update and append their results to the results file, as `settings-shot`
//! does.

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use gpui_kit::*;
use wormhole_core::notify::Kind;

use super::{Ctx, append, file_name, nth};
use crate::platform::notifications::Permission;
use crate::workspace::ambient::TabFacts;
use crate::workspace::{menu_bar, notifications, quick_panel};

/// Run `f` after this update and append what it returns to this window's results.
pub(super) fn later(ctx: &mut Ctx, f: impl FnOnce(&mut App) -> String + 'static) {
    let out = ctx.out.to_path_buf();
    ctx.cx.defer(move |cx| {
        let text = f(cx);
        append(&out, &text);
    });
}

/// `menubar`: the status item's icon and menu as they would be built now.
pub(super) fn menubar(ctx: &mut Ctx, _: &str) -> Result<String> {
    later(ctx, |cx| menu_bar::dump(cx));
    Ok(String::new())
}

/// `menubar-icons [prefix]`: the status item's icon in every state, as TIFFs in <dir> (status
/// items aren't in GPUI's render; view them with Preview or `sips -s format png`).
pub(super) fn menubar_icons(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let prefix = file_name(arg, "status-item")?.to_string();
    let dir = ctx.dir.to_path_buf();
    later(ctx, move |cx| match menu_bar::save_icons(&dir, &prefix, cx) {
        Ok(paths) => paths.iter().map(|p| format!("saved {}\n", p.display())).collect(),
        Err(err) => format!("menubar-icons failed: {err:#}\n"),
    });
    Ok(String::new())
}

/// `dock`: the Dock's badge, how often it bounced, and its menu.
pub(super) fn dock(ctx: &mut Ctx, _: &str) -> Result<String> {
    later(ctx, |cx| menu_bar::dock_dump(cx));
    Ok(String::new())
}

fn kind(name: &str) -> Result<Kind> {
    Ok(match name {
        "needs-you" => Kind::NeedsYou,
        "agent-done" => Kind::AgentDone,
        "command-done" => Kind::CommandDone,
        "command-failed" => Kind::CommandFailed,
        "agent-failed" => Kind::AgentFailed,
        "bell" => Kind::Bell,
        "notice" => Kind::Notice,
        other => {
            return Err(anyhow!(
                "kind {other:?}: needs-you, agent-done, agent-failed, command-done, command-failed, bell or \
                 notice"
            ));
        }
    })
}

/// The n-th terminal of this window (as `state` numbers them).
fn terminal(ctx: &mut Ctx, n: &str) -> Result<TabFacts> {
    let key = ctx.window.is_window_active();
    let terminals = ctx.workspace.tab_facts(key, ctx.cx);
    terminals.into_iter().nth(nth(n)).ok_or_else(|| anyhow!("no terminal {n:?} in this window"))
}

/// `notify-sim <kind> <n> [post]`: what the rules decide for that event in the n-th terminal
/// (delivery, level, sound, bounce, the words). With `post` it is also delivered, through the
/// route this build has (`WORMHOLE_NOTIFY=log` keeps it from the notification center).
pub(super) fn notify_sim(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut words = arg.split_whitespace();
    let kind = kind(words.next().unwrap_or_default())?;
    let facts = terminal(ctx, words.next().unwrap_or("1"))?;
    let post = words.next() == Some("post");
    later(ctx, move |cx| notifications::simulate(kind, &facts, post, cx));
    Ok(String::new())
}

/// `notify-log`: what the `log` route kept, and the permission as macOS last said.
pub(super) fn notify_log(ctx: &mut Ctx, _: &str) -> Result<String> {
    later(ctx, |cx| {
        let mut out = format!(
            "permission={:?} time_sensitive={}\n",
            notifications::permission(cx),
            notifications::time_sensitive(cx)
        );
        for notice in notifications::logged(cx) {
            out.push_str(&format!(
                "{} [{:?}{}] {:?} / {:?} / {:?} thread={} {:?}\n",
                notice.id,
                notice.level,
                if notice.sound { ", sound" } else { "" },
                notice.title,
                notice.subtitle,
                notice.body,
                notice.thread,
                notice.category,
            ));
        }
        out
    });
    Ok(String::new())
}

/// `notify-permission <not-asked|allowed|denied|outside|checking>`: pretend macOS said so, to see the
/// welcome's (and Settings') page for each. A test instance has no macOS: asking (the welcome's Allow, or
/// a banner turned on) is counted and taken as answered yes.
pub(super) fn notify_permission(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let permission = match arg.trim() {
        "not-asked" => Permission::NotAsked,
        "allowed" => Permission::Allowed { time_sensitive: false },
        "denied" => Permission::Denied,
        "outside" => Permission::OutsideBundle,
        "checking" => Permission::Checking,
        other => {
            return Err(anyhow!("permission {other:?}: not-asked, allowed, denied, outside or checking"));
        }
    };
    later(ctx, move |cx| {
        notifications::pretend_permission(permission, cx);
        format!("permission={permission:?}\n")
    });
    Ok(String::new())
}

/// `quick-panel [close|where]`: open the hotkey's quick panel (or close it); `where` says where it
/// is on its display (600 wide, centred, its top at 20 % of the screen).
pub(super) fn quick_panel(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let arg = arg.trim().to_string();
    later(ctx, move |cx| {
        match (arg.as_str(), quick_panel::is_open(cx)) {
            ("close", true) => quick_panel::close(cx),
            ("where", true) => {
                let placed = panel(cx).and_then(|handle| {
                    handle.update(cx, |_, window, cx| {
                        let display = window.display(cx).map(|d| d.bounds().size);
                        format!("quick panel {:?} on a display of {:?}\n", window.bounds(), display)
                    })
                });
                return placed.unwrap_or_else(|err| format!("quick-panel where failed: {err:#}\n"));
            }
            ("", false) => quick_panel::open(cx),
            _ => {}
        }
        String::new()
    });
    Ok(String::new())
}

/// The quick panel's window, while open.
fn panel(cx: &App) -> Result<AnyWindowHandle> {
    quick_panel::window(cx).ok_or_else(|| anyhow!("the quick panel isn't open"))
}

/// `quick-panel-shot <name>`: the panel drawn to <dir>/<name>.png.
pub(super) fn quick_panel_shot(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let name = file_name(arg, "quick-panel")?;
    let path: PathBuf = ctx.dir.join(format!("{name}.png"));
    later(ctx, move |cx| {
        let shot = panel(cx).and_then(|handle| {
            handle.update(cx, |_, window, cx| {
                window.draw(cx).clear(cx);
                window.render_to_image()
            })?
        });
        match shot.and_then(|image| Ok(image.save(&path)?)) {
            Ok(()) => format!("saved {}\n", path.display()),
            Err(err) => format!("quick-panel-shot failed: {err:#}\n"),
        }
    });
    Ok(String::new())
}

/// `quick-panel-key <keystroke>`: press a key in the panel (`down`, `enter`, `cmd-enter`, `escape`).
pub(super) fn quick_panel_key(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let keystroke = Keystroke::parse(arg.trim())?;
    later(ctx, move |cx| {
        let pressed = panel(cx).and_then(|handle| {
            handle.update(cx, |_, window, cx| {
                let event = KeyDownEvent { keystroke, is_held: false, prefer_character_input: false };
                window.dispatch_event(PlatformInput::KeyDown(event), cx);
            })
        });
        pressed.err().map(|err| format!("quick-panel-key failed: {err:#}\n")).unwrap_or_default()
    });
    Ok(String::new())
}

/// `quick-panel-text <text>`: type into the panel's focused field (search or Reply).
pub(super) fn quick_panel_text(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let keys: Vec<Keystroke> = arg
        .chars()
        .filter_map(|c| Keystroke::parse(&if c == ' ' { "space".into() } else { c.to_string() }).ok())
        .collect();
    later(ctx, move |cx| {
        let typed = panel(cx).and_then(|handle| {
            handle.update(cx, |_, window, cx| {
                for keystroke in keys {
                    window.dispatch_keystroke(keystroke, cx);
                }
            })
        });
        typed.err().map(|err| format!("quick-panel-text failed: {err:#}\n")).unwrap_or_default()
    });
    Ok(String::new())
}

/// `jump`: ⌥⌘J, as the key does it.
pub(super) fn jump(ctx: &mut Ctx, _: &str) -> Result<String> {
    later(ctx, |cx| {
        crate::workspace::ambient::jump_to_waiting(cx);
        String::new()
    });
    Ok(String::new())
}
