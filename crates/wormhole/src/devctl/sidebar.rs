//! The terminal list: groups, pins, holding ⌘, its scroll.

use anyhow::{Result, anyhow};
use gpui_kit::*;

use super::sessions::expand_home;
use super::{Ctx, coordinates, nth};

/// Fold the group called `label`, or open it.
pub(super) fn fold(ctx: &mut Ctx, arg: &str) -> Result<String> {
    if ctx.workspace.devctl_fold(arg.trim(), ctx.cx) {
        Ok(String::new())
    } else {
        Err(anyhow!("no group {:?}", arg.trim()))
    }
}

/// Pin the `n`th terminal (as `state` numbers them), or unpin it.
pub(super) fn pin(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.toggle_pin(nth(arg), ctx.cx);
    Ok(String::new())
}

/// `group-new <name> <row>…`: a new group of the terminals in those rows (as `state` numbers them).
/// Prints its id.
pub(super) fn new_group(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut words = arg.split_whitespace();
    let name = words.next().ok_or_else(|| anyhow!("expected a name and rows"))?;
    let rows: Vec<usize> = words.map(nth).collect();
    let group = ctx.workspace.devctl_new_group(name, &rows, ctx.cx);
    Ok(format!("group {group}"))
}

/// `folder-color [path] <name|#rrggbb|none>`: colour the folder (the active terminal's without a path;
/// `~/` is the home folder) or take its colour away, as the band's menu does. Prints which.
pub(super) fn folder_color(ctx: &mut Ctx, arg: &str) -> Result<String> {
    use wormhole_core::folder_color::FolderColor;
    use wormhole_core::project::ProjectColorChoice;
    let (path, name) = match arg.trim().rsplit_once(char::is_whitespace) {
        Some((path, name)) => (Some(path.trim()), name),
        None => (None, arg.trim()),
    };
    let color = match name {
        "none" => None,
        name => Some(ProjectColorChoice::parse(name).map_err(|_| {
            let known: Vec<&str> = FolderColor::ALL.iter().map(|color| color.key()).collect();
            anyhow!("expected none, a #rrggbb colour or one of {}, got {name:?}", known.join(", "))
        })?),
    };
    let path = path.map(|path| std::path::PathBuf::from(expand_home(path)));
    if let Some(path) = path.as_deref().filter(|path| !path.is_absolute()) {
        return Err(anyhow!("expected an absolute path, got {:?}", path.display()));
    }
    let folder = ctx.workspace.devctl_folder_color(path.as_deref(), color, ctx.cx).map_err(|e| anyhow!(e))?;
    Ok(format!("{} {}", color.map_or("no colour".into(), ProjectColorChoice::to_value), folder.display()))
}

pub(super) fn scroll(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.devctl_sidebar_scroll(arg.trim().parse().unwrap_or(0.0), ctx.cx);
    Ok(String::new())
}

/// Press ⌘ alone (and keep it held) or let go of it, as the keyboard would.
pub(super) fn hold_cmd(ctx: &mut Ctx, held: bool) -> Result<String> {
    let modifiers = if held { Modifiers::command() } else { Modifiers::default() };
    let event =
        PlatformInput::ModifiersChanged(ModifiersChangedEvent { modifiers, capslock: Capslock::default() });
    // Deferred like the clicks: the workspace's listener needs the workspace itself.
    ctx.window.defer(ctx.cx, move |window, cx| {
        window.dispatch_event(event, cx);
    });
    Ok(String::new())
}

/// A right-click (a row's context menu).
pub(super) fn right_click(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let (position, modifiers, button) = (coordinates(arg)?, Modifiers::default(), MouseButton::Right);
    ctx.window.defer(ctx.cx, move |window, cx| {
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent { position, pressed_button: None, modifiers }),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                button,
                position,
                modifiers,
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent { button, position, modifiers, click_count: 1 }),
            cx,
        );
    });
    Ok(String::new())
}
