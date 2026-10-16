//! The terminal list: grouping, the filter and its popover, pins, holding ⌘, its scroll.

use anyhow::{Result, anyhow};
use gpui_kit::*;
use wormhole_core::sidebar::{Grouping, Show};

use super::sessions::expand_home;
use super::{Ctx, coordinates, nth};

pub(super) fn group(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let grouping = match arg.trim() {
        "none" => Grouping::None,
        "folder" => Grouping::Folder,
        "custom" | "groups" => Grouping::Custom,
        other => return Err(anyhow!("expected none|folder|custom, got {other:?}")),
    };
    ctx.workspace.set_grouping(grouping, ctx.cx);
    Ok(String::new())
}

pub(super) fn show(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let show = match arg.trim() {
        "all" | "" => Show::All,
        "needs_you" => Show::NeedsYou,
        "working" => Show::Working,
        "finished" => Show::Finished,
        "idle" => Show::Idle,
        other => return Err(anyhow!("expected all|needs_you|working|finished|idle, got {other:?}")),
    };
    ctx.workspace.set_show(show, ctx.cx);
    Ok(String::new())
}

pub(super) fn search(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.set_search(arg.trim(), ctx.window, ctx.cx);
    Ok(String::new())
}

pub(super) fn filter_popover(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.devctl_toggle_filter_popover(ctx.cx);
    Ok(String::new())
}

/// Fold the band called `label`, or open it.
pub(super) fn fold(ctx: &mut Ctx, arg: &str) -> Result<String> {
    if ctx.workspace.devctl_fold(arg.trim(), ctx.cx) {
        Ok(String::new())
    } else {
        Err(anyhow!("no band {:?}", arg.trim()))
    }
}

/// Pin the `n`th terminal (as `state` numbers them), or unpin it.
pub(super) fn pin(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.toggle_pin(nth(arg), ctx.cx);
    Ok(String::new())
}

/// `group-new <name> <row>…`: a new group of the terminals in those rows (as `state` numbers them),
/// shown as groups. Prints its id.
pub(super) fn new_group(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut words = arg.split_whitespace();
    let name = words.next().ok_or_else(|| anyhow!("expected a name and rows"))?;
    let rows: Vec<usize> = words.map(nth).collect();
    let group = ctx.workspace.devctl_new_group(name, &rows, ctx.cx);
    Ok(format!("group {group}"))
}

/// Pin the folder `arg` names (`~/` is the home folder), or unpin it: the one list of every window,
/// saved in `config.json`. Prints which it did.
pub(super) fn pin_folder(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let folder = std::path::PathBuf::from(expand_home(arg.trim()));
    if !folder.is_absolute() {
        return Err(anyhow!("expected an absolute path, got {:?}", arg.trim()));
    }
    let pinned = ctx.workspace.devctl_pin_folder(&folder, ctx.cx);
    Ok(format!("{} {}", if pinned { "pinned" } else { "unpinned" }, folder.display()))
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
